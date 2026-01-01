use crate::api::doc::ApiDoc;
use crate::api::graphql::{create_schema, graphql_handler, graphql_playground};
use crate::api::handlers::{content_handler, health_handler, metrics_handler, rate_limit_handler, dashboard_handler, dashboard_js_handler, dashboard_css_handler};
use crate::api::kaspacom_handlers::{
    // KRC20 handlers
    trade_stats_handler, floor_price_handler, sold_orders_handler, last_order_sold_handler,
    hot_mints_handler, token_info_handler, tokens_logos_handler, open_orders_handler,
    historical_data_handler,
    // KRC721 handlers
    krc721_mints_handler, krc721_sold_orders_handler, krc721_listed_orders_handler,
    krc721_trade_stats_handler, krc721_hot_mints_handler, krc721_floor_price_handler,
    krc721_tokens_handler, krc721_collection_info_handler, krc721_metadata_handler,
    krc721_image_url_handler,
    // KNS handlers
    kns_sold_orders_handler, kns_trade_stats_handler, kns_listed_orders_handler,
    // Configuration handlers
    available_tokens_handler as kaspa_tokens_handler, token_exchanges_handler, cache_stats_handler,
};
use crate::api::state::AppState;
use axum::{routing::{get, post}, Router};

use std::time::Duration;
use tower::ServiceBuilder;
use axum::http::HeaderValue;
use tower_http::cors::{Any, AllowOrigin, CorsLayer};
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::timeout::TimeoutLayer;
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::TraceLayer;
use tracing::Level;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

