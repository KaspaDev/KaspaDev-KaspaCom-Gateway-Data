//! HTTP handlers for Kaspa.com L1 Marketplace API endpoints.
//!
//! These handlers provide cache-first access to the Kaspa.com API,
//! serving data from local cache when available.

use crate::api::state::AppState;
use crate::domain::{
    FloorPriceEntry, HistoricalDataResponse, HotMint, KnsOrder, KnsTradeStatsResponse,
    Krc721CollectionInfo, NftMetadata, NftMint, NftOrder, NftTokensResponse, NftTradeStatsResponse,
    OpenOrdersResponse, SoldOrder, TokenInfo, TokenLogo, TradeStatsResponse,
};
use crate::infrastructure::CacheStats;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

// ============================================================================
// Query Parameters
// ============================================================================

/// Query parameters for trade stats endpoint
#[derive(Debug, Clone, Deserialize, IntoParams, Validate)]
#[serde(rename_all = "camelCase")]
pub struct TradeStatsQuery {
    /// Time frame for statistics (e.g., "6h", "24h", "7d")
    #[serde(default = "default_time_frame")]
    #[validate(length(min = 1, max = 10))]
    pub time_frame: String,
    /// Optional ticker filter (will be normalized to uppercase)
    #[validate(length(max = 50))]
    pub ticker: Option<String>,
}

/// Query parameters for floor price endpoint
#[derive(Debug, Clone, Deserialize, IntoParams, Validate)]
pub struct FloorPriceQuery {
    /// Optional ticker filter
    #[validate(length(max = 50))]
    pub ticker: Option<String>,
}

/// Query parameters for sold orders endpoint
#[derive(Debug, Clone, Deserialize, IntoParams, Validate)]
pub struct SoldOrdersQuery {
    /// Optional ticker filter
    #[validate(length(max = 50))]
    pub ticker: Option<String>,
    /// Time window in minutes (default: 60)
    #[validate(range(min = 1.0, max = 10080.0))] // 1 minute to 7 days
    pub minutes: Option<f64>,
}

/// Query parameters for hot mints endpoint
#[derive(Debug, Clone, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct HotMintsQuery {
    /// Time interval (e.g., "1h", "6h", "24h")
    #[serde(default = "default_time_interval")]
    pub time_interval: String,
}

/// Query parameters for historical data endpoint
#[derive(Debug, Clone, Deserialize, IntoParams, Validate)]
#[serde(rename_all = "camelCase")]
pub struct HistoricalDataQuery {
    /// Time frame (e.g., "15m", "1h", "6h", "24h", "7d", "30d")
    #[serde(default = "default_time_frame")]
    #[validate(length(min = 1, max = 10))]
    pub time_frame: String,
    /// Token ticker (required)
    #[validate(length(min = 1, max = 50))]
    pub ticker: String,
}

/// Query parameters for KNS trade stats endpoint
#[derive(Debug, Clone, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct KnsTradeStatsQuery {
    /// Time frame for statistics
    #[serde(default = "default_time_frame")]
    pub time_frame: String,
    /// Optional asset filter (domain name)
    pub asset: Option<String>,
}

fn default_time_frame() -> String {
    "6h".to_string()
}

fn default_time_interval() -> String {
    "1h".to_string()
}

// ============================================================================
// Response Types
// ============================================================================

/// Response for available tokens endpoint
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AvailableTokensResponse {
    /// List of configured tokens
    pub tokens: Vec<String>,
    /// Total count
    pub count: usize,
}

/// Response for token exchanges endpoint
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TokenExchangesResponse {
    /// Token ticker
    pub ticker: String,
    /// List of exchanges that support this token
    pub exchanges: Vec<String>,
}

/// Error response
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

// ============================================================================
// KRC20 Token Handlers
// ============================================================================

