use std::time::Duration;

use sqlx::Row;
use tauri::{AppHandle, State};
use tokio::net::TcpStream;
use tokio::time::timeout;

use crate::domain::types::{DashboardStats, NetworkHealth};
use crate::error::AppError;
use crate::state::AppState;

use super::verify::get_pool;

#[tauri::command]
pub async fn get_dashboard_stats(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<DashboardStats, AppError> {
    let pool = get_pool(&app_handle, &state).await?;

    let total_row = sqlx::query("SELECT COUNT(*) as c FROM verify_results")
        .fetch_one(&pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;
    let total_verified_all_time: i64 = total_row.try_get("c").unwrap_or(0);

    let valid_row = sqlx::query("SELECT COUNT(*) as c FROM verify_results WHERE verdict = 'Valid'")
        .fetch_one(&pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;
    let valid_count: i64 = valid_row.try_get("c").unwrap_or(0);

    let catch_all_row =
        sqlx::query("SELECT COUNT(*) as c FROM catch_all_cache WHERE is_catch_all = 1")
            .fetch_one(&pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
    let catch_all_domain_count: i64 = catch_all_row.try_get("c").unwrap_or(0);

    let today_prefix = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let today_row = sqlx::query("SELECT COUNT(*) as c FROM verify_results WHERE checked_at LIKE ?")
        .bind(format!("{today_prefix}%"))
        .fetch_one(&pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;
    let verified_today: i64 = today_row.try_get("c").unwrap_or(0);

    let overall_valid_rate = if total_verified_all_time > 0 {
        valid_count as f32 / total_verified_all_time as f32
    } else {
        0.0
    };

    Ok(DashboardStats {
        total_verified_all_time: total_verified_all_time as u64,
        overall_valid_rate,
        catch_all_domain_count: catch_all_domain_count as u64,
        verified_today: verified_today as u64,
    })
}

#[tauri::command]
pub async fn check_network_health() -> Result<NetworkHealth, AppError> {
    // Gmail's MX is a stable, near-universally-reachable target for a
    // 25-port connectivity self-check — if this fails, it's almost always
    // the local network/ISP blocking outbound port 25, not Gmail being down.
    let result = timeout(
        Duration::from_secs(5),
        TcpStream::connect(("gmail-smtp-in.l.google.com", 25)),
    )
    .await;

    let (port25_reachable, detail) = match result {
        Ok(Ok(_)) => (true, None),
        Ok(Err(e)) => (false, Some(e.to_string())),
        Err(_) => (false, Some("连接超时".to_string())),
    };

    Ok(NetworkHealth {
        port25_reachable,
        checked_at: chrono::Utc::now().to_rfc3339(),
        detail,
    })
}
