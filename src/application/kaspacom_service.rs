//! Kaspa.com marketplace data service with cache-first approach.
//!
//! This service provides access to all Kaspa.com API endpoints with automatic
//! tiered caching (Redis + Parquet) to reduce load on the remote API.

use crate::application::cache_service::{ttl, CacheService};
use crate::domain::{
    FloorPriceEntry, HistoricalDataResponse, HotMint, KnsOrder, KnsListedOrdersResponse,
    KnsTradeStatsResponse, Krc721CollectionInfo, NftMetadata, NftMint, NftOrder, NftTokensResponse,
    NftTradeStatsResponse, OpenOrdersResponse, SoldOrder, TokenInfo, TokenLogo, TokensConfig,
    TradeStatsResponse,
};
use crate::infrastructure::{cache_categories, KaspaComClient};
use anyhow::Result;
use serde_json::Value;
use std::sync::Arc;
use tracing::info;

/// Kaspa.com marketplace data service
///
/// Provides cache-first access to all Kaspa.com API endpoints.
/// Data is fetched from local cache when available, with automatic
/// refresh from the remote API on cache miss.
pub struct KaspaComService {
    cache: Arc<CacheService>,
    tokens_config: TokensConfig,
}

impl KaspaComService {
    /// Create a new service instance
    pub fn new(cache: Arc<CacheService>, tokens_config: TokensConfig) -> Self {
        info!(
            "Initialized KaspaComService with {} configured tokens",
            tokens_config.get_tokens().len()
        );
        Self {
            cache,
            tokens_config,
        }
    }

    /// Get the tokens configuration
    pub fn tokens_config(&self) -> &TokensConfig {
        &self.tokens_config
    }

    /// Get cache statistics
    pub fn get_cache_stats(&self) -> Result<crate::infrastructure::CacheStats> {
        self.cache.get_stats()
    }

    // ========================================================================
    // KRC20 Token Endpoints
    // ========================================================================

    /// Get trade statistics for KRC20 tokens
    pub async fn get_trade_stats(
        &self,
        time_frame: &str,
        ticker: Option<&str>,
    ) -> Result<TradeStatsResponse> {
        let ticker = ticker.map(KaspaComClient::normalize_ticker);
        let cache_key = match &ticker {
            Some(t) => format!("kaspa:trade_stats:{}:{}", time_frame, t),
            None => format!("kaspa:trade_stats:{}", time_frame),
        };
        let parquet_key = match &ticker {
            Some(t) => format!("{}_{}", time_frame, t),
            None => time_frame.to_string(),
        };

        let client = self.cache.client().clone();
        let tf = time_frame.to_string();
        let tk = ticker.clone();

        self.cache
            .get_cached(
                &cache_key,
                cache_categories::TRADE_STATS,
                &parquet_key,
                ttl::WARM_REDIS_SECS,
                ttl::WARM_PARQUET_SECS,
                || async move { client.fetch_trade_stats(&tf, tk.as_deref()).await },
            )
            .await
    }

    /// Get floor prices for KRC20 tokens
    pub async fn get_floor_prices(&self, ticker: Option<&str>) -> Result<Vec<FloorPriceEntry>> {
        let ticker = ticker.map(KaspaComClient::normalize_ticker);
        let cache_key = match &ticker {
            Some(t) => format!("kaspa:floor_price:{}", t),
            None => "kaspa:floor_price:all".to_string(),
        };
        let parquet_key = ticker.as_deref().unwrap_or("all").to_string();

        let client = self.cache.client().clone();
        let tk = ticker.clone();

        self.cache
            .get_cached(
                &cache_key,
                cache_categories::FLOOR_PRICES,
                &parquet_key,
                ttl::HOT_REDIS_SECS,
                ttl::HOT_PARQUET_SECS,
                || async move { client.fetch_floor_prices(tk.as_deref()).await },
            )
            .await
    }

