/**
 * Path Manager Module
 *
 * Handles long path support, path shortening, and path mapping management.
 * Implements Windows UNC prefix support and content-based hashing for path shortening.
 */
use crate::error::Result;
use crate::services::MetadataDB;
use dashmap::DashMap;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, warn};
use unicode_normalization::UnicodeNormalization;

/// Hash algorithm for path shortening
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashAlgorithm {
    SHA256,
}

/// Path configuration with OS-specific defaults
#[derive(Debug, Clone)]
pub struct PathConfig {
    /// Maximum path length for the operating system
    pub max_path_length: usize,
    /// Threshold (as fraction) to trigger path shortening (0.8 = 80%)
    pub shortening_threshold: f32,
    /// Enable Windows long path support (UNC prefix)
    pub enable_long_paths: bool,
    /// Hash algorithm to use for path shortening
    pub hash_algorithm: HashAlgorithm,
    /// Length of hash to use (in characters)
    pub hash_length: usize,
}

impl Default for PathConfig {
    fn default() -> Self {
        Self::os_default()
    }
}

impl PathConfig {
    /// Create OS-specific default configuration
    pub fn os_default() -> Self {
        #[cfg(target_os = "windows")]
        {
            Self {
                max_path_length: 260,
                shortening_threshold: 0.8,
                enable_long_paths: true,
                hash_algorithm: HashAlgorithm::SHA256,
                hash_length: 16,
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            Self {
                max_path_length: 4096,
                shortening_threshold: 0.8,
                enable_long_paths: false,
                hash_algorithm: HashAlgorithm::SHA256,
                hash_length: 16,
            }
        }
    }

    /// Get the threshold length that triggers path shortening
    pub fn shortening_trigger_length(&self) -> usize {
        (self.max_path_length as f32 * self.shortening_threshold) as usize
    }
}

/// Path Manager for handling long paths and path shortening
pub struct PathManager {
    config: PathConfig,
    metadata_db: Arc<MetadataDB>,
    /// In-memory cache for fast lookups (short_path -> original_path)
    shortening_cache: DashMap<String, String>,
}

impl PathManager {
    /// Create a new PathManager with the given configuration and database
    pub fn new(config: PathConfig, metadata_db: Arc<MetadataDB>) -> Self {
        Self {
            config,
            metadata_db,
            shortening_cache: DashMap::new(),
        }
    }

    /// Get the PathManager configuration
    pub fn config(&self) -> &PathConfig {
        &self.config
    }

    /// Predict the final path length before extraction
    ///
    /// Considers:
    /// - Base path length
    /// - Archive name length
    /// - Internal path length
    /// - Depth (for nested archives, adds separators)
    pub fn predict_path_length(
        &self,
        base_path: &Path,
        archive_name: &str,
        internal_path: &str,
        depth: usize,
    ) -> usize {
        let base_len = base_path.to_string_lossy().len();
        let archive_len = archive_name.len();
        let internal_len = internal_path.len();

        // Account for path separators: base/archive/internal
        // Plus additional separators for depth
        let separator_count = 2 + depth;

        base_len + archive_len + internal_len + separator_count
    }

    /// Resolve extraction path, applying shortening if needed
    ///
    /// Returns the path to use for extraction, which may be shortened if the
    /// original path exceeds the OS limit or shortening threshold.
    pub async fn resolve_extraction_path(
        &self,
        workspace_id: &str,
        full_path: &Path,
    ) -> Result<PathBuf> {
        // Normalize the path to NFC form
        let normalized = self.normalize_path(full_path);
        let path_str = normalized.to_string_lossy();
        let path_len = path_str.len();

        // Check if path needs shortening
        if path_len <= self.config.shortening_trigger_length() {
            // Path is within limits, apply long path prefix if needed
            return Ok(self.apply_long_path_prefix(&normalized));
        }

        // Check cache first
        let cache_key = format!("{}:{}", workspace_id, path_str);
        if let Some(cached) = self.shortening_cache.get(&cache_key) {
            debug!("Using cached shortened path for {}", path_str);
            return Ok(PathBuf::from(cached.value()));
        }

        // Check database
        if let Some(short_path) = self
            .metadata_db
            .get_short_path(workspace_id, &path_str)
            .await
            .map_err(|e| {
                crate::error::AppError::archive_error(
                    format!("Database error: {}", e),
                    Some(full_path.to_path_buf()),
                )
            })?
        {
            debug!("Found existing shortened path in database");
            self.shortening_cache.insert(cache_key, short_path.clone());
            return Ok(PathBuf::from(short_path));
        }

        // Need to create a shortened path
        let short_path = self.create_shortened_path(&normalized).await?;

        // Store in database
        self.metadata_db
            .store_mapping(workspace_id, short_path.to_str().unwrap(), &path_str)
            .await
            .map_err(|e| {
                crate::error::AppError::archive_error(
                    format!("Failed to store path mapping: {}", e),
                    Some(full_path.to_path_buf()),
                )
            })?;

        // Cache it
        self.shortening_cache
            .insert(cache_key, short_path.to_string_lossy().to_string());

        debug!(
            "Created shortened path: {:?} -> {:?}",
            full_path, short_path
        );

        Ok(self.apply_long_path_prefix(&short_path))
    }

