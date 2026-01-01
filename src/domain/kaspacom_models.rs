//! Domain models for Kaspa.com L1 Marketplace API responses.
//!
//! These models represent the data structures returned by the Kaspa.com API
//! and are designed to be compatible with both JSON serialization and Parquet storage.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

// ============================================================================
// KRC20 Token Models
// ============================================================================

/// Trade statistics response from `/api/trade-stats`
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TradeStatsResponse {
    /// Total number of trades on Kaspiano marketplace
    pub total_trades_kaspiano: i64,
    /// Total volume in KAS
    pub total_volume_kas_kaspiano: String,
    /// Total volume in USD
    pub total_volume_usd_kaspiano: String,
    /// Per-token statistics
    #[serde(default)]
    pub tokens: Vec<TokenTradeStats>,
}

/// Trade statistics for a single token
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TokenTradeStats {
    pub ticker: String,
    pub total_trades: i64,
    /// Volume in KAS (numeric, not string like the outer totals)
    #[serde(rename = "totalVolumeKAS")]
    pub total_volume_kas: f64,
    pub total_volume_usd: String,
}

/// Floor price entry from `/api/floor-price`
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FloorPriceEntry {
    pub ticker: String,
    pub floor_price: f64,
    /// Cache metadata - when this was cached (Unix timestamp)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_at: Option<i64>,
}

/// Sold order from `/api/sold-orders`
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SoldOrder {
    #[serde(rename = "_id")]
    pub id: String,
    pub ticker: String,
    pub amount: i64,
    pub price_per_token: f64,
    pub total_price: f64,
    pub seller_address: String,
    #[serde(default)]
    pub buyer_address: Option<String>,
    pub created_at: i64,
    pub status: String,
    #[serde(default)]
    pub fulfillment_timestamp: Option<i64>,
}

/// Hot minting token from `/api/hot-mints`
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HotMint {
    pub ticker: String,
    pub change_total_mints: i64,
    pub total_mint_percentage: f64,
    pub total_holders: i64,
}

/// Comprehensive token info from `/api/token-info/:ticker`
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TokenInfo {
    pub ticker: String,
    /// Creation timestamp (milliseconds since epoch)
    #[serde(default)]
    pub creation_date: Option<i64>,
    pub total_supply: i64,
    pub total_mint_times: i64,
    pub total_minted: i64,
    #[serde(default)]
    pub total_minted_percent: f64,
    pub total_holders: i64,
    #[serde(default)]
    pub pre_minted_supply: i64,
    pub mint_limit: i64,
    #[serde(default)]
    pub dev_wallet: Option<String>,
    #[serde(default)]
    pub total_trades: i64,
    pub state: String,
    #[serde(default)]
    pub price: f64,
    #[serde(default)]
    pub market_cap: f64,
    #[serde(default)]
    pub volume_usd: f64,
    #[serde(default)]
    pub volume_kas: f64,
    #[serde(default)]
    pub rank: Option<i32>,
    /// Top holders as raw JSON (flexible structure)
    #[serde(default)]
    pub top_holders: Option<serde_json::Value>,
    #[serde(default)]
    pub metadata: Option<TokenMetadata>,
}

/// Token metadata (socials, description, etc.)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TokenMetadata {
    #[serde(default)]
    pub description: Option<String>,
    #[serde(rename = "logoUrl", default)]
    pub logo_url: Option<String>,
    #[serde(rename = "bannerUrl", default)]
    pub banner_url: Option<String>,
    #[serde(default)]
    pub contacts: Option<Vec<String>>,
    #[serde(default)]
    pub socials: Option<TokenSocials>,
}

/// Token social links
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TokenSocials {
    #[serde(default)]
    pub website: Option<String>,
    #[serde(default)]
    pub discord: Option<String>,
    #[serde(default)]
    pub telegram: Option<String>,
    #[serde(default)]
    pub x: Option<String>,
    #[serde(default)]
    pub github: Option<String>,
    #[serde(default)]
    pub medium: Option<String>,
    #[serde(default)]
    pub reddit: Option<String>,
    #[serde(default)]
    pub whitepaper: Option<String>,
    #[serde(default)]
    pub audit: Option<String>,
    #[serde(default)]
    pub contract: Option<String>,
}

/// Token logo entry from `/api/tokens-logos`
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TokenLogo {
    pub ticker: String,
    pub logo: String,
}

/// Open orders response from `/api/open-orders`
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OpenOrdersResponse {
    pub tickers: Vec<String>,
}

