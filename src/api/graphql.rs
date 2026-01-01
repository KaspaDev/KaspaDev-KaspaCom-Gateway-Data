//! GraphQL schema and handlers for flexible data queries.

use crate::api::state::AppState;
use crate::domain::{
    HistoricalDataResponse, HotMint, KnsOrder, KnsTradeStatsResponse,
    Krc721CollectionInfo, NftMetadata, NftMint, NftOrder, NftTradeStatsResponse, OpenOrdersResponse,
    SoldOrder, TokenInfo, TokenLogo, TradeStatsResponse,
};
use async_graphql::{Context, ErrorExtensions, Object, Result as GraphQLResult, ServerError};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Helper function to create GraphQL errors with proper error codes and context
fn create_graphql_error(
    message: impl Into<String>,
    error_code: &str,
    operation: Option<&str>,
) -> async_graphql::Error {
    let error = ServerError::new(message, None);
    error.extend_with(|_, e| {
        e.set("code", error_code);
        e.set("timestamp", chrono::Utc::now().to_rfc3339());
        e.set("request_id", Uuid::new_v4().to_string());
        if let Some(op) = operation {
            e.set("operation", op);
        }
    });
    error.into()
}

/// GraphQL root query type.
pub struct Query;

#[Object]
impl Query {
    // ========================================================================
    // KRC20 Token Queries
    // ========================================================================

    /// Get trade statistics for KRC20 tokens.
    /// 
    /// Returns aggregated trading data including total volume (USD/KAS),
    /// number of trades, and unique buyers/sellers for a specified time frame.
    /// Can be filtered by specific ticker.
    async fn trade_stats(
        &self,
        ctx: &Context<'_>,
        time_frame: Option<String>,
        ticker: Option<String>,
    ) -> GraphQLResult<TradeStats> {
        let state = ctx.data::<AppState>()?;
        let time_frame = time_frame.as_deref().unwrap_or("6h");
        let response = state
            .kaspacom_service
            .get_trade_stats(time_frame, ticker.as_deref())
            .await
            .map_err(|e| create_graphql_error(
                format!("Failed to get trade stats: {}", e),
                "TRADE_STATS_ERROR",
                Some("tradeStats"),
            ))?;
        
        Ok(TradeStats::from(response))
    }

    /// Get floor prices for KRC20 tokens.
    /// 
    /// Returns the lowest listing price per token across all active orders.
    /// Can fetch for a specific ticker or all tokens.
    async fn krc20_floor_prices(
        &self,
        ctx: &Context<'_>,
        ticker: Option<String>,
    ) -> GraphQLResult<Vec<FloorPrice>> {
        let state = ctx.data::<AppState>()?;
        let response = state
            .kaspacom_service
            .get_floor_prices(ticker.as_deref())
            .await
            .map_err(|e| create_graphql_error(
                format!("Failed to get floor prices: {}", e),
                "FLOOR_PRICES_ERROR",
                Some("krc20FloorPrices"),
            ))?;
        
        Ok(response.into_iter().map(FloorPrice::from).collect())
    }

    /// Get recently sold orders for KRC20 tokens.
    /// 
    /// Returns all completed trades within the specified time window (in minutes).
    /// Includes order details, prices, and participant addresses.
    async fn sold_orders(
        &self,
        ctx: &Context<'_>,
        ticker: Option<String>,
        minutes: Option<f64>,
    ) -> GraphQLResult<Vec<Order>> {
        let state = ctx.data::<AppState>()?;
        let response = state
            .kaspacom_service
            .get_sold_orders(ticker.as_deref(), minutes)
            .await
            .map_err(|e| create_graphql_error(
                format!("Failed to get sold orders: {}", e),
                "SOLD_ORDERS_ERROR",
                Some("soldOrders"),
            ))?;
        
        Ok(response.into_iter().map(Order::from).collect())
    }

    /// Get the most recent sold order.
    /// 
    /// Returns the single latest completed trade across all KRC20 tokens
    /// with full order details.
    async fn last_order_sold(
        &self,
        ctx: &Context<'_>,
    ) -> GraphQLResult<Order> {
        let state = ctx.data::<AppState>()?;
        let response = state
            .kaspacom_service
            .get_last_order_sold()
            .await
            .map_err(|e| create_graphql_error(
                format!("Failed to get last order sold: {}", e),
                "LAST_ORDER_SOLD_ERROR",
                Some("lastOrderSold"),
            ))?;
        
        Ok(Order::from(response))
    }

    /// Get hot minting tokens.
    /// 
    /// Returns the top 5 tokens with the highest change in mint counts
    /// within the specified time interval. Useful for identifying trending tokens.
    async fn hot_mints(
        &self,
        ctx: &Context<'_>,
        time_interval: Option<String>,
    ) -> GraphQLResult<Vec<HotMintData>> {
        let state = ctx.data::<AppState>()?;
        let time_interval = time_interval.as_deref().unwrap_or("1h");
        let response = state
            .kaspacom_service
            .get_hot_mints(time_interval)
            .await
            .map_err(|e| create_graphql_error(
                format!("Failed to get hot mints: {}", e),
                "HOT_MINTS_ERROR",
                Some("hotMints"),
            ))?;
        
        Ok(response.into_iter().map(HotMintData::from).collect())
    }

