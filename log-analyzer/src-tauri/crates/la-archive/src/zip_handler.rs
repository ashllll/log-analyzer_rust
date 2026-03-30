use crate::archive_handler::{ArchiveHandler, ExtractionSummary};
use async_trait::async_trait;
use la_core::error::{AppError, Result};
use la_core::utils::path_security::{
    validate_and_sanitize_archive_path, PathValidationResult, SecurityConfig,
};
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
                // 验证最终路径不逃逸提取目录（防御 ZIP Slip 末级绕过）
                if !out_path.starts_with(&target_path) {
                    warn!(
                        path = %safe_path.display(),
                        "ZIP Slip 尝试被拦截，跳过此条目"
                    );
                    continue;
                }
                // 安全检查：跳过 ZIP 内的符号链接，防止沙箱逃逸
                // ZIP 格式支持符号链接条目，若解压到目标目录会创建指向任意路径的链接
                // 使用 unix_mode() 检查符号链接：S_IFLNK = 0o120000
                let is_symlink = file
                    .unix_mode()
                    .map(|mode| mode & 0o170000 == 0o120000)
                    .unwrap_or(false);
                if is_symlink {
                    warn!(
                        name = %name,
                        "ZIP 条目为符号链接，跳过以防沙箱逃逸"
                    );
                    continue;
                }

                let size = file.size();

                if file.is_dir() {
                    if let Err(e) = std::fs::create_dir_all(&out_path) {
                        warn!(path = ?out_path, error = %e, "创建 ZIP 目录条目失败，跳过");
                    }
                } else {
                    // Check limits before extraction
                    let would_exceed_limits = size > max_file_size
                        || summary.total_size + size > max_total_size
                        || summary.files_extracted + 1 > max_file_count;

                    if would_exceed_limits {
                        // Log skipped file details instead of silently skipping
                        if size > max_file_size {
                            warn!(
                                file = %name,
                                file_size = size,
                                max_allowed = max_file_size,
                                "Skipping file exceeding max_file_size limit"
                            );
                        } else if summary.total_size + size > max_total_size {
                            warn!(
                                file = %name,
                                file_size = size,
                                current_total = summary.total_size,
                                max_total = max_total_size,
                                "Skipping file - would exceed max_total_size limit"
                            );
                        } else {
                            warn!(
                                file = %name,
                                current_count = summary.files_extracted,
                                max_count = max_file_count,
                                "Skipping file - would exceed max_file_count limit"
                            );
                        }
                        continue;
                    }

                    if let Some(parent) = out_path.parent() {
                        if let Err(e) = std::fs::create_dir_all(parent) {
                            warn!(path = ?parent, error = %e, "创建 ZIP 条目父目录失败，跳过此文件");
                            continue;
                        }
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
