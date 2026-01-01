//! Cache service for tiered caching (Redis + Parquet).
//!
//! This service provides a unified caching layer that checks:
//! 1. Redis (hot cache) - for frequently accessed data
//! 2. Parquet (warm/cold cache) - for persistent local storage
//! 3. Remote API - as a last resort when cache misses

use crate::domain::CacheRepository;
use crate::infrastructure::{KaspaComClient, ParquetStore, RateLimiter, RedisRepository};
use anyhow::Result;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tracing::{debug, info, warn};

/// TTL configurations for different data types
pub mod ttl {
    /// Hot data - floor prices, recent orders (30 seconds Redis, 5 min Parquet)
    pub const HOT_REDIS_SECS: u64 = 30;
    pub const HOT_PARQUET_SECS: u64 = 300;

    /// Warm data - trade stats, token stats (5 min Redis, 15 min Parquet)
    pub const WARM_REDIS_SECS: u64 = 300;
    pub const WARM_PARQUET_SECS: u64 = 900;

    /// Cold data - token info, historical data (30 min Redis, 1 hour Parquet)
    pub const COLD_REDIS_SECS: u64 = 1800;
    pub const COLD_PARQUET_SECS: u64 = 3600;

    /// Static data - logos, metadata (1 hour Redis, 24 hours Parquet)
    pub const STATIC_REDIS_SECS: u64 = 3600;
    pub const STATIC_PARQUET_SECS: u64 = 86400;
}

/// Per-category cache statistics
#[derive(Debug, Default)]
struct CategoryCacheStats {
    hits: AtomicU64,
    misses: AtomicU64,
    requests: AtomicU64,
}

/// Tiered cache service combining Redis (hot) and Parquet (warm/cold) caching
pub struct CacheService {
    redis: Arc<RedisRepository>,
    parquet: Arc<ParquetStore>,
    client: Arc<KaspaComClient>,
    rate_limiter: Arc<RateLimiter>,
    /// Counter for requests served from cache (incremented on cache hits)
    cache_hits: Arc<AtomicU64>,
    /// Per-category cache statistics
    category_stats: Arc<Mutex<HashMap<String, CategoryCacheStats>>>,
}