    /// Get comprehensive token information.
    /// 
    /// Returns detailed token information including supply, holders, trading metrics,
    /// market cap, price, and metadata (logo, socials, description).
    async fn token_info(
        &self,
        ctx: &Context<'_>,
        ticker: String,
    ) -> GraphQLResult<TokenInfoData> {
        let state = ctx.data::<AppState>()?;
        let response = state
            .kaspacom_service
            .get_token_info(&ticker)
            .await
            .map_err(|e| create_graphql_error(
                format!("Failed to get token info: {}", e),
                "TOKEN_INFO_ERROR",
                Some("tokenInfo"),
            ))?;
        
        Ok(TokenInfoData::from(response))
    }

    /// Get token logos.
    /// 
    /// Returns logo URLs for tokens. Can fetch a specific token logo or all token logos.
    async fn token_logos(
        &self,
        ctx: &Context<'_>,
        ticker: Option<String>,
    ) -> GraphQLResult<Vec<TokenLogoData>> {
        let state = ctx.data::<AppState>()?;
        let response = state
            .kaspacom_service
            .get_tokens_logos(ticker.as_deref())
            .await
            .map_err(|e| create_graphql_error(
                format!("Failed to get token logos: {}", e),
                "TOKEN_LOGOS_ERROR",
                Some("tokenLogos"),
            ))?;
        
        Ok(response.into_iter().map(TokenLogoData::from).collect())
    }

    /// Get tickers with active open orders.
    /// 
    /// Returns a list of token tickers that currently have active buy or sell orders
    /// in the marketplace.
    async fn open_orders(
        &self,
        ctx: &Context<'_>,
    ) -> GraphQLResult<OpenOrders> {
        let state = ctx.data::<AppState>()?;
        let response = state
            .kaspacom_service
            .get_open_orders()
            .await
            .map_err(|e| create_graphql_error(
                format!("Failed to get open orders: {}", e),
                "OPEN_ORDERS_ERROR",
                Some("openOrders"),
            ))?;
        
        Ok(OpenOrders::from(response))
    }

    /// Get historical price/volume data.
    /// 
    /// Returns historical trading data for charting and analysis.
    async fn historical_data(
        &self,
        ctx: &Context<'_>,
        time_frame: String,
        ticker: String,
    ) -> GraphQLResult<HistoricalData> {
        let state = ctx.data::<AppState>()?;
        let response = state
            .kaspacom_service
            .get_historical_data(&time_frame, &ticker)
            .await
            .map_err(|e| create_graphql_error(
                format!("Failed to get historical data: {}", e),
                "HISTORICAL_DATA_ERROR",
                Some("historicalData"),
            ))?;
        
        Ok(HistoricalData::from(response))
    }

    // ========================================================================
    // KRC721 NFT Queries
    // ========================================================================

    /// Get recent NFT mints.
    /// 
    /// Returns recently minted NFTs. Can be filtered by specific collection ticker
    /// or return all recent mints.
    async fn krc721_mints(
        &self,
        ctx: &Context<'_>,
        ticker: Option<String>,
    ) -> GraphQLResult<Vec<NftMintData>> {
        let state = ctx.data::<AppState>()?;
        let response = state
            .kaspacom_service
            .get_krc721_mints(ticker.as_deref())
            .await
            .map_err(|e| create_graphql_error(
                format!("Failed to get KRC721 mints: {}", e),
                "KRC721_MINTS_ERROR",
                Some("krc721Mints"),
            ))?;
        
        Ok(response.into_iter().map(NftMintData::from).collect())
    }

    /// Get sold NFT orders.
    /// 
    /// Returns completed NFT trades within the specified time window.
    async fn krc721_sold_orders(
        &self,
        ctx: &Context<'_>,
        ticker: Option<String>,
        minutes: Option<f64>,
    ) -> GraphQLResult<Vec<NftOrderData>> {
        let state = ctx.data::<AppState>()?;
        let response = state
            .kaspacom_service
            .get_krc721_sold_orders(ticker.as_deref(), minutes)
            .await
            .map_err(|e| create_graphql_error(
                format!("Failed to get KRC721 sold orders: {}", e),
                "KRC721_SOLD_ORDERS_ERROR",
                Some("krc721SoldOrders"),
            ))?;
        
        Ok(response.into_iter().map(NftOrderData::from).collect())
    }

    /// Get listed NFT orders.
    /// 
    /// Returns currently listed NFTs for sale.
    async fn krc721_listed_orders(
        &self,
        ctx: &Context<'_>,
        ticker: Option<String>,
    ) -> GraphQLResult<Vec<NftOrderData>> {
        let state = ctx.data::<AppState>()?;
        let response = state
            .kaspacom_service
            .get_krc721_listed_orders(ticker.as_deref())
            .await
            .map_err(|e| create_graphql_error(
                format!("Failed to get KRC721 listed orders: {}", e),
                "KRC721_LISTED_ORDERS_ERROR",
                Some("krc721ListedOrders"),
            ))?;
        
        Ok(response.into_iter().map(NftOrderData::from).collect())
    }

    /// Get NFT trade statistics.
    /// 
    /// Returns aggregated trading data for NFT collections.
    async fn krc721_trade_stats(
        &self,
        ctx: &Context<'_>,
        time_frame: Option<String>,
        ticker: Option<String>,
    ) -> GraphQLResult<NftTradeStats> {
        let state = ctx.data::<AppState>()?;
        let time_frame = time_frame.as_deref().unwrap_or("6h");
        let response = state
            .kaspacom_service
            .get_krc721_trade_stats(time_frame, ticker.as_deref())
            .await
            .map_err(|e| create_graphql_error(
                format!("Failed to get KRC721 trade stats: {}", e),
                "KRC721_TRADE_STATS_ERROR",
                Some("krc721TradeStats"),
            ))?;
        
        Ok(NftTradeStats::from(response))
    }

