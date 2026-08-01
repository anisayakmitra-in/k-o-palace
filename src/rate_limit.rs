//! In-memory rate limiter for K-O Palace.
//!
//! Uses a sliding window per key (IP address or publisher ID).
//! Does NOT trust X-Forwarded-For unless explicitly configured.

use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

const MAX_TRACKED_KEYS: usize = 10_000;

/// Rate limiter with per-key sliding windows.
#[derive(Debug)]
pub struct RateLimiter {
    /// Key -> list of request timestamps within the window.
    windows: Mutex<HashMap<String, Vec<Instant>>>,
    /// Window duration (default: 60 seconds).
    window: Duration,
    /// Max requests per window per key.
    max_requests: usize,
    /// Maximum number of distinct keys retained at once.
    max_keys: usize,
}

impl RateLimiter {
    pub fn new(max_per_minute: usize) -> Self {
        Self::with_max_keys(max_per_minute, MAX_TRACKED_KEYS)
    }

    fn with_max_keys(max_per_minute: usize, max_keys: usize) -> Self {
        Self {
            windows: Mutex::new(HashMap::new()),
            window: Duration::from_secs(60),
            max_requests: max_per_minute,
            max_keys: max_keys.max(1),
        }
    }

    /// Check if a request from the given key is allowed.
    /// Returns Ok(()) if allowed, Err(retry_after_secs) if rate limited.
    pub async fn check(&self, key: &str) -> Result<(), u64> {
        if self.max_requests == 0 {
            return Err(self.window.as_secs().max(1));
        }

        let mut windows = self.windows.lock().await;
        let now = Instant::now();
        let cutoff = now - self.window;

        if !windows.contains_key(key) && windows.len() >= self.max_keys {
            Self::purge_expired(&mut windows, cutoff);
            if windows.len() >= self.max_keys {
                let oldest_key = windows
                    .iter()
                    .min_by_key(|(_, timestamps)| timestamps.last().copied().unwrap_or(now))
                    .map(|(tracked_key, _)| tracked_key.clone());
                if let Some(oldest_key) = oldest_key {
                    windows.remove(&oldest_key);
                }
            }
        }

        let entry = windows.entry(key.to_string()).or_default();
        // Remove expired timestamps
        entry.retain(|&t| t > cutoff);

        if entry.len() >= self.max_requests {
            // Calculate retry-after: time until the oldest request expires
            let oldest = entry.first().unwrap();
            let retry_after = self
                .window
                .checked_sub(now.duration_since(*oldest))
                .map(|d| d.as_secs())
                .unwrap_or(1)
                .max(1);
            return Err(retry_after);
        }

        entry.push(now);
        Ok(())
    }

    /// Remove expired entries to prevent unbounded memory growth.
    pub async fn cleanup(&self) {
        let mut windows = self.windows.lock().await;
        let now = Instant::now();
        let cutoff = now - self.window;
        Self::purge_expired(&mut windows, cutoff);
    }

    fn purge_expired(windows: &mut HashMap<String, Vec<Instant>>, cutoff: Instant) {
        for entry in windows.values_mut() {
            entry.retain(|&t| t > cutoff);
        }
        windows.retain(|_, v| !v.is_empty());
    }
}

/// Rate limiter registry for different endpoint categories.
#[derive(Debug)]
pub struct RateLimiters {
    pub publish: RateLimiter,
    pub search: RateLimiter,
    pub download: RateLimiter,
    pub review: RateLimiter,
    pub auth: RateLimiter,
    pub resolve: RateLimiter,
    pub read: RateLimiter,
}

impl RateLimiters {
    pub fn from_config(config: &crate::config::PalaceConfig) -> Self {
        let s = &config.security;
        Self {
            publish: RateLimiter::new(s.rate_limit_publish_per_minute as usize),
            search: RateLimiter::new(s.rate_limit_search_per_minute as usize),
            download: RateLimiter::new(s.rate_limit_download_per_minute as usize),
            review: RateLimiter::new(s.rate_limit_review_per_minute as usize),
            auth: RateLimiter::new(s.rate_limit_auth_per_minute as usize),
            resolve: RateLimiter::new(s.rate_limit_resolve_per_minute as usize),
            read: RateLimiter::new(600), // 10/sec default for anonymous reads
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn allows_up_to_limit() {
        let limiter = RateLimiter::new(3);
        assert!(limiter.check("key1").await.is_ok());
        assert!(limiter.check("key1").await.is_ok());
        assert!(limiter.check("key1").await.is_ok());
    }

    #[tokio::test]
    async fn blocks_over_limit() {
        let limiter = RateLimiter::new(2);
        assert!(limiter.check("key1").await.is_ok());
        assert!(limiter.check("key1").await.is_ok());
        let result = limiter.check("key1").await;
        assert!(result.is_err());
        let retry = result.unwrap_err();
        assert!(retry >= 1);
    }

    #[tokio::test]
    async fn different_keys_independent() {
        let limiter = RateLimiter::new(2);
        assert!(limiter.check("key1").await.is_ok());
        assert!(limiter.check("key1").await.is_ok());
        // key2 should still be allowed
        assert!(limiter.check("key2").await.is_ok());
        assert!(limiter.check("key2").await.is_ok());
    }

    #[tokio::test]
    async fn retry_after_is_positive() {
        let limiter = RateLimiter::new(1);
        limiter.check("key1").await.unwrap();
        let retry = limiter.check("key1").await.unwrap_err();
        assert!(retry >= 1);
    }

    #[tokio::test]
    async fn caps_tracked_keys() {
        let limiter = RateLimiter::with_max_keys(1, 3);
        for key in ["key1", "key2", "key3"] {
            limiter.check(key).await.unwrap();
        }

        limiter.check("key4").await.unwrap();
        let windows = limiter.windows.lock().await;
        assert_eq!(windows.len(), 3);
    }

    #[tokio::test]
    async fn zero_request_limit_is_rejected_without_panicking() {
        let limiter = RateLimiter::new(0);
        let result = limiter.check("key1").await;
        assert_eq!(result, Err(60));
    }
}