    /// Get the original path from a shortened path
    pub async fn resolve_original_path(
        &self,
        workspace_id: &str,
        short_path: &Path,
    ) -> Result<PathBuf> {
        let short_str = short_path.to_string_lossy();

        // Check cache first
        let cache_key = format!("{}:{}", workspace_id, short_str);
        if let Some(cached) = self.shortening_cache.get(&cache_key) {
            return Ok(PathBuf::from(cached.value()));
        }

        // Query database
        let original = self
            .metadata_db
            .get_original_path(workspace_id, &short_str)
            .await
            .map_err(|e| {
                crate::error::AppError::archive_error(
                    format!("Database error: {}", e),
                    Some(short_path.to_path_buf()),
                )
            })?
            .ok_or_else(|| {
                crate::error::AppError::not_found(format!(
                    "No mapping found for shortened path: {}",
                    short_str
                ))
            })?;

        // Cache it
        self.shortening_cache.insert(cache_key, original.clone());

        Ok(PathBuf::from(original))
    }

    /// Apply Windows long path support (UNC prefix)
    ///
    /// On Windows, prepends \\?\ to paths exceeding 260 characters
    /// to enable long path support.
    fn apply_long_path_prefix(&self, path: &Path) -> PathBuf {
        #[cfg(target_os = "windows")]
        {
            if !self.config.enable_long_paths {
                return path.to_path_buf();
            }

            let path_str = path.to_string_lossy();

            // Only apply if path exceeds standard Windows limit
            if path_str.len() <= 260 {
                return path.to_path_buf();
            }

            // Don't apply if already has UNC prefix
            if path_str.starts_with(r"\\?\") {
                return path.to_path_buf();
            }

            // Apply UNC prefix
            let unc_path = if path.is_absolute() {
                format!(r"\\?\{}", dunce::simplified(path).display())
            } else {
                // For relative paths, make absolute first
                match std::env::current_dir() {
                    Ok(current) => {
                        let absolute = current.join(path);
                        format!(r"\\?\{}", dunce::simplified(&absolute).display())
                    }
                    Err(_) => path.to_string_lossy().to_string(),
                }
            };

            PathBuf::from(unc_path)
        }

        #[cfg(not(target_os = "windows"))]
        {
            path.to_path_buf()
        }
    }

    /// Generate content-based hash for a path component
    pub(crate) fn hash_path_component(&self, component: &str) -> String {
        match self.config.hash_algorithm {
            HashAlgorithm::SHA256 => {
                let mut hasher = Sha256::new();
                hasher.update(component.as_bytes());
                let result = hasher.finalize();

                // Convert to hex and truncate to configured length
                let hex = format!("{:x}", result);
                hex.chars().take(self.config.hash_length).collect()
            }
        }
    }

    /// Create a shortened path using hierarchical approach
    ///
    /// Strategy:
    /// 1. Split path into components
    /// 2. Identify longest components
    /// 3. Apply hash-based shortening to longest components
    /// 4. Preserve file extension
    /// 5. Handle collisions with counter suffix
    async fn create_shortened_path(&self, path: &Path) -> Result<PathBuf> {
        let components: Vec<_> = path
            .components()
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .collect();

        if components.is_empty() {
            return Ok(path.to_path_buf());
        }

        // Find the longest component (usually the filename)
        let (longest_idx, longest_component) = components
            .iter()
            .enumerate()
            .max_by_key(|(_, c)| c.len())
            .unwrap();

        // Separate filename and extension
        let (name, ext) = if let Some(pos) = longest_component.rfind('.') {
            let (n, e) = longest_component.split_at(pos);
            (n.to_string(), e.to_string())
        } else {
            (longest_component.clone(), String::new())
        };

        // Hash the name part
        let hashed = self.hash_path_component(&name);
        let shortened_component = format!("{}{}", hashed, ext);

        // Rebuild path with shortened component
        let mut new_components = components.clone();
        new_components[longest_idx] = shortened_component;

        let mut short_path = PathBuf::new();
        for component in new_components {
            short_path.push(component);
        }

        // Handle collisions
        let final_path = self.handle_collision(short_path).await?;

        Ok(final_path)
    }

