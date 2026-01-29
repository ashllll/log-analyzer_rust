use crate::archive::archive_handler::{ArchiveHandler, ExtractionSummary};
use crate::error::{AppError, Result};
use crate::utils::path_security::{
    validate_and_sanitize_archive_path, PathValidationResult, SecurityConfig,
};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::warn;

use zip::ZipArchive;

/**
 * ZIP文件处理器 (稳定版本)
 */
pub struct ZipHandler;

#[async_trait]
impl ArchiveHandler for ZipHandler {
    fn can_handle(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|s| s.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("zip"))
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

        let summary = tokio::task::spawn_blocking(move || {
            let file = std::fs::File::open(&source_path)?;
            let mut archive =
                ZipArchive::new(file).map_err(|e| AppError::archive_error(e.to_string(), None))?;
            let mut summary = ExtractionSummary::new();
            let security_config = SecurityConfig::default();

            for i in 0..archive.len() {
                let mut file = match archive.by_index(i) {
                    Ok(f) => f,
                    Err(e) => {
                        warn!("Failed to read zip entry {}: {}", i, e);
                        continue;
                    }
                };
                let name = file.name().to_string();

                let validation = validate_and_sanitize_archive_path(&name, &security_config);
                let safe_path = match validation {
                    PathValidationResult::Unsafe(_) => continue,
                    PathValidationResult::Valid(p) => PathBuf::from(p),
                    PathValidationResult::RequiresSanitization(_, p) => PathBuf::from(p),
                };

                let out_path = target_path.join(&safe_path);
                let size = file.size();

                if file.is_dir() {
                    let _ = std::fs::create_dir_all(&out_path);
                } else {
                    if size > max_file_size
                        || summary.total_size + size > max_total_size
                        || summary.files_extracted + 1 > max_file_count
                    {
                        continue;
                    }

                    if let Some(parent) = out_path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }

                    match std::fs::File::create(&out_path) {
                        Ok(mut out_file) => {
                            if let Err(e) = std::io::copy(&mut file, &mut out_file) {
                                warn!("Failed to extract file {:?}: {}", out_path, e);
                            } else {
                                summary.add_file(safe_path, size);
                            }
                            // 显式释放文件句柄
                            drop(out_file);
                        }
                        Err(e) => {
                            warn!("Failed to create file {:?}: {}", out_path, e);
                        }
                    }
                }
            }
            // 显式释放归档文件句柄
            drop(archive);
            Ok::<ExtractionSummary, AppError>(summary)
        })
        .await
        .map_err(|e| AppError::archive_error(e.to_string(), None))??;

        Ok(summary)
    }

    fn file_extensions(&self) -> Vec<&str> {
        vec!["zip"]
    }
}