/// Historical data response from `/api/historical-data`
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HistoricalDataResponse {
    pub time_frame: String,
    pub bucket_size: String,
    pub ticker: String,
    pub data_points: Vec<HistoricalDataPoint>,
    pub total_data_points: i32,
}

/// Single historical data point
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HistoricalDataPoint {
    pub timestamp: i64,
    #[serde(rename = "totalVolumeKAS")]
    pub total_volume_kas: f64,
    pub average_price: f64,
    pub trade_count: i32,
    pub ticker: String,
}

// ============================================================================
// KRC721 NFT Models
// ============================================================================

/// NFT mint entry from `/api/krc721/mint`
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NftMint {
    pub ticker: String,
    pub token_id: String,
    pub minter_address: String,
    pub timestamp: i64,
    pub metadata_uri: String,
}

/// NFT order (sold or listed) from `/api/krc721/sold-orders` or `/api/krc721/listed-orders`
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NftOrder {
    #[serde(rename = "_id")]
    pub id: String,
    pub ticker: String,
    pub token_id: String,
    pub price: f64,
    pub seller_address: String,
    #[serde(default)]
    pub buyer_address: Option<String>,
    pub created_at: i64,
    pub status: String,
    #[serde(default)]
    pub fulfillment_timestamp: Option<i64>,
}

/// NFT trade stats from `/api/krc721/trade-stats`
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NftTradeStatsResponse {
    pub total_trades_kaspiano: i64,
    pub total_volume_kas_kaspiano: String,
    pub total_volume_usd_kaspiano: String,
    #[serde(default)]
    pub collections: Vec<NftCollectionStats>,
}

/// Per-collection NFT stats
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NftCollectionStats {
    pub ticker: String,
    pub total_trades: i64,
    #[serde(rename = "totalVolumeKAS")]
    pub total_volume_kas: f64, // API returns integer, not string
    pub total_volume_usd: String,
}

/// NFT token filter for POST `/api/krc721/tokens`
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NftTokenFilter {
    #[serde(default)]
    pub ticker: Option<String>,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub is_listed: Option<bool>,
    #[serde(default)]
    pub min_price: Option<f64>,
    #[serde(default)]
    pub max_price: Option<f64>,
    #[serde(default)]
    pub page: Option<i32>,
    #[serde(default)]
    pub limit: Option<i32>,
}

/// NFT tokens response from POST `/api/krc721/tokens`
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NftTokensResponse {
    pub items: Vec<NftToken>,
    pub total_count: i64,
}

/// Individual NFT token
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NftToken {
    #[serde(rename = "_id")]
    pub id: String,
    pub token_id: i64, // API returns integer, not string
    pub ticker: String,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub is_listed: Option<bool>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default)]
    pub listing_price: Option<f64>,
    #[serde(default)]
    pub traits: Option<HashMap<String, NftTrait>>,
    #[serde(default)]
    pub rarity_rank: Option<i32>,
}

/// NFT trait with rarity
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NftTrait {
    pub value: String,
    pub rarity: f64,
}

// ============================================================================
// KNS Domain Models
// ============================================================================

/// KNS domain order from `/api/kns/sold-orders` or `/api/kns/listed-orders`
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct KnsOrder {
    #[serde(rename = "_id")]
    pub id: String,
    pub asset_id: String, // e.g., "mywallet.kas"
    pub price: f64,
    pub seller_address: String,
    #[serde(default)]
    pub buyer_address: Option<String>,
    pub created_at: i64,
    pub status: String,
    #[serde(default)]
    pub fulfillment_timestamp: Option<i64>,
}

/// Wrapper for KNS listed orders response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct KnsListedOrdersResponse {
    pub orders: Vec<KnsOrder>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct KnsTradeStatsResponse {
    pub total_trades_kaspiano: i64,
    #[serde(default, deserialize_with = "deserialize_string_from_number")]
    pub total_volume_kas_kaspiano: String,
    #[serde(default, deserialize_with = "deserialize_string_from_number")]
    pub total_volume_usd_kaspiano: String,
}

