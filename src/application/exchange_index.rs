//! Exchange index for fast lookup of exchange-to-tokens mapping.
//!
//! Builds and maintains an in-memory index from the local filesystem,
//! allowing fast lookups without GitHub API calls.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// In-memory index mapping exchanges to their tokens.
#[derive(Clone, Debug)]
pub struct ExchangeIndex {
    /// Map of exchange name -> list of token names
    exchange_to_tokens: Arc<RwLock<HashMap<String, Vec<String>>>>,
    /// Base data directory path
    data_path: String,
}

impl ExchangeIndex {
    /// Create a new exchange index.
    ///
    /// # Arguments
    ///
    /// * `data_path` - Path to the data directory (e.g., "/app/data" or "./data")
    pub fn new<P: AsRef<Path>>(data_path: P) -> Self {
        Self {
            exchange_to_tokens: Arc::new(RwLock::new(HashMap::new())),
            data_path: data_path.as_ref().to_string_lossy().to_string(),
        }
    }

    /// Build the index by scanning the local filesystem.
    ///
    /// This should be called at startup and periodically to refresh the index.
    pub async fn rebuild(&self) -> anyhow::Result<usize> {
        let data_path = Path::new(&self.data_path);
        
        if !data_path.exists() {
            warn!("Data directory does not exist: {}", self.data_path);
            return Ok(0);
        }

        let mut exchange_map: HashMap<String, Vec<String>> = HashMap::new();

        // Read all token directories
        let mut dir = match fs::read_dir(data_path).await {
            Ok(dir) => dir,
            Err(e) => {
                warn!("Failed to read data directory: {}", e);
                return Ok(0);
            }
        };

        while let Some(entry) = dir.next_entry().await? {
            let token_path = entry.path();
            
            // Only process directories (tokens)
            if !token_path.is_dir() {
                continue;
            }

            let token_name = entry.file_name().to_string_lossy().to_string();

            // Read exchanges for this token
            let mut token_dir = match fs::read_dir(&token_path).await {
                Ok(dir) => dir,
                Err(e) => {
                    warn!("Failed to read token directory {}: {}", token_name, e);
                    continue;
                }
            };

            while let Some(exchange_entry) = token_dir.next_entry().await? {
                let exchange_path = exchange_entry.path();
                
                if exchange_path.is_dir() {
                    let exchange_name = exchange_entry.file_name().to_string_lossy().to_string();
                    
                    exchange_map
                        .entry(exchange_name)
                        .or_insert_with(Vec::new)
                        .push(token_name.clone());
                }
            }
        }

        // Sort tokens for each exchange
        for tokens in exchange_map.values_mut() {
            tokens.sort();
        }

        let count = exchange_map.len();
        
        // Update the index
        *self.exchange_to_tokens.write().await = exchange_map;

        info!("Exchange index rebuilt: {} exchanges found", count);
        Ok(count)
    }

    /// Get tokens for a specific exchange.
    ///
    /// Returns an empty vector if the exchange is not found.
    pub async fn get_tokens(&self, exchange: &str) -> Vec<String> {
        let index = self.exchange_to_tokens.read().await;
        index
            .get(&exchange.to_lowercase())
            .cloned()
            .unwrap_or_else(|| {
                // Try case-insensitive match
                for (key, value) in index.iter() {
                    if key.to_lowercase() == exchange.to_lowercase() {
                        return value.clone();
                    }
                }
                vec![]
            })
    }

    /// Get all exchanges.
    pub async fn get_exchanges(&self) -> Vec<String> {
        let index = self.exchange_to_tokens.read().await;
        let mut exchanges: Vec<String> = index.keys().cloned().collect();
        exchanges.sort();
        exchanges
    }

    /// Check if the index has been built (has any data).
    pub async fn is_initialized(&self) -> bool {
        let index = self.exchange_to_tokens.read().await;
        !index.is_empty()
    }

    /// Get the count of exchanges in the index.
    pub async fn exchange_count(&self) -> usize {
        let index = self.exchange_to_tokens.read().await;
        index.len()
    }
}

