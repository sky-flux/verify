use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Tracks the last probe time per domain and enforces a cooldown between
/// consecutive probes against the same mail server, so a batch run doesn't
/// hammer one domain and get the source IP blocklisted.
pub struct RateLimiter {
    last_probe: Mutex<HashMap<String, Instant>>,
    cooldown: Duration,
}

impl RateLimiter {
    pub fn new(cooldown: Duration) -> Self {
        RateLimiter {
            last_probe: Mutex::new(HashMap::new()),
            cooldown,
        }
    }

    /// Returns how long the caller must wait before probing `domain` again.
    /// `Duration::ZERO` means the domain is clear to probe right now.
    pub fn wait_time(&self, domain: &str) -> Duration {
        let map = self.last_probe.lock().unwrap();
        match map.get(domain) {
            Some(last) => {
                let elapsed = last.elapsed();
                if elapsed >= self.cooldown {
                    Duration::ZERO
                } else {
                    self.cooldown - elapsed
                }
            }
            None => Duration::ZERO,
        }
    }

    /// Records that `domain` was just probed, starting a fresh cooldown window.
    pub fn record_probe(&self, domain: &str) {
        let mut map = self.last_probe.lock().unwrap();
        map.insert(domain.to_string(), Instant::now());
    }

    /// Blocks (async sleep) until `domain` is clear to probe, then records the probe.
    pub async fn acquire(&self, domain: &str) {
        let wait = self.wait_time(domain);
        if !wait.is_zero() {
            tokio::time::sleep(wait).await;
        }
        self.record_probe(domain);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_probe_for_a_domain_needs_no_wait() {
        let limiter = RateLimiter::new(Duration::from_secs(3));
        assert_eq!(limiter.wait_time("example.com"), Duration::ZERO);
    }

    #[test]
    fn probing_again_immediately_requires_waiting_close_to_full_cooldown() {
        let limiter = RateLimiter::new(Duration::from_millis(200));
        limiter.record_probe("example.com");
        let wait = limiter.wait_time("example.com");
        assert!(wait > Duration::ZERO);
        assert!(wait <= Duration::from_millis(200));
    }

    #[test]
    fn different_domains_do_not_share_cooldowns() {
        let limiter = RateLimiter::new(Duration::from_secs(5));
        limiter.record_probe("a.com");
        assert_eq!(limiter.wait_time("b.com"), Duration::ZERO);
    }

    #[tokio::test]
    async fn acquire_waits_out_the_cooldown_before_returning() {
        let limiter = RateLimiter::new(Duration::from_millis(50));
        limiter.record_probe("example.com");
        let start = Instant::now();
        limiter.acquire("example.com").await;
        assert!(start.elapsed() >= Duration::from_millis(45));
    }

    #[tokio::test]
    async fn acquire_does_not_wait_once_cooldown_has_elapsed() {
        let limiter = RateLimiter::new(Duration::from_millis(20));
        limiter.record_probe("example.com");
        tokio::time::sleep(Duration::from_millis(30)).await;
        let start = Instant::now();
        limiter.acquire("example.com").await;
        assert!(start.elapsed() < Duration::from_millis(15));
    }
}
