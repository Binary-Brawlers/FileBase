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
