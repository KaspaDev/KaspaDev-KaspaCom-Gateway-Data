//! Ticker service for simplified token data access.
//!
//! Provides convenience methods for accessing aggregated token statistics
//! across all exchanges without requiring directory navigation.

use crate::application::ExchangeIndex;
use crate::domain::{CacheRepository, ContentRepository, ContentType, RepoConfig};
use base64::{engine::general_purpose, Engine as _};
use chrono::{Duration, NaiveDate, Utc};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};
use utoipa::ToSchema;

/// Response structure for ticker stats endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TickerStatsResponse {
    /// Token symbol/name
    pub token: String,
    /// Response timestamp (ISO 8601)
    pub timestamp: String,
    /// Range requested (today, 7d, 30d)
    pub range: String,
    /// Per-exchange statistics
    pub exchanges: Vec<ExchangeStats>,
    /// Aggregated statistics across all exchanges
    pub aggregate: AggregateStats,
}

/// Statistics for a single exchange.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ExchangeStats {
    /// Exchange identifier
    pub exchange: String,
    /// Last trade price
    pub last: Option<f64>,
    /// 24h high price
    pub high: Option<f64>,
    /// 24h low price
    pub low: Option<f64>,
    /// 24h volume (base currency)
    pub volume_24h: Option<f64>,
    /// 24h price change percentage
    pub change_pct: Option<f64>,
    /// Number of data points in range
    pub data_points: usize,
}

/// Aggregated statistics across all exchanges.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AggregateStats {
    /// Average price across exchanges
    pub avg_price: Option<f64>,
    /// Total volume across all exchanges
    pub total_volume_24h: Option<f64>,
    /// Volume-weighted average price
    pub vwap: Option<f64>,
    /// Number of active exchanges
    pub exchange_count: usize,
}

/// Response structure for ticker history endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TickerHistoryResponse {
    /// Token symbol/name
    pub token: String,
    /// Range requested
    pub range: String,
    /// Data resolution
    pub resolution: String,
    /// OHLCV data points
    pub data: Vec<OhlcvPoint>,
}

/// Response structure for simplified timeseries endpoint.
/// Returns data as simple [timestamp, price] pairs for easy chart consumption.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TimeseriesResponse {
    /// Token symbol/name
    pub token: String,
    /// Range requested
    pub range: String,
    /// Data resolution
    pub resolution: String,
    /// Timeseries data points
    pub data: Vec<TimeseriesPoint>,
}

/// Response structure for available tickers endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AvailableTickersResponse {
    /// List of available token/ticker names
    pub tickers: Vec<String>,
    /// Total count of available tickers
    pub count: usize,
}

/// Response structure for exchanges endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ExchangesResponse {
    /// List of exchanges with their tokens
    pub exchanges: Vec<ExchangeInfo>,
    /// Total count of exchanges
    pub count: usize,
}

/// Information about an exchange and its tokens.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ExchangeInfo {
    /// Exchange identifier
    pub exchange: String,
    /// List of KRC20 tokens available on this exchange
    pub tokens: Vec<String>,
    /// Total count of tokens on this exchange
    pub token_count: usize,
}

/// Response structure for exchange detail endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ExchangeDetailResponse {
    /// Exchange identifier
    pub exchange: String,
    /// Range requested (today, 7d, 30d)
    pub range: String,
    /// Response timestamp (ISO 8601)
    pub timestamp: String,
    /// List of tokens with their stats on this exchange
    pub tokens: Vec<ExchangeTokenRow>,
    /// Total count of tokens
    pub count: usize,
}

/// Token statistics for a specific exchange.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ExchangeTokenRow {
    /// Token symbol/name
    pub token: String,
    /// Last trade price
    pub last: Option<f64>,
    /// 24h high price
    pub high: Option<f64>,
    /// 24h low price
    pub low: Option<f64>,
    /// 24h volume (base currency)
    pub volume_24h: Option<f64>,
    /// 24h price change percentage
    pub change_pct: Option<f64>,
    /// Number of data points in range
    pub data_points: usize,
}

/// Simple timeseries data point for easy chart consumption.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TimeseriesPoint {
    /// Unix timestamp (seconds)
    pub timestamp: i64,
    /// Price at this timestamp
    pub price: f64,
}

/// Single OHLCV data point for charting.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OhlcvPoint {
    /// Unix timestamp (seconds)
    pub timestamp: i64,
    /// Open price
    pub open: f64,
    /// High price
    pub high: f64,
    /// Low price
    pub low: f64,
    /// Close price
    pub close: f64,
    /// Volume
    pub volume: f64,
}

