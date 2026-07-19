use tauri::{AppHandle, State};

use crate::domain::types::VerifyResult;
use crate::error::AppError;
use crate::infra::db::{self, HistoryQuery};
use crate::state::AppState;

use super::verify::get_pool;

#[tauri::command]
pub async fn fetch_history(
    domain_filter: Option<String>,
    email_search: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<VerifyResult>, AppError> {
    let pool = get_pool(&app_handle, &state).await?;
    let query = HistoryQuery {
        domain_filter,
        email_search,
        limit: limit.unwrap_or(50),
        offset: offset.unwrap_or(0),
    };
    db::fetch_history(&pool, &query).await
}

#[tauri::command]
pub async fn count_history(
    domain_filter: Option<String>,
    email_search: Option<String>,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<i64, AppError> {
    let pool = get_pool(&app_handle, &state).await?;
    let query = HistoryQuery {
        domain_filter,
        email_search,
        ..Default::default()
    };
    db::count_history(&pool, &query).await
}

#[tauri::command]
pub async fn fetch_distinct_domains(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<String>, AppError> {
    let pool = get_pool(&app_handle, &state).await?;
    db::distinct_domains(&pool).await
}