pub fn create_router(state: AppState, allowed_origins: String) -> Router {
    // Create GraphQL schema
    let schema = create_schema(state.clone());
    // Configure CORS based on configuration
    let cors = if allowed_origins == "*" {
        CorsLayer::permissive()
    } else {
        // Parse comma-separated origins, filter out invalid ones
        let origin_values: Vec<HeaderValue> = allowed_origins
            .split(',')
            .filter_map(|s| {
                let trimmed = s.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    trimmed.parse::<HeaderValue>().ok()
                }
            })
            .collect();
        
        if origin_values.is_empty() {
            tracing::warn!("No valid CORS origins found, falling back to permissive CORS");
            CorsLayer::permissive()
        } else if origin_values.len() == 1 {
            // Single origin
            CorsLayer::new()
                .allow_origin(AllowOrigin::exact(origin_values[0].clone()))
                .allow_methods(Any)
                .allow_headers(Any)
        } else {
            // Multiple origins - use list
            CorsLayer::new()
                .allow_origin(AllowOrigin::list(origin_values))
                .allow_methods(Any)
                .allow_headers(Any)
        }
    };

    // Create middleware stack with security headers and observability
    let middleware = ServiceBuilder::new()
        // Request tracing and metrics
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &axum::http::Request<_>| {
                    let method = request.method();
                    let uri = request.uri();
                    let path = uri.path();
                    
                    tracing::span!(
                        Level::INFO,
                        "http_request",
                        method = %method,
                        path = %path,
                        uri = %uri
                    )
                })
                .on_request(|_request: &axum::http::Request<_>, _span: &tracing::Span| {
                    // Request started
                })
                .on_response(|response: &axum::http::Response<_>, latency: std::time::Duration, _span: &tracing::Span| {
                    let status = response.status();
                    let status_code = status.as_u16();
                    let status_class_str: &'static str = Box::leak(format!("{}xx", status_code / 100).into_boxed_str());
                    let status_code_str: &'static str = Box::leak(status_code.to_string().into_boxed_str());
                    
                    // Extract method and path from span if available, otherwise use defaults
                    let method = _span.metadata()
                        .and_then(|m| m.fields().iter().find(|f| f.name() == "method"))
                        .map(|_| "GET") // Simplified - actual method would come from request
                        .unwrap_or("unknown");
                    let path = _span.metadata()
                        .and_then(|m| m.fields().iter().find(|f| f.name() == "path"))
                        .map(|_| "unknown")
                        .unwrap_or("unknown");
                    
                    // Record comprehensive metrics
                    metrics::counter!("http_requests_total", "method" => method, "path" => path, "status" => status_code_str, "status_class" => status_class_str)
                        .increment(1);
                    
                    // Record duration metrics
                    metrics::histogram!("http_request_duration_seconds", "method" => method, "path" => path, "status" => status_code_str)
                        .record(latency.as_secs_f64());
                    
                    // Log slow requests
                    if latency.as_millis() > 1000 {
                        tracing::warn!("Slow HTTP request: {}ms", latency.as_millis());
                    }
                })
                .on_failure(|_error: tower_http::classify::ServerErrorsFailureClass, _latency: std::time::Duration, _span: &tracing::Span| {
                    metrics::counter!("http_requests_total", "status" => "error", "status_class" => "5xx")
                        .increment(1);
                })
        )
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(TimeoutLayer::with_status_code(
            axum::http::StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(60),
        ))
        // Security headers
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::X_FRAME_OPTIONS,
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::X_XSS_PROTECTION,
            HeaderValue::from_static("1; mode=block"),
        ))
        .layer(cors);

    Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        // Dashboard (development)
        .route("/dashboard", get(dashboard_handler))
        .route("/krcbot-dashboard.js", get(dashboard_js_handler))
        .route("/theme.css", get(dashboard_css_handler))
        // System endpoints (no versioning)
        .route("/health", get(health_handler))
        .route("/metrics", get(metrics_handler))
        .route("/rate-limit", get(rate_limit_handler))
        // OpenAPI spec (downloadable)
        .route("/v1/openapi.json", get(|| async { axum::Json(ApiDoc::openapi()) }))
        // V1 API endpoints (existing GitHub-based)
        // V1 API endpoints (existing GitHub-based) - moved to bottom

        // Ticker convenience endpoints (legacy - removed)
        // .route("/v1/tickers", get(available_tickers_handler))
        // .route("/v1/exchanges", get(exchanges_handler))
        // .route("/v1/exchange/{exchange}", get(exchange_detail_handler))
        // .route("/v1/ticker/{token}", get(ticker_stats_handler))
        // .route("/v1/ticker/{token}/history", get(ticker_history_handler))
        // .route("/v1/ticker/{token}/timeseries", get(ticker_timeseries_handler))
        // ====================================================================
        // Kaspa.com L1 Marketplace API (heavy-cache layer)
        // ====================================================================
        // KRC20 Token endpoints
        .route("/v1/api/kaspa/trade-stats", get(trade_stats_handler))
        .route("/v1/api/kaspa/floor-price", get(floor_price_handler))
        .route("/v1/api/kaspa/sold-orders", get(sold_orders_handler))
        .route("/v1/api/kaspa/last-order-sold", get(last_order_sold_handler))
        .route("/v1/api/kaspa/hot-mints", get(hot_mints_handler))
        .route("/v1/api/kaspa/token-info/{ticker}", get(token_info_handler))
        .route("/v1/api/kaspa/tokens-logos", get(tokens_logos_handler))
        .route("/v1/api/kaspa/open-orders", get(open_orders_handler))
        .route("/v1/api/kaspa/historical-data", get(historical_data_handler))
        // KRC721 NFT endpoints
        .route("/v1/api/kaspa/krc721/mint", get(krc721_mints_handler))
        .route("/v1/api/kaspa/krc721/sold-orders", get(krc721_sold_orders_handler))
        .route("/v1/api/kaspa/krc721/listed-orders", get(krc721_listed_orders_handler))
        .route("/v1/api/kaspa/krc721/trade-stats", get(krc721_trade_stats_handler))
        .route("/v1/api/kaspa/krc721/hot-mints", get(krc721_hot_mints_handler))
        .route("/v1/api/kaspa/krc721/floor-price", get(krc721_floor_price_handler))
        .route("/v1/api/kaspa/krc721/tokens", post(krc721_tokens_handler))
        .route("/v1/api/kaspa/krc721/collection/{ticker}", get(krc721_collection_info_handler))
        .route("/v1/api/kaspa/krc721/metadata/{ticker}/{token_id}", get(krc721_metadata_handler))
        .route("/v1/api/kaspa/krc721/image/{ticker}/{token_id}", get(krc721_image_url_handler))
        // KNS Domain endpoints
        .route("/v1/api/kaspa/kns/sold-orders", get(kns_sold_orders_handler))
        .route("/v1/api/kaspa/kns/trade-stats", get(kns_trade_stats_handler))
        .route("/v1/api/kaspa/kns/listed-orders", get(kns_listed_orders_handler))
        // Configuration & Cache endpoints
        .route("/v1/api/kaspa/tokens", get(kaspa_tokens_handler))
        .route("/v1/api/kaspa/tokens/{token}/exchanges", get(token_exchanges_handler))
        .route("/v1/api/kaspa/cache/stats", get(cache_stats_handler))
        // GraphQL endpoint (schema passed via extension layer)
        .route("/graphql", get(graphql_playground).post(graphql_handler))
        // Legacy route for backwards compatibility (can be removed later)
        .route("/api/{source}/{owner}/{repo}/{*path}", get(content_handler))
        // Generic V1 API (moved here to allow specific routes to take precedence)
        .route(
            "/v1/api/{source}/{owner}/{repo}/{*path}",
            get(content_handler),
        )
        .layer(axum::Extension(schema))
        .layer(middleware)
        .with_state(state)
}