    /// Get hot minting NFT collections.
    /// 
    /// Returns collections with the highest mint activity.
    async fn krc721_hot_mints(
        &self,
        ctx: &Context<'_>,
        time_interval: Option<String>,
    ) -> GraphQLResult<Vec<HotMintData>> {
        let state = ctx.data::<AppState>()?;
        let time_interval = time_interval.as_deref().unwrap_or("1h");
        let response = state
            .kaspacom_service
            .get_krc721_hot_mints(time_interval)
            .await
            .map_err(|e| create_graphql_error(
                format!("Failed to get KRC721 hot mints: {}", e),
                "KRC721_HOT_MINTS_ERROR",
                Some("krc721HotMints"),
            ))?;
        
        Ok(response.into_iter().map(HotMintData::from).collect())
    }

    /// Get NFT floor prices.
    /// 
    /// Returns the lowest listing price per NFT collection.
    async fn krc721_floor_prices(
        &self,
        ctx: &Context<'_>,
        ticker: Option<String>,
    ) -> GraphQLResult<Vec<FloorPrice>> {
        let state = ctx.data::<AppState>()?;
        let response = state
            .kaspacom_service
            .get_krc721_floor_prices(ticker.as_deref())
            .await
            .map_err(|e| create_graphql_error(
                format!("Failed to get KRC721 floor prices: {}", e),
                "KRC721_FLOOR_PRICES_ERROR",
                Some("krc721FloorPrices"),
            ))?;
        
        Ok(response.into_iter().map(FloorPrice::from).collect())
    }

    /// Get KRC721 collection information.
    /// 
    /// Returns detailed collection info including holders, supply, rarity, and metadata.
    async fn krc721_collection_info(
        &self,
        ctx: &Context<'_>,
        ticker: String,
    ) -> GraphQLResult<Krc721CollectionInfoData> {
        let state = ctx.data::<AppState>()?;
        let response = state
            .kaspacom_service
            .get_krc721_collection_info(&ticker)
            .await
            .map_err(|e| create_graphql_error(
                format!("Failed to get collection info: {}", e),
                "COLLECTION_INFO_ERROR",
                Some("krc721CollectionInfo"),
            ))?;
        
        Ok(Krc721CollectionInfoData::from(response))
    }

    /// Get NFT metadata.
    /// 
    /// Returns metadata for a specific NFT including image, name, traits, and attributes.
    async fn nft_metadata(
        &self,
        ctx: &Context<'_>,
        ticker: String,
        token_id: i64,
    ) -> GraphQLResult<NftMetadataData> {
        let state = ctx.data::<AppState>()?;
        let response = state
            .kaspacom_service
            .get_nft_metadata(&ticker, token_id)
            .await
            .map_err(|e| create_graphql_error(
                format!("Failed to get NFT metadata: {}", e),
                "NFT_METADATA_ERROR",
                Some("nftMetadata"),
            ))?;
        
        Ok(NftMetadataData::from(response))
    }

    // ========================================================================
    // KNS Domain Queries
    // ========================================================================

    /// Get sold KNS domain orders.
    /// 
    /// Returns completed KNS domain sales within the specified time window.
    async fn kns_sold_orders(
        &self,
        ctx: &Context<'_>,
        minutes: Option<f64>,
    ) -> GraphQLResult<Vec<KnsOrderData>> {
        let state = ctx.data::<AppState>()?;
        let response = state
            .kaspacom_service
            .get_kns_sold_orders(minutes)
            .await
            .map_err(|e| create_graphql_error(
                format!("Failed to get KNS sold orders: {}", e),
                "KNS_SOLD_ORDERS_ERROR",
                Some("knsSoldOrders"),
            ))?;
        
        Ok(response.into_iter().map(KnsOrderData::from).collect())
    }

    /// Get KNS trade statistics.
    /// 
    /// Returns aggregated trading data for KNS domains.
    async fn kns_trade_stats(
        &self,
        ctx: &Context<'_>,
        time_frame: Option<String>,
        asset: Option<String>,
    ) -> GraphQLResult<KnsTradeStats> {
        let state = ctx.data::<AppState>()?;
        let time_frame = time_frame.as_deref().unwrap_or("6h");
        let response = state
            .kaspacom_service
            .get_kns_trade_stats(time_frame, asset.as_deref())
            .await
            .map_err(|e| create_graphql_error(
                format!("Failed to get KNS trade stats: {}", e),
                "KNS_TRADE_STATS_ERROR",
                Some("knsTradeStats"),
            ))?;
        
        Ok(KnsTradeStats::from(response))
    }

    /// Get listed KNS domains.
    /// 
    /// Returns currently listed KNS domains for sale.
    async fn kns_listed_orders(
        &self,
        ctx: &Context<'_>,
    ) -> GraphQLResult<Vec<KnsOrderData>> {
        let state = ctx.data::<AppState>()?;
        let response = state
            .kaspacom_service
            .get_kns_listed_orders()
            .await
            .map_err(|e| create_graphql_error(
                format!("Failed to get KNS listed orders: {}", e),
                "KNS_LISTED_ORDERS_ERROR",
                Some("knsListedOrders"),
            ))?;
        
        Ok(response.into_iter().map(KnsOrderData::from).collect())
    }
}