    /// Get recently sold orders
    pub async fn get_sold_orders(
        &self,
        ticker: Option<&str>,
        minutes: Option<f64>,
    ) -> Result<Vec<SoldOrder>> {
        let ticker = ticker.map(KaspaComClient::normalize_ticker);
        let mins = minutes.unwrap_or(60.0);
        let cache_key = match &ticker {
            Some(t) => format!("kaspa:sold_orders:{}:{}", t, mins as i64),
            None => format!("kaspa:sold_orders:all:{}", mins as i64),
        };
        let parquet_key = match &ticker {
            Some(t) => format!("{}_{}", t, mins as i64),
            None => format!("all_{}", mins as i64),
        };

        let client = self.cache.client().clone();
        let tk = ticker.clone();

        self.cache
            .get_cached(
                &cache_key,
                cache_categories::ORDERS,
                &parquet_key,
                ttl::HOT_REDIS_SECS,
                ttl::HOT_PARQUET_SECS,
                || async move { client.fetch_sold_orders(tk.as_deref(), Some(mins)).await },
            )
            .await
    }

    /// Get the most recent sold order
    pub async fn get_last_order_sold(&self) -> Result<SoldOrder> {
        let cache_key = "kaspa:last_order_sold";
        let parquet_key = "last";

        let client = self.cache.client().clone();

        self.cache
            .get_cached(
                cache_key,
                cache_categories::ORDERS,
                parquet_key,
                ttl::HOT_REDIS_SECS,
                ttl::HOT_PARQUET_SECS,
                || async move { client.fetch_last_order_sold().await },
            )
            .await
    }

    /// Get hot minting tokens
    pub async fn get_hot_mints(&self, time_interval: &str) -> Result<Vec<HotMint>> {
        let cache_key = format!("kaspa:hot_mints:{}", time_interval);
        let parquet_key = time_interval.to_string();

        let client = self.cache.client().clone();
        let ti = time_interval.to_string();

        self.cache
            .get_cached(
                &cache_key,
                cache_categories::HOT_MINTS,
                &parquet_key,
                ttl::WARM_REDIS_SECS,
                ttl::WARM_PARQUET_SECS,
                || async move { client.fetch_hot_mints(&ti).await },
            )
            .await
    }

    /// Get comprehensive token info
    pub async fn get_token_info(&self, ticker: &str) -> Result<TokenInfo> {
        let ticker = KaspaComClient::normalize_ticker(ticker);
        let cache_key = format!("kaspa:token_info:{}", ticker);
        let parquet_key = ticker.clone();

        let client = self.cache.client().clone();
        let tk = ticker.clone();

        self.cache
            .get_cached(
                &cache_key,
                cache_categories::TOKEN_INFO,
                &parquet_key,
                ttl::COLD_REDIS_SECS,
                ttl::COLD_PARQUET_SECS,
                || async move { client.fetch_token_info(&tk).await },
            )
            .await
    }

    /// Get token logos
    pub async fn get_tokens_logos(&self, ticker: Option<&str>) -> Result<Vec<TokenLogo>> {
        let ticker = ticker.map(KaspaComClient::normalize_ticker);
        let cache_key = match &ticker {
            Some(t) => format!("kaspa:logos:{}", t),
            None => "kaspa:logos:all".to_string(),
        };
        let parquet_key = ticker.as_deref().unwrap_or("all").to_string();

        let client = self.cache.client().clone();
        let tk = ticker.clone();

        self.cache
            .get_cached(
                &cache_key,
                cache_categories::LOGOS,
                &parquet_key,
                ttl::STATIC_REDIS_SECS,
                ttl::STATIC_PARQUET_SECS,
                || async move { client.fetch_tokens_logos(tk.as_deref()).await },
            )
            .await
    }

    /// Get tickers with active open orders
    pub async fn get_open_orders(&self) -> Result<OpenOrdersResponse> {
        let cache_key = "kaspa:open_orders";
        let parquet_key = "active";

        let client = self.cache.client().clone();

        self.cache
            .get_cached(
                cache_key,
                cache_categories::ORDERS,
                parquet_key,
                ttl::HOT_REDIS_SECS,
                ttl::HOT_PARQUET_SECS,
                || async move { client.fetch_open_orders().await },
            )
            .await
    }