/// Get trade statistics for KRC20 tokens
#[utoipa::path(
    get,
    path = "/v1/api/kaspa/trade-stats",
    params(TradeStatsQuery),
    responses(
        (status = 200, description = "Trade statistics data", body = TradeStatsResponse),
        (status = 400, description = "Invalid input parameters", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    description = "Returns aggregated trading data including total volume (USD/KAS), number of trades, and unique buyers/sellers for a specified time frame. Can be filtered by specific ticker.",
    tag = "KRC20"
)]
pub async fn trade_stats_handler(
    Query(query): Query<TradeStatsQuery>,
    State(state): State<AppState>,
) -> Result<Json<TradeStatsResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Validate input
    if let Err(validation_errors) = query.validate() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Validation failed".to_string(),
                details: Some(format!("{:?}", validation_errors)),
            }),
        ));
    }
    state
        .kaspacom_service
        .get_trade_stats(&query.time_frame, query.ticker.as_deref())
        .await
        .map(Json)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to fetch trade stats".to_string(),
                    details: Some(e.to_string()),
                }),
            )
        })
}

/// Get floor prices for KRC20 tokens
#[utoipa::path(
    get,
    path = "/v1/api/kaspa/floor-price",
    params(FloorPriceQuery),
    responses(
        (status = 200, description = "Floor price data", body = Vec<FloorPriceEntry>),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    description = "Returns the lowest listing price per token across all active orders. Can fetch for a specific ticker or all tokens.",
    tag = "KRC20"
)]
pub async fn floor_price_handler(
    Query(query): Query<FloorPriceQuery>,
    State(state): State<AppState>,
) -> Result<Json<Vec<FloorPriceEntry>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .kaspacom_service
        .get_floor_prices(query.ticker.as_deref())
        .await
        .map(Json)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to fetch floor prices".to_string(),
                    details: Some(e.to_string()),
                }),
            )
        })
}

/// Get recently sold orders for KRC20 tokens
#[utoipa::path(
    get,
    path = "/v1/api/kaspa/sold-orders",
    params(SoldOrdersQuery),
    responses(
        (status = 200, description = "List of sold orders", body = Vec<SoldOrder>),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    description = "Returns all completed trades within the specified time window (in minutes). Includes order details, prices, and participant addresses.",
    tag = "KRC20"
)]
pub async fn sold_orders_handler(
    Query(query): Query<SoldOrdersQuery>,
    State(state): State<AppState>,
) -> Result<Json<Vec<SoldOrder>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .kaspacom_service
        .get_sold_orders(query.ticker.as_deref(), query.minutes)
        .await
        .map(Json)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to fetch sold orders".to_string(),
                    details: Some(e.to_string()),
                }),
            )
        })
}

/// Get the most recent sold order
#[utoipa::path(
    get,
    path = "/v1/api/kaspa/last-order-sold",
    responses(
        (status = 200, description = "Most recent sold order", body = SoldOrder),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    description = "Returns the single latest completed trade across all KRC20 tokens with full order details.",
    tag = "KRC20"
)]
pub async fn last_order_sold_handler(
    State(state): State<AppState>,
) -> Result<Json<SoldOrder>, (StatusCode, Json<ErrorResponse>)> {
    state
        .kaspacom_service
        .get_last_order_sold()
        .await
        .map(Json)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to fetch last sold order".to_string(),
                    details: Some(e.to_string()),
                }),
            )
        })
}

/// Get hot minting tokens
#[utoipa::path(
    get,
    path = "/v1/api/kaspa/hot-mints",
    params(HotMintsQuery),
    responses(
        (status = 200, description = "List of hot minting tokens", body = Vec<HotMint>),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    description = "Returns the top 5 tokens with the highest change in mint counts within the specified time interval. Useful for identifying trending tokens.",
    tag = "KRC20"
)]
pub async fn hot_mints_handler(
    Query(query): Query<HotMintsQuery>,
    State(state): State<AppState>,
) -> Result<Json<Vec<HotMint>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .kaspacom_service
        .get_hot_mints(&query.time_interval)
        .await
        .map(Json)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to fetch hot mints".to_string(),
                    details: Some(e.to_string()),
                }),
            )
        })
}

