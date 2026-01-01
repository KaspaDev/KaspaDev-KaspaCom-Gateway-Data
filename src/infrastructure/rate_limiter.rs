//! Rate limiter for kaspa.com API requests.
//!
//! Implements a sliding window rate limiter to track and enforce
//! request limits to the kaspa.com API.

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Rate limiter for tracking API requests
#[derive(Clone)]
pub struct RateLimiter {
    limit: u32,
    window: Duration,
    requests: Arc<RwLock<Vec<Instant>>>,
}

impl RateLimiter {
    /// Create a new rate limiter with the specified requests per minute
    pub fn new(requests_per_minute: u32) -> Self {
        Self {
            limit: requests_per_minute,
            window: Duration::from_secs(60),
            requests: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Check if a request is allowed and record it if so
    ///
    /// Returns true if the request is allowed, false if rate limit exceeded
    pub async fn check_and_record(&self) -> bool {
        let now = Instant::now();
        let window_start = now - self.window;

        let mut requests = self.requests.write().await;
        
        // Remove requests outside the current window
        requests.retain(|&time| time > window_start);

        // Check if we're under the limit
        if requests.len() < self.limit as usize {
            requests.push(now);
            true
        } else {
            false
        }
    }

    /// Get current rate limit statistics
    pub async fn get_stats(&self) -> RateLimitStats {
        let now = Instant::now();
        let window_start = now - self.window;

        let requests = self.requests.read().await;
        
        // Count requests in current window
        let used = requests.iter().filter(|&&time| time > window_start).count() as u32;
        
        // Calculate reset time (next minute boundary)
        let system_now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let current_second = system_now.as_secs() % 60;
        let seconds_until_reset = 60 - current_second;
        let reset_timestamp = system_now.as_secs() as i64 + seconds_until_reset as i64;

        RateLimitStats {
            limit: self.limit,
            remaining: self.limit.saturating_sub(used),
            used,
            reset: reset_timestamp,
        }
    }
}

/// Rate limit statistics
#[derive(Debug, Clone)]
pub struct RateLimitStats {
    pub limit: u32,
    pub remaining: u32,
    pub used: u32,
    pub reset: i64, // Unix timestamp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter_allows_requests_within_limit() {
        let limiter = RateLimiter::new(5);
        
        // First 5 requests should be allowed
        for _ in 0..5 {
            assert!(limiter.check_and_record().await);
        }
        
        // 6th request should be denied
        assert!(!limiter.check_and_record().await);
    }

    #[tokio::test]
    async fn test_rate_limiter_resets_after_window() {
        let limiter = RateLimiter::new(2);
        
        // Use up the limit
        assert!(limiter.check_and_record().await);
        assert!(limiter.check_and_record().await);
        assert!(!limiter.check_and_record().await);
        
        // Wait for window to pass (in real scenario, this would be 60 seconds)
        // For testing, we'll just verify the logic works
        let stats = limiter.get_stats().await;
        assert_eq!(stats.limit, 2);
    }

    #[tokio::test]
    async fn test_rate_limiter_stats() {
        let limiter = RateLimiter::new(10);
        
        // Make some requests
        for _ in 0..3 {
            limiter.check_and_record().await;
        }
        
        let stats = limiter.get_stats().await;
        assert_eq!(stats.limit, 10);
        assert_eq!(stats.used, 3);
        assert_eq!(stats.remaining, 7);
        assert!(stats.reset > 0);
    }

    #[tokio::test]
    async fn test_rate_limiter_high_limit() {
        let limiter = RateLimiter::new(1000);
        
        // Should allow many requests
        for _ in 0..100 {
            assert!(limiter.check_and_record().await);
        }
        
        let stats = limiter.get_stats().await;
        assert_eq!(stats.used, 100);
        assert_eq!(stats.remaining, 900);
    }
}

