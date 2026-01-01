use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        // System & Content Handlers
        crate::api::handlers::health_handler,
        crate::api::handlers::metrics_handler,
        crate::api::handlers::rate_limit_handler,
        // Kaspa.com KRC20 Handlers
        crate::api::kaspacom_handlers::trade_stats_handler,
        crate::api::kaspacom_handlers::floor_price_handler,
        crate::api::kaspacom_handlers::sold_orders_handler,
        crate::api::kaspacom_handlers::last_order_sold_handler,
        crate::api::kaspacom_handlers::hot_mints_handler,
        crate::api::kaspacom_handlers::token_info_handler,
        crate::api::kaspacom_handlers::tokens_logos_handler,
        crate::api::kaspacom_handlers::open_orders_handler,
        crate::api::kaspacom_handlers::historical_data_handler,
        // Kaspa.com KRC721 Handlers
        crate::api::kaspacom_handlers::krc721_mints_handler,
        crate::api::kaspacom_handlers::krc721_sold_orders_handler,
        crate::api::kaspacom_handlers::krc721_listed_orders_handler,
        crate::api::kaspacom_handlers::krc721_trade_stats_handler,
        crate::api::kaspacom_handlers::krc721_hot_mints_handler,
        crate::api::kaspacom_handlers::krc721_floor_price_handler,
        crate::api::kaspacom_handlers::krc721_tokens_handler,
        crate::api::kaspacom_handlers::krc721_collection_info_handler,
        crate::api::kaspacom_handlers::krc721_metadata_handler,
        crate::api::kaspacom_handlers::krc721_image_url_handler,
        // Kaspa.com KNS Handlers
        crate::api::kaspacom_handlers::kns_sold_orders_handler,
        crate::api::kaspacom_handlers::kns_trade_stats_handler,
        crate::api::kaspacom_handlers::kns_listed_orders_handler,
        // Kaspa.com Configuration Handlers
        crate::api::kaspacom_handlers::available_tokens_handler,
        crate::api::kaspacom_handlers::token_exchanges_handler,
        crate::api::kaspacom_handlers::cache_stats_handler
    ),
    components(
        schemas(
            // Existing schemas
            crate::api::handlers::HealthResponse,
            crate::api::handlers::HealthDependencies,
            crate::api::handlers::RateLimitResponse,
            crate::api::handlers::RateLimitResources,
            crate::api::handlers::RateLimitInfo,
            // Kaspa.com schemas
            crate::domain::TradeStatsResponse,
            crate::domain::TokenTradeStats,
            crate::domain::FloorPriceEntry,
            crate::domain::SoldOrder,
            crate::domain::HotMint,
            crate::domain::TokenInfo,
            crate::domain::TokenLogo,
            crate::domain::OpenOrdersResponse,
            crate::domain::HistoricalDataResponse,
            crate::api::kaspacom_handlers::AvailableTokensResponse,
            crate::api::kaspacom_handlers::TokenExchangesResponse,
            crate::api::kaspacom_handlers::ErrorResponse,
            crate::domain::NftMint,
            crate::domain::NftOrder,
            crate::domain::NftTokensResponse,
            crate::domain::NftTradeStatsResponse,
            crate::domain::NftToken,
            crate::domain::NftCollectionStats,
            crate::domain::KnsOrder,
            crate::domain::KnsTradeStatsResponse,
            crate::domain::KnsListedOrdersResponse,
            crate::domain::Krc721CollectionInfo,
            crate::domain::NftMetadata,
            crate::domain::NftAttribute,
            crate::domain::CollectionMetadataInfo,
            crate::domain::CollectionHolder,
            crate::infrastructure::CacheStats,
            crate::infrastructure::CategoryStats
        )
    ),
    tags(
        (name = "system", description = "System endpoints for health checks and metrics"),
        (name = "KRC20", description = "KRC20 Token endpoints from Kaspa.com L1 Marketplace"),
        (name = "KRC721", description = "KRC721 NFT endpoints from Kaspa.com L1 Marketplace"),
        (name = "KNS", description = "KNS Domain endpoints from Kaspa.com L1 Marketplace"),
        (name = "Configuration", description = "API Configuration endpoints"),
        (name = "Cache", description = "Cache management and statistics")
    ),
    info(
        title = "KaspaDev KaspaCom Data API",
        version = "0.1.0",
        description = "Production-ready REST API gateway for accessing Kaspa.com L1 Marketplace data from GitHub repositories and Kaspa.com L1 Marketplace with Redis caching, rate limiting, and comprehensive observability.",
        contact(
            name = "KaspaDev",
            url = "https://github.com/KaspaDev/Kaspa-Exchange-Data"
        )
    )
)]
pub struct ApiDoc;
