//! Parquet-based local cache storage for Kaspa.com API data.
//!
//! This module provides efficient local storage for cached API responses
//! using the Parquet columnar format for compression and fast reads.

use anyhow::{Context, Result};
use arrow::array::{ArrayRef, RecordBatch, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow_json::reader::ReaderBuilder;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use parquet::arrow::ArrowWriter;
use parquet::basic::Compression;
use parquet::file::properties::WriterProperties;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn};
use utoipa::ToSchema;

/// Cache categories for organizing data
pub mod categories {
    pub const TOKEN_INFO: &str = "tokens";
    pub const TRADE_STATS: &str = "trade_stats";
    pub const FLOOR_PRICES: &str = "floor_prices";
    pub const HISTORICAL: &str = "historical";
    pub const ORDERS: &str = "orders";
    pub const HOT_MINTS: &str = "hot_mints";
    pub const LOGOS: &str = "logos";
    pub const KRC721: &str = "krc721";
    pub const KNS: &str = "kns";
}

/// Parquet-based local cache storage
///
/// Stores cached API responses as Parquet files organized by category.
/// Each cached entry also has a corresponding metadata JSON file to track
/// cache timestamps and TTL.
#[derive(Clone)]
pub struct ParquetStore {
    base_path: PathBuf,
}

impl ParquetStore {
    /// Create a new ParquetStore with the given base path
    pub fn new(base_path: &str) -> Self {
        let path = PathBuf::from(base_path);
        
        // Ensure base directory exists
        if let Err(e) = fs::create_dir_all(&path) {
            warn!("Failed to create cache directory {}: {}", base_path, e);
        }

        Self { base_path: path }
    }

    /// Get the Parquet file path for a cached entry
    fn parquet_path(&self, category: &str, key: &str) -> PathBuf {
        let category_path = self.base_path.join(category);
        category_path.join(format!("{}.parquet", key))
    }

    /// Get the metadata JSON file path for a cached entry
    fn metadata_path(&self, category: &str, key: &str) -> PathBuf {
        let category_path = self.base_path.join(category);
        category_path.join(format!("{}.meta.json", key))
    }

    /// Ensure the category directory exists
    fn ensure_category_dir(&self, category: &str) -> Result<()> {
        let category_path = self.base_path.join(category);
        fs::create_dir_all(&category_path)
            .with_context(|| format!("Failed to create category directory: {}", category))?;
        Ok(())
    }

    /// Check if a cached entry exists and is not expired
    pub fn is_valid(&self, category: &str, key: &str, max_age_secs: u64) -> bool {
        let meta_path = self.metadata_path(category, key);
        let parquet_path = self.parquet_path(category, key);

        // Both files must exist
        if !meta_path.exists() || !parquet_path.exists() {
            return false;
        }

        // Check metadata for expiration
        match self.read_metadata(&meta_path) {
            Ok(meta) => {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                let age = now.saturating_sub(meta.cached_at as u64);
                age < max_age_secs
            }
            Err(_) => false,
        }
    }

