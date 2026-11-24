use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sea_orm::DatabaseConnection;

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub redis: redis::Client,
    pub config: Arc<Config>,
    pub rate_limiter: Arc<RateLimiter>,
    pub started_at: Instant,
}

#[derive(Default)]
pub struct RateLimiter {
    buckets: Mutex<HashMap<String, VecDeque<Instant>>>,
}

impl RateLimiter {
    pub fn allow(&self, key: &str, limit: usize, window: Duration) -> bool {
        if limit == 0 {
            return true;
        }

        let now = Instant::now();
        let cutoff = now.checked_sub(window).unwrap_or(now);
        let mut buckets = self.buckets.lock().expect("rate limiter lock poisoned");
        let bucket = buckets.entry(key.to_string()).or_default();

        while bucket.front().is_some_and(|instant| *instant <= cutoff) {
            bucket.pop_front();
        }

        if bucket.len() >= limit {
            return false;
        }

        bucket.push_back(now);
        true
    }
}

#[cfg(test)]
impl AppState {
    /// State with no database connection, for unit tests of pure request logic.
    pub fn disconnected_for_tests(config: Config) -> Self {
        let redis = match redis::Client::open(config.redis_url.as_str()) {
            Ok(client) => client,
            Err(error) => panic!("invalid test redis url: {error}"),
        };
        Self {
            db: DatabaseConnection::Disconnected,
            redis,
            config: Arc::new(config),
            rate_limiter: Arc::new(RateLimiter::default()),
            started_at: Instant::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINUTE: Duration = Duration::from_secs(60);

    #[test]
    fn allows_requests_up_to_the_limit_then_blocks() {
        let limiter = RateLimiter::default();
        assert!(limiter.allow("auth:1.2.3.4", 2, MINUTE));
        assert!(limiter.allow("auth:1.2.3.4", 2, MINUTE));
        assert!(!limiter.allow("auth:1.2.3.4", 2, MINUTE));
    }

    #[test]
    fn keys_are_limited_independently() {
        let limiter = RateLimiter::default();
        assert!(limiter.allow("auth:a", 1, MINUTE));
        assert!(!limiter.allow("auth:a", 1, MINUTE));
        assert!(limiter.allow("auth:b", 1, MINUTE));
    }

    #[test]
    fn zero_limit_means_unlimited() {
        let limiter = RateLimiter::default();
        for _ in 0..100 {
            assert!(limiter.allow("upload:x", 0, MINUTE));
        }
    }

    #[test]
    fn requests_are_allowed_again_after_the_window_passes() {
        let limiter = RateLimiter::default();
        let window = Duration::from_millis(30);
        assert!(limiter.allow("transform:x", 1, window));
        assert!(!limiter.allow("transform:x", 1, window));
        std::thread::sleep(Duration::from_millis(40));
        assert!(limiter.allow("transform:x", 1, window));
    }
}