// ============================================================================
// GraphQL Type Definitions
// ============================================================================

/// GraphQL type for Floor Price data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FloorPrice {
    pub ticker: String,
    pub floor_price: f64,
    pub volume: f64,
}

#[Object]
impl FloorPrice {
    async fn ticker(&self) -> &str {
        &self.ticker
    }
    async fn floor_price(&self) -> f64 {
        self.floor_price
    }
    async fn volume(&self) -> f64 {
        self.volume
    }
}

impl From<crate::domain::FloorPriceEntry> for FloorPrice {
    fn from(entry: crate::domain::FloorPriceEntry) -> Self {
        Self {
            ticker: entry.ticker,
            floor_price: entry.floor_price,
            volume: 0.0,
        }
    }
}

/// Trade statistics response.
#[derive(Debug, Clone)]
pub struct TradeStats {
    pub total_trades_kaspiano: i64,
    pub total_volume_kas_kaspiano: String,
    pub total_volume_usd_kaspiano: String,
    pub tokens: Vec<TokenTradeStats>,
}

#[Object]
impl TradeStats {
    async fn total_trades_kaspiano(&self) -> i64 {
        self.total_trades_kaspiano
    }
    async fn total_volume_kas_kaspiano(&self) -> &str {
        &self.total_volume_kas_kaspiano
    }
    async fn total_volume_usd_kaspiano(&self) -> &str {
        &self.total_volume_usd_kaspiano
    }
    async fn tokens(&self) -> &Vec<TokenTradeStats> {
        &self.tokens
    }
}

impl From<TradeStatsResponse> for TradeStats {
    fn from(resp: TradeStatsResponse) -> Self {
        Self {
            total_trades_kaspiano: resp.total_trades_kaspiano,
            total_volume_kas_kaspiano: resp.total_volume_kas_kaspiano,
            total_volume_usd_kaspiano: resp.total_volume_usd_kaspiano,
            tokens: resp.tokens.into_iter().map(TokenTradeStats::from).collect(),
        }
    }
}

/// Trade statistics for a single token.
#[derive(Debug, Clone)]
pub struct TokenTradeStats {
    pub ticker: String,
    pub total_trades: i64,
    pub total_volume_kas: f64,
    pub total_volume_usd: String,
}

#[Object]
impl TokenTradeStats {
    async fn ticker(&self) -> &str {
        &self.ticker
    }
    async fn total_trades(&self) -> i64 {
        self.total_trades
    }
    async fn total_volume_kas(&self) -> f64 {
        self.total_volume_kas
    }
    async fn total_volume_usd(&self) -> &str {
        &self.total_volume_usd
    }
}

impl From<crate::domain::TokenTradeStats> for TokenTradeStats {
    fn from(stats: crate::domain::TokenTradeStats) -> Self {
        Self {
            ticker: stats.ticker,
            total_trades: stats.total_trades,
            total_volume_kas: stats.total_volume_kas,
            total_volume_usd: stats.total_volume_usd,
        }
    }
}

/// Order data (sold or listed).
#[derive(Debug, Clone)]
pub struct Order {
    pub id: String,
    pub ticker: String,
    pub amount: i64,
    pub price_per_token: f64,
    pub total_price: f64,
    pub seller_address: String,
    pub buyer_address: Option<String>,
    pub created_at: i64,
    pub status: String,
    pub fulfillment_timestamp: Option<i64>,
}

#[Object]
impl Order {
    async fn id(&self) -> &str {
        &self.id
    }
    async fn ticker(&self) -> &str {
        &self.ticker
    }
    async fn amount(&self) -> i64 {
        self.amount
    }
    async fn price_per_token(&self) -> f64 {
        self.price_per_token
    }
    async fn total_price(&self) -> f64 {
        self.total_price
    }
    async fn seller_address(&self) -> &str {
        &self.seller_address
    }
    async fn buyer_address(&self) -> Option<&str> {
        self.buyer_address.as_deref()
    }
    async fn created_at(&self) -> i64 {
        self.created_at
    }
    async fn status(&self) -> &str {
        &self.status
    }
    async fn fulfillment_timestamp(&self) -> Option<i64> {
        self.fulfillment_timestamp
    }
}

impl From<SoldOrder> for Order {
    fn from(order: SoldOrder) -> Self {
        Self {
            id: order.id,
            ticker: order.ticker,
            amount: order.amount,
            price_per_token: order.price_per_token,
            total_price: order.total_price,
            seller_address: order.seller_address,
            buyer_address: order.buyer_address,
            created_at: order.created_at,
            status: order.status,
            fulfillment_timestamp: order.fulfillment_timestamp,
        }
    }
}

/// Hot minting token data.
#[derive(Debug, Clone)]
pub struct HotMintData {
    pub ticker: String,
    pub change_total_mints: i64,
    pub total_mint_percentage: f64,
    pub total_holders: i64,
}

#[Object]
impl HotMintData {
    async fn ticker(&self) -> &str {
        &self.ticker
    }
    async fn change_total_mints(&self) -> i64 {
        self.change_total_mints
    }
    async fn total_mint_percentage(&self) -> f64 {
        self.total_mint_percentage
    }
    async fn total_holders(&self) -> i64 {
        self.total_holders
    }
}

impl From<HotMint> for HotMintData {
    fn from(mint: HotMint) -> Self {
        Self {
            ticker: mint.ticker,
            change_total_mints: mint.change_total_mints,
            total_mint_percentage: mint.total_mint_percentage,
            total_holders: mint.total_holders,
        }
    }
}

