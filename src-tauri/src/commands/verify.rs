use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio_util::sync::CancellationToken;

use crate::domain::batch::{self, BatchOptions, EmailProber, ProgressReporter};
use crate::domain::dns;
use crate::domain::types::{BatchSummary, Verdict, VerifyResult};
use crate::domain::verify::EmailVerifier;
use crate::error::AppError;
use crate::infra::db;
use crate::state::AppState;

/// How long to wait before retrying addresses that came back `Unknown`
/// (greylist/temporary rejection or a transient connection failure) — long
/// enough that a greylisting mail server has likely accepted the retry per
/// its own greylist window, per prompt.md's "等30秒后对这批 unknown 结果重新探测一次".
const GREYLIST_RETRY_DELAY: Duration = Duration::from_secs(30);

pub(crate) async fn get_pool(
    app_handle: &AppHandle,
    state: &State<'_, AppState>,
) -> Result<sqlx::SqlitePool, AppError> {
    if let Some(pool) = state.db_pool.get() {
        return Ok(pool.clone());
    }
    let data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Io(e.to_string()))?;
    std::fs::create_dir_all(&data_dir)?;
    let db_path = data_dir.join("sky-flux-verify.sqlite");
    let url = format!("sqlite://{}?mode=rwc", db_path.to_string_lossy());
    let pool = db::connect(&url).await?;
    let _ = state.db_pool.set(pool.clone());
    Ok(pool)
}

/// Verifies one email, consulting and updating the domain-level catch-all
/// cache (`catch_all_cache` table) so a batch of addresses at the same
/// domain only pays for one catch-all probe instead of one per address —
/// the caching strategy the spec calls for under "域名分组优化".
async fn verify_with_catch_all_cache(
    pool: &sqlx::SqlitePool,
    verifier: &EmailVerifier,
    email: &str,
) -> Result<VerifyResult, AppError> {
    let domain = dns::extract_domain(email).map(|d| d.to_string());
    let cached_catch_all = match &domain {
        Some(d) => db::is_domain_catch_all(pool, d).await?,
        None => None,
    };

    let result = verifier.verify(email, cached_catch_all).await;

    if cached_catch_all.is_none() {
        if let (Some(d), Some(is_catch_all)) = (&domain, result.catch_all) {
            db::upsert_catch_all(pool, d, is_catch_all).await?;
        }
    }

    Ok(result)
}

