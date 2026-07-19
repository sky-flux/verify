use tauri::{AppHandle, State};
use tauri_plugin_store::StoreExt;

use crate::domain::types::{validate_settings, Settings};
use crate::error::AppError;
use crate::state::AppState;

const STORE_FILE: &str = "settings.json";
const STORE_KEY: &str = "settings";

#[tauri::command]
pub async fn get_settings(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<Settings, AppError> {
    let store = app_handle
        .store(STORE_FILE)
        .map_err(|e| AppError::Io(e.to_string()))?;
    let settings = match store.get(STORE_KEY) {
        Some(value) => serde_json::from_value(value.clone()).unwrap_or_default(),
        None => Settings::default(),
    };
    // Must go through apply_settings (not just `state.settings`) so the live
    // RateLimiter is rebuilt with the persisted cooldown — otherwise a saved
    // cooldown value silently has no effect until the user re-saves via
    // update_settings in the same session.
    state.apply_settings(settings.clone()).await;
    Ok(settings)
}

#[tauri::command]
pub async fn update_settings(
    settings: Settings,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    validate_settings(&settings)
        .map_err(|(field, message)| AppError::InvalidSetting { field, message })?;

    let store = app_handle
        .store(STORE_FILE)
        .map_err(|e| AppError::Io(e.to_string()))?;
    store.set(
        STORE_KEY,
        serde_json::to_value(&settings).map_err(|e| AppError::Io(e.to_string()))?,
    );
    store.save().map_err(|e| AppError::Io(e.to_string()))?;

    state.apply_settings(settings).await;
    Ok(())
}