/// Query parameters for ticker stats endpoint.
#[derive(Debug, Clone, Deserialize, utoipa::IntoParams)]
pub struct TickerStatsQuery {
    /// Lookback range: today, 7d, 30d (default: today)
    #[param(default = "today", example = "7d")]
    pub range: Option<String>,
}

/// Query parameters for ticker history endpoint.
#[derive(Debug, Clone, Deserialize, utoipa::IntoParams)]
pub struct TickerHistoryQuery {
    /// Lookback range: today, 7d, 30d (default: 7d)
    #[param(default = "7d", example = "7d")]
    pub range: Option<String>,
    /// Data resolution: 1m, 5m, 15m, 30m, 1h, 4h, 1d (default: 1h)
    #[param(default = "1h", example = "1h")]
    pub resolution: Option<String>,
}

/// Query parameters for exchange detail endpoint.
#[derive(Debug, Clone, Deserialize, utoipa::IntoParams)]
pub struct ExchangeDetailQuery {
    /// Lookback range: today, 7d, 30d (default: today)
    #[param(default = "today", example = "today")]
    pub range: Option<String>,
}

/// Service for ticker-focused operations.
#[derive(Clone)]
pub struct TickerService {
    content_repo: Arc<dyn ContentRepository>,
    local_repo: Option<Arc<dyn ContentRepository>>,
    cache_repo: Arc<dyn CacheRepository>,
    default_repo: RepoConfig,
    exchange_index: Option<Arc<ExchangeIndex>>,
}

impl TickerService {
    pub fn new(
        content_repo: Arc<dyn ContentRepository>,
        cache_repo: Arc<dyn CacheRepository>,
        default_repo: RepoConfig,
    ) -> Self {
        Self {
            content_repo,
            local_repo: None,
            cache_repo,
            default_repo,
            exchange_index: None,
        }
    }

    /// Create a new TickerService with local filesystem support.
    pub fn with_local(
        content_repo: Arc<dyn ContentRepository>,
        local_repo: Option<Arc<dyn ContentRepository>>,
        cache_repo: Arc<dyn CacheRepository>,
        default_repo: RepoConfig,
        exchange_index: Option<Arc<ExchangeIndex>>,
    ) -> Self {
        Self {
            content_repo,
            local_repo,
            cache_repo,
            default_repo,
            exchange_index,
        }
    }

    /// Get the repository to use (local if available, otherwise GitHub).
    fn get_repo(&self) -> Arc<dyn ContentRepository> {
        self.local_repo
            .as_ref()
            .cloned()
            .unwrap_or_else(|| self.content_repo.clone())
    }

    /// Get current stats for a token across all exchanges.
    pub async fn get_ticker_stats(
        &self,
        token: String,
        range: String,
    ) -> anyhow::Result<TickerStatsResponse> {
        let cache_key = format!("v1:ticker:{}:stats:{}", token, range);

        // Check cache first
        if let Ok(Some(cached)) = self.cache_repo.get(&cache_key).await {
            if let Ok(response) = serde_json::from_str::<TickerStatsResponse>(&cached) {
                info!("Cache HIT: {}", cache_key);
                metrics::counter!("cache_operations_total", "operation" => "hit").increment(1);
                return Ok(response);
            }
        }
        metrics::counter!("cache_operations_total", "operation" => "miss").increment(1);

        // Discover exchanges for this token
        let repo = self.get_repo();
        let token_path = format!("data/{}", token.to_lowercase());
        let exchanges = repo
            .list_directory(&self.default_repo, &token_path)
            .await?;

        let exchange_dirs: Vec<_> = exchanges
            .into_iter()
            .filter(|e| e.item_type == ContentType::Dir)
            .collect();

        if exchange_dirs.is_empty() {
            anyhow::bail!("No exchanges found for token: {}", token);
        }

        // Calculate date range
        let (start_date, end_date) = Self::calculate_date_range(&range);

        // Fetch stats from each exchange concurrently
        let repo_clone = repo.clone();
        let mut exchange_stats = Vec::new();
        let fetches = futures::stream::iter(exchange_dirs)
            .map(|exchange| {
                let repo = repo_clone.clone();
                let config = self.default_repo.clone();
                let token = token.clone();
                let start = start_date;
                let end = end_date;
                async move {
                    Self::fetch_exchange_stats(repo, config, token, exchange.name, start, end).await
                }
            })
            .buffer_unordered(10)
            .collect::<Vec<_>>()
            .await;

        for result in fetches {
            match result {
                Ok(stats) => exchange_stats.push(stats),
                Err(e) => warn!("Failed to fetch exchange stats: {}", e),
            }
        }

        // Calculate aggregate stats
        let aggregate = Self::calculate_aggregate(&exchange_stats);

        let response = TickerStatsResponse {
            token: token.clone(),
            timestamp: Utc::now().to_rfc3339(),
            range: range.clone(),
            exchanges: exchange_stats,
            aggregate,
        };

        // Cache result (5 min TTL)
        if let Ok(json) = serde_json::to_string(&response) {
            let _ = self.cache_repo.set(&cache_key, &json, 300).await;
        }

        Ok(response)
    }

