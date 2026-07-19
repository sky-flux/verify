use std::sync::Arc;
use std::time::Duration;

use sqlx::SqlitePool;
use tokio::sync::{Mutex, OnceCell, RwLock};
use tokio_util::sync::CancellationToken;

use crate::domain::rate_limiter::RateLimiter;
use crate::domain::types::Settings;

pub struct AppState {
    pub rate_limiter: Arc<RwLock<Arc<RateLimiter>>>,
    pub cancel_token: Mutex<Option<CancellationToken>>,
    pub db_pool: OnceCell<SqlitePool>,
    pub settings: RwLock<Settings>,
}

impl AppState {
    pub fn new() -> Self {
        let settings = Settings::default();
        let cooldown = Duration::from_secs(settings.domain_cooldown_seconds as u64);
        AppState {
            rate_limiter: Arc::new(RwLock::new(Arc::new(RateLimiter::new(cooldown)))),
            cancel_token: Mutex::new(None),
            db_pool: OnceCell::new(),
            settings: RwLock::new(settings),
        }
    }

    pub async fn apply_settings(&self, settings: Settings) {
        let cooldown = Duration::from_secs(settings.domain_cooldown_seconds as u64);
        *self.rate_limiter.write().await = Arc::new(RateLimiter::new(cooldown));
        *self.settings.write().await = settings;
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn new_state_starts_with_default_settings() {
        let state = AppState::new();
        let settings = state.settings.read().await;
        assert_eq!(*settings, Settings::default());
    }

    #[tokio::test]
    async fn apply_settings_replaces_stored_settings_and_rate_limiter_cooldown() {
        let state = AppState::new();
        let new_settings = Settings {
            domain_cooldown_seconds: 9,
            ..Settings::default()
        };
        state.apply_settings(new_settings.clone()).await;

        let settings = state.settings.read().await;
        assert_eq!(settings.domain_cooldown_seconds, 9);
    }
}