/// Get comprehensive token info
#[utoipa::path(
    get,
    path = "/v1/api/kaspa/token-info/{ticker}",
    params(
        ("ticker" = String, Path, description = "Token ticker (e.g., SLOW, NACHO)")
    ),
    responses(
        (status = 200, description = "Detailed token information", body = TokenInfo),
        (status = 404, description = "Token not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    description = "Returns detailed token information including supply, holders, trading metrics, market cap, price, and metadata (logo, socials, description).",
    tag = "KRC20"
)]
pub async fn token_info_handler(
    Path(ticker): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<TokenInfo>, (StatusCode, Json<ErrorResponse>)> {
    state
        .kaspacom_service
        .get_token_info(&ticker)
        .await
        .map(Json)
        .map_err(|e| {
            let error_str = e.to_string();
            let status = if error_str.contains("404") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (
                status,
                Json(ErrorResponse {
                    error: "Failed to fetch token info".to_string(),
                    details: Some(error_str),
                }),
            )
        })
}

/// Get token logos
#[utoipa::path(
    get,
    path = "/v1/api/kaspa/tokens-logos",
    params(FloorPriceQuery),
    responses(
        (status = 200, description = "List of token logos", body = Vec<TokenLogo>),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    description = "Returns logo URLs for tokens. Can fetch a specific token logo or all token logos.",
    tag = "KRC20"
)]
pub async fn tokens_logos_handler(
    Query(query): Query<FloorPriceQuery>,
    State(state): State<AppState>,
) -> Result<Json<Vec<TokenLogo>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .kaspacom_service
        .get_tokens_logos(query.ticker.as_deref())
        .await
        .map(Json)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to fetch token logos".to_string(),
                    details: Some(e.to_string()),
                }),
            )
        })
}

/// Get tickers with active open orders
#[utoipa::path(
    get,
    path = "/v1/api/kaspa/open-orders",
    responses(
        (status = 200, description = "List of tickers with open orders", body = OpenOrdersResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    description = "Returns a list of token tickers that currently have active buy or sell orders in the marketplace.",
    tag = "KRC20"
)]
pub async fn open_orders_handler(
    State(state): State<AppState>,
) -> Result<Json<OpenOrdersResponse>, (StatusCode, Json<ErrorResponse>)> {
    state
        .kaspacom_service
        .get_open_orders()
        .await
        .map(Json)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to fetch open orders".to_string(),
                    details: Some(e.to_string()),
                }),
            )
        })
}

/// Get historical price/volume data
#[utoipa::path(
    get,
    path = "/v1/api/kaspa/historical-data",
    params(HistoricalDataQuery),
    responses(
        (status = 200, description = "Historical data", body = HistoricalDataResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "KRC20"
)]
pub async fn historical_data_handler(
    Query(query): Query<HistoricalDataQuery>,
    State(state): State<AppState>,
) -> Result<Json<HistoricalDataResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Validate input
    if let Err(validation_errors) = query.validate() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Validation failed".to_string(),
                details: Some(format!("{:?}", validation_errors)),
            }),
        ));
    }
    state
        .kaspacom_service
        .get_historical_data(&query.time_frame, &query.ticker)
        .await
        .map(Json)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to fetch historical data".to_string(),
                    details: Some(e.to_string()),
                }),
            )
        })
}

// ============================================================================
// KRC721 NFT Handlers
// ============================================================================

/// Get recent NFT mints
#[utoipa::path(
    get,
    path = "/v1/api/kaspa/krc721/mint",
    params(FloorPriceQuery),
    responses(
        (status = 200, description = "List of recent NFT mints", body = Vec<NftMint>),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    description = "Returns recently minted NFTs. Can be filtered by specific collection ticker or return all recent mints.",
    tag = "KRC721"
)]
pub async fn krc721_mints_handler(
    Query(query): Query<FloorPriceQuery>,
    State(state): State<AppState>,
) -> Result<Json<Vec<NftMint>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .kaspacom_service
        .get_krc721_mints(query.ticker.as_deref())
        .await
        .map(Json)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to fetch KRC721 mints".to_string(),
                    details: Some(e.to_string()),
                }),
            )
        })
}

