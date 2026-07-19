use std::time::{Duration, Instant};

use uuid::Uuid;

use super::catch_all;
use super::dns;
use super::rate_limiter::RateLimiter;
use super::smtp::{self, HandshakeParams};
use super::types::{Settings, Verdict, VerifyResult};

/// Combines the individual signals gathered while verifying one address into
/// a final verdict. Pulled out as a pure function so the decision logic is
/// unit-testable without a network stack: syntax/MX failures short-circuit
/// to Invalid before any SMTP probe would even make sense, and a catch-all
/// domain downgrades an otherwise-Valid SMTP response to RiskyCatchAll since
/// the server accepts RCPT TO for any address there.
pub fn resolve_verdict(
    syntax_valid: bool,
    mx_found: bool,
    is_catch_all: bool,
    smtp_verdict: Verdict,
) -> Verdict {
    if !syntax_valid || !mx_found {
        return Verdict::Invalid;
    }
    if is_catch_all && smtp_verdict == Verdict::Valid {
        return Verdict::RiskyCatchAll;
    }
    smtp_verdict
}

pub struct EmailVerifier {
    pub rate_limiter: std::sync::Arc<RateLimiter>,
    pub settings: Settings,
}

impl EmailVerifier {
    /// Runs the full verification pipeline for one address: syntax check,
    /// MX lookup, domain-level rate limiting, catch-all probe, and the real
    /// RCPT TO probe — never sending DATA, so no mail is ever delivered.
    /// `cached_catch_all` lets callers skip the catch-all probe (and its
    /// rate-limited network round trip) when the domain's status is already
    /// known from a prior probe in the same batch or from persisted cache.
    pub async fn verify(&self, email: &str, cached_catch_all: Option<bool>) -> VerifyResult {
        let start = Instant::now();
        let email = email.trim();
        let syntax_valid = dns::is_syntax_valid(email);

        if !syntax_valid {
            return self.finish(
                email,
                false,
                false,
                vec![],
                None,
                None,
                String::new(),
                None,
                start,
            );
        }

        let domain = dns::extract_domain(email).unwrap_or_default();
        let mx = dns::lookup_mx(domain, self.settings.dns_resolver).await;

        if !mx.mx_found {
            return self.finish(
                email,
                true,
                false,
                vec![],
                None,
                None,
                String::new(),
                None,
                start,
            );
        }

        let host = mx
            .hosts
            .first()
            .cloned()
            .unwrap_or_else(|| domain.to_string());
        let mail_from = format!("verify@{}", self.settings.helo_domain);
        let timeout = Duration::from_secs(self.settings.smtp_timeout_seconds as u64);

        let is_catch_all = match cached_catch_all {
            Some(v) => v,
            None => {
                self.rate_limiter.acquire(domain).await;
                catch_all::is_catch_all(
                    domain,
                    &host,
                    &self.settings.helo_domain,
                    &mail_from,
                    timeout,
                )
                .await
            }
        };

        self.rate_limiter.acquire(domain).await;
        let outcome = smtp::probe_rcpt(HandshakeParams {
            host: &host,
            port: 25,
            helo_domain: &self.settings.helo_domain,
            mail_from: &mail_from,
            rcpt_to: email,
            timeout,
        })
        .await;

        self.finish(
            email,
            true,
            true,
            mx.hosts,
            Some(is_catch_all),
            outcome.code,
            outcome.message,
            outcome.error,
            start,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn finish(
        &self,
        email: &str,
        syntax_valid: bool,
        mx_found: bool,
        mx_records: Vec<String>,
        catch_all: Option<bool>,
        smtp_code: Option<u16>,
        smtp_message: String,
        error: Option<String>,
        start: Instant,
    ) -> VerifyResult {
        let smtp_verdict = match smtp_code {
            Some(code) => smtp::classify_code(code),
            None if mx_found && syntax_valid => Verdict::Unknown,
            None => Verdict::Invalid,
        };
        let verdict = resolve_verdict(
            syntax_valid,
            mx_found,
            catch_all.unwrap_or(false),
            smtp_verdict,
        );
        VerifyResult {
            id: Uuid::now_v7().to_string(),
            email: email.to_string(),
            syntax_valid,
            mx_found,
            mx_records,
            catch_all,
            smtp_code,
            smtp_message,
            error,
            verdict,
            checked_at: chrono::Utc::now().to_rfc3339(),
            duration_ms: start.elapsed().as_millis() as u64,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_syntax_always_yields_invalid_regardless_of_other_signals() {
        assert_eq!(
            resolve_verdict(false, true, true, Verdict::Valid),
            Verdict::Invalid
        );
    }

    #[test]
    fn missing_mx_always_yields_invalid() {
        assert_eq!(
            resolve_verdict(true, false, false, Verdict::Valid),
            Verdict::Invalid
        );
    }

    #[test]
    fn catch_all_domain_with_valid_smtp_response_is_downgraded_to_risky() {
        assert_eq!(
            resolve_verdict(true, true, true, Verdict::Valid),
            Verdict::RiskyCatchAll
        );
    }

    #[test]
    fn catch_all_domain_with_invalid_smtp_response_stays_invalid() {
        // A catch-all flag doesn't matter if the RCPT itself was rejected —
        // that's an unusual but possible server misconfiguration.
        assert_eq!(
            resolve_verdict(true, true, true, Verdict::Invalid),
            Verdict::Invalid
        );
    }

    #[test]
    fn non_catch_all_domain_passes_through_the_smtp_verdict_unchanged() {
        assert_eq!(
            resolve_verdict(true, true, false, Verdict::Valid),
            Verdict::Valid
        );
        assert_eq!(
            resolve_verdict(true, true, false, Verdict::Unknown),
            Verdict::Unknown
        );
    }
}