#[tauri::command]
pub async fn verify_single_email(
    email: String,
    // Set when re-verifying an existing history row (History's "重新验证"):
    // reusing that row's id makes the upsert in `db::insert_result` update
    // it in place instead of inserting a duplicate entry for the same
    // address. Omitted (None) for every fresh verification, which always
    // gets a new id and so always inserts a new row.
    existing_id: Option<String>,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<VerifyResult, AppError> {
    let settings = state.settings.read().await.clone();
    let rate_limiter = state.rate_limiter.read().await.clone();
    let verifier = EmailVerifier {
        rate_limiter,
        settings,
    };

    let pool = get_pool(&app_handle, &state).await?;
    let mut result = verify_with_catch_all_cache(&pool, &verifier, &email).await?;
    if let Some(id) = existing_id {
        result.id = id;
    }
    db::insert_result(&pool, &result).await?;

    Ok(result)
}

struct TauriProgressReporter(AppHandle);
impl ProgressReporter for TauriProgressReporter {
    fn report(&self, completed: u32, total: u32) {
        let _ = self.0.emit("verify-progress", (completed, total));
    }
}

struct DomainProber {
    verifier: Arc<EmailVerifier>,
    pool: sqlx::SqlitePool,
}
#[async_trait]
impl EmailProber for DomainProber {
    async fn probe(&self, email: &str) -> VerifyResult {
        match verify_with_catch_all_cache(&self.pool, &self.verifier, email).await {
            Ok(result) => result,
            Err(e) => VerifyResult {
                id: uuid::Uuid::now_v7().to_string(),
                email: email.to_string(),
                syntax_valid: false,
                mx_found: false,
                mx_records: vec![],
                catch_all: None,
                smtp_code: None,
                smtp_message: String::new(),
                error: Some(e.to_string()),
                verdict: crate::domain::types::Verdict::Unknown,
                checked_at: chrono::Utc::now().to_rfc3339(),
                duration_ms: 0,
            },
        }
    }
}

#[tauri::command]
pub async fn verify_batch_emails(
    emails: Vec<String>,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(Vec<VerifyResult>, BatchSummary), AppError> {
    let settings = state.settings.read().await.clone();
    let rate_limiter = state.rate_limiter.read().await.clone();
    let max_concurrent_domains = settings.max_concurrent_domains as usize;

    let cancel = CancellationToken::new();
    *state.cancel_token.lock().await = Some(cancel.clone());

    let pool = get_pool(&app_handle, &state).await?;
    let verifier = Arc::new(EmailVerifier {
        rate_limiter,
        settings,
    });
    let prober: Arc<dyn EmailProber> = Arc::new(DomainProber {
        verifier,
        pool: pool.clone(),
    });
    let reporter: Arc<dyn ProgressReporter> = Arc::new(TauriProgressReporter(app_handle.clone()));

    let (mut results, _) = batch::run_batch(
        emails,
        prober.clone(),
        reporter.clone(),
        cancel.clone(),
        BatchOptions {
            max_concurrent_domains,
        },
    )
    .await;

    retry_unknown_results_once(&mut results, prober, reporter, &cancel).await;

    *state.cancel_token.lock().await = None;

    for result in &results {
        db::insert_result(&pool, result).await?;
    }

    let summary = BatchSummary::from_results(&results);
    Ok((results, summary))
}

/// One delayed retry pass over every `Unknown` result — greylisting mail
/// servers commonly accept a retry after their own hold window, and a
/// simple connection hiccup may also just clear up. Skips entirely if the
/// user cancelled the batch, and re-checks cancellation after the wait in
/// case they cancelled during it. Retried results replace the originals
/// in-place; anything not retried (or that fails again) is left untouched
/// rather than discarded.
async fn retry_unknown_results_once(
    results: &mut [VerifyResult],
    prober: Arc<dyn EmailProber>,
    reporter: Arc<dyn ProgressReporter>,
    cancel: &CancellationToken,
) {
    let retry_indices: Vec<usize> = results
        .iter()
        .enumerate()
        .filter(|(_, r)| r.verdict == Verdict::Unknown)
        .map(|(i, _)| i)
        .collect();

    if retry_indices.is_empty() || cancel.is_cancelled() {
        return;
    }

    log::info!(
        "retrying {} unknown result(s) after greylist delay",
        retry_indices.len()
    );
    tokio::time::sleep(GREYLIST_RETRY_DELAY).await;

    if cancel.is_cancelled() {
        return;
    }

    let total = retry_indices.len() as u32;
    for (done, &idx) in retry_indices.iter().enumerate() {
        if cancel.is_cancelled() {
            break;
        }
        let email = results[idx].email.clone();
        results[idx] = prober.probe(&email).await;
        reporter.report(done as u32 + 1, total);
    }
}

#[tauri::command]
pub async fn cancel_batch_verification(state: State<'_, AppState>) -> Result<(), AppError> {
    let guard = state.cancel_token.lock().await;
    match guard.as_ref() {
        Some(token) => {
            token.cancel();
            Ok(())
        }
        None => Err(AppError::NoBatchRunning),
    }
}

#[tauri::command]
pub async fn export_results_to_csv(
    results: Vec<VerifyResult>,
    file_path: String,
) -> Result<(), AppError> {
    let csv_text = crate::infra::csv_export::build_csv_export(&results)?;
    std::fs::write(&file_path, csv_text)?;
    Ok(())
}

/// Parses an imported CSV/TXT file's raw text content into a list of email
/// addresses. Per the project's core principle, CSV parsing is Rust's job —
/// the frontend must never do this itself, only pass through the raw file
/// content it read via `tauri-plugin-fs`.
#[tauri::command]
pub async fn parse_imported_emails(content: String) -> Result<Vec<String>, AppError> {
    crate::infra::csv_export::parse_email_list(&content)
}