    /// Read cache metadata from JSON file
    fn read_metadata(&self, path: &Path) -> Result<CacheMetadata> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let meta: CacheMetadata = serde_json::from_reader(reader)?;
        Ok(meta)
    }

    /// Write cache metadata to JSON file
    fn write_metadata(&self, path: &Path, ttl_seconds: u64) -> Result<()> {
        let meta = CacheMetadata::new(ttl_seconds);
        let file = File::create(path)?;
        serde_json::to_writer_pretty(file, &meta)?;
        Ok(())
    }

    /// Write JSON value to Parquet file
    ///
    /// This method stores arbitrary JSON as Parquet by converting it to
    /// Arrow format and writing with compression.
    pub fn write_json(&self, category: &str, key: &str, data: &Value, ttl_seconds: u64) -> Result<()> {
        self.ensure_category_dir(category)?;
        
        let parquet_path = self.parquet_path(category, key);
        let meta_path = self.metadata_path(category, key);

        // Wrap single objects in an array for Arrow compatibility
        let json_array = match data {
            Value::Array(arr) => format!("[{}]", arr.iter()
                .map(|v| serde_json::to_string(v).unwrap_or_default())
                .collect::<Vec<_>>()
                .join(",")),
            _ => format!("[{}]", serde_json::to_string(data)?),
        };

        // Create Arrow schema from JSON
        let schema = self.infer_schema_from_json(data)?;
        
        // Convert JSON to Arrow RecordBatch
        let cursor = std::io::Cursor::new(json_array.as_bytes());
        let mut reader = ReaderBuilder::new(Arc::new(schema.clone()))
            .build(cursor)?;

        // Create Parquet writer with compression
        let file = File::create(&parquet_path)
            .with_context(|| format!("Failed to create Parquet file: {:?}", parquet_path))?;

        let props = WriterProperties::builder()
            .set_compression(Compression::SNAPPY)
            .build();

        let mut writer = ArrowWriter::try_new(file, Arc::new(schema), Some(props))?;

        // Write all batches
        while let Some(batch) = reader.next() {
            let batch = batch?;
            writer.write(&batch)?;
        }

        writer.close()?;

        // Write metadata
        self.write_metadata(&meta_path, ttl_seconds)?;

        debug!("Wrote cache entry: {}/{}", category, key);
        Ok(())
    }

    /// Infer Arrow schema from JSON value
    fn infer_schema_from_json(&self, _data: &Value) -> Result<Schema> {
        // For simplicity, we store complex data as a single JSON string column
        // This allows flexible schema while still benefiting from Parquet compression
        let fields = vec![
            Field::new("data", DataType::Utf8, false),
            Field::new("cached_at", DataType::Int64, false),
        ];
        Ok(Schema::new(fields))
    }

    /// Write data with simple schema (JSON string + metadata)
    ///
    /// This is the primary write method - stores JSON as a string in Parquet
    /// for maximum flexibility.
    pub fn write_simple(&self, category: &str, key: &str, data: &Value, ttl_seconds: u64) -> Result<()> {
        self.ensure_category_dir(category)?;
        
        let parquet_path = self.parquet_path(category, key);
        let meta_path = self.metadata_path(category, key);

        // Serialize data to JSON string
        let json_string = serde_json::to_string(data)?;
        let now = chrono::Utc::now().timestamp();

        // Create simple schema
        let schema = Arc::new(Schema::new(vec![
            Field::new("data", DataType::Utf8, false),
            Field::new("cached_at", DataType::Int64, false),
        ]));

        // Create record batch
        let data_array: ArrayRef = Arc::new(StringArray::from(vec![json_string.as_str()]));
        let cached_at_array: ArrayRef = Arc::new(arrow::array::Int64Array::from(vec![now]));

        let batch = RecordBatch::try_new(schema.clone(), vec![data_array, cached_at_array])?;

        // Write to Parquet
        let file = File::create(&parquet_path)
            .with_context(|| format!("Failed to create Parquet file: {:?}", parquet_path))?;

        let props = WriterProperties::builder()
            .set_compression(Compression::SNAPPY)
            .build();

        let mut writer = ArrowWriter::try_new(file, schema, Some(props))?;
        writer.write(&batch)?;
        writer.close()?;

        // Write metadata
        self.write_metadata(&meta_path, ttl_seconds)?;

        debug!("Wrote cache entry (simple): {}/{}", category, key);
        Ok(())
    }

    /// Read JSON value from Parquet file
    ///
    /// Returns None if the file doesn't exist or is corrupted.
    pub fn read_json(&self, category: &str, key: &str) -> Result<Option<Value>> {
        let parquet_path = self.parquet_path(category, key);

        if !parquet_path.exists() {
            return Ok(None);
        }

        let file = File::open(&parquet_path)
            .with_context(|| format!("Failed to open Parquet file: {:?}", parquet_path))?;

        let builder = ParquetRecordBatchReaderBuilder::try_new(file)?;
        let mut reader = builder.build()?;

        // Read first batch
        if let Some(batch) = reader.next() {
            let batch = batch?;
            
            // Get the data column
            if let Some(col) = batch.column_by_name("data") {
                if let Some(string_array) = col.as_any().downcast_ref::<StringArray>() {
                    if let Some(json_str) = string_array.value(0).into() {
                        let value: Value = serde_json::from_str(json_str)?;
                        debug!("Read cache entry: {}/{}", category, key);
                        return Ok(Some(value));
                    }
                }
            }
        }

        Ok(None)
    }

    /// Read and deserialize typed data from cache
    pub fn read<T: DeserializeOwned>(&self, category: &str, key: &str) -> Result<Option<T>> {
        match self.read_json(category, key)? {
            Some(value) => {
                let data: T = serde_json::from_value(value)?;
                Ok(Some(data))
            }
            None => Ok(None),
        }
    }

    /// Write typed data to cache
    pub fn write<T: Serialize>(&self, category: &str, key: &str, data: &T, ttl_seconds: u64) -> Result<()> {
        let value = serde_json::to_value(data)?;
        self.write_simple(category, key, &value, ttl_seconds)
    }

    /// List all cached keys in a category
    pub fn list_keys(&self, category: &str) -> Result<Vec<String>> {
        let category_path = self.base_path.join(category);
        
        if !category_path.exists() {
            return Ok(vec![]);
        }

        let mut keys = Vec::new();
        for entry in fs::read_dir(&category_path)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().map_or(false, |ext| ext == "parquet") {
                if let Some(stem) = path.file_stem() {
                    keys.push(stem.to_string_lossy().to_string());
                }
            }
        }

        Ok(keys)
    }

    /// Delete a cached entry
    pub fn delete(&self, category: &str, key: &str) -> Result<()> {
        let parquet_path = self.parquet_path(category, key);
        let meta_path = self.metadata_path(category, key);

        if parquet_path.exists() {
            fs::remove_file(&parquet_path)?;
        }
        if meta_path.exists() {
            fs::remove_file(&meta_path)?;
        }

        debug!("Deleted cache entry: {}/{}", category, key);
        Ok(())
    }

    /// Delete all expired entries in a category
    pub fn cleanup_expired(&self, category: &str, max_age_secs: u64) -> Result<usize> {
        let keys = self.list_keys(category)?;
        let mut deleted = 0;

        for key in keys {
            if !self.is_valid(category, &key, max_age_secs) {
                self.delete(category, &key)?;
                deleted += 1;
            }
        }

        if deleted > 0 {
            info!("Cleaned up {} expired entries from {}", deleted, category);
        }

        Ok(deleted)
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> Result<CacheStats> {
        let mut total_keys = 0;
        let mut total_size = 0u64;
        let mut category_stats = std::collections::HashMap::new();

        for category in &[
            categories::TOKEN_INFO,
            categories::TRADE_STATS,
            categories::FLOOR_PRICES,
            categories::HISTORICAL,
            categories::ORDERS,
            categories::HOT_MINTS,
            categories::LOGOS,
            categories::KRC721,
            categories::KNS,
        ] {
            let keys = self.list_keys(category).unwrap_or_default();
            let mut cat_size = 0u64;
            
            if !keys.is_empty() {
                // Calculate size
                let category_path = self.base_path.join(category);
                for key in &keys {
                    let parquet_path = category_path.join(format!("{}.parquet", key));
                    if let Ok(metadata) = fs::metadata(&parquet_path) {
                        cat_size += metadata.len();
                    }
                }
                
                total_keys += keys.len();
                total_size += cat_size;
            }

            category_stats.insert(category.to_string(), CategoryStats {
                keys: keys.len(),
                size_bytes: cat_size,
                description: self.get_category_description(category),
                hits: 0, // Will be set by CacheService
                misses: 0, // Will be set by CacheService
                requests: 0, // Will be set by CacheService
            });
        }

        Ok(CacheStats {
            total_keys,
            total_size_bytes: total_size,
            categories_count: category_stats.len(),
            base_path: self.base_path.to_string_lossy().to_string(),
            categories: category_stats,
            cache_hits: 0, // Will be set by CacheService
        })
    }

    fn get_category_description(&self, category: &str) -> String {
        match category {
            categories::TOKEN_INFO => "Token Information (Supply, Market Cap)",
            categories::TRADE_STATS => "Trade Statistics (Volume, High/Low)",
            categories::FLOOR_PRICES => "Floor Prices",
            categories::HISTORICAL => "Historical Data (OHLCV)",
            categories::ORDERS => "Market Orders",
            categories::HOT_MINTS => "Trending Mints",
            categories::LOGOS => "Token Images",
            categories::KRC721 => "NFT Collections & Metadata",
            categories::KNS => "Kaspa Name Service",
            _ => "Unknown Category",
        }.to_string()
    }
}