impl CacheService {
    /// Create a new cache service
    pub fn new(
        redis: Arc<RedisRepository>,
        parquet: Arc<ParquetStore>,
        client: Arc<KaspaComClient>,
        rate_limiter: Arc<RateLimiter>,
    ) -> Self {
        Self {
            redis,
            parquet,
            client,
            rate_limiter,
            cache_hits: Arc::new(AtomicU64::new(0)),
            category_stats: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Record a cache hit for a category
    fn record_category_hit(&self, category: &str) {
        if let Ok(mut stats) = self.category_stats.lock() {
            let cat_stats = stats.entry(category.to_string()).or_insert_with(|| CategoryCacheStats::default());
            cat_stats.hits.fetch_add(1, Ordering::Relaxed);
            cat_stats.requests.fetch_add(1, Ordering::Relaxed);
        } else {
            warn!("Failed to acquire lock for category stats (mutex poisoned)");
        }
    }

    /// Record a cache miss for a category
    fn record_category_miss(&self, category: &str) {
        if let Ok(mut stats) = self.category_stats.lock() {
            let cat_stats = stats.entry(category.to_string()).or_insert_with(|| CategoryCacheStats::default());
            cat_stats.misses.fetch_add(1, Ordering::Relaxed);
            cat_stats.requests.fetch_add(1, Ordering::Relaxed);
        } else {
            warn!("Failed to acquire lock for category stats (mutex poisoned)");
        }
    }

    /// Get the underlying Kaspa.com client for direct API access
    pub fn client(&self) -> &KaspaComClient {
        &self.client
    }

    /// Get data with tiered cache lookup
    ///
    /// Flow:
    /// 1. Check Redis (hot cache)
    /// 2. Check Parquet (warm/cold cache)  
    /// 3. Fetch from API & populate both caches
    pub async fn get_cached<T, F, Fut>(
        &self,
        redis_key: &str,
        parquet_category: &str,
        parquet_key: &str,
        redis_ttl_secs: u64,
        parquet_ttl_secs: u64,
        fetcher: F,
    ) -> Result<T>
    where
        T: Serialize + DeserializeOwned + Clone,
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<Value>>,
    {
        // 1. Try Redis first (hot cache)
        if let Ok(Some(cached)) = self.get_from_redis::<T>(redis_key).await {
            debug!("Redis cache hit: {}", redis_key);
            self.cache_hits.fetch_add(1, Ordering::Relaxed);
            self.record_category_hit(parquet_category);
            return Ok(cached);
        }

        // 2. Try Parquet (warm/cold cache)
        if self.parquet.is_valid(parquet_category, parquet_key, parquet_ttl_secs) {
            if let Ok(Some(cached)) = self.parquet.read::<T>(parquet_category, parquet_key) {
                debug!("Parquet cache hit: {}/{}", parquet_category, parquet_key);
                self.cache_hits.fetch_add(1, Ordering::Relaxed);
                self.record_category_hit(parquet_category);
                
                // Populate Redis for faster subsequent access
                if let Ok(json) = serde_json::to_string(&cached) {
                    let _ = self.redis.set(redis_key, &json, redis_ttl_secs).await;
                }
                
                return Ok(cached);
            }
        }

        // 3. Fetch from remote API (with rate limiting)
        info!("Cache miss, fetching from API: {}", redis_key);
        self.record_category_miss(parquet_category);
        
        // Check rate limit before making API call
        if !self.rate_limiter.check_and_record().await {
            anyhow::bail!(
                "Rate limit exceeded: {} requests/minute limit reached. Please wait before retrying.",
                self.rate_limiter.get_stats().await.limit
            );
        }
        
        let value = fetcher().await?;

        // Parse the response
        let data: T = serde_json::from_value(value.clone())?;

        // Populate both caches
        self.populate_caches(
            redis_key,
            parquet_category,
            parquet_key,
            &value,
            redis_ttl_secs,
            parquet_ttl_secs,
        )
        .await;

        Ok(data)
    }

    /// Get raw JSON with tiered cache lookup
    pub async fn get_cached_json<F, Fut>(
        &self,
        redis_key: &str,
        parquet_category: &str,
        parquet_key: &str,
        redis_ttl_secs: u64,
        parquet_ttl_secs: u64,
        fetcher: F,
    ) -> Result<Value>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<Value>>,
    {
        // 1. Try Redis first (hot cache)
        if let Ok(Some(cached)) = self.redis.get(redis_key).await {
            if let Ok(value) = serde_json::from_str::<Value>(&cached) {
                debug!("Redis cache hit (JSON): {}", redis_key);
                self.cache_hits.fetch_add(1, Ordering::Relaxed);
                self.record_category_hit(parquet_category);
                return Ok(value);
            }
        }

        // 2. Try Parquet (warm/cold cache)
        if self.parquet.is_valid(parquet_category, parquet_key, parquet_ttl_secs) {
            if let Ok(Some(value)) = self.parquet.read_json(parquet_category, parquet_key) {
                debug!("Parquet cache hit (JSON): {}/{}", parquet_category, parquet_key);
                self.cache_hits.fetch_add(1, Ordering::Relaxed);
                self.record_category_hit(parquet_category);
                
                // Populate Redis
                if let Ok(json) = serde_json::to_string(&value) {
                    let _ = self.redis.set(redis_key, &json, redis_ttl_secs).await;
                }
                
                return Ok(value);
            }
        }

        // 3. Fetch from API (with rate limiting)
        info!("Cache miss (JSON), fetching from API: {}", redis_key);
        self.record_category_miss(parquet_category);
        
        // Check rate limit before making API call
        if !self.rate_limiter.check_and_record().await {
            anyhow::bail!(
                "Rate limit exceeded: {} requests/minute limit reached. Please wait before retrying.",
                self.rate_limiter.get_stats().await.limit
            );
        }
        
        let value = fetcher().await?;

        // Populate caches
        self.populate_caches(
            redis_key,
            parquet_category,
            parquet_key,
            &value,
            redis_ttl_secs,
            parquet_ttl_secs,
        )
        .await;

        Ok(value)
    }

    /// Force refresh from API and update all cache layers
    pub async fn refresh<F, Fut>(
        &self,
        redis_key: &str,
        parquet_category: &str,
        parquet_key: &str,
        redis_ttl_secs: u64,
        parquet_ttl_secs: u64,
        fetcher: F,
    ) -> Result<Value>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<Value>>,
    {
        info!("Force refreshing: {}", redis_key);
        
        // Check rate limit before making API call
        if !self.rate_limiter.check_and_record().await {
            anyhow::bail!(
                "Rate limit exceeded: {} requests/minute limit reached. Please wait before retrying.",
                self.rate_limiter.get_stats().await.limit
            );
        }
        
        let value = fetcher().await?;

        self.populate_caches(
            redis_key,
            parquet_category,
            parquet_key,
            &value,
            redis_ttl_secs,
            parquet_ttl_secs,
        )
        .await;

        Ok(value)
    }

    /// Populate both cache layers
    async fn populate_caches(
        &self,
        redis_key: &str,
        parquet_category: &str,
        parquet_key: &str,
        value: &Value,
        redis_ttl_secs: u64,
        parquet_ttl_secs: u64,
    ) {
        // Write to Redis
        if let Ok(json) = serde_json::to_string(value) {
            if let Err(e) = self.redis.set(redis_key, &json, redis_ttl_secs).await {
                warn!("Failed to write to Redis cache: {}", e);
            }
        }

        // Write to Parquet
        if let Err(e) = self.parquet.write_simple(parquet_category, parquet_key, value, parquet_ttl_secs) {
            warn!("Failed to write to Parquet cache: {}", e);
        }
    }

    /// Get from Redis and deserialize
    async fn get_from_redis<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        match self.redis.get(key).await? {
            Some(cached) => {
                let data: T = serde_json::from_str(&cached)?;
                Ok(Some(data))
            }
            None => Ok(None),
        }
    }

    /// Invalidate cache entry in both layers
    pub async fn invalidate(&self, redis_key: &str, parquet_category: &str, parquet_key: &str) -> Result<()> {
        // Redis doesn't have a delete method in the trait, so we just let it expire
        // For Parquet, we can delete the file
        self.parquet.delete(parquet_category, parquet_key)?;
        info!("Invalidated cache: {}", redis_key);
        Ok(())
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> Result<crate::infrastructure::CacheStats> {
        let mut stats = self.parquet.get_stats()?;
        stats.cache_hits = self.cache_hits.load(Ordering::Relaxed);
        
        // Merge per-category cache statistics
        let category_stats_map = match self.category_stats.lock() {
            Ok(map) => map,
            Err(_) => {
                warn!("Failed to acquire lock for category stats (mutex poisoned), returning stats without per-category metrics");
                return Ok(stats);
            }
        };
        for (category, cat_stats) in category_stats_map.iter() {
            let hits = cat_stats.hits.load(Ordering::Relaxed);
            let misses = cat_stats.misses.load(Ordering::Relaxed);
            let requests = cat_stats.requests.load(Ordering::Relaxed);
            
            if let Some(cat_stat) = stats.categories.get_mut(category) {
                // Update existing category
                cat_stat.hits = hits;
                cat_stat.misses = misses;
                cat_stat.requests = requests;
            } else {
                // Create new category entry if it has cache activity but no parquet files yet
                if requests > 0 {
                    use crate::infrastructure::parquet_store::CategoryStats;
                    stats.categories.insert(category.clone(), CategoryStats {
                        keys: 0,
                        size_bytes: 0,
                        description: format!("{} (cache activity)", category),
                        hits,
                        misses,
                        requests,
                    });
                }
            }
        }
        
        Ok(stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ttl_values() {
        // Ensure TTL hierarchy makes sense
        assert!(ttl::HOT_REDIS_SECS < ttl::HOT_PARQUET_SECS);
        assert!(ttl::WARM_REDIS_SECS < ttl::WARM_PARQUET_SECS);
        assert!(ttl::COLD_REDIS_SECS < ttl::COLD_PARQUET_SECS);
        assert!(ttl::STATIC_REDIS_SECS < ttl::STATIC_PARQUET_SECS);
    }
}