fn deserialize_string_from_number<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrNumber {
        String(String),
        Number(serde_json::Number),
        Null,
    }

    // Handle null or missing by deriving default if we fail? 
    // Actually default is handled by serde(default) BEFORE calling this? 
    // No, deserialize_with is called. 
    // Helper to handle Option? No, field is String.
    // If field is missing, serde(default) uses Default::default() (empty string).
    // If field is present but null?
    
    match Option::<StringOrNumber>::deserialize(deserializer)? {
        Some(StringOrNumber::String(s)) => Ok(s),
        Some(StringOrNumber::Number(n)) => Ok(n.to_string()),
        Some(StringOrNumber::Null) | None => Ok("0".to_string()),
    }
}

// ============================================================================
// Token Configuration
// ============================================================================

/// Token configuration loaded from tokens_config.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokensConfig {
    pub tokens: HashMap<String, TokenExchanges>,
}

/// Exchange availability for a token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenExchanges {
    pub exchanges: Vec<String>,
}

impl TokensConfig {
    /// Load configuration from a JSON file
    pub fn load(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Get all token names (original case from config)
    pub fn get_tokens(&self) -> Vec<String> {
        self.tokens.keys().cloned().collect()
    }

    /// Get the uppercase ticker for API calls
    pub fn get_ticker(token: &str) -> String {
        token.to_uppercase()
    }

    /// Get exchanges for a token (case-insensitive lookup)
    pub fn get_exchanges(&self, token: &str) -> Option<&Vec<String>> {
        // Try exact match first
        self.tokens
            .get(token)
            .or_else(|| {
                // Fall back to case-insensitive match
                self.tokens
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case(token))
                    .map(|(_, v)| v)
            })
            .map(|t| &t.exchanges)
    }

    /// Check if a token exists in config (case-insensitive)
    pub fn has_token(&self, token: &str) -> bool {
        self.tokens.contains_key(token)
            || self
                .tokens
                .keys()
                .any(|k| k.eq_ignore_ascii_case(token))
    }
}

// ============================================================================
// Cache Metadata
// ============================================================================

/// Metadata for cached entries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMetadata {
    /// When this entry was cached (Unix timestamp)
    pub cached_at: i64,
    /// Source of the data
    pub source: String,
    /// TTL in seconds
    pub ttl_seconds: u64,
}

impl CacheMetadata {
    pub fn new(ttl_seconds: u64) -> Self {
        Self {
            cached_at: chrono::Utc::now().timestamp(),
            source: "api.kaspa.com".to_string(),
            ttl_seconds,
        }
    }

    pub fn is_expired(&self) -> bool {
        let now = chrono::Utc::now().timestamp();
        now - self.cached_at > self.ttl_seconds as i64
    }
}

// ============================================================================
// KRC721 External API Models (krc721.stream + api.kaspa.com)
// ============================================================================

/// NFT metadata from krc721.stream `/krc721/mainnet/metadata/{ticker}/{tokenId}`
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NftMetadata {
    /// IPFS image URL (e.g., "ipfs://bafybei...")
    pub image: String,
    /// NFT name (e.g., "Bitcoin the Turtle #173")
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub attributes: Vec<NftAttribute>,
}

/// NFT attribute/trait
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NftAttribute {
    pub trait_type: String,
    pub value: String,
}

/// Collection info from api.kaspa.com `/krc721/{ticker}`
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Krc721CollectionInfo {
    pub ticker: String,
    pub total_supply: i64,
    pub total_minted: i64,
    #[serde(default)]
    pub total_minted_percent: f64,
    pub total_holders: i64,
    #[serde(default)]
    pub price: f64,
    /// Base IPFS URI for the collection
    #[serde(default)]
    pub buri: Option<String>,
    /// Deployer address
    #[serde(default)]
    pub deployer: Option<String>,
    /// Creation timestamp (milliseconds)
    #[serde(default)]
    pub creation_date: Option<i64>,
    /// State: "deployed", "minting", etc.
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub metadata: Option<CollectionMetadataInfo>,
    /// Top holders list
    #[serde(default)]
    pub holders: Vec<CollectionHolder>,
}

/// Collection metadata from api.kaspa.com
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CollectionMetadataInfo {
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub banner_url: Option<String>,
    #[serde(default)]
    pub trend_banner_url: Option<String>,
    #[serde(rename = "xUrl")]
    #[serde(default)]
    pub x_url: Option<String>,
    #[serde(default)]
    pub telegram_url: Option<String>,
    #[serde(default)]
    pub discord_url: Option<String>,
    #[serde(default)]
    pub is_verified: Option<bool>,
    #[serde(default)]
    pub collection_royalty: Option<f64>,
}

/// Holder entry in collection info
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CollectionHolder {
    pub owner: String,
    pub count: i64,
}
