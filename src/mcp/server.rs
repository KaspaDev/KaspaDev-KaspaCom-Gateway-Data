//! READONLY MCP (Model Context Protocol) server for data access.
//!
//! This module implements a read-only MCP server that exposes exchange data
//! through the Model Context Protocol. The server communicates via JSON-RPC 2.0
//! over stdio or HTTP.
//!
//! # Usage
//!
//! The MCP server can be run as a standalone binary that communicates via stdio:
//!
//! ```bash
//! cargo run --bin mcp-server
//! ```
//!
//! Or integrated into the main API server as an HTTP endpoint.

use crate::api::state::AppState;
use jsonrpc_core::{Error, ErrorCode, Params, Result, Value};
use jsonrpc_derive::rpc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// MCP server implementation.
#[derive(Clone)]
pub struct McpServer {
    state: Arc<AppState>,
}

impl McpServer {
    pub fn new(state: AppState) -> Self {
        Self {
            state: Arc::new(state),
        }
    }
}

#[rpc]
pub trait McpRpc {
    /// Get ticker statistics for a token.
    #[rpc(name = "get_ticker_stats")]
    fn get_ticker_stats(&self, token: String, range: Option<String>) -> Result<Value>;

    /// Get exchange-specific data.
    #[rpc(name = "get_exchange_data")]
    fn get_exchange_data(
        &self,
        exchange: String,
        token: Option<String>,
        range: Option<String>,
    ) -> Result<Value>;

    /// List all available tokens.
    #[rpc(name = "list_tokens")]
    fn list_tokens(&self) -> Result<Value>;

    /// List all exchanges.
    #[rpc(name = "list_exchanges")]
    fn list_exchanges(&self) -> Result<Value>;

    /// Get timeseries data for a token.
    #[rpc(name = "get_timeseries")]
    fn get_timeseries(
        &self,
        token: String,
        range: String,
        resolution: String,
    ) -> Result<Value>;
}

impl McpRpc for McpServer {
    fn get_ticker_stats(&self, token: String, range: Option<String>) -> Result<Value> {
        let range = range.unwrap_or_else(|| "today".to_string());
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            match self.state.ticker_service.get_ticker_stats(token, range).await {
                Ok(response) => serde_json::to_value(response)
                    .map_err(|e| Error::new(ErrorCode::InternalError, Some(e.to_string()), None)),
                Err(e) => Err(Error::new(
                    ErrorCode::InternalError,
                    Some(e.to_string()),
                    None,
                )),
            }
        })
    }

    fn get_exchange_data(
        &self,
        exchange: String,
        token: Option<String>,
        range: Option<String>,
    ) -> Result<Value> {
        let range = range.unwrap_or_else(|| "today".to_string());
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let result = if let Some(token) = token {
                // If token specified, get exchange detail for that token
                // This is a simplified version - full implementation would filter by token
                self.state.ticker_service.get_exchange_detail(exchange, range).await
            } else {
                self.state.ticker_service.get_exchange_detail(exchange, range).await
            };

            match result {
                Ok(response) => serde_json::to_value(response)
                    .map_err(|e| Error::new(ErrorCode::InternalError, Some(e.to_string()), None)),
                Err(e) => Err(Error::new(
                    ErrorCode::InternalError,
                    Some(e.to_string()),
                    None,
                )),
            }
        })
    }

    fn list_tokens(&self) -> Result<Value> {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            match self.state.ticker_service.get_available_tickers().await {
                Ok(response) => serde_json::to_value(response)
                    .map_err(|e| Error::new(ErrorCode::InternalError, Some(e.to_string()), None)),
                Err(e) => Err(Error::new(
                    ErrorCode::InternalError,
                    Some(e.to_string()),
                    None,
                )),
            }
        })
    }

    fn list_exchanges(&self) -> Result<Value> {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            match self.state.ticker_service.get_exchanges().await {
                Ok(response) => serde_json::to_value(response)
                    .map_err(|e| Error::new(ErrorCode::InternalError, Some(e.to_string()), None)),
                Err(e) => Err(Error::new(
                    ErrorCode::InternalError,
                    Some(e.to_string()),
                    None,
                )),
            }
        })
    }

    fn get_timeseries(
        &self,
        token: String,
        range: String,
        resolution: String,
    ) -> Result<Value> {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            match self
                .state
                .ticker_service
                .get_timeseries(token, range, resolution)
                .await
            {
                Ok(response) => serde_json::to_value(response)
                    .map_err(|e| Error::new(ErrorCode::InternalError, Some(e.to_string()), None)),
                Err(e) => Err(Error::new(
                    ErrorCode::InternalError,
                    Some(e.to_string()),
                    None,
                )),
            }
        })
    }
}