    /// Get historical data for a token (for charting).
    pub async fn get_ticker_history(
        &self,
        token: String,
        range: String,
        resolution: String,
    ) -> anyhow::Result<TickerHistoryResponse> {
        let cache_key = format!("v1:ticker:{}:history:{}:{}", token, range, resolution);

        // Check cache first
        if let Ok(Some(cached)) = self.cache_repo.get(&cache_key).await {
            if let Ok(response) = serde_json::from_str::<TickerHistoryResponse>(&cached) {
                info!("Cache HIT: {}", cache_key);
                metrics::counter!("cache_operations_total", "operation" => "hit").increment(1);
                return Ok(response);
            }
        }
        metrics::counter!("cache_operations_total", "operation" => "miss").increment(1);

        // Discover exchanges for this token
        let repo = self.get_repo();
        let token_path = format!("data/{}", token.to_lowercase());
        let exchanges = repo
            .list_directory(&self.default_repo, &token_path)
            .await?;

        let exchange_dirs: Vec<_> = exchanges
            .into_iter()
            .filter(|e| e.item_type == ContentType::Dir)
            .collect();

        if exchange_dirs.is_empty() {
            anyhow::bail!("No exchanges found for token: {}", token);
        }

        let (start_date, end_date) = Self::calculate_date_range(&range);

        // Collect raw data from exchanges - try up to 10 to find ones with data
        let repo_clone = repo.clone();
        let mut all_data: Vec<serde_json::Value> = Vec::new();
        let mut exchanges_with_data = 0;
        const MAX_EXCHANGES: usize = 5;
        const MAX_TRIES: usize = 15;

        for exchange in exchange_dirs.iter().take(MAX_TRIES) {
            if exchanges_with_data >= MAX_EXCHANGES {
                break;
            }
            
            match Self::fetch_exchange_raw_data(
                repo_clone.clone(),
                self.default_repo.clone(),
                token.clone(),
                exchange.name.clone(),
                start_date,
                end_date,
            )
            .await
            {
                Ok(data) => {
                    if !data.is_empty() {
                        info!("Found {} data points from {} for history", data.len(), exchange.name);
                        all_data.extend(data);
                        exchanges_with_data += 1;
                    }
                }
                Err(e) => warn!("Failed to fetch data from {}: {}", exchange.name, e),
            }
        }

        info!("Total raw data points collected: {} for {} history", all_data.len(), token);

        // Aggregate into OHLCV based on resolution
        let ohlcv_data = Self::aggregate_to_ohlcv(&all_data, &resolution);
        
        info!("OHLCV data points after aggregation: {} for {} (resolution: {})", ohlcv_data.len(), token, resolution);

        let response = TickerHistoryResponse {
            token: token.clone(),
            range: range.clone(),
            resolution: resolution.clone(),
            data: ohlcv_data,
        };

        // Cache result (5 min TTL)
        if let Ok(json) = serde_json::to_string(&response) {
            let _ = self.cache_repo.set(&cache_key, &json, 300).await;
        }

        Ok(response)
    }

    fn calculate_date_range(range: &str) -> (NaiveDate, NaiveDate) {
        let today = Utc::now().date_naive();
        let start = match range {
            "today" => today,
            "7d" => today - Duration::days(7),
            "30d" => today - Duration::days(30),
            _ => today,
        };
        (start, today)
    }