/// Token information data.
#[derive(Debug, Clone)]
pub struct TokenInfoData {
    pub ticker: String,
    pub creation_date: Option<i64>,
    pub total_supply: i64,
    pub total_mint_times: i64,
    pub total_minted: i64,
    pub total_minted_percent: f64,
    pub total_holders: i64,
    pub pre_minted_supply: i64,
    pub mint_limit: i64,
    pub dev_wallet: Option<String>,
    pub total_trades: i64,
    pub state: String,
    pub price: f64,
    pub market_cap: f64,
    pub volume_usd: f64,
    pub volume_kas: f64,
    pub rank: Option<i32>,
}

#[Object]
impl TokenInfoData {
    async fn ticker(&self) -> &str {
        &self.ticker
    }
    async fn creation_date(&self) -> Option<i64> {
        self.creation_date
    }
    async fn total_supply(&self) -> i64 {
        self.total_supply
    }
    async fn total_mint_times(&self) -> i64 {
        self.total_mint_times
    }
    async fn total_minted(&self) -> i64 {
        self.total_minted
    }
    async fn total_minted_percent(&self) -> f64 {
        self.total_minted_percent
    }
    async fn total_holders(&self) -> i64 {
        self.total_holders
    }
    async fn pre_minted_supply(&self) -> i64 {
        self.pre_minted_supply
    }
    async fn mint_limit(&self) -> i64 {
        self.mint_limit
    }
    async fn dev_wallet(&self) -> Option<&str> {
        self.dev_wallet.as_deref()
    }
    async fn total_trades(&self) -> i64 {
        self.total_trades
    }
    async fn state(&self) -> &str {
        &self.state
    }
    async fn price(&self) -> f64 {
        self.price
    }
    async fn market_cap(&self) -> f64 {
        self.market_cap
    }
    async fn volume_usd(&self) -> f64 {
        self.volume_usd
    }
    async fn volume_kas(&self) -> f64 {
        self.volume_kas
    }
    async fn rank(&self) -> Option<i32> {
        self.rank
    }
}

impl From<TokenInfo> for TokenInfoData {
    fn from(info: TokenInfo) -> Self {
        Self {
            ticker: info.ticker,
            creation_date: info.creation_date,
            total_supply: info.total_supply,
            total_mint_times: info.total_mint_times,
            total_minted: info.total_minted,
            total_minted_percent: info.total_minted_percent,
            total_holders: info.total_holders,
            pre_minted_supply: info.pre_minted_supply,
            mint_limit: info.mint_limit,
            dev_wallet: info.dev_wallet,
            total_trades: info.total_trades,
            state: info.state,
            price: info.price,
            market_cap: info.market_cap,
            volume_usd: info.volume_usd,
            volume_kas: info.volume_kas,
            rank: info.rank,
        }
    }
}

/// Token logo data.
#[derive(Debug, Clone)]
pub struct TokenLogoData {
    pub ticker: String,
    pub logo: String,
}

#[Object]
impl TokenLogoData {
    async fn ticker(&self) -> &str {
        &self.ticker
    }
    async fn logo(&self) -> &str {
        &self.logo
    }
}

impl From<TokenLogo> for TokenLogoData {
    fn from(logo: TokenLogo) -> Self {
        Self {
            ticker: logo.ticker,
            logo: logo.logo,
        }
    }
}

/// Open orders response.
#[derive(Debug, Clone)]
pub struct OpenOrders {
    pub tickers: Vec<String>,
}

#[Object]
impl OpenOrders {
    async fn tickers(&self) -> &Vec<String> {
        &self.tickers
    }
}

impl From<OpenOrdersResponse> for OpenOrders {
    fn from(resp: OpenOrdersResponse) -> Self {
        Self {
            tickers: resp.tickers,
        }
    }
}

/// Historical data response.
#[derive(Debug, Clone)]
pub struct HistoricalData {
    pub time_frame: String,
    pub bucket_size: String,
    pub ticker: String,
    pub data_points: Vec<HistoricalDataPoint>,
    pub total_data_points: i32,
}

#[Object]
impl HistoricalData {
    async fn time_frame(&self) -> &str {
        &self.time_frame
    }
    async fn bucket_size(&self) -> &str {
        &self.bucket_size
    }
    async fn ticker(&self) -> &str {
        &self.ticker
    }
    async fn data_points(&self) -> &Vec<HistoricalDataPoint> {
        &self.data_points
    }
    async fn total_data_points(&self) -> i32 {
        self.total_data_points
    }
}

impl From<HistoricalDataResponse> for HistoricalData {
    fn from(resp: HistoricalDataResponse) -> Self {
        Self {
            time_frame: resp.time_frame,
            bucket_size: resp.bucket_size,
            ticker: resp.ticker,
            data_points: resp.data_points.into_iter().map(HistoricalDataPoint::from).collect(),
            total_data_points: resp.total_data_points,
        }
    }
}

/// Single historical data point.
#[derive(Debug, Clone)]
pub struct HistoricalDataPoint {
    pub timestamp: i64,
    pub total_volume_kas: f64,
    pub average_price: f64,
    pub trade_count: i32,
    pub ticker: String,
}

