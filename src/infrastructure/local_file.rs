//! Local filesystem repository implementation.
//!
//! Reads data directly from the local filesystem, useful when data is mounted
//! as a volume (e.g., in Docker). Falls back gracefully when files don't exist.

use crate::domain::{Content, ContentRepository, ContentType, RepoConfig};
use async_trait::async_trait;
use serde_json::Value;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::warn;

/// Local filesystem repository that reads from a base directory.
pub struct LocalFileRepository {
    base_path: PathBuf,
}

impl LocalFileRepository {
    /// Create a new local file repository.
    ///
    /// # Arguments
    ///
    /// * `base_path` - Base directory path (e.g., "/app/data" or "./data")
    ///
    /// # Examples
    ///
    /// ```
    /// use gatewayapi::infrastructure::LocalFileRepository;
    ///
    /// let repo = LocalFileRepository::new("/app/data");
    /// ```
    pub fn new<P: AsRef<Path>>(base_path: P) -> Self {
        Self {
            base_path: base_path.as_ref().to_path_buf(),
        }
    }

    /// Check if the base path exists and is accessible.
    pub fn is_available(&self) -> bool {
        self.base_path.exists() && self.base_path.is_dir()
    }

    fn resolve_path(&self, path: &str) -> PathBuf {
        // Remove leading slash and "data/" or "data" prefix if present
        // This allows paths like "data" or "data/kaspa" to work correctly
        let mut clean_path = path.trim_start_matches('/');
        if clean_path == "data" {
            clean_path = "";
        } else if clean_path.starts_with("data/") {
            clean_path = &clean_path[5..]; // Remove "data/" prefix
        }
        
        // Security: Prevent path traversal attacks
        // Normalize the path and ensure it stays within base_path
        let joined = self.base_path.join(clean_path);
        
        // Canonicalize both paths to resolve any .. or . components
        // Then verify the resolved path is still within base_path
        if let (Ok(canonical_joined), Ok(base_canonical)) = (
            std::fs::canonicalize(&joined),
            std::fs::canonicalize(&self.base_path)
        ) {
            if canonical_joined.starts_with(&base_canonical) {
                canonical_joined
            } else {
                // Path traversal detected - return base_path to prevent access
                warn!("Path traversal attempt detected: {}", path);
                self.base_path.clone()
            }
        } else {
            // If canonicalization fails (path doesn't exist yet), use join but validate components
            // Reject paths containing ".." or starting with "/"
            if clean_path.contains("..") || clean_path.starts_with('/') {
                warn!("Invalid path component detected: {}", path);
                self.base_path.clone()
            } else {
                joined
            }
        }
    }

    async fn list_directory_internal(&self, path: &Path) -> anyhow::Result<Vec<Content>> {
        let mut entries = Vec::new();

        let mut dir = fs::read_dir(path).await?;
        while let Some(entry) = dir.next_entry().await? {
            let file_path = entry.path();
            let file_name = entry.file_name();
            let name = file_name.to_string_lossy().to_string();

            let metadata = entry.metadata().await?;
            let item_type = if metadata.is_dir() {
                ContentType::Dir
            } else if metadata.is_file() {
                ContentType::File
            } else {
                ContentType::Unknown
            };

            // Build relative path from base_path
            let relative_path = file_path
                .strip_prefix(&self.base_path)
                .unwrap_or(&file_path)
                .to_string_lossy()
                .replace('\\', "/");

            entries.push(Content {
                name,
                path: format!("data/{}", relative_path),
                item_type,
                content: None,
                encoding: None,
                html_url: None,
                download_url: None,
                url: format!("file://{}", file_path.display()),
            });
        }

        Ok(entries)
    }

    async fn read_file_content(&self, path: &Path) -> anyhow::Result<Content> {
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let content_str = fs::read_to_string(path).await?;
        
        // Try to parse as JSON to validate
        let _: Value = serde_json::from_str(&content_str)?;

        // Encode as base64 for consistency with GitHub API format
        use base64::{engine::general_purpose, Engine as _};
        let encoded = general_purpose::STANDARD.encode(&content_str);

        let relative_path = path
            .strip_prefix(&self.base_path)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");

        Ok(Content {
            name: file_name,
            path: format!("data/{}", relative_path),
            item_type: ContentType::File,
            content: Some(encoded),
            encoding: Some("base64".to_string()),
            html_url: None,
            download_url: Some(format!("file://{}", path.display())),
            url: format!("file://{}", path.display()),
        })
    }
}

#[async_trait]
impl ContentRepository for LocalFileRepository {
    async fn get_content(&self, _config: &RepoConfig, path: &str) -> anyhow::Result<Content> {
        let file_path = self.resolve_path(path);

        if !file_path.exists() {
            anyhow::bail!("File not found: {}", file_path.display());
        }

        if file_path.is_dir() {
            // For directory, use list_directory instead
            anyhow::bail!("Path is a directory, use list_directory instead");
        }

        self.read_file_content(&file_path).await
    }

    async fn list_directory(
        &self,
        _config: &RepoConfig,
        path: &str,
    ) -> anyhow::Result<Vec<Content>> {
        let dir_path = self.resolve_path(path);

        if !dir_path.exists() {
            anyhow::bail!("Directory not found: {}", dir_path.display());
        }

        if !dir_path.is_dir() {
            anyhow::bail!("Path is not a directory: {}", dir_path.display());
        }

        self.list_directory_internal(&dir_path).await
    }

    async fn get_raw_file(&self, url: &str) -> anyhow::Result<Value> {
        // Extract path from file:// URL
        if url.starts_with("file://") {
            let path_str = url.trim_start_matches("file://");
            let path = Path::new(path_str);
            
            // Security: Validate path is within base_path
            if let (Ok(canonical_path), Ok(base_canonical)) = (
                std::fs::canonicalize(path),
                std::fs::canonicalize(&self.base_path)
            ) {
                if !canonical_path.starts_with(&base_canonical) {
                    anyhow::bail!("Access denied: Path outside base directory");
                }
            } else {
                // If canonicalization fails, check if path contains base_path as prefix
                if !path.starts_with(&self.base_path) {
                    anyhow::bail!("Access denied: Path outside base directory");
                }
            }
            
            let content_str = fs::read_to_string(path).await?;
            let json: Value = serde_json::from_str(&content_str)?;
            Ok(json)
        } else {
            anyhow::bail!("Unsupported URL scheme: {}", url);
        }
    }
}

