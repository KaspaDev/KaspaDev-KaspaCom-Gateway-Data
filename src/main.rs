//! KaspaDev KaspaCom Data API Gateway
//!
//! A production-ready REST API gateway for accessing Kaspa exchange data from GitHub repositories,
//! with Redis caching, rate limiting, and comprehensive observability.
//!
//! # Architecture
//!
//! The API follows clean/onion architecture with clear separation of concerns:
//! - **Domain**: Core business entities and repository traits
//! - **Application**: Business logic and use cases
//! - **Infrastructure**: External integrations (GitHub, Redis)
//! - **API**: HTTP handlers, routing, and middleware
//!
//! # Features
//!
//! - ✅ GitHub API integration with rate limit handling and exponential backoff
//! - ✅ Redis caching with connection pooling for performance
//! - ✅ Prometheus metrics for observability
//! - ✅ Request correlation IDs for distributed tracing
//! - ✅ Input validation and proper error handling
//! - ✅ Graceful shutdown for zero-downtime deployments
//! - ✅ API versioning (`/api-docs/` prefix)
//!
//! # Configuration
//!
//! The API is configured via `config.yaml` and environment variables:
//! - `GITHUB_TOKEN`: GitHub personal access token (optional)
//!   - If set: Uses authenticated requests (5,000 req/hour limit)
//!   - If not set: Uses unauthenticated requests (60 req/hour limit for public repos)
//! - `REDIS_URL`: Redis connection string (default: redis://localhost:6379)
//! - `RUST_LOG`: Logging level (default: info)
//!
//! # Quick Start
//!
//! ```bash
//! # Optional: Set environment variables for higher rate limits
//! export GITHUB_TOKEN="your_token_here"  # Optional - works without it for public repos
//! export REDIS_URL="redis://localhost:6379"
//!
//! # Run the server
//! cargo run --release
//!
//! # Test endpoints
//! curl http://localhost:3010/health
//! curl http://localhost:3010/metrics
//! curl "http://localhost:3010/v1/api/github/owner/repo/path"
//! ```

mod api;
mod application;
mod domain;
mod infrastructure;

use crate::api::routes::create_router;
use crate::api::state::AppState;
use crate::application::{CacheService, ContentService, ExchangeIndex, KaspaComService, TickerService};
use crate::domain::{RepoConfig, TokensConfig};
use crate::infrastructure::{GitHubRepository, KaspaComClient, LocalFileRepository, ParquetStore, RateLimiter, RedisRepository};
use anyhow::Context;
use serde::Deserialize;
use std::env;
use std::fs;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Top-level application configuration loaded from `config.yaml`.
///
/// Contains server settings and repository whitelist configuration.
#[derive(Deserialize, Debug, Clone)]
struct Config {
    /// Server configuration (host, port, CORS origins)
    server: ServerConfig,
    /// Rate limiting configuration for kaspa.com API
    #[serde(default)]
    rate_limit: RateLimitConfig,
    /// List of allowed repositories that can be accessed through the API
    allowed_repos: Vec<RepoConfig>,
}

/// Rate limiting configuration
#[derive(Deserialize, Debug, Clone, Default)]
struct RateLimitConfig {
    /// Maximum requests per minute to kaspa.com API
    #[serde(default = "default_requests_per_minute")]
    requests_per_minute: u32,
}

fn default_requests_per_minute() -> u32 {
    1000
}

/// Server configuration settings.
///
/// Defines how the HTTP server should bind and what CORS origins to allow.
#[derive(Deserialize, Debug, Clone)]
struct ServerConfig {
    /// Host address to bind to (default: "0.0.0.0")
    #[serde(default = "default_host")]
    host: String,
    /// Port number to listen on (default: 3010)
    #[serde(default = "default_port")]
    port: u16,
    /// Comma-separated list of allowed CORS origins (default: "*")
    #[serde(default = "default_allowed_origins")]
    allowed_origins: String,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}