    /// Get historical price/volume data
    pub async fn get_historical_data(
        &self,
        time_frame: &str,
        ticker: &str,
    ) -> Result<HistoricalDataResponse> {
        let ticker = KaspaComClient::normalize_ticker(ticker);
        let cache_key = format!("kaspa:historical:{}:{}", ticker, time_frame);
        let parquet_key = format!("{}_{}", ticker, time_frame);

        let client = self.cache.client().clone();
        let tk = ticker.clone();
        let tf = time_frame.to_string();

        self.cache
            .get_cached(
                &cache_key,
                cache_categories::HISTORICAL,
                &parquet_key,
                ttl::COLD_REDIS_SECS,
                ttl::COLD_PARQUET_SECS,
                || async move { client.fetch_historical_data(&tf, &tk).await },
            )
            .await
    }

    // ========================================================================
    // KRC721 NFT Endpoints
    // ========================================================================

    /// Get recent NFT mints
    pub async fn get_krc721_mints(&self, ticker: Option<&str>) -> Result<Vec<NftMint>> {
        let ticker = ticker.map(KaspaComClient::normalize_ticker);
        let cache_key = match &ticker {
            Some(t) => format!("kaspa:krc721:mints:{}", t),
            None => "kaspa:krc721:mints:all".to_string(),
        };
        let parquet_key = ticker.as_deref().unwrap_or("all").to_string();

        let client = self.cache.client().clone();
        let tk = ticker.clone();

        self.cache
            .get_cached(
                &cache_key,
                cache_categories::KRC721,
                &format!("mints_{}", parquet_key),
                ttl::WARM_REDIS_SECS,
                ttl::WARM_PARQUET_SECS,
                || async move { client.fetch_krc721_mints(tk.as_deref()).await },
            )
            .await
    }

    /// Get sold NFT orders
    pub async fn get_krc721_sold_orders(
        &self,
        ticker: Option<&str>,
        minutes: Option<f64>,
    ) -> Result<Vec<NftOrder>> {
        let ticker = ticker.map(KaspaComClient::normalize_ticker);
        let mins = minutes.unwrap_or(60.0);
        let cache_key = match &ticker {
            Some(t) => format!("kaspa:krc721:sold:{}:{}", t, mins as i64),
            None => format!("kaspa:krc721:sold:all:{}", mins as i64),
        };
        let parquet_key = match &ticker {
            Some(t) => format!("sold_{}_{}", t, mins as i64),
            None => format!("sold_all_{}", mins as i64),
        };

        let client = self.cache.client().clone();
        let tk = ticker.clone();

        self.cache
            .get_cached(
                &cache_key,
                cache_categories::KRC721,
                &parquet_key,
                ttl::HOT_REDIS_SECS,
                ttl::HOT_PARQUET_SECS,
                || async move { client.fetch_krc721_sold_orders(tk.as_deref(), Some(mins)).await },
            )
            .await
    }

    /// Get listed NFT orders
    pub async fn get_krc721_listed_orders(&self, ticker: Option<&str>) -> Result<Vec<NftOrder>> {
        let ticker = ticker.map(KaspaComClient::normalize_ticker);
        let cache_key = match &ticker {
            Some(t) => format!("kaspa:krc721:listed:{}", t),
            None => "kaspa:krc721:listed:all".to_string(),
        };
        let parquet_key = match &ticker {
            Some(t) => format!("listed_{}", t),
            None => "listed_all".to_string(),
        };

        let client = self.cache.client().clone();
        let tk = ticker.clone();

        self.cache
            .get_cached(
                &cache_key,
                cache_categories::KRC721,
                &parquet_key,
                ttl::HOT_REDIS_SECS,
                ttl::HOT_PARQUET_SECS,
                || async move { client.fetch_krc721_listed_orders(tk.as_deref()).await },
            )
            .await
    }