#[Object]
impl HistoricalDataPoint {
    async fn timestamp(&self) -> i64 {
        self.timestamp
    }
    async fn total_volume_kas(&self) -> f64 {
        self.total_volume_kas
    }
    async fn average_price(&self) -> f64 {
        self.average_price
    }
    async fn trade_count(&self) -> i32 {
        self.trade_count
    }
    async fn ticker(&self) -> &str {
        &self.ticker
    }
}

impl From<crate::domain::HistoricalDataPoint> for HistoricalDataPoint {
    fn from(point: crate::domain::HistoricalDataPoint) -> Self {
        Self {
            timestamp: point.timestamp,
            total_volume_kas: point.total_volume_kas,
            average_price: point.average_price,
            trade_count: point.trade_count,
            ticker: point.ticker,
        }
    }
}

/// NFT mint data.
#[derive(Debug, Clone)]
pub struct NftMintData {
    pub ticker: String,
    pub token_id: String,
    pub minter_address: String,
    pub timestamp: i64,
    pub metadata_uri: String,
}

#[Object]
impl NftMintData {
    async fn ticker(&self) -> &str {
        &self.ticker
    }
    async fn token_id(&self) -> &str {
        &self.token_id
    }
    async fn minter_address(&self) -> &str {
        &self.minter_address
    }
    async fn timestamp(&self) -> i64 {
        self.timestamp
    }
    async fn metadata_uri(&self) -> &str {
        &self.metadata_uri
    }
}

impl From<NftMint> for NftMintData {
    fn from(mint: NftMint) -> Self {
        Self {
            ticker: mint.ticker,
            token_id: mint.token_id,
            minter_address: mint.minter_address,
            timestamp: mint.timestamp,
            metadata_uri: mint.metadata_uri,
        }
    }
}

/// NFT order data.
#[derive(Debug, Clone)]
pub struct NftOrderData {
    pub id: String,
    pub ticker: String,
    pub token_id: String,
    pub price: f64,
    pub seller_address: String,
    pub buyer_address: Option<String>,
    pub created_at: i64,
    pub status: String,
    pub fulfillment_timestamp: Option<i64>,
}

#[Object]
impl NftOrderData {
    async fn id(&self) -> &str {
        &self.id
    }
    async fn ticker(&self) -> &str {
        &self.ticker
    }
    async fn token_id(&self) -> &str {
        &self.token_id
    }
    async fn price(&self) -> f64 {
        self.price
    }
    async fn seller_address(&self) -> &str {
        &self.seller_address
    }
    async fn buyer_address(&self) -> Option<&str> {
        self.buyer_address.as_deref()
    }
    async fn created_at(&self) -> i64 {
        self.created_at
    }
    async fn status(&self) -> &str {
        &self.status
    }
    async fn fulfillment_timestamp(&self) -> Option<i64> {
        self.fulfillment_timestamp
    }
}

impl From<NftOrder> for NftOrderData {
    fn from(order: NftOrder) -> Self {
        Self {
            id: order.id,
            ticker: order.ticker,
            token_id: order.token_id,
            price: order.price,
            seller_address: order.seller_address,
            buyer_address: order.buyer_address,
            created_at: order.created_at,
            status: order.status,
            fulfillment_timestamp: order.fulfillment_timestamp,
        }
    }
}

/// NFT trade statistics.
#[derive(Debug, Clone)]
pub struct NftTradeStats {
    pub total_trades_kaspiano: i64,
    pub total_volume_kas_kaspiano: String,
    pub total_volume_usd_kaspiano: String,
    pub collections: Vec<NftCollectionStats>,
}

#[Object]
impl NftTradeStats {
    async fn total_trades_kaspiano(&self) -> i64 {
        self.total_trades_kaspiano
    }
    async fn total_volume_kas_kaspiano(&self) -> &str {
        &self.total_volume_kas_kaspiano
    }
    async fn total_volume_usd_kaspiano(&self) -> &str {
        &self.total_volume_usd_kaspiano
    }
    async fn collections(&self) -> &Vec<NftCollectionStats> {
        &self.collections
    }
}

impl From<NftTradeStatsResponse> for NftTradeStats {
    fn from(resp: NftTradeStatsResponse) -> Self {
        Self {
            total_trades_kaspiano: resp.total_trades_kaspiano,
            total_volume_kas_kaspiano: resp.total_volume_kas_kaspiano,
            total_volume_usd_kaspiano: resp.total_volume_usd_kaspiano,
            collections: resp.collections.into_iter().map(NftCollectionStats::from).collect(),
        }
    }
}

/// Per-collection NFT stats.
#[derive(Debug, Clone)]
pub struct NftCollectionStats {
    pub ticker: String,
    pub total_trades: i64,
    pub total_volume_kas: f64,
    pub total_volume_usd: String,
}

#[Object]
impl NftCollectionStats {
    async fn ticker(&self) -> &str {
        &self.ticker
    }
    async fn total_trades(&self) -> i64 {
        self.total_trades
    }
    async fn total_volume_kas(&self) -> f64 {
        self.total_volume_kas
    }
    async fn total_volume_usd(&self) -> &str {
        &self.total_volume_usd
    }
}

impl From<crate::domain::NftCollectionStats> for NftCollectionStats {
    fn from(stats: crate::domain::NftCollectionStats) -> Self {
        Self {
            ticker: stats.ticker,
            total_trades: stats.total_trades,
            total_volume_kas: stats.total_volume_kas,
            total_volume_usd: stats.total_volume_usd,
        }
    }
}