/// Get sold NFT orders
#[utoipa::path(
    get,
    path = "/v1/api/kaspa/krc721/sold-orders",
    params(SoldOrdersQuery),
    responses(
        (status = 200, description = "Sold NFT orders", body = Vec<NftOrder>),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "KRC721"
)]
pub async fn krc721_sold_orders_handler(
    Query(query): Query<SoldOrdersQuery>,
    State(state): State<AppState>,
) -> Result<Json<Vec<NftOrder>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .kaspacom_service
        .get_krc721_sold_orders(query.ticker.as_deref(), query.minutes)
        .await
        .map(Json)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to fetch KRC721 sold orders".to_string(),
                    details: Some(e.to_string()),
                }),
            )
        })
}

/// Get listed NFT orders
#[utoipa::path(
    get,
    path = "/v1/api/kaspa/krc721/listed-orders",
    params(FloorPriceQuery),
    responses(
        (status = 200, description = "Listed NFT orders", body = Vec<NftOrder>),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "KRC721"
)]
pub async fn krc721_listed_orders_handler(
    Query(query): Query<FloorPriceQuery>,
    State(state): State<AppState>,
) -> Result<Json<Vec<NftOrder>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .kaspacom_service
        .get_krc721_listed_orders(query.ticker.as_deref())
        .await
        .map(Json)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to fetch KRC721 listed orders".to_string(),
                    details: Some(e.to_string()),
                }),
            )
        })
}

/// Get NFT trade statistics
#[utoipa::path(
    get,
    path = "/v1/api/kaspa/krc721/trade-stats",
    params(TradeStatsQuery),
    responses(
        (status = 200, description = "NFT trade statistics", body = NftTradeStatsResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "KRC721"
)]
pub async fn krc721_trade_stats_handler(
    Query(query): Query<TradeStatsQuery>,
    State(state): State<AppState>,
) -> Result<Json<NftTradeStatsResponse>, (StatusCode, Json<ErrorResponse>)> {
    state
        .kaspacom_service
        .get_krc721_trade_stats(&query.time_frame, query.ticker.as_deref())
        .await
        .map(Json)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to fetch KRC721 trade stats".to_string(),
                    details: Some(e.to_string()),
                }),
            )
        })
}

/// Get hot minting NFT collections
#[utoipa::path(
    get,
    path = "/v1/api/kaspa/krc721/hot-mints",
    params(HotMintsQuery),
    responses(
        (status = 200, description = "Hot minting NFT collections", body = Vec<HotMint>),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "KRC721"
)]
pub async fn krc721_hot_mints_handler(
    Query(query): Query<HotMintsQuery>,
    State(state): State<AppState>,
) -> Result<Json<Vec<HotMint>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .kaspacom_service
        .get_krc721_hot_mints(&query.time_interval)
        .await
        .map(Json)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to fetch KRC721 hot mints".to_string(),
                    details: Some(e.to_string()),
                }),
            )
        })
}

/// Get NFT floor prices
#[utoipa::path(
    get,
    path = "/v1/api/kaspa/krc721/floor-price",
    params(FloorPriceQuery),
    responses(
        (status = 200, description = "NFT floor prices", body = Vec<FloorPriceEntry>),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "KRC721"
)]
pub async fn krc721_floor_price_handler(
    Query(query): Query<FloorPriceQuery>,
    State(state): State<AppState>,
) -> Result<Json<Vec<FloorPriceEntry>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .kaspacom_service
        .get_krc721_floor_prices(query.ticker.as_deref())
        .await
        .map(Json)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to fetch KRC721 floor prices".to_string(),
                    details: Some(e.to_string()),
                }),
            )
        })
}