    /// Get NFT trade statistics
    pub async fn get_krc721_trade_stats(
        &self,
        time_frame: &str,
        ticker: Option<&str>,
    ) -> Result<NftTradeStatsResponse> {
        let ticker = ticker.map(KaspaComClient::normalize_ticker);
        let cache_key = match &ticker {
            Some(t) => format!("kaspa:krc721:stats:{}:{}", time_frame, t),
            None => format!("kaspa:krc721:stats:{}", time_frame),
        };
        let parquet_key = match &ticker {
            Some(t) => format!("stats_{}_{}", time_frame, t),
            None => format!("stats_{}", time_frame),
        };

        let client = self.cache.client().clone();
        let tf = time_frame.to_string();
        let tk = ticker.clone();

        self.cache
            .get_cached(
                &cache_key,
                cache_categories::KRC721,
                &parquet_key,
                ttl::WARM_REDIS_SECS,
                ttl::WARM_PARQUET_SECS,
                || async move { client.fetch_krc721_trade_stats(&tf, tk.as_deref()).await },
            )
            .await
    }

    /// Get hot minting NFT collections
    pub async fn get_krc721_hot_mints(&self, time_interval: &str) -> Result<Vec<HotMint>> {
        let cache_key = format!("kaspa:krc721:hot_mints:{}", time_interval);
        let parquet_key = format!("hot_mints_{}", time_interval);

        let client = self.cache.client().clone();
        let ti = time_interval.to_string();

        self.cache
            .get_cached(
                &cache_key,
                cache_categories::KRC721,
                &parquet_key,
                ttl::WARM_REDIS_SECS,
                ttl::WARM_PARQUET_SECS,
                || async move { client.fetch_krc721_hot_mints(&ti).await },
            )
            .await
    }

    /// Get NFT floor prices
    pub async fn get_krc721_floor_prices(&self, ticker: Option<&str>) -> Result<Vec<FloorPriceEntry>> {
        let ticker = ticker.map(KaspaComClient::normalize_ticker);
        let cache_key = match &ticker {
            Some(t) => format!("kaspa:krc721:floor:{}", t),
            None => "kaspa:krc721:floor:all".to_string(),
        };
        let parquet_key = match &ticker {
            Some(t) => format!("floor_{}", t),
            None => "floor_all".to_string(),
        };

        let client = self.cache.client().clone();
        let tk = ticker.clone();

        self.cache
            .get_cached(
                &cache_key,
                cache_categories::KRC721,
                &parquet_key,
                ttl::HOT_REDIS_SECS,
                ttl::HOT_PARQUET_SECS,
                || async move { client.fetch_krc721_floor_prices(tk.as_deref()).await },
            )
            .await
    }

    /// Get filtered NFT tokens with pagination
    pub async fn get_krc721_tokens(&self, filter: &Value) -> Result<NftTokensResponse> {
        // For filtered queries, we don't cache as the filter varies too much
        // In production, you might want to cache common filter combinations
        let client = self.cache.client();
        let value = client.fetch_krc721_tokens(filter).await?;
        Ok(serde_json::from_value(value)?)
    }

    /// Get KRC721 collection info (holders, supply, rarity)
    pub async fn get_krc721_collection_info(&self, ticker: &str) -> Result<Krc721CollectionInfo> {
        let normalized = ticker.to_uppercase();
        let cache_key = format!("kaspa:krc721:collection:{}", normalized);
        let parquet_key = format!("collection_{}", normalized);

        let client = self.cache.client().clone();
        let ticker_clone = normalized.clone();

        self.cache
            .get_cached(
                &cache_key,
                cache_categories::KRC721,
                &parquet_key,
                ttl::WARM_REDIS_SECS,
                ttl::WARM_PARQUET_SECS,
                || async move { client.fetch_krc721_collection_info(&ticker_clone).await },
            )
            .await
    }