/// KRC721 collection information.
#[derive(Debug, Clone)]
pub struct Krc721CollectionInfoData {
    pub ticker: String,
    pub total_supply: i64,
    pub total_minted: i64,
    pub total_minted_percent: f64,
    pub total_holders: i64,
    pub price: f64,
    pub buri: Option<String>,
    pub deployer: Option<String>,
    pub creation_date: Option<i64>,
    pub state: Option<String>,
}

#[Object]
impl Krc721CollectionInfoData {
    async fn ticker(&self) -> &str {
        &self.ticker
    }
    async fn total_supply(&self) -> i64 {
        self.total_supply
    }
    async fn total_minted(&self) -> i64 {
        self.total_minted
    }
    async fn total_minted_percent(&self) -> f64 {
        self.total_minted_percent
    }
    async fn total_holders(&self) -> i64 {
        self.total_holders
    }
    async fn price(&self) -> f64 {
        self.price
    }
    async fn buri(&self) -> Option<&str> {
        self.buri.as_deref()
    }
    async fn deployer(&self) -> Option<&str> {
        self.deployer.as_deref()
    }
    async fn creation_date(&self) -> Option<i64> {
        self.creation_date
    }
    async fn state(&self) -> Option<&str> {
        self.state.as_deref()
    }
}

impl From<Krc721CollectionInfo> for Krc721CollectionInfoData {
    fn from(info: Krc721CollectionInfo) -> Self {
        Self {
            ticker: info.ticker,
            total_supply: info.total_supply,
            total_minted: info.total_minted,
            total_minted_percent: info.total_minted_percent,
            total_holders: info.total_holders,
            price: info.price,
            buri: info.buri,
            deployer: info.deployer,
            creation_date: info.creation_date,
            state: info.state,
        }
    }
}

/// NFT metadata.
#[derive(Debug, Clone)]
pub struct NftMetadataData {
    pub image: String,
    pub name: String,
    pub description: Option<String>,
    pub attributes: Vec<NftAttribute>,
}

#[Object]
impl NftMetadataData {
    async fn image(&self) -> &str {
        &self.image
    }
    async fn name(&self) -> &str {
        &self.name
    }
    async fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
    async fn attributes(&self) -> &Vec<NftAttribute> {
        &self.attributes
    }
}

impl From<NftMetadata> for NftMetadataData {
    fn from(metadata: NftMetadata) -> Self {
        Self {
            image: metadata.image,
            name: metadata.name,
            description: metadata.description,
            attributes: metadata.attributes.into_iter().map(NftAttribute::from).collect(),
        }
    }
}

/// NFT attribute/trait.
#[derive(Debug, Clone)]
pub struct NftAttribute {
    pub trait_type: String,
    pub value: String,
}

#[Object]
impl NftAttribute {
    async fn trait_type(&self) -> &str {
        &self.trait_type
    }
    async fn value(&self) -> &str {
        &self.value
    }
}

impl From<crate::domain::NftAttribute> for NftAttribute {
    fn from(attr: crate::domain::NftAttribute) -> Self {
        Self {
            trait_type: attr.trait_type,
            value: attr.value,
        }
    }
}

/// KNS order data.
#[derive(Debug, Clone)]
pub struct KnsOrderData {
    pub id: String,
    pub asset_id: String,
    pub price: f64,
    pub seller_address: String,
    pub buyer_address: Option<String>,
    pub created_at: i64,
    pub status: String,
    pub fulfillment_timestamp: Option<i64>,
}

#[Object]
impl KnsOrderData {
    async fn id(&self) -> &str {
        &self.id
    }
    async fn asset_id(&self) -> &str {
        &self.asset_id
    }
    async fn price(&self) -> f64 {
        self.price
    }
    async fn seller_address(&self) -> &str {
        &self.seller_address
    }
    async fn buyer_address(&self) -> Option<&str> {
        self.buyer_address.as_deref()
    }
    async fn created_at(&self) -> i64 {
        self.created_at
    }
    async fn status(&self) -> &str {
        &self.status
    }
    async fn fulfillment_timestamp(&self) -> Option<i64> {
        self.fulfillment_timestamp
    }
}

impl From<KnsOrder> for KnsOrderData {
    fn from(order: KnsOrder) -> Self {
        Self {
            id: order.id,
            asset_id: order.asset_id,
            price: order.price,
            seller_address: order.seller_address,
            buyer_address: order.buyer_address,
            created_at: order.created_at,
            status: order.status,
            fulfillment_timestamp: order.fulfillment_timestamp,
        }
    }
}

/// KNS trade statistics.
#[derive(Debug, Clone)]
pub struct KnsTradeStats {
    pub total_trades_kaspiano: i64,
    pub total_volume_kas_kaspiano: String,
    pub total_volume_usd_kaspiano: String,
}

#[Object]
impl KnsTradeStats {
    async fn total_trades_kaspiano(&self) -> i64 {
        self.total_trades_kaspiano
    }
    async fn total_volume_kas_kaspiano(&self) -> &str {
        &self.total_volume_kas_kaspiano
    }
    async fn total_volume_usd_kaspiano(&self) -> &str {
        &self.total_volume_usd_kaspiano
    }
}

impl From<KnsTradeStatsResponse> for KnsTradeStats {
    fn from(resp: KnsTradeStatsResponse) -> Self {
        Self {
            total_trades_kaspiano: resp.total_trades_kaspiano,
            total_volume_kas_kaspiano: resp.total_volume_kas_kaspiano,
            total_volume_usd_kaspiano: resp.total_volume_usd_kaspiano,
        }
    }
}

