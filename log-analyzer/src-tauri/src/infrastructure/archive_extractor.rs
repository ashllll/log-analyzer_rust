//! ArchiveExtractor adapter — wraps `la_archive::ArchiveManager`.
//!
//! Delegates extraction, format detection, and validation to the
//! existing archive processing infrastructure.

use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;

use la_archive::ArchiveManager;
use la_core::domain::{ArchiveEntry, ArchiveExtractor, ExtractionPolicy, ExtractionSummary};
use la_core::error::{AppError, Result};
use la_core::models::config::ArchiveConfig;
use la_core::models::HandlersConfig;

/// Adapter that wraps `ArchiveManager` behind the `ArchiveExtractor` trait.
pub struct ArchiveManagerAdapter {
    manager: Arc<ArchiveManager>,
}

impl ArchiveManagerAdapter {
    /// Create a new adapter, wrapping a pre-built `ArchiveManager`.
    pub fn new(manager: Arc<ArchiveManager>) -> Self {
        Self { manager }
    }

    /// Create an adapter with handler toggles from extraction policy.
    ///
    /// Only handlers enabled in `handlers_cfg` are registered in the
    /// underlying `ArchiveManager`. This allows runtime control over
    /// which archive formats are supported (ZIP/TAR/GZ/7Z/RAR).
    pub fn with_handlers_config(config: ArchiveConfig, handlers_cfg: &HandlersConfig) -> Self {
        let manager = Arc::new(ArchiveManager::with_handlers_config(config, handlers_cfg));
        Self { manager }
    }
}

#[async_trait]
impl ArchiveExtractor for ArchiveManagerAdapter {
    async fn extract(&self, source: &Path, target_dir: &Path) -> Result<ExtractionSummary> {
        let result = self.manager.extract_archive(source, target_dir).await?;

        Ok(ExtractionSummary {
            files_extracted: result.files_extracted,
            total_bytes: result.total_size,
            max_depth_reached: 0, // ArchiveManager doesn't track depth
        })
    }

    fn list_contents(&self, source: &Path) -> Result<Vec<ArchiveEntry>> {
        // ArchiveManager doesn't expose a list-contents API directly.
        // Fall back to extracting to a temporary directory and listing results.
        let temp_dir = tempfile::tempdir().map_err(|e| {
            AppError::io_error(format!("Failed to create temp dir for listing: {e}"), None)
        })?;

        let handle = tokio::runtime::Handle::current();
        let result = handle.block_on(self.manager.extract_archive(source, temp_dir.path()))?;

        let entries: Vec<ArchiveEntry> = result
            .extracted_files
            .iter()
            .map(|p| {
                let size = std::fs::metadata(p).map(|m| m.len()).unwrap_or(0);
                ArchiveEntry {
                    path: p
                        .strip_prefix(temp_dir.path())
                        .unwrap_or(p)
                        .to_string_lossy()
                        .to_string(),
                    size_bytes: size,
                }
            })
            .collect();

        Ok(entries)
    }

    fn supported_formats(&self) -> Vec<String> {
        self.manager.supported_extensions()
    }

    fn validate(&self, path: &Path, policy: &ExtractionPolicy) -> Result<()> {
        // Validate path
        if !path.exists() {
            return Err(AppError::validation_error(format!(
                "Archive path does not exist: {}",
                path.display()
            )));
        }
        if !path.is_file() {
            return Err(AppError::validation_error(format!(
                "Archive path is not a file: {}",
                path.display()
            )));
        }

        // Validate format is supported by checking file extension
        let supported = self.supported_formats();
        let is_supported = path
            .extension()
            .map(|ext| {
                let ext_str = ext.to_string_lossy().to_lowercase();
                supported
                    .iter()
                    .any(|s| s == &ext_str || path.to_string_lossy().to_lowercase().ends_with(s))
            })
            .unwrap_or(false);

        if !is_supported {
            let ext = path
                .extension()
                .map(|e| e.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".to_string());
            return Err(AppError::validation_error(format!(
                "Unsupported archive format: .{ext}. Supported: {supported:?}"
            )));
        }

        // Validate policy constraints
        if policy.max_depth < 1 || policy.max_depth > 20 {
            return Err(AppError::validation_error(format!(
                "max_depth must be between 1 and 20, got {}",
                policy.max_depth
            )));
        }
        if policy.max_file_size == 0 {
            return Err(AppError::validation_error("max_file_size must be positive"));
        }
        if policy.max_total_size == 0 {
            return Err(AppError::validation_error(
                "max_total_size must be positive",
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_nonexistent_path() {
        let manager = Arc::new(ArchiveManager::new());
        let adapter = ArchiveManagerAdapter::new(manager);

        let path = Path::new("/nonexistent/archive.zip");
        let policy = ExtractionPolicy {
            max_depth: 10,
            max_file_size: 100 * 1024 * 1024,
            max_total_size: 500 * 1024 * 1024,
        };

        let result = adapter.validate(path, &policy);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));
    }

    #[test]
    fn test_validate_unsupported_format() {
        let manager = Arc::new(ArchiveManager::new());
        let adapter = ArchiveManagerAdapter::new(manager);

        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("test.xyz");
        std::fs::write(&path, b"fake").unwrap();

        let policy = ExtractionPolicy {
            max_depth: 10,
            max_file_size: 100 * 1024 * 1024,
            max_total_size: 500 * 1024 * 1024,
        };

        let result = adapter.validate(&path, &policy);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unsupported archive format"));
    }

    #[test]
    fn test_validate_invalid_policy_max_depth() {
        let manager = Arc::new(ArchiveManager::new());
        let adapter = ArchiveManagerAdapter::new(manager);

        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("test.zip");
        std::fs::write(&path, b"fake").unwrap();

        let policy = ExtractionPolicy {
            max_depth: 0, // invalid
            max_file_size: 100 * 1024 * 1024,
            max_total_size: 500 * 1024 * 1024,
        };

        let result = adapter.validate(&path, &policy);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("max_depth must be between 1 and 20"));
    }

    #[test]
    fn test_validate_invalid_policy_file_size() {
        let manager = Arc::new(ArchiveManager::new());
        let adapter = ArchiveManagerAdapter::new(manager);

        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("test.zip");
        std::fs::write(&path, b"fake").unwrap();

        let policy = ExtractionPolicy {
            max_depth: 10,
            max_file_size: 0, // invalid
            max_total_size: 500 * 1024 * 1024,
        };

        let result = adapter.validate(&path, &policy);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("max_file_size must be positive"));
    }

    #[test]
    fn test_supported_formats() {
        let manager = Arc::new(ArchiveManager::new());
        let adapter = ArchiveManagerAdapter::new(manager);

        let formats = adapter.supported_formats();
        assert!(formats.contains(&"zip".to_string()));
        assert!(formats.contains(&"tar".to_string()));
        assert!(formats.contains(&"gz".to_string()));
    }
}