fn default_port() -> u16 {
    3010
}
fn default_allowed_origins() -> String {
    "*".to_string()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let github_token = env::var("GITHUB_TOKEN").ok();
    if github_token.is_none() {
        tracing::warn!("GITHUB_TOKEN not found in env - using unauthenticated requests (60 req/hour limit for public repos). For higher limits (5,000 req/hour), set GITHUB_TOKEN in .env");
    } else {
        tracing::info!("GITHUB_TOKEN found - using authenticated requests (5,000 req/hour limit)");
    }

    let log_format = std::env::var("LOG_FORMAT").unwrap_or_else(|_| "text".to_string());
    let env_filter = EnvFilter::new(
        std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
    );

    if log_format.eq_ignore_ascii_case("json") {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer().json())
            .init();
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer())
            .init();
    }

    // Load Config
    let config_content = fs::read_to_string("config.yaml")
        .context("Failed to read config.yaml - ensure file exists in working directory")?;
    let config: Config = serde_yaml::from_str(&config_content)
        .context("Failed to parse config.yaml - check YAML syntax and structure")?;

    let redis_url = env::var("REDIS_URL").ok();

    // Infrastructure
    let github_repo = Arc::new(GitHubRepository::new(github_token));
    let redis_repo = Arc::new(RedisRepository::new(redis_url));

    // Try to initialize local file repository (for Docker volume mounts)
    let data_path = std::env::var("DATA_PATH").unwrap_or_else(|_| "/app/data".to_string());
    let local_repo: Option<Arc<LocalFileRepository>> = {
        let repo = Arc::new(LocalFileRepository::new(&data_path));
        if repo.is_available() {
            tracing::info!("Local filesystem repository available at: {}", data_path);
            Some(repo)
        } else {
            tracing::warn!("Local filesystem repository not available at: {}, using GitHub API only", data_path);
            None
        }
    };

    // Initialize exchange index if local repo is available
    let exchange_index: Option<Arc<ExchangeIndex>> = if local_repo.is_some() {
        let index = Arc::new(ExchangeIndex::new(&data_path));
        // Build index in background (non-blocking)
        let index_clone = index.clone();
        tokio::spawn(async move {
            if let Err(e) = index_clone.rebuild().await {
                tracing::warn!("Failed to build exchange index: {}", e);
            }
        });
        Some(index)
    } else {
        None
    };

    // Get default repo for ticker service (first allowed repo)
    let default_repo = config
        .allowed_repos
        .first()
        .cloned()
        .expect("At least one allowed repo must be configured");

    // Application
    let content_service = Arc::new(ContentService::new(
        github_repo.clone(),
        redis_repo.clone(),
        config.allowed_repos.clone(),
    ));

    let ticker_service = Arc::new(TickerService::with_local(
        github_repo,
        local_repo.map(|r| r as Arc<dyn crate::domain::ContentRepository>),
        redis_repo.clone(),
        default_repo,
        exchange_index,
    ));

    // ========================================================================
    // Kaspa.com L1 Marketplace API (heavy-cache layer)
    // ========================================================================
    
    // Load tokens configuration
    let tokens_config_path = env::var("TOKENS_CONFIG_PATH")
        .unwrap_or_else(|_| "data/tokens_config.json".to_string());
    let tokens_config = TokensConfig::load(&tokens_config_path)
        .unwrap_or_else(|e| {
            tracing::warn!("Failed to load tokens_config.json: {}, using empty config", e);
            TokensConfig { tokens: std::collections::HashMap::new() }
        });
    tracing::info!("Loaded {} tokens from configuration", tokens_config.get_tokens().len());

    // Initialize Parquet cache storage
    let cache_path = env::var("CACHE_PATH").unwrap_or_else(|_| "data/cache".to_string());
    let parquet_store = Arc::new(ParquetStore::new(&cache_path));
    tracing::info!("Parquet cache storage initialized at: {}", cache_path);

    // Initialize rate limiter for kaspa.com API
    let rate_limiter = Arc::new(RateLimiter::new(config.rate_limit.requests_per_minute));
    tracing::info!("Rate limiter initialized: {} requests/minute", config.rate_limit.requests_per_minute);

    // Initialize Kaspa.com API client
    let kaspacom_client = Arc::new(KaspaComClient::new());

    // Create tiered cache service (Redis + Parquet)
    let cache_service = Arc::new(CacheService::new(
        redis_repo,
        parquet_store,
        kaspacom_client,
        rate_limiter.clone(),
    ));

    // Create Kaspa.com service
    let kaspacom_service = Arc::new(KaspaComService::new(
        cache_service,
        tokens_config,
    ));

    let state = AppState {
        content_service,
        ticker_service,
        kaspacom_service,
        rate_limiter,
    };

    let app = create_router(state, config.server.allowed_origins.clone());

    // Allow PORT env var override
    let port = env::var("PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(config.server.port);
    let addr = format!("{}:{}", config.server.host, port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .with_context(|| format!("Failed to bind to address {}", addr))?;
    tracing::info!("GitRows Rust API server running at http://{}", addr);
    tracing::info!("Allowed repos: {:?}", config.allowed_repos);

    // Graceful shutdown handling
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("Server error during operation")?;

    Ok(())
}

/// Wait for SIGTERM or SIGINT (Ctrl+C) to initiate graceful shutdown
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received Ctrl+C, initiating graceful shutdown");
        },
        _ = terminate => {
            tracing::info!("Received SIGTERM, initiating graceful shutdown");
        },
    }
}