/// Create the GraphQL schema with security and performance features.
pub fn create_schema(state: AppState) -> Schema<Query, EmptyMutation, async_graphql::EmptySubscription> {
    Schema::build(Query, EmptyMutation::default(), async_graphql::EmptySubscription)
        .data(state)
        .limit_depth(10) // Maximum query depth
        .limit_complexity(1000) // Maximum query complexity
        .finish()
}

/// Placeholder for mutations (read-only for now).
#[derive(async_graphql::MergedObject, Default)]
pub struct EmptyMutation;

use async_graphql::Schema;
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::extract::Extension;

/// Maximum allowed GraphQL query size (50KB)
const MAX_QUERY_SIZE: usize = 50 * 1024;

/// GraphQL POST endpoint handler with enhanced error handling, logging, validation, and metrics.
pub async fn graphql_handler(
    Extension(schema): Extension<Schema<Query, EmptyMutation, async_graphql::EmptySubscription>>,
    req: GraphQLRequest,
) -> GraphQLResponse {
    let request = req.into_inner();
    
    // Extract operation name for metrics (if available) - convert to static string for metrics compatibility
    let operation_name = request.operation_name.as_deref().unwrap_or("unknown").to_string();
    let op_name_static: &'static str = Box::leak(operation_name.clone().into_boxed_str());
    
    // Validate query size
    if request.query.len() > MAX_QUERY_SIZE {
        tracing::warn!("GraphQL query too large: {} bytes (max: {})", request.query.len(), MAX_QUERY_SIZE);
        
        // Record metrics for validation error
        metrics::counter!("graphql_queries_total", "operation" => op_name_static, "status" => "validation_error", "error_code" => "QUERY_TOO_LARGE")
            .increment(1);
        
        let mut response = async_graphql::Response::default();
        let error = ServerError::new(
            format!(
                "Query too large: {} bytes. Maximum allowed size is {} bytes.",
                request.query.len(),
                MAX_QUERY_SIZE
            ),
            None,
        );
        error.extend_with(|_, e| {
            e.set("code", "QUERY_TOO_LARGE");
        });
        response.errors.push(error);
        return response.into();
    }
    
    // Validate query is not empty
    if request.query.trim().is_empty() {
        // Record metrics for validation error
        metrics::counter!("graphql_queries_total", "operation" => op_name_static, "status" => "validation_error", "error_code" => "EMPTY_QUERY")
            .increment(1);
        
        let mut response = async_graphql::Response::default();
        let error = ServerError::new("Query cannot be empty", None);
        error.extend_with(|_, e| {
            e.set("code", "EMPTY_QUERY");
        });
        response.errors.push(error);
        return response.into();
    }
    
    // Log query for debugging (sanitize sensitive data if needed)
    tracing::debug!("GraphQL query: {} bytes, operation: {}", request.query.len(), operation_name);
    
    // Record query size metric
    metrics::histogram!("graphql_query_size_bytes", "operation" => op_name_static)
        .record(request.query.len() as f64);
    
    let start = std::time::Instant::now();
    let response = schema.execute(request).await;
    let duration = start.elapsed();
    let duration_ms = duration.as_millis() as f64;
    
    // Extract complexity if available from response extensions
    // Note: Complexity is tracked by async-graphql internally, but may not be directly accessible
    // We'll use 0.0 as default since complexity is already limited by schema configuration
    let complexity = 0.0;
    
    // Determine status for metrics
    let status = if response.errors.is_empty() {
        "success"
    } else {
        "error"
    };
    
    // Record comprehensive metrics
    metrics::counter!("graphql_queries_total", "operation" => op_name_static, "status" => status)
        .increment(1);
    
    metrics::histogram!("graphql_query_duration_ms", "operation" => op_name_static)
        .record(duration_ms);
    
    if complexity > 0.0 {
        metrics::histogram!("graphql_query_complexity", "operation" => op_name_static)
            .record(complexity);
    }
    
    // Record error metrics
    if !response.errors.is_empty() {
        for error in &response.errors {
            // Extract error code from extensions if available
            let error_code_str = error
                .extensions
                .as_ref()
                .and_then(|ext| ext.get("code"))
                .and_then(|v| {
                    // Convert async_graphql::Value to string
                    match v {
                        async_graphql::Value::String(s) => Some(s.clone()),
                        _ => None,
                    }
                })
                .unwrap_or_else(|| "UNKNOWN_ERROR".to_string());
            
            // Convert to static string for metrics
            let error_code_static: &'static str = Box::leak(error_code_str.into_boxed_str());
            
            metrics::counter!("graphql_errors_total", "operation" => op_name_static, "error_code" => error_code_static)
                .increment(1);
        }
    }
    
    // Log slow queries
    if duration.as_millis() > 500 {
        tracing::warn!("Slow GraphQL query took {:?} (operation: {})", duration, operation_name);
        metrics::counter!("graphql_slow_queries_total", "operation" => op_name_static)
            .increment(1);
    }
    
    // Log errors
    if let Some(errors) = response.errors.first() {
        tracing::error!("GraphQL error: {} (operation: {})", errors.message, operation_name);
    }
    
    response.into()
}

/// GraphQL GET endpoint handler (for GraphiQL/Playground).
pub async fn graphql_playground() -> impl axum::response::IntoResponse {
    axum::response::Html(
        async_graphql::http::playground_source(
            async_graphql::http::GraphQLPlaygroundConfig::new("/graphql")
        )
    )
}
