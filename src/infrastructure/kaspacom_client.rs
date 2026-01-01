//! HTTP client for Kaspa.com L1 Marketplace API.
//!
//! This client is used for fetching data from the remote API when cache misses occur.
//! It handles ticker normalization (uppercase), retry logic, and error handling.

use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;
use tokio_retry::strategy::{jitter, ExponentialBackoff};
use tokio_retry::Retry;
use tracing::{debug, info};

/// Base URL for Kaspa.com API
const BASE_URL: &str = "https://api.kaspa.com";

/// Request timeout in seconds
const REQUEST_TIMEOUT_SECS: u64 = 30;

/// Maximum retry attempts
const MAX_RETRIES: usize = 3;

/// Kaspa.com L1 Marketplace API Client
///
/// This client is used only for fetching fresh data from the remote API.
/// All responses should be cached via the CacheService before being returned.
#[derive(Clone)]
pub struct KaspaComClient {
    client: Client,
    base_url: String,
}

impl KaspaComClient {
    /// Create a new client with default configuration
    pub fn new() -> Self {
        Self::with_base_url(BASE_URL)
    }

    /// Create a new client with a custom base URL (for testing)
    pub fn with_base_url(base_url: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .user_agent("KaspaDevCacheProxy/1.0")
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            base_url: base_url.to_string(),
        }
    }

    /// Normalize ticker to uppercase for API compatibility.
    ///
    /// The Kaspa.com API requires uppercase tickers. This method ensures
    /// all ticker parameters are properly formatted.
    pub fn normalize_ticker(ticker: &str) -> String {
        ticker.to_uppercase()
    }

    /// Internal method to make a GET request with retry logic
    async fn get(&self, path: &str) -> Result<Value> {
        let url = format!("{}{}", self.base_url, path);
        debug!("Fetching from Kaspa.com API: {}", url);

        let retry_strategy = ExponentialBackoff::from_millis(100)
            .map(jitter)
            .take(MAX_RETRIES);

        let response = Retry::spawn(retry_strategy, || async {
            self.client
                .get(&url)
                .header("Accept", "application/json")
                .send()
                .await
        })
        .await
        .with_context(|| format!("Failed to fetch from {}", url))?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            anyhow::bail!(
                "API request failed with status {}: {}",
                status,
                error_body
            );
        }

        let json: Value = response
            .json()
            .await
            .with_context(|| format!("Failed to parse JSON from {}", url))?;

        Ok(json)
    }

    /// Internal method to make a POST request with retry logic
    async fn post(&self, path: &str, body: &Value) -> Result<Value> {
        let url = format!("{}{}", self.base_url, path);
        debug!("POST to Kaspa.com API: {}", url);

        let retry_strategy = ExponentialBackoff::from_millis(100)
            .map(jitter)
            .take(MAX_RETRIES);

        let response = Retry::spawn(retry_strategy, || async {
            self.client
                .post(&url)
                .header("Accept", "application/json")
                .header("Content-Type", "application/json")
                .json(body)
                .send()
                .await
        })
        .await
        .with_context(|| format!("Failed to POST to {}", url))?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            anyhow::bail!(
                "API request failed with status {}: {}",
                status,
                error_body
            );
        }

        let json: Value = response
            .json()
            .await
            .with_context(|| format!("Failed to parse JSON from {}", url))?;

        Ok(json)
    }

    // ========================================================================
    // KRC20 Token Endpoints
    // ========================================================================

    /// Fetch trade statistics for KRC20 tokens
    ///
    /// GET /api/trade-stats?timeFrame=6h&ticker=TICKER
    pub async fn fetch_trade_stats(
        &self,
        time_frame: &str,
        ticker: Option<&str>,
    ) -> Result<Value> {
        let mut path = format!("/api/trade-stats?timeFrame={}", time_frame);
        if let Some(t) = ticker {
            path.push_str(&format!("&ticker={}", Self::normalize_ticker(t)));
        }
        info!("Fetching trade stats: {}", path);
        self.get(&path).await
    }

    /// Fetch floor prices for KRC20 tokens
    ///
    /// GET /api/floor-price?ticker=TICKER
    pub async fn fetch_floor_prices(&self, ticker: Option<&str>) -> Result<Value> {
        let path = match ticker {
            Some(t) => format!("/api/floor-price?ticker={}", Self::normalize_ticker(t)),
            None => "/api/floor-price".to_string(),
        };
        info!("Fetching floor prices: {}", path);
        self.get(&path).await
    }

    /// Fetch recently sold orders
    ///
    /// GET /api/sold-orders?ticker=TICKER&minutes=60
    pub async fn fetch_sold_orders(
        &self,
        ticker: Option<&str>,
        minutes: Option<f64>,
    ) -> Result<Value> {
        let mut path = "/api/sold-orders".to_string();
        let mut has_params = false;

        if let Some(t) = ticker {
            path.push_str(&format!("?ticker={}", Self::normalize_ticker(t)));
            has_params = true;
        }

        if let Some(m) = minutes {
            let sep = if has_params { "&" } else { "?" };
            path.push_str(&format!("{}minutes={}", sep, m));
        }

        info!("Fetching sold orders: {}", path);
        self.get(&path).await
    }

    /// Fetch the most recent sold order
    ///
    /// GET /api/last-order-sold
    pub async fn fetch_last_order_sold(&self) -> Result<Value> {
        info!("Fetching last order sold");
        self.get("/api/last-order-sold").await
    }

    /// Fetch hot minting tokens
    ///
    /// GET /api/hot-mints?timeInterval=1h
    pub async fn fetch_hot_mints(&self, time_interval: &str) -> Result<Value> {
        let path = format!("/api/hot-mints?timeInterval={}", time_interval);
        info!("Fetching hot mints: {}", path);
        self.get(&path).await
    }

    /// Fetch comprehensive token info
    ///
    /// GET /api/token-info/:ticker
    pub async fn fetch_token_info(&self, ticker: &str) -> Result<Value> {
        let path = format!("/api/token-info/{}", Self::normalize_ticker(ticker));
        info!("Fetching token info: {}", path);
        self.get(&path).await
    }

    /// Fetch token logos
    ///
    /// GET /api/tokens-logos?ticker=TICKER
    pub async fn fetch_tokens_logos(&self, ticker: Option<&str>) -> Result<Value> {
        let path = match ticker {
            Some(t) => format!("/api/tokens-logos?ticker={}", Self::normalize_ticker(t)),
            None => "/api/tokens-logos".to_string(),
        };
        info!("Fetching token logos: {}", path);
        self.get(&path).await
    }

    /// Fetch tickers with active open orders
    ///
    /// GET /api/open-orders
    pub async fn fetch_open_orders(&self) -> Result<Value> {
        info!("Fetching open orders");
        self.get("/api/open-orders").await
    }

    /// Fetch historical price/volume data
    ///
    /// GET /api/historical-data?timeFrame=7d&ticker=TICKER
    pub async fn fetch_historical_data(&self, time_frame: &str, ticker: &str) -> Result<Value> {
        let path = format!(
            "/api/historical-data?timeFrame={}&ticker={}",
            time_frame,
            Self::normalize_ticker(ticker)
        );
        info!("Fetching historical data: {}", path);
        self.get(&path).await
    }

    // ========================================================================
    // KRC721 NFT Endpoints
    // ========================================================================

    /// Fetch recent NFT mints
    ///
    /// GET /api/krc721/mint?ticker=TICKER
    pub async fn fetch_krc721_mints(&self, ticker: Option<&str>) -> Result<Value> {
        let path = match ticker {
            Some(t) => format!("/api/krc721/mint?ticker={}", Self::normalize_ticker(t)),
            None => "/api/krc721/mint".to_string(),
        };
        info!("Fetching KRC721 mints: {}", path);
        self.get(&path).await
    }

    /// Fetch sold NFT orders
    ///
    /// GET /api/krc721/sold-orders?ticker=TICKER&minutes=60
    pub async fn fetch_krc721_sold_orders(
        &self,
        ticker: Option<&str>,
        minutes: Option<f64>,
    ) -> Result<Value> {
        let mut path = "/api/krc721/sold-orders".to_string();
        let mut has_params = false;

        if let Some(t) = ticker {
            path.push_str(&format!("?ticker={}", Self::normalize_ticker(t)));
            has_params = true;
        }

        if let Some(m) = minutes {
            let sep = if has_params { "&" } else { "?" };
            path.push_str(&format!("{}minutes={}", sep, m));
        }

        info!("Fetching KRC721 sold orders: {}", path);
        self.get(&path).await
    }

    /// Fetch listed NFT orders
    ///
    /// GET /api/krc721/listed-orders?ticker=TICKER
    pub async fn fetch_krc721_listed_orders(&self, ticker: Option<&str>) -> Result<Value> {
        let path = match ticker {
            Some(t) => format!("/api/krc721/listed-orders?ticker={}", Self::normalize_ticker(t)),
            None => "/api/krc721/listed-orders".to_string(),
        };
        info!("Fetching KRC721 listed orders: {}", path);
        self.get(&path).await
    }

    /// Fetch NFT trade statistics
    ///
    /// GET /api/krc721/trade-stats?timeFrame=6h&ticker=TICKER
    pub async fn fetch_krc721_trade_stats(
        &self,
        time_frame: &str,
        ticker: Option<&str>,
    ) -> Result<Value> {
        let mut path = format!("/api/krc721/trade-stats?timeFrame={}", time_frame);
        if let Some(t) = ticker {
            path.push_str(&format!("&ticker={}", Self::normalize_ticker(t)));
        }
        info!("Fetching KRC721 trade stats: {}", path);
        self.get(&path).await
    }

    /// Fetch hot minting NFT collections
    ///
    /// GET /api/krc721/hot-mints?timeInterval=1h
    pub async fn fetch_krc721_hot_mints(&self, time_interval: &str) -> Result<Value> {
        let path = format!("/api/krc721/hot-mints?timeInterval={}", time_interval);
        info!("Fetching KRC721 hot mints: {}", path);
        self.get(&path).await
    }

    /// Fetch NFT floor prices
    ///
    /// GET /api/krc721/floor-price?ticker=TICKER
    pub async fn fetch_krc721_floor_prices(&self, ticker: Option<&str>) -> Result<Value> {
        let path = match ticker {
            Some(t) => format!("/api/krc721/floor-price?ticker={}", Self::normalize_ticker(t)),
            None => "/api/krc721/floor-price".to_string(),
        };
        info!("Fetching KRC721 floor prices: {}", path);
        self.get(&path).await
    }

    /// Fetch filtered NFT tokens with pagination
    ///
    /// POST /api/krc721/tokens
    pub async fn fetch_krc721_tokens(&self, filter: &Value) -> Result<Value> {
        info!("Fetching KRC721 tokens with filter");
        self.post("/api/krc721/tokens", filter).await
    }

    // ========================================================================
    // KNS Domain Endpoints
    // ========================================================================

    /// Fetch sold KNS domain orders
    ///
    /// GET /api/kns/sold-orders?minutes=60
    pub async fn fetch_kns_sold_orders(&self, minutes: Option<f64>) -> Result<Value> {
        let path = match minutes {
            Some(m) => format!("/api/kns/sold-orders?minutes={}", m),
            None => "/api/kns/sold-orders".to_string(),
        };
        info!("Fetching KNS sold orders: {}", path);
        self.get(&path).await
    }

    /// Fetch KNS trade statistics
    ///
    /// GET /api/kns/trade-stats?timeFrame=6h&asset=domain.kas
    pub async fn fetch_kns_trade_stats(
        &self,
        time_frame: &str,
        asset: Option<&str>,
    ) -> Result<Value> {
        let mut path = format!("/api/kns/trade-stats?timeFrame={}", time_frame);
        if let Some(a) = asset {
            path.push_str(&format!("&asset={}", a));
        }
        info!("Fetching KNS trade stats: {}", path);
        self.get(&path).await
    }

    /// Fetch listed KNS domains
    ///
    /// GET /api/kns/listed-orders
    pub async fn fetch_kns_listed_orders(&self) -> Result<Value> {
        info!("Fetching KNS listed orders");
        self.get("/api/kns/listed-orders").await
    }

    // ========================================================================
    // KRC721 Collection & Metadata Endpoints (External APIs)
    // ========================================================================

    /// Fetch KRC721 collection info from api.kaspa.com
    ///
    /// GET /krc721/{ticker}
    pub async fn fetch_krc721_collection_info(&self, ticker: &str) -> Result<Value> {
        let path = format!("/krc721/{}", Self::normalize_ticker(ticker));
        info!("Fetching KRC721 collection info: {}", path);
        self.get(&path).await
    }

    /// Fetch NFT metadata from krc721.stream cache
    ///
    /// GET https://cache.krc721.stream/krc721/mainnet/metadata/{ticker}/{tokenId}
    pub async fn fetch_nft_metadata(&self, ticker: &str, token_id: i64) -> Result<Value> {
        let url = format!(
            "https://cache.krc721.stream/krc721/mainnet/metadata/{}/{}",
            Self::normalize_ticker(ticker),
            token_id
        );
        info!("Fetching NFT metadata from krc721.stream: {}", url);

        let response = self.client
            .get(&url)
            .header("Accept", "application/json")
            .send()
            .await
            .with_context(|| format!("Failed to fetch NFT metadata from {}", url))?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            anyhow::bail!("NFT metadata request failed with status {}: {}", status, error_body);
        }

        let json: Value = response
            .json()
            .await
            .with_context(|| format!("Failed to parse NFT metadata JSON from {}", url))?;

        Ok(json)
    }

    /// Get optimized NFT image URL from krc721.stream CDN
    ///
    /// Returns the CDN URL directly without fetching
    pub fn get_nft_image_url(ticker: &str, token_id: i64) -> String {
        format!(
            "https://cache.krc721.stream/krc721/mainnet/optimized/{}/{}",
            Self::normalize_ticker(ticker),
            token_id
        )
    }
}

impl Default for KaspaComClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_ticker() {
        assert_eq!(KaspaComClient::normalize_ticker("slow"), "SLOW");
        assert_eq!(KaspaComClient::normalize_ticker("SLOW"), "SLOW");
        assert_eq!(KaspaComClient::normalize_ticker("Nacho"), "NACHO");
        assert_eq!(KaspaComClient::normalize_ticker("kasper"), "KASPER");
    }

    #[test]
    fn test_client_creation() {
        let client = KaspaComClient::new();
        assert_eq!(client.base_url, BASE_URL);

        let custom_client = KaspaComClient::with_base_url("http://localhost:8080");
        assert_eq!(custom_client.base_url, "http://localhost:8080");
    }

    #[test]
    fn test_normalize_ticker_edge_cases() {
        // Test empty string
        assert_eq!(KaspaComClient::normalize_ticker(""), "");
        
        // Test mixed case
        assert_eq!(KaspaComClient::normalize_ticker("sLoW"), "SLOW");
        
        // Test with numbers
        assert_eq!(KaspaComClient::normalize_ticker("token123"), "TOKEN123");
        
        // Test already uppercase
        assert_eq!(KaspaComClient::normalize_ticker("KASPA"), "KASPA");
    }
}