    /// Get NFT metadata from krc721.stream
    pub async fn get_nft_metadata(&self, ticker: &str, token_id: i64) -> Result<NftMetadata> {
        // Metadata is relatively static, so we can cache it for longer
        let normalized = ticker.to_uppercase();
        let cache_key = format!("kaspa:krc721:metadata:{}:{}", normalized, token_id);
        let parquet_key = format!("metadata_{}_{}", normalized, token_id);

        let client = self.cache.client().clone();
        let ticker_clone = normalized.clone();

        self.cache
            .get_cached(
                &cache_key,
                cache_categories::KRC721,
                &parquet_key,
                ttl::COLD_REDIS_SECS, // Longer TTL for metadata
                ttl::COLD_PARQUET_SECS,
                || async move { client.fetch_nft_metadata(&ticker_clone, token_id).await },
            )
            .await
    }

    // ========================================================================
    // KNS Domain Endpoints
    // ========================================================================

    /// Get sold KNS domain orders
    pub async fn get_kns_sold_orders(&self, minutes: Option<f64>) -> Result<Vec<KnsOrder>> {
        let mins = minutes.unwrap_or(60.0);
        let cache_key = format!("kaspa:kns:sold:{}", mins as i64);
        let parquet_key = format!("sold_{}", mins as i64);

        let client = self.cache.client().clone();

        self.cache
            .get_cached(
                &cache_key,
                cache_categories::KNS,
                &parquet_key,
                ttl::HOT_REDIS_SECS,
                ttl::HOT_PARQUET_SECS,
                || async move { client.fetch_kns_sold_orders(Some(mins)).await },
            )
            .await
    }

    /// Get KNS trade statistics
    pub async fn get_kns_trade_stats(
        &self,
        time_frame: &str,
        asset: Option<&str>,
    ) -> Result<KnsTradeStatsResponse> {
        let cache_key = match asset {
            Some(a) => format!("kaspa:kns:stats:{}:{}", time_frame, a),
            None => format!("kaspa:kns:stats:{}", time_frame),
        };
        let parquet_key = match asset {
            Some(a) => format!("stats_{}_{}", time_frame, a),
            None => format!("stats_{}", time_frame),
        };

        let client = self.cache.client().clone();
        let tf = time_frame.to_string();
        let ast = asset.map(|s| s.to_string());

        self.cache
            .get_cached(
                &cache_key,
                cache_categories::KNS,
                &parquet_key,
                ttl::WARM_REDIS_SECS,
                ttl::WARM_PARQUET_SECS,
                || async move { client.fetch_kns_trade_stats(&tf, ast.as_deref()).await },
            )
            .await
    }

    /// Get listed KNS domains
    pub async fn get_kns_listed_orders(&self) -> Result<Vec<KnsOrder>> {
        let cache_key = "kaspa:kns:listed";
        let parquet_key = "listed";

        let client = self.cache.client().clone();

        // Fetch wrapper and extract orders
        let wrapper: KnsListedOrdersResponse = self.cache
            .get_cached(
                cache_key,
                cache_categories::KNS,
                parquet_key,
                ttl::HOT_REDIS_SECS,
                ttl::HOT_PARQUET_SECS,
                || async move { client.fetch_kns_listed_orders().await },
            )
            .await?;
        
        Ok(wrapper.orders)
    }

    // ========================================================================
    // Token Configuration Helpers
    // ========================================================================

    /// Get list of all configured tokens
    pub fn get_configured_tokens(&self) -> Vec<String> {
        self.tokens_config.get_tokens()
    }

    /// Get exchanges for a specific token
    pub fn get_token_exchanges(&self, token: &str) -> Option<Vec<String>> {
        self.tokens_config.get_exchanges(token).cloned()
    }

    /// Check if a token is configured
    pub fn is_token_configured(&self, token: &str) -> bool {
        self.tokens_config.has_token(token)
    }
}