/// Cache metadata stored alongside each Parquet file
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CacheMetadata {
    /// Unix timestamp when cached
    pub cached_at: i64,
    /// Data source
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
}

/// Detailed statistics for a cache category
#[derive(Debug, Clone, serde::Serialize, ToSchema)]
pub struct CategoryStats {
    pub keys: usize,
    pub size_bytes: u64,
    pub description: String,
    /// Number of cache hits for this category
    #[serde(default)]
    pub hits: u64,
    /// Number of cache misses for this category
    #[serde(default)]
    pub misses: u64,
    /// Total number of requests for this category
    #[serde(default)]
    pub requests: u64,
}

/// Cache statistics
#[derive(Debug, Clone, serde::Serialize, ToSchema)]
pub struct CacheStats {
    pub total_keys: usize,
    pub total_size_bytes: u64,
    pub categories_count: usize,
    pub base_path: String,
    pub categories: std::collections::HashMap<String, CategoryStats>,
    /// Number of requests served from cache (incremented on cache hits)
    #[serde(default)]
    pub cache_hits: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn test_parquet_store_write_read() {
        let dir = tempdir().unwrap();
        let store = ParquetStore::new(dir.path().to_str().unwrap());

        let data = json!({
            "ticker": "SLOW",
            "price": 0.00015,
            "volume": 1000.5
        });

        // Write
        store.write_simple("test", "test_key", &data, 3600).unwrap();

        // Check validity
        assert!(store.is_valid("test", "test_key", 3600));

        // Read back
        let read_data = store.read_json("test", "test_key").unwrap();
        assert!(read_data.is_some());
        
        let read_value = read_data.unwrap();
        assert_eq!(read_value["ticker"], "SLOW");
        assert_eq!(read_value["price"], 0.00015);
    }