/// Get filtered NFT tokens with pagination
#[utoipa::path(
    post,
    path = "/v1/api/kaspa/krc721/tokens",
    request_body = serde_json::Value,
    responses(
        (status = 200, description = "Filtered NFT tokens", body = NftTokensResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "KRC721"
)]
pub async fn krc721_tokens_handler(
    State(state): State<AppState>,
    Json(filter): Json<serde_json::Value>,
) -> Result<Json<NftTokensResponse>, (StatusCode, Json<ErrorResponse>)> {
    state
        .kaspacom_service
        .get_krc721_tokens(&filter)
        .await
        .map(Json)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to fetch KRC721 tokens".to_string(),
                    details: Some(e.to_string()),
                }),
            )
        })
}

/// Get KRC721 collection info (holders, supply, rarity)
#[utoipa::path(
    get,
    path = "/v1/api/kaspa/krc721/collection/{ticker}",
    params(
        ("ticker" = String, Path, description = "NFT collection ticker (e.g., BITCOIN)")
    ),
    responses(
        (status = 200, description = "Collection information", body = Krc721CollectionInfo),
        (status = 404, description = "Collection not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "KRC721"
)]
pub async fn krc721_collection_info_handler(
    Path(ticker): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<Krc721CollectionInfo>, (StatusCode, Json<ErrorResponse>)> {
    state
        .kaspacom_service
        .get_krc721_collection_info(&ticker)
        .await
        .map(Json)
        .map_err(|e| {
            let error_str = e.to_string();
            let status = if error_str.contains("404") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (
                status,
                Json(ErrorResponse {
                    error: "Failed to fetch collection info".to_string(),
                    details: Some(error_str),
                }),
            )
        })
}

/// Get NFT metadata (image, name, traits) from krc721.stream cache
#[utoipa::path(
    get,
    path = "/v1/api/kaspa/krc721/metadata/{ticker}/{token_id}",
    params(
        ("ticker" = String, Path, description = "NFT collection ticker"),
        ("token_id" = i64, Path, description = "Token ID within the collection")
    ),
    responses(
        (status = 200, description = "NFT metadata", body = NftMetadata),
        (status = 404, description = "Metadata not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "KRC721"
)]
pub async fn krc721_metadata_handler(
    Path((ticker, token_id)): Path<(String, i64)>,
    State(state): State<AppState>,
) -> Result<Json<NftMetadata>, (StatusCode, Json<ErrorResponse>)> {
    state
        .kaspacom_service
        .get_nft_metadata(&ticker, token_id)
        .await
        .map(Json)
        .map_err(|e| {
            let error_str = e.to_string();
            let status = if error_str.contains("404") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (
                status,
                Json(ErrorResponse {
                    error: "Failed to fetch NFT metadata".to_string(),
                    details: Some(error_str),
                }),
            )
        })
}

/// Get optimized NFT image URL from krc721.stream CDN
#[utoipa::path(
    get,
    path = "/v1/api/kaspa/krc721/image/{ticker}/{token_id}",
    params(
        ("ticker" = String, Path, description = "NFT collection ticker"),
        ("token_id" = i64, Path, description = "Token ID within the collection")
    ),
    responses(
        (status = 200, description = "Image URL", body = String)
    ),
    tag = "KRC721"
)]
pub async fn krc721_image_url_handler(
    Path((ticker, token_id)): Path<(String, i64)>,
) -> impl IntoResponse {
    use crate::infrastructure::KaspaComClient;
    let url = KaspaComClient::get_nft_image_url(&ticker, token_id);
    Json(serde_json::json!({ "imageUrl": url }))
}

// ============================================================================
// KNS Domain Handlers
// ============================================================================