    async fn fetch_exchange_stats(
        repo: Arc<dyn ContentRepository>,
        config: RepoConfig,
        token: String,
        exchange: String,
        _start_date: NaiveDate,
        _end_date: NaiveDate,
    ) -> anyhow::Result<ExchangeStats> {
        // Try to get data file - try today first, then fall back to previous days
        let today = Utc::now().date_naive();
        let days_to_try = [today, today - Duration::days(1), today - Duration::days(2)];

        for date in days_to_try {
            let year = date.format("%Y");
            let month = date.format("%m");
            let date_path = format!(
                "data/{}/{}/{}/{}/{}-raw.json",
                token.to_lowercase(),
                exchange,
                year,
                month,
                date.format("%Y-%m-%d")
            );

            // Try to fetch the file
            match repo.get_content(&config, &date_path).await {
                Ok(content) => {
                    // Parse the content
                    if let (Some(raw), Some(enc)) = (content.content, content.encoding) {
                        if enc == "base64" {
                            let clean = raw.replace('\n', "");
                            if let Ok(bytes) = general_purpose::STANDARD.decode(&clean) {
                                if let Ok(s) = String::from_utf8(bytes) {
                                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&s) {
                                        info!("Found data for {} from {} for date {}", token, exchange, date);
                                        return Self::parse_exchange_stats(&exchange, &json);
                                    }
                                }
                            }
                        }
                    }
                }
                Err(_) => {
                    // Try next day
                    continue;
                }
            }
        }