    #[test]
    fn test_list_keys() {
        let dir = tempdir().unwrap();
        let store = ParquetStore::new(dir.path().to_str().unwrap());

        store.write_simple("tokens", "SLOW", &json!({"a": 1}), 3600).unwrap();
        store.write_simple("tokens", "NACHO", &json!({"b": 2}), 3600).unwrap();

        let keys = store.list_keys("tokens").unwrap();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"SLOW".to_string()));
        assert!(keys.contains(&"NACHO".to_string()));
    }

    #[test]
    fn test_cache_stats_serialization() {
        let mut categories = std::collections::HashMap::new();
        categories.insert("test".to_string(), CategoryStats {
            keys: 10,
            size_bytes: 1000,
            description: "Test".to_string(),
            hits: 0,
            misses: 0,
            requests: 0,
        });

        let mut cat_stats = std::collections::HashMap::new();
        cat_stats.insert("test".to_string(), CategoryStats {
            keys: 10,
            size_bytes: 1000,
            description: "Test".to_string(),
            hits: 0,
            misses: 0,
            requests: 0,
        });
        
        let stats = CacheStats {
            total_keys: 10,
            total_size_bytes: 1000,
            categories_count: 1,
            base_path: "data".to_string(),
            categories: cat_stats,
            cache_hits: 0,
        };

        let json = serde_json::to_string(&stats).unwrap();
        println!("JSON: {}", json);
        assert!(json.contains("\"categories\""));
        assert!(json.contains("\"test\""));
    }
}
