use crate::archive::archive_handler::{ArchiveHandler, ExtractionSummary};
use crate::error::{AppError, Result};
use crate::utils::path_security::{
    validate_and_sanitize_archive_path, PathValidationResult, SecurityConfig,
};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::warn;

use flate2::read::GzDecoder;
use tar::Archive;

/**
 * TAR文件处理器 (稳定版本)
 */
pub struct TarHandler;

#[async_trait]
impl ArchiveHandler for TarHandler {
    fn can_handle(&self, path: &Path) -> bool {
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            let lower_ext = ext.to_lowercase();
            if lower_ext == "tar" || lower_ext == "tgz" {
                return true;
            }
            if lower_ext == "gz" {
                if let Some(stem) = path.file_stem() {
                    if let Some(stem_str) = stem.to_str() {
                        return stem_str.to_lowercase().ends_with(".tar");
                    }
                }
            }
        }
        false
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

        let summary = tokio::task::spawn_blocking(move || {
            let file = std::fs::File::open(&source_path)?;
            let mut summary = ExtractionSummary::new();
            let security_config = SecurityConfig::default();

            let is_gzipped = source_path
                .extension()
                .and_then(|s| s.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("gz") || ext.eq_ignore_ascii_case("tgz"))
                .unwrap_or(false);

            if is_gzipped {
                let decoder = GzDecoder::new(file);
                let mut archive = Archive::new(decoder);
                Self::extract_sync(
                    &mut archive,
                    &target_path,
                    &mut summary,
                    max_file_size,
                    max_total_size,
                    max_file_count,
                    &security_config,
                )?;
            } else {
                let mut archive = Archive::new(file);
                Self::extract_sync(
                    &mut archive,
                    &target_path,
                    &mut summary,
                    max_file_size,
                    max_total_size,
                    max_file_count,
                    &security_config,
                )?;
            }

            Ok::<ExtractionSummary, AppError>(summary)
        })
        .await
        .map_err(|e| AppError::archive_error(e.to_string(), None))??;

        Ok(summary)
    }

    fn file_extensions(&self) -> Vec<&str> {
        vec!["tar", "tar.gz", "tgz"]
    }
}

impl TarHandler {
    fn extract_sync<R: std::io::Read>(
        archive: &mut Archive<R>,
        target_dir: &Path,
        summary: &mut ExtractionSummary,
        max_file_size: u64,
        max_total_size: u64,
        max_file_count: usize,
        security_config: &SecurityConfig,
    ) -> Result<()> {
        let entries = archive
            .entries()
            .map_err(|e| AppError::archive_error(e.to_string(), None))?;

        for entry_result in entries {
            let mut entry = match entry_result {
                Ok(e) => e,
                Err(e) => {
                    warn!("Failed to read tar entry: {}", e);
                    continue;
                }
            };
            let path = match entry.path() {
                Ok(p) => p.to_path_buf(),
                Err(e) => {
                    warn!("Failed to get entry path: {}", e);
                    continue;
                }
            };
            let path_str = path.to_string_lossy().to_string();

            let validation = validate_and_sanitize_archive_path(&path_str, security_config);
            let safe_path = match validation {
                PathValidationResult::Unsafe(_) => continue,
                PathValidationResult::Valid(p) => PathBuf::from(p),
                PathValidationResult::RequiresSanitization(_, p) => PathBuf::from(p),
            };

            let out_path = target_dir.join(&safe_path);
            let size = entry.header().size().unwrap_or(0);

            if entry.header().entry_type().is_file() {
                // Check limits before extraction
                let would_exceed_limits = size > max_file_size
                    || summary.total_size + size > max_total_size
                    || summary.files_extracted + 1 > max_file_count;

                if would_exceed_limits {
                    // Log skipped file details instead of silently skipping
                    let path_str = safe_path.to_string_lossy();
                    if size > max_file_size {
                        warn!(
                            file = %path_str,
                            file_size = size,
                            max_allowed = max_file_size,
                            "Skipping file exceeding max_file_size limit"
                        );
                    } else if summary.total_size + size > max_total_size {
                        warn!(
                            file = %path_str,
                            file_size = size,
                            current_total = summary.total_size,
                            max_total = max_total_size,
                            "Skipping file - would exceed max_total_size limit"
                        );
                    } else {
                        warn!(
                            file = %path_str,
                            current_count = summary.files_extracted,
                            max_count = max_file_count,
                            "Skipping file - would exceed max_file_count limit"
                        );
                    }
                    continue;
                }

                if let Some(parent) = out_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }

                if let Err(e) = entry.unpack(&out_path) {
                    warn!("Failed to unpack entry {:?}: {}", out_path, e);
                } else {
                    summary.add_file(safe_path, size);
                }
            } else if entry.header().entry_type().is_dir() {
                let _ = std::fs::create_dir_all(&out_path);
            }
        }
        Ok(())
    }
}
