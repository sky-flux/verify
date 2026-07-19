use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use futures::stream::{FuturesUnordered, StreamExt};
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

use super::types::{BatchSummary, VerifyResult};

pub trait ProgressReporter: Send + Sync {
    fn report(&self, completed: u32, total: u32);
}

/// A no-op reporter, useful for tests and for callers that don't care about
/// progress (e.g. programmatic/CLI reuse of this crate).
pub struct NullProgressReporter;
impl ProgressReporter for NullProgressReporter {
    fn report(&self, _completed: u32, _total: u32) {}
}

/// Abstraction over "verify one email address", so `run_batch` can be unit
/// tested without a real network/SMTP stack. The production implementation
/// wires this to dns::lookup_mx + catch_all::is_catch_all + smtp::probe_rcpt
/// + rate_limiter::RateLimiter behind a Tauri command.
#[async_trait]
pub trait EmailProber: Send + Sync {
    async fn probe(&self, email: &str) -> VerifyResult;
}

/// Deduplicates and normalizes a raw list of pasted/imported email strings.
/// Case-insensitive dedup on the whole address, trimming whitespace and
/// dropping empty lines, since that's what users actually paste.
pub fn dedupe_emails(raw: &[String]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();
    for email in raw {
        let trimmed = email.trim();
        if trimmed.is_empty() {
            continue;
        }
        let key = trimmed.to_lowercase();
        if seen.insert(key) {
            result.push(trimmed.to_string());
        }
    }
    result
}

/// Groups emails by their `@domain` suffix, preserving first-seen order of
/// domains. Emails with no '@' are grouped under an empty-string domain key
/// so they still get processed (and will simply fail syntax validation).
pub fn group_by_domain(emails: &[String]) -> Vec<(String, Vec<String>)> {
    let mut order: Vec<String> = Vec::new();
    let mut groups: HashMap<String, Vec<String>> = HashMap::new();
    for email in emails {
        let domain = email
            .rsplit_once('@')
            .map(|(_, d)| d.to_lowercase())
            .unwrap_or_default();
        if !groups.contains_key(&domain) {
            order.push(domain.clone());
        }
        groups.entry(domain).or_default().push(email.clone());
    }
    order
        .into_iter()
        .map(|domain| {
            let emails = groups.remove(&domain).unwrap_or_default();
            (domain, emails)
        })
        .collect()
}

pub struct BatchOptions {
    pub max_concurrent_domains: usize,
}