    /// Handle path collisions by appending counter suffix
    async fn handle_collision(&self, path: PathBuf) -> Result<PathBuf> {
        // Check if path already exists in filesystem
        if !path.exists() {
            return Ok(path);
        }

        // Extract filename and extension
        let file_name = path
            .file_name()
            .ok_or_else(|| crate::error::AppError::validation_error("Invalid path"))?
            .to_string_lossy();

        let (name, ext) = if let Some(pos) = file_name.rfind('.') {
            let (n, e) = file_name.split_at(pos);
            (n.to_string(), e.to_string())
        } else {
            (file_name.to_string(), String::new())
        };

        // Try adding counter suffix
        for counter in 1..=999 {
            let new_name = format!("{}_{:03}{}", name, counter, ext);
            let mut new_path = path.clone();
            new_path.set_file_name(new_name);

            if !new_path.exists() {
                warn!("Path collision detected, using counter: {:?}", new_path);
                return Ok(new_path);
            }
        }

        Err(crate::error::AppError::validation_error(
            "Unable to resolve path collision after 999 attempts",
        ))
    }

    /// Normalize path to NFC form for consistent handling
    fn normalize_path(&self, path: &Path) -> PathBuf {
        let path_str = path.to_string_lossy();
        let normalized: String = path_str.nfc().collect();
        PathBuf::from(normalized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> PathConfig {
        PathConfig {
            max_path_length: 100,
            shortening_threshold: 0.8,
            enable_long_paths: true,
            hash_algorithm: HashAlgorithm::SHA256,
            hash_length: 16,
        }
    }

    async fn create_test_manager() -> PathManager {
        let db = Arc::new(MetadataDB::new(":memory:").await.unwrap());
        PathManager::new(create_test_config(), db)
    }

    #[test]
    fn test_predict_path_length() {
        let config = create_test_config();
        let db = Arc::new(
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(MetadataDB::new(":memory:"))
                .unwrap(),
        );
        let manager = PathManager::new(config, db);

        let base = Path::new("/base/path");
        let archive = "archive.zip";
        let internal = "internal/file.txt";
        let depth = 2;

        let predicted = manager.predict_path_length(base, archive, internal, depth);

        // /base/path + / + archive.zip + / + internal/file.txt + 2 separators for depth
        // 10 + 1 + 11 + 1 + 17 + 2 = 42
        assert_eq!(predicted, 42);
    }

    #[test]
    fn test_hash_path_component() {
        let config = create_test_config();
        let db = Arc::new(
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(MetadataDB::new(":memory:"))
                .unwrap(),
        );
        let manager = PathManager::new(config, db);

        let component = "very_long_component_name_that_needs_hashing";
        let hash1 = manager.hash_path_component(component);
        let hash2 = manager.hash_path_component(component);

        // Hash should be deterministic
        assert_eq!(hash1, hash2);

        // Hash should be 16 characters
        assert_eq!(hash1.len(), 16);
    }

    #[tokio::test]
    async fn test_normalize_path() {
        let manager = create_test_manager().await;

        // Test with Unicode path
        let path = Path::new("café/file.txt");
        let normalized = manager.normalize_path(path);

        // Should normalize to NFC form
        assert!(normalized.to_string_lossy().contains("café"));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_apply_long_path_prefix() {
        let config = create_test_config();
        let db = Arc::new(
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(MetadataDB::new(":memory:"))
                .unwrap(),
        );
        let manager = PathManager::new(config, db);

        // Short path should not get prefix
        let short_path = Path::new("C:\\short\\path.txt");
        let result = manager.apply_long_path_prefix(short_path);
        assert!(!result.to_string_lossy().starts_with(r"\\?\"));

        // Long path should get prefix
        let long_component = "a".repeat(300);
        let long_path = PathBuf::from(format!("C:\\{}", long_component));
        let result = manager.apply_long_path_prefix(&long_path);
        assert!(result.to_string_lossy().starts_with(r"\\?\"));
    }
}