/// Get sold KNS domain orders
#[utoipa::path(
    get,
    path = "/v1/api/kaspa/kns/sold-orders",
    params(SoldOrdersQuery),
    responses(
        (status = 200, description = "Sold KNS orders", body = Vec<KnsOrder>),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "KNS"
)]
pub async fn kns_sold_orders_handler(
    Query(query): Query<SoldOrdersQuery>,
    State(state): State<AppState>,
) -> Result<Json<Vec<KnsOrder>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .kaspacom_service
        .get_kns_sold_orders(query.minutes)
        .await
        .map(Json)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to fetch KNS sold orders".to_string(),
                    details: Some(e.to_string()),
                }),
            )
        })
}

/// Get KNS trade statistics
#[utoipa::path(
    get,
    path = "/v1/api/kaspa/kns/trade-stats",
    params(KnsTradeStatsQuery),
    responses(
        (status = 200, description = "KNS trade statistics", body = KnsTradeStatsResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "KNS"
)]
pub async fn kns_trade_stats_handler(
    Query(query): Query<KnsTradeStatsQuery>,
    State(state): State<AppState>,
) -> Result<Json<KnsTradeStatsResponse>, (StatusCode, Json<ErrorResponse>)> {
    state
        .kaspacom_service
        .get_kns_trade_stats(&query.time_frame, query.asset.as_deref())
        .await
        .map(Json)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to fetch KNS trade stats".to_string(),
                    details: Some(e.to_string()),
                }),
            )
        })
}

/// Get listed KNS domains
#[utoipa::path(
    get,
    path = "/v1/api/kaspa/kns/listed-orders",
    responses(
        (status = 200, description = "Listed KNS domains", body = Vec<KnsOrder>),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "KNS"
)]
pub async fn kns_listed_orders_handler(
    State(state): State<AppState>,
) -> Result<Json<Vec<KnsOrder>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .kaspacom_service
        .get_kns_listed_orders()
        .await
        .map(Json)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to fetch KNS listed orders".to_string(),
                    details: Some(e.to_string()),
                }),
            )
        })
}

// ============================================================================
// Configuration & Cache Handlers
// ============================================================================

/// Get list of configured tokens
#[utoipa::path(
    get,
    path = "/v1/api/kaspa/tokens",
    responses(
        (status = 200, description = "Available tokens", body = AvailableTokensResponse)
    ),
    tag = "Configuration"
)]
pub async fn available_tokens_handler(
    State(state): State<AppState>,
) -> Json<AvailableTokensResponse> {
    let tokens = state.kaspacom_service.get_configured_tokens();
    Json(AvailableTokensResponse {
        count: tokens.len(),
        tokens,
    })
}

/// Get exchanges for a specific token
#[utoipa::path(
    get,
    path = "/v1/api/kaspa/tokens/{token}/exchanges",
    params(
        ("token" = String, Path, description = "Token name (e.g., Kaspa, Nacho)")
    ),
    responses(
        (status = 200, description = "Token exchanges", body = TokenExchangesResponse),
        (status = 404, description = "Token not found", body = ErrorResponse)
    ),
    tag = "Configuration"
)]
pub async fn token_exchanges_handler(
    Path(token): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<TokenExchangesResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.kaspacom_service.get_token_exchanges(&token) {
        Some(exchanges) => Ok(Json(TokenExchangesResponse {
            ticker: token,
            exchanges,
        })),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Token '{}' not found in configuration", token),
                details: None,
            }),
        )),
    }
}

/// Get cache statistics
#[utoipa::path(
    get,
    path = "/v1/api/kaspa/cache/stats",
    responses(
        (status = 200, description = "Cache statistics", body = CacheStats),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Cache"
)]
pub async fn cache_stats_handler(
    State(state): State<AppState>,
) -> Result<Json<CacheStats>, (StatusCode, Json<ErrorResponse>)> {
    state
        .kaspacom_service
        .get_cache_stats()
        .map(Json)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to get cache stats".to_string(),
                    details: Some(e.to_string()),
                }),
            )
        })
}