/// Runs verification across all `emails`, deduped and grouped by domain, with
/// at most `max_concurrent_domains` domains being probed concurrently (each
/// domain's own emails are still probed sequentially, since RCPT probes
/// against the same server should be rate-limited, not parallelized).
/// Reports progress after every completed email and stops early — without
/// probing remaining emails — once `cancel` is triggered.
pub async fn run_batch(
    emails: Vec<String>,
    prober: Arc<dyn EmailProber>,
    reporter: Arc<dyn ProgressReporter>,
    cancel: CancellationToken,
    options: BatchOptions,
) -> (Vec<VerifyResult>, BatchSummary) {
    let deduped = dedupe_emails(&emails);
    let total = deduped.len() as u32;
    let groups = group_by_domain(&deduped);

    let semaphore = Arc::new(Semaphore::new(options.max_concurrent_domains.max(1)));
    let completed = Arc::new(std::sync::atomic::AtomicU32::new(0));

    let mut tasks = FuturesUnordered::new();
    for (_, domain_emails) in groups {
        let semaphore = semaphore.clone();
        let prober = prober.clone();
        let reporter = reporter.clone();
        let cancel = cancel.clone();
        let completed = completed.clone();
        tasks.push(tokio::spawn(async move {
            let _permit = semaphore.acquire_owned().await.unwrap();
            let mut results = Vec::new();
            for email in domain_emails {
                if cancel.is_cancelled() {
                    break;
                }
                let result = prober.probe(&email).await;
                results.push(result);
                let done = completed.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                reporter.report(done, total);
            }
            results
        }));
    }

    let mut all_results = Vec::new();
    while let Some(joined) = tasks.next().await {
        if let Ok(mut results) = joined {
            all_results.append(&mut results);
        }
    }

    let summary = BatchSummary::from_results(&all_results);
    (all_results, summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::types::Verdict;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Mutex;

    fn make_result(email: &str, verdict: Verdict) -> VerifyResult {
        VerifyResult {
            id: "id".into(),
            email: email.into(),
            syntax_valid: true,
            mx_found: true,
            mx_records: vec![],
            catch_all: None,
            smtp_code: Some(250),
            smtp_message: "OK".into(),
            error: None,
            verdict,
            checked_at: "2026-01-01T00:00:00Z".into(),
            duration_ms: 1,
        }
    }

    struct FakeProber;
    #[async_trait]
    impl EmailProber for FakeProber {
        async fn probe(&self, email: &str) -> VerifyResult {
            make_result(email, Verdict::Valid)
        }
    }

    struct CountingReporter {
        calls: Mutex<Vec<(u32, u32)>>,
    }
    impl ProgressReporter for CountingReporter {
        fn report(&self, completed: u32, total: u32) {
            self.calls.lock().unwrap().push((completed, total));
        }
    }

    #[test]
    fn dedupe_emails_removes_case_insensitive_duplicates() {
        let input = vec![
            "User@Example.com".to_string(),
            "user@example.com".to_string(),
            "other@example.com".to_string(),
        ];
        let result = dedupe_emails(&input);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn dedupe_emails_drops_blank_lines_and_trims_whitespace() {
        let input = vec![
            "  a@example.com  ".to_string(),
            "".to_string(),
            "   ".to_string(),
        ];
        let result = dedupe_emails(&input);
        assert_eq!(result, vec!["a@example.com".to_string()]);
    }

    #[test]
    fn group_by_domain_groups_correctly_and_preserves_domain_order() {
        let input = vec![
            "a@x.com".to_string(),
            "b@y.com".to_string(),
            "c@x.com".to_string(),
        ];
        let groups = group_by_domain(&input);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].0, "x.com");
        assert_eq!(
            groups[0].1,
            vec!["a@x.com".to_string(), "c@x.com".to_string()]
        );
        assert_eq!(groups[1].0, "y.com");
    }

    #[tokio::test]
    async fn run_batch_dedupes_and_returns_one_result_per_unique_email() {
        let emails = vec![
            "a@x.com".to_string(),
            "a@x.com".to_string(),
            "b@y.com".to_string(),
        ];
        let (results, summary) = run_batch(
            emails,
            Arc::new(FakeProber),
            Arc::new(NullProgressReporter),
            CancellationToken::new(),
            BatchOptions {
                max_concurrent_domains: 5,
            },
        )
        .await;
        assert_eq!(results.len(), 2);
        assert_eq!(summary.total, 2);
        assert_eq!(summary.valid, 2);
    }

    #[tokio::test]
    async fn run_batch_reports_progress_for_every_completed_email() {
        let emails = vec!["a@x.com".to_string(), "b@y.com".to_string()];
        let reporter = Arc::new(CountingReporter {
            calls: Mutex::new(Vec::new()),
        });
        let (_results, _summary) = run_batch(
            emails,
            Arc::new(FakeProber),
            reporter.clone(),
            CancellationToken::new(),
            BatchOptions {
                max_concurrent_domains: 5,
            },
        )
        .await;
        let calls = reporter.calls.lock().unwrap();
        assert_eq!(calls.len(), 2);
        for (completed, total) in calls.iter() {
            assert_eq!(*total, 2);
            assert!(*completed <= 2);
        }
    }

    #[tokio::test]
    async fn run_batch_stops_probing_a_domain_once_cancelled() {
        struct CountingProber {
            calls: AtomicU32,
        }
        #[async_trait]
        impl EmailProber for CountingProber {
            async fn probe(&self, email: &str) -> VerifyResult {
                self.calls.fetch_add(1, Ordering::SeqCst);
                make_result(email, Verdict::Valid)
            }
        }

        let cancel = CancellationToken::new();
        cancel.cancel();

        let emails = vec!["a@x.com".to_string(), "b@x.com".to_string()];
        let (results, _summary) = run_batch(
            emails,
            Arc::new(CountingProber {
                calls: AtomicU32::new(0),
            }),
            Arc::new(NullProgressReporter),
            cancel,
            BatchOptions {
                max_concurrent_domains: 5,
            },
        )
        .await;
        // Cancelled before any probe started, so nothing should have run.
        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn run_batch_with_empty_input_returns_empty_summary() {
        let (results, summary) = run_batch(
            vec![],
            Arc::new(FakeProber),
            Arc::new(NullProgressReporter),
            CancellationToken::new(),
            BatchOptions {
                max_concurrent_domains: 5,
            },
        )
        .await;
        assert!(results.is_empty());
        assert_eq!(summary.total, 0);
    }
}
