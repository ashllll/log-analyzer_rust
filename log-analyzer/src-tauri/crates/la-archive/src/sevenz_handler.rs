use crate::archive_handler::{ArchiveHandler, ExtractionSummary};
use crate::symlink_guard::ensure_no_symlink_components;
use async_trait::async_trait;
use la_core::error::{AppError, Result};
use la_core::utils::path_security::{
    validate_and_sanitize_archive_path, PathValidationResult, SecurityConfig,
};
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
                    // 验证最终路径不逃逸提取目录（防御 7z Slip 末级绕过）
                    if !out_path.starts_with(&target_path) {
                        warn!(
                            path = %safe_path.display(),
                            "7z Slip 尝试被拦截，跳过此条目"
                        );
                        return Ok(true);
                    }
                    let size = entry.size();

                    if entry.is_directory() {
                        if let Err(error) = ensure_no_symlink_components(&target_path, &out_path) {
                            warn!(
                                path = %out_path.display(),
                                error = %error,
                                "7z 条目目标路径包含符号链接，跳过此条目"
                            );
                            return Ok(true);
                        }
                        let _ = std::fs::create_dir_all(&out_path);
                    } else {
                        if size > max_file_size
                            || summary.total_size + size > max_total_size
                            || summary.files_extracted + 1 > max_file_count
                        {
                            return Ok(true);
                        }

                        if let Some(parent) = out_path.parent() {
                            if let Err(error) = ensure_no_symlink_components(&target_path, parent) {
                                warn!(
                                    path = %parent.display(),
                                    error = %error,
                                    "7z 条目父目录路径包含符号链接，跳过此条目"
                                );
                                return Ok(true);
                            }
                            let _ = std::fs::create_dir_all(parent);
                        }

                        if let Err(error) = ensure_no_symlink_components(&target_path, &out_path) {
                            warn!(
                                path = %out_path.display(),
                                error = %error,
                                "7z 条目目标路径包含符号链接，跳过此条目"
                            );
                            return Ok(true);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sevenz_handler_can_handle() {
        let handler = SevenZHandler;

        // 应该能处理 .7z 文件
        assert!(handler.can_handle(Path::new("test.7z")));
        assert!(handler.can_handle(Path::new("test.7Z")));
        assert!(handler.can_handle(Path::new("/path/to/archive.7z")));

        // 不应该处理其他格式
        assert!(!handler.can_handle(Path::new("test.zip")));
        assert!(!handler.can_handle(Path::new("test.tar")));
        assert!(!handler.can_handle(Path::new("test.rar")));
        assert!(!handler.can_handle(Path::new("test.gz")));
        assert!(!handler.can_handle(Path::new("test.txt")));
        assert!(!handler.can_handle(Path::new("test")));
    }

    #[test]
    fn test_sevenz_handler_file_extensions() {
        let handler = SevenZHandler;
        let extensions = handler.file_extensions();

        assert_eq!(extensions, vec!["7z"]);
    }

    #[tokio::test]
    async fn test_extract_invalid_7z() {
        let temp_dir = tempfile::TempDir::new().expect("创建临时目录失败");
        let sevenz_file = temp_dir.path().join("test.7z");
        let output_dir = temp_dir.path().join("output");

        // 创建一个无效的 7z 文件
        std::fs::write(&sevenz_file, b"This is not a valid 7z file").unwrap();

        let handler = SevenZHandler;
        let result = handler.extract(&sevenz_file, &output_dir).await;

        // 由于文件内容无效，应该返回错误
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_extract_nonexistent_7z() {
        let temp_dir = tempfile::TempDir::new().expect("创建临时目录失败");
        let sevenz_file = temp_dir.path().join("nonexistent.7z");
        let output_dir = temp_dir.path().join("output");

        let handler = SevenZHandler;
        let result = handler.extract(&sevenz_file, &output_dir).await;

        assert!(result.is_err());
    }
}
