use crate::archive::archive_handler::{ArchiveHandler, ExtractionSummary};
use crate::error::{AppError, Result};
use crate::utils::path_security::{
    validate_and_sanitize_archive_path, PathValidationResult, SecurityConfig,
};
use async_trait::async_trait;
use std::path::Path;
use tracing::warn;

use tokio::fs;

use unrar::Archive;

/**
 * RAR文件处理器 (纯Rust/C绑定版本)
 */
pub struct RarHandler;

#[async_trait]
impl ArchiveHandler for RarHandler {
    fn can_handle(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|s| s.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("rar"))
            .unwrap_or(false)
    }

    async fn extract_with_limits(
        &self,
        source: &Path,
        target_dir: &Path,
        max_file_size: u64,
        max_total_size: u64,
        max_file_count: usize,
    ) -> Result<ExtractionSummary> {
        fs::create_dir_all(target_dir).await?;

        let source_path = source.to_path_buf();
        let target_path = target_dir.to_path_buf();

        // unrar crate uses libunrar, which is synchronous
        let summary = tokio::task::spawn_blocking(move || {
            let mut summary = ExtractionSummary::new();
            let mut archive = Archive::new(&source_path)
                .open_for_processing()
                .map_err(|e| {
                    AppError::archive_error(
                        format!("Failed to open RAR: {}", e),
                        Some(source_path.clone()),
                    )
                })?;

            while let Some(header) = archive
                .read_header()
                .map_err(|e| AppError::archive_error(e.to_string(), None))?
            {
                let entry = header.entry();
                let name = entry.filename.to_string_lossy();

                let validation =
                    validate_and_sanitize_archive_path(&name, &SecurityConfig::default());
                let safe_path = match validation {
                    PathValidationResult::Unsafe(_) => {
                        archive = header
                            .skip()
                            .map_err(|e| AppError::archive_error(e.to_string(), None))?;
                        continue;
                    }
                    PathValidationResult::Valid(p) => std::path::PathBuf::from(p),
                    PathValidationResult::RequiresSanitization(_, p) => std::path::PathBuf::from(p),
                };

                let out_path = target_path.join(&safe_path);
                let size = entry.unpacked_size;

                if entry.is_directory() {
                    let _ = std::fs::create_dir_all(&out_path);
                    archive = header
                        .skip()
                        .map_err(|e| AppError::archive_error(e.to_string(), None))?;
                } else {
                    if size > max_file_size
                        || summary.total_size + size > max_total_size
                        || summary.files_extracted + 1 > max_file_count
                    {
                        archive = header
                            .skip()
                            .map_err(|e| AppError::archive_error(e.to_string(), None))?;
                        continue;
                    }

                    if let Some(parent) = out_path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }

                    match header.extract_to(&target_path) {
                        Ok(new_archive) => {
                            archive = new_archive;
                            summary.add_file(safe_path, size);
                        }
                        Err(e) => {
                            warn!("Failed to extract RAR entry {:?}: {}", out_path, e);
                            // unrar crate consumes the header on error, we might need to handle this
                            return Err(AppError::archive_error(e.to_string(), None));
                        }
                    }
                }
            }

            Ok::<ExtractionSummary, AppError>(summary)
        })
        .await
        .map_err(|e| AppError::archive_error(e.to_string(), None))??;

        Ok(summary)
    }

    fn file_extensions(&self) -> Vec<&str> {
        vec!["rar"]
    }
}