        // Return empty stats if no data found in any of the days
        Ok(ExchangeStats {
            exchange,
            last: None,
            high: None,
            low: None,
            volume_24h: None,
            change_pct: None,
            data_points: 0,
        })
    }

    fn parse_exchange_stats(
        exchange: &str,
        json: &serde_json::Value,
    ) -> anyhow::Result<ExchangeStats> {
        let data = json.get("data").and_then(|d| d.as_array());

        if let Some(arr) = data {
            if arr.is_empty() {
                return Ok(ExchangeStats {
                    exchange: exchange.to_string(),
                    last: None,
                    high: None,
                    low: None,
                    volume_24h: None,
                    change_pct: None,
                    data_points: 0,
                });
            }

            // Get latest data point
            let latest = &arr[arr.len() - 1];

            // Calculate high/low across all data points
            let mut high: Option<f64> = None;
            let mut low: Option<f64> = None;
            let mut total_volume: f64 = 0.0;

            for point in arr {
                if let Some(h) = point.get("high").and_then(|v| v.as_f64()) {
                    high = Some(high.map_or(h, |curr| curr.max(h)));
                }
                if let Some(l) = point.get("low").and_then(|v| v.as_f64()) {
                    low = Some(low.map_or(l, |curr| curr.min(l)));
                }
                if let Some(v) = point.get("quoteVolume").and_then(|v| v.as_f64()) {
                    total_volume = v; // Use latest quoteVolume as it's cumulative
                }
            }

            Ok(ExchangeStats {
                exchange: exchange.to_string(),
                last: latest.get("last").and_then(|v| v.as_f64()),
                high,
                low,
                volume_24h: Some(total_volume),
                change_pct: latest.get("percentage").and_then(|v| v.as_f64()),
                data_points: arr.len(),
            })
        } else {
            Ok(ExchangeStats {
                exchange: exchange.to_string(),
                last: None,
                high: None,
                low: None,
                volume_24h: None,
                change_pct: None,
                data_points: 0,
            })
        }
    }

    fn calculate_aggregate(exchanges: &[ExchangeStats]) -> AggregateStats {
        let active_exchanges: Vec<_> = exchanges
            .iter()
            .filter(|e| e.last.is_some())
            .collect();

        if active_exchanges.is_empty() {
            return AggregateStats {
                avg_price: None,
                total_volume_24h: None,
                vwap: None,
                exchange_count: 0,
            };
        }

        let sum_price: f64 = active_exchanges
            .iter()
            .filter_map(|e| e.last)
            .sum();
        let avg_price = sum_price / active_exchanges.len() as f64;

        let total_volume: f64 = active_exchanges
            .iter()
            .filter_map(|e| e.volume_24h)
            .sum();

        // Calculate VWAP (volume-weighted average price)
        let mut weighted_sum = 0.0;
        let mut volume_sum = 0.0;
        for e in &active_exchanges {
            if let (Some(price), Some(vol)) = (e.last, e.volume_24h) {
                weighted_sum += price * vol;
                volume_sum += vol;
            }
        }
        let vwap = if volume_sum > 0.0 {
            Some(weighted_sum / volume_sum)
        } else {
            None
        };

        AggregateStats {
            avg_price: Some(avg_price),
            total_volume_24h: Some(total_volume),
            vwap,
            exchange_count: active_exchanges.len(),
        }
    }

    async fn fetch_exchange_raw_data(
        repo: Arc<dyn ContentRepository>,
        config: RepoConfig,
        token: String,
        exchange: String,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> anyhow::Result<Vec<serde_json::Value>> {
        let mut all_data = Vec::new();
        let mut current = start_date;
        
        info!("Fetching raw data for {}/{} from {} to {}", token, exchange, start_date, end_date);

        while current <= end_date {
            let year = current.format("%Y");
            let month = current.format("%m");
            let date_path = format!(
                "data/{}/{}/{}/{}/{}-raw.json",
                token.to_lowercase(),
                exchange,
                year,
                month,
                current.format("%Y-%m-%d")
            );
            
            info!("Trying to fetch: {}", date_path);

            if let Ok(content) = repo.get_content(&config, &date_path).await {
                // Try to use get_raw_file if URL is available (more efficient for local files)
                let file_url = content.download_url.as_ref().or_else(|| Some(&content.url));
                if let Some(url) = file_url {
                    if url.starts_with("file://") {
                        match repo.get_raw_file(url).await {
                            Ok(json) => {
                                // Already parsed JSON from get_raw_file
                                if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
                                    if !data.is_empty() {
                                        all_data.extend(data.clone());
                                    }
                                }
                                continue; // Successfully processed, continue to next file
                            }
                            Err(e) => {
                                warn!("Failed to read raw file from {}: {}", url, e);
                                // Fall through to base64 decode method
                            }
                        }
                    }
                }

                // Fallback: decode base64 content (GitHub API or LocalFileRepository)
                if let (Some(raw), Some(enc)) = (content.content, content.encoding) {
                    if enc == "base64" {
                        let clean = raw.replace('\n', "");
                        if let Ok(bytes) = general_purpose::STANDARD.decode(&clean) {
                            if let Ok(s) = String::from_utf8(bytes) {
                                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&s) {
                                    if let Some(data) = json.get("data").and_then(|d| d.as_array())
                                    {
                                        if !data.is_empty() {
                                            info!("Successfully loaded {} data points from {}", data.len(), date_path);
                                            all_data.extend(data.clone());
                                        } else {
                                            warn!("File {} exists but data array is empty", date_path);
                                        }
                                    } else {
                                        warn!("File {} exists but no 'data' array found", date_path);
                                    }
                                } else {
                                    warn!("File {} exists but failed to parse as JSON", date_path);
                                }
                            } else {
                                warn!("File {} exists but failed to decode UTF-8", date_path);
                            }
                        } else {
                            warn!("File {} exists but failed to decode base64", date_path);
                        }
                    }
                } else {
                    warn!("File {} not found or has no content", date_path);
                }
            } else {
                warn!("Failed to get content for {}: file not found", date_path);
            }

            current += Duration::days(1);
        }
        
        info!("Total data points collected for {}/{}: {}", token, exchange, all_data.len());

        Ok(all_data)
    }

    fn aggregate_to_ohlcv(data: &[serde_json::Value], resolution: &str) -> Vec<OhlcvPoint> {
        if data.is_empty() {
            return vec![];
        }

        let interval_secs: i64 = match resolution {
            "1m" => 60,
            "5m" => 300,
            "15m" => 900,
            "30m" => 1800,
            "1h" => 3600,
            "4h" => 14400,
            "1d" => 86400,
            _ => 3600, // Default to 1h
        };

        // Group data points by time bucket
        let mut buckets: std::collections::BTreeMap<i64, Vec<&serde_json::Value>> =
            std::collections::BTreeMap::new();

        for point in data {
            if let Some(ts) = point.get("timestamp").and_then(|v| v.as_i64()) {
                // Convert milliseconds to seconds and bucket
                let ts_secs = ts / 1000;
                let bucket = (ts_secs / interval_secs) * interval_secs;
                buckets.entry(bucket).or_default().push(point);
            }
        }

        // Convert buckets to OHLCV
        buckets
            .into_iter()
            .map(|(timestamp, points)| {
                let mut open = 0.0;
                let mut high = f64::MIN;
                let mut low = f64::MAX;
                let mut close = 0.0;
                let mut volume = 0.0;

                if let Some(first) = points.first() {
                    open = first.get("last").and_then(|v| v.as_f64()).unwrap_or(0.0);
                }
                if let Some(last) = points.last() {
                    close = last.get("last").and_then(|v| v.as_f64()).unwrap_or(0.0);
                }

                for p in &points {
                    if let Some(h) = p.get("high").and_then(|v| v.as_f64()) {
                        high = high.max(h);
                    }
                    if let Some(l) = p.get("low").and_then(|v| v.as_f64()) {
                        low = low.min(l);
                    }
                    if let Some(v) = p.get("quoteVolume").and_then(|v| v.as_f64()) {
                        volume = v; // Use latest as it's cumulative
                    }
                }

                // Fix edge cases
                if high == f64::MIN {
                    high = close;
                }
                if low == f64::MAX {
                    low = close;
                }

                OhlcvPoint {
                    timestamp,
                    open,
                    high,
                    low,
                    close,
                    volume,
                }
            })
            .collect()
    }

    /// Get list of available tickers/tokens.
    /// 
    /// Returns all tokens that have data available in the repository.
    /// This is useful for discovering which tickers can be queried.
    /// 
    /// # Returns
    /// 
    /// AvailableTickersResponse with a list of ticker names.
    /// 
    /// # Example
    /// 
    /// ```rust,no_run
    /// let tickers = ticker_service.get_available_tickers().await?;
    /// // Returns: AvailableTickersResponse { tickers: vec!["kaspa", "slow", "nacho"], count: 3 }
    /// ```
    pub async fn get_available_tickers(&self) -> anyhow::Result<AvailableTickersResponse> {
        let cache_key = "v1:tickers:list";

        // Check cache first (cache for 1 hour since this changes infrequently)
        if let Ok(Some(cached)) = self.cache_repo.get(cache_key).await {
            if let Ok(response) = serde_json::from_str::<AvailableTickersResponse>(&cached) {
                info!("Cache HIT: {}", cache_key);
                metrics::counter!("cache_operations_total", "operation" => "hit").increment(1);
                return Ok(response);
            }
        }
        metrics::counter!("cache_operations_total", "operation" => "miss").increment(1);

        // List the data directory to discover tokens
        let repo = self.get_repo();
        let data_dir = "data";
        let items = repo
            .list_directory(&self.default_repo, data_dir)
            .await?;

        // Filter for directories (tokens) only
        let tickers: Vec<String> = items
            .into_iter()
            .filter(|item| item.item_type == ContentType::Dir)
            .map(|item| item.name)
            .collect();

        let response = AvailableTickersResponse {
            count: tickers.len(),
            tickers,
        };

        // Cache result (1 hour TTL)
        if let Ok(json) = serde_json::to_string(&response) {
            let _ = self.cache_repo.set(cache_key, &json, 3600).await;
        }

        Ok(response)
    }

    /// Get simplified timeseries data for easy chart consumption.
    /// 
    /// Returns price data as simple timestamp/price pairs,
    /// which is easier to consume for most charting libraries.
    /// 
    /// # Arguments
    /// 
    /// * `token` - Token name (e.g., "kaspa")
    /// * `range` - Time range: "today", "7d", or "30d"
    /// * `resolution` - Data resolution: "1m", "5m", "15m", "30m", "1h", "4h", or "1d"
    /// 
    /// # Returns
    /// 
    /// TimeseriesResponse with data points containing timestamp and price.
    /// 
    /// # Example
    /// 
    /// ```rust,no_run
    /// // Get hourly data for the last 7 days
    /// let timeseries = ticker_service.get_timeseries("kaspa", "7d", "1h").await?;
    /// // Each point has: { timestamp: 1704067200, price: 0.04512 }
    /// ```
    pub async fn get_timeseries(
        &self,
        token: String,
        range: String,
        resolution: String,
    ) -> anyhow::Result<TimeseriesResponse> {
        let history = self.get_ticker_history(token.clone(), range.clone(), resolution.clone()).await?;
        
        let data: Vec<TimeseriesPoint> = history
            .data
            .into_iter()
            .map(|point| TimeseriesPoint {
                timestamp: point.timestamp,
                price: point.close,
            })
            .collect();

        Ok(TimeseriesResponse {
            token,
            range,
            resolution,
            data,
        })
    }

    /// Get timeseries data with all OHLCV values for advanced charts.
    /// 
    /// Similar to `get_timeseries` but returns full OHLCV data points.
    /// Use this when you need high/low/open/close for candlestick or advanced charts.
    pub async fn get_timeseries_ohlcv(
        &self,
        token: String,
        range: String,
        resolution: String,
    ) -> anyhow::Result<Vec<OhlcvPoint>> {
        let history = self.get_ticker_history(token, range, resolution).await?;
        Ok(history.data)
    }

    /// Get list of exchanges with their associated KRC20 tokens.
    /// 
    /// Returns all exchanges that have data available, with a list of tokens
    /// that are available on each exchange. This is useful for discovering
    /// which exchanges support which tokens.
    /// 
    /// # Returns
    /// 
    /// ExchangesResponse with a list of exchanges and their tokens.
    /// 
    /// # Example
    /// 
    /// ```rust,no_run
    /// let exchanges = ticker_service.get_exchanges().await?;
    /// // Returns: ExchangesResponse {
    /// //   exchanges: vec![
    /// //     ExchangeInfo { exchange: "ascendex", tokens: vec!["kaspa", "slow"], token_count: 2 },
    /// //     ExchangeInfo { exchange: "binance", tokens: vec!["kaspa"], token_count: 1 },
    /// //   ],
    /// //   count: 2
    /// // }
    /// ```
    pub async fn get_exchanges(&self) -> anyhow::Result<ExchangesResponse> {
        let cache_key = "v1:exchanges:list";

        // Check cache first (cache for 1 hour since this changes infrequently)
        if let Ok(Some(cached)) = self.cache_repo.get(cache_key).await {
            if let Ok(response) = serde_json::from_str::<ExchangesResponse>(&cached) {
                info!("Cache HIT: {}", cache_key);
                metrics::counter!("cache_operations_total", "operation" => "hit").increment(1);
                return Ok(response);
            }
        }
        metrics::counter!("cache_operations_total", "operation" => "miss").increment(1);

        // Try to use exchange index if available (fast path)
        if let Some(ref index) = self.exchange_index {
            if index.is_initialized().await {
                let exchange_names = index.get_exchanges().await;
                let mut exchanges = Vec::new();

                for exchange_name in exchange_names {
                    let tokens = index.get_tokens(&exchange_name).await;
                    exchanges.push(ExchangeInfo {
                        exchange: exchange_name,
                        token_count: tokens.len(),
                        tokens,
                    });
                }

                exchanges.sort_by(|a, b| a.exchange.cmp(&b.exchange));

                let response = ExchangesResponse {
                    count: exchanges.len(),
                    exchanges,
                };

                // Cache result (1 hour TTL)
                if let Ok(json) = serde_json::to_string(&response) {
                    let _ = self.cache_repo.set(cache_key, &json, 3600).await;
                }

                return Ok(response);
            }
        }

        // Fallback: use repository to discover (slower, requires API calls)
        let repo = self.get_repo();
        let data_dir = "data";
        let items = repo.list_directory(&self.default_repo, data_dir).await?;

        // Filter for directories (tokens) only
        let token_dirs: Vec<String> = items
            .into_iter()
            .filter(|item| item.item_type == ContentType::Dir)
            .map(|item| item.name)
            .collect();

        // Build a map of exchange -> tokens
        let mut exchange_map: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();

        // For each token, discover its exchanges
        for token in token_dirs {
            let token_path = format!("data/{}", token);
            match repo.list_directory(&self.default_repo, &token_path).await {
                Ok(exchange_items) => {
                    for exchange_item in exchange_items {
                        if exchange_item.item_type == ContentType::Dir {
                            let exchange_name = exchange_item.name;
                            exchange_map
                                .entry(exchange_name.clone())
                                .or_insert_with(Vec::new)
                                .push(token.clone());
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to list exchanges for token {}: {}", token, e);
                }
            }
        }

        // Convert map to sorted list of ExchangeInfo
        let mut exchanges: Vec<ExchangeInfo> = exchange_map
            .into_iter()
            .map(|(exchange, mut tokens)| {
                tokens.sort();
                ExchangeInfo {
                    exchange,
                    token_count: tokens.len(),
                    tokens,
                }
            })
            .collect();

        // Sort exchanges by name
        exchanges.sort_by(|a, b| a.exchange.cmp(&b.exchange));

        let response = ExchangesResponse {
            count: exchanges.len(),
            exchanges,
        };

        // Cache result (1 hour TTL)
        if let Ok(json) = serde_json::to_string(&response) {
            let _ = self.cache_repo.set(cache_key, &json, 3600).await;
        }

        Ok(response)
    }

    /// Get detailed information about a specific exchange with all its tokens and statistics.
    /// 
    /// Returns all tokens available on the specified exchange with their current
    /// statistics (price, volume, change, etc.). This is useful for exchange
    /// detail pages that need to display all tokens on a specific exchange.
    /// 
    /// # Arguments
    /// 
    /// * `exchange` - Exchange identifier (e.g., "ascendex", "binance")
    /// * `range` - Time range: "today", "7d", or "30d"
    /// 
    /// # Returns
    /// 
    /// ExchangeDetailResponse with exchange info and list of tokens with stats.
    /// 
    /// # Example
    /// 
    /// ```rust,no_run
    /// let detail = ticker_service.get_exchange_detail("ascendex", "today").await?;
    /// // Returns: ExchangeDetailResponse {
    /// //   exchange: "ascendex",
    /// //   tokens: vec![
    /// //     ExchangeTokenRow { token: "kaspa", last: Some(0.045), ... },
    /// //     ExchangeTokenRow { token: "slow", last: Some(0.000123), ... },
    /// //   ],
    /// //   count: 2
    /// // }
    /// ```
    pub async fn get_exchange_detail(
        &self,
        exchange: String,
        range: String,
    ) -> anyhow::Result<ExchangeDetailResponse> {
        let cache_key = format!("v1:exchange:{}:detail:{}", exchange, range);

        // Check cache first
        if let Ok(Some(cached)) = self.cache_repo.get(&cache_key).await {
            if let Ok(response) = serde_json::from_str::<ExchangeDetailResponse>(&cached) {
                info!("Cache HIT: {}", cache_key);
                metrics::counter!("cache_operations_total", "operation" => "hit").increment(1);
                return Ok(response);
            }
        }
        metrics::counter!("cache_operations_total", "operation" => "miss").increment(1);

        // Try to use exchange index if available (fast path)
        let tokens_with_exchange = if let Some(ref index) = self.exchange_index {
            if index.is_initialized().await {
                let tokens = index.get_tokens(&exchange).await;
                if tokens.is_empty() {
                    anyhow::bail!("Exchange not found: {}", exchange);
                }
                tokens
            } else {
                vec![] // Index not initialized, fall through to repository method
            }
        } else {
            vec![] // No index, fall through to repository method
        };

        // Fallback: use repository to discover tokens for this exchange
        let tokens_with_exchange = if tokens_with_exchange.is_empty() {
            let repo = self.get_repo();
            let data_dir = "data";
            let items = repo.list_directory(&self.default_repo, data_dir).await?;

            // Filter for directories (tokens) only
            let token_dirs: Vec<String> = items
                .into_iter()
                .filter(|item| item.item_type == ContentType::Dir)
                .map(|item| item.name)
                .collect();

            // For each token, check if it has this exchange
            let mut found_tokens = Vec::new();
            for token in token_dirs {
                let token_path = format!("data/{}", token);
                match repo.list_directory(&self.default_repo, &token_path).await {
                    Ok(exchange_items) => {
                        for exchange_item in exchange_items {
                            if exchange_item.item_type == ContentType::Dir
                                && exchange_item.name.to_lowercase() == exchange.to_lowercase()
                            {
                                found_tokens.push(token);
                                break; // Found the exchange for this token, move to next token
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to list exchanges for token {}: {}", token, e);
                    }
                }
            }

            if found_tokens.is_empty() {
                anyhow::bail!("Exchange not found: {}", exchange);
            }
            found_tokens
        } else {
            tokens_with_exchange
        };

        // Calculate date range
        let (start_date, end_date) = Self::calculate_date_range(&range);

        // Fetch stats for each token on this exchange concurrently
        let repo = self.get_repo();
        let mut token_rows = Vec::new();
        let fetches: Vec<anyhow::Result<ExchangeTokenRow>> = futures::stream::iter(tokens_with_exchange)
            .map(|token| {
                let repo = repo.clone();
                let config = self.default_repo.clone();
                let exchange_name = exchange.clone();
                let start = start_date;
                let end = end_date;
                async move {
                    let stats = Self::fetch_exchange_stats(
                        repo,
                        config,
                        token.clone(),
                        exchange_name,
                        start,
                        end,
                    )
                    .await?;
                    
                    // Convert ExchangeStats to ExchangeTokenRow
                    Ok(ExchangeTokenRow {
                        token,
                        last: stats.last,
                        high: stats.high,
                        low: stats.low,
                        volume_24h: stats.volume_24h,
                        change_pct: stats.change_pct,
                        data_points: stats.data_points,
                    })
                }
            })
            .buffer_unordered(10)
            .collect::<Vec<anyhow::Result<ExchangeTokenRow>>>()
            .await;

        for result in fetches {
            match result {
                Ok(row) => {
                    // Only include tokens that have data
                    if row.data_points > 0 {
                        token_rows.push(row);
                    }
                }
                Err(e) => warn!("Failed to fetch token stats: {}", e),
            }
        }

        // Sort tokens alphabetically
        token_rows.sort_by(|a, b| a.token.cmp(&b.token));

        let response = ExchangeDetailResponse {
            exchange: exchange.clone(),
            range: range.clone(),
            timestamp: Utc::now().to_rfc3339(),
            count: token_rows.len(),
            tokens: token_rows,
        };

        // Cache result (5 min TTL)
        if let Ok(json) = serde_json::to_string(&response) {
            let _ = self.cache_repo.set(&cache_key, &json, 300).await;
        }

        Ok(response)
    }
}
