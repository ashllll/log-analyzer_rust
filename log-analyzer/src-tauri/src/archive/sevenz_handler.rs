use crate::archive::archive_handler::{ArchiveHandler, ExtractionSummary};
use crate::error::{AppError, Result};
use crate::utils::path_security::{
    validate_and_sanitize_archive_path, PathValidationResult, SecurityConfig,
};
use async_trait::async_trait;
use std::path::Path;
use tracing::warn;

use tokio::fs;

use sevenz_rust::SevenZReader;

/**
 * 7z文件处理器 (稳定版本)
 */
pub struct SevenZHandler;

#[async_trait]
impl ArchiveHandler for SevenZHandler {
    fn can_handle(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|s| s.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("7z"))
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
            let mut summary = ExtractionSummary::new();
            let mut reader = SevenZReader::open(&source_path, sevenz_rust::Password::empty())
                .map_err(|e| {
                    AppError::archive_error(
                        format!("Failed to open 7z: {}", e),
                        Some(source_path.clone()),
                    )
                })?;

            let security_config = SecurityConfig::default();

            reader
                .for_each_entries(|entry, reader| {
                    let name = entry.name();
                    let validation = validate_and_sanitize_archive_path(name, &security_config);

                    let safe_path = match validation {
                        PathValidationResult::Unsafe(_) => return Ok(true),
                        PathValidationResult::Valid(p) => std::path::PathBuf::from(p),
                        PathValidationResult::RequiresSanitization(_, p) => {
                            std::path::PathBuf::from(p)
                        }
                    };

                    let out_path = target_path.join(&safe_path);
                    let size = entry.size();

                    if entry.is_directory() {
                        let _ = std::fs::create_dir_all(&out_path);
                    } else {
                        if size > max_file_size
                            || summary.total_size + size > max_total_size
                            || summary.files_extracted + 1 > max_file_count
                        {
                            return Ok(true);
                        }

                        if let Some(parent) = out_path.parent() {
                            let _ = std::fs::create_dir_all(parent);
                        }

                        match std::fs::File::create(&out_path) {
                            Ok(mut out_file) => {
                                if let Err(e) = std::io::copy(reader, &mut out_file) {
                                    warn!("Failed to extract 7z entry {:?}: {}", out_path, e);
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
                    Ok(true)
                })
                .map_err(|e| AppError::archive_error(e.to_string(), None))?;

            // 显式释放归档读取器句柄
            drop(reader);
            Ok::<ExtractionSummary, AppError>(summary)
        })
        .await
        .map_err(|e| AppError::archive_error(e.to_string(), None))??;

        Ok(summary)
    }

    fn file_extensions(&self) -> Vec<&str> {
        vec!["7z"]
    }
}
