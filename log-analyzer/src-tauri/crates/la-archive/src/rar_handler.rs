#[cfg(feature = "rar-support")]
use crate::archive_handler::{ArchiveHandler, ExtractionSummary};
#[cfg(feature = "rar-support")]
use crate::symlink_guard::{ensure_no_symlink_components, reject_extracted_symlink};
#[cfg(feature = "rar-support")]
use async_trait::async_trait;
#[cfg(feature = "rar-support")]
use la_core::error::{AppError, Result};
#[cfg(feature = "rar-support")]
use la_core::utils::path_security::{
    validate_and_sanitize_archive_path, PathValidationResult, SecurityConfig,
};
#[cfg(feature = "rar-support")]
use std::path::Path;
#[cfg(feature = "rar-support")]
use tracing::warn;

#[cfg(feature = "rar-support")]
use tokio::fs;

#[cfg(feature = "rar-support")]
use unrar::Archive;

#[cfg(feature = "rar-support")]
/**
 * RAR文件处理器 (纯Rust/C绑定版本)
 */
pub struct RarHandler;

#[cfg(feature = "rar-support")]
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
                let (name, safe_path, out_path, size, is_directory) = {
                    let entry = header.entry();
                    let name = entry.filename.to_string_lossy().to_string();

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
                        PathValidationResult::RequiresSanitization(_, p) => {
                            std::path::PathBuf::from(p)
                        }
                    };

                    let out_path = target_path.join(&safe_path);

                    // 防御性边界检查：确保最终路径不逃逸提取目录（防御 ZIP Slip）
                    if !out_path.starts_with(&target_path) {
                        warn!(
                            path = %safe_path.display(),
                            "RAR 条目路径逃逸提取目录，跳过此条目"
                        );
                        archive = header
                            .skip()
                            .map_err(|e| AppError::archive_error(e.to_string(), None))?;
                        continue;
                    }
                    let size = entry.unpacked_size;
                    let is_directory = entry.is_directory();

                    (name, safe_path, out_path, size, is_directory)
                };

                if is_directory {
                    if let Err(error) = ensure_no_symlink_components(&target_path, &out_path) {
                        warn!(
                            path = %out_path.display(),
                            error = %error,
                            "RAR 目录条目目标路径包含符号链接，跳过此条目"
                        );
                        summary.add_error(format!(
                            "Skipped RAR entry {} because extraction path traverses a symbolic link",
                            name
                        ));
                        archive = header
                            .skip()
                            .map_err(|e| AppError::archive_error(e.to_string(), None))?;
                        continue;
                    }
                    if let Err(e) = std::fs::create_dir_all(&out_path) {
                        warn!(path = ?out_path, error = %e, "创建 RAR 目录条目失败，跳过");
                    }
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
                        ensure_no_symlink_components(&target_path, parent)?;
                        std::fs::create_dir_all(parent).map_err(|e| {
                            AppError::archive_error(
                                format!("创建 RAR 条目父目录失败: {e}"),
                                Some(parent.to_path_buf()),
                            )
                        })?;
                    }

                    ensure_no_symlink_components(&target_path, &out_path)?;

                    match header.extract_to(&out_path) {
                        Ok(new_archive) => {
                            archive = new_archive;
                            if let Err(error) = reject_extracted_symlink(&out_path) {
                                warn!(
                                    path = %out_path.display(),
                                    error = %error,
                                    "RAR 条目被提取为符号链接，已删除并跳过"
                                );
                                summary.add_error(format!(
                                    "Removed extracted symbolic link entry {}",
                                    name
                                ));
                                continue;
                            }
                            // TOCTOU defense: re-verify after extraction that the file
                            // did not escape the target directory via symlink race.
                            if let Ok(canonical) = out_path.canonicalize() {
                                if let Ok(canonical_target) = target_path.canonicalize() {
                                    if !canonical.starts_with(&canonical_target) {
                                        warn!(
                                            path = %out_path.display(),
                                            "RAR TOCTOU path traversal detected, removing and skipping"
                                        );
                                        let _ = std::fs::remove_file(&out_path);
                                        summary.add_error(format!(
                                            "Removed RAR entry {} due to path traversal (TOCTOU)",
                                            name
                                        ));
                                        continue;
                                    }
                                }
                            }
                            summary.add_file(safe_path, size);
                        }
                        Err(e) => {
                            let error_message =
                                format!("Failed to extract RAR entry {}: {}", name, e);
                            warn!(
                                path = ?out_path,
                                error = %e,
                                "RAR 条目提取失败，清理残留文件后中止"
                            );
                            summary.add_error(error_message.clone());
                            // 清理可能的残留文件，防止不完整文件留在磁盘
                            if out_path.exists() {
                                let _ = std::fs::remove_file(&out_path);
                            }
                            if summary.files_extracted == 0 {
                                return Err(AppError::archive_error(error_message, None));
                            }
                            return Ok::<ExtractionSummary, AppError>(summary);
                        }
                    }
                }
            }

            Ok::<ExtractionSummary, AppError>(summary)
        })
        .await
        .map_err(|e| {
            if e.is_panic() {
                AppError::Internal(format!("RAR handler panicked: {}", e))
            } else {
                AppError::archive_error(e.to_string(), None)
            }
        })??;

        Ok(summary)
    }

    fn file_extensions(&self) -> Vec<&str> {
        vec!["rar"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    fn find_unrar_fixture(name: &str) -> Option<PathBuf> {
        let mut candidates = Vec::new();

        if let Ok(cargo_home) = std::env::var("CARGO_HOME") {
            candidates.push(PathBuf::from(cargo_home));
        }

        if let Ok(home) = std::env::var("HOME") {
            candidates.push(PathBuf::from(home).join(".cargo"));
        }

        for cargo_home in candidates {
            let registry_src = cargo_home.join("registry").join("src");
            let Ok(entries) = std::fs::read_dir(registry_src) else {
                continue;
            };

            for entry in entries.flatten() {
                let path = entry.path().join("unrar-0.5.8").join("data").join(name);
                if path.exists() {
                    return Some(path);
                }
            }
        }

        None
    }

    #[test]
    fn test_rar_handler_can_handle() {
        let handler = RarHandler;

        // 应该能处理 .rar 文件
        assert!(handler.can_handle(Path::new("test.rar")));
        assert!(handler.can_handle(Path::new("test.RAR")));
        assert!(handler.can_handle(Path::new("/path/to/archive.rar")));

        // 不应该处理其他格式
        assert!(!handler.can_handle(Path::new("test.zip")));
        assert!(!handler.can_handle(Path::new("test.tar")));
        assert!(!handler.can_handle(Path::new("test.gz")));
        assert!(!handler.can_handle(Path::new("test.txt")));
        assert!(!handler.can_handle(Path::new("test")));
    }

    #[test]
    fn test_rar_handler_file_extensions() {
        let handler = RarHandler;
        let extensions = handler.file_extensions();

        assert_eq!(extensions, vec!["rar"]);
    }

    #[tokio::test]
    async fn test_extract_rar_without_support() {
        // 测试在没有 rar-support feature 时的行为
        let temp_dir = tempfile::TempDir::new().expect("创建临时目录失败");
        let rar_file = temp_dir.path().join("test.rar");
        let output_dir = temp_dir.path().join("output");

        // 创建一个虚拟的 RAR 文件（即使内容无效）
        std::fs::write(&rar_file, b"RAR").unwrap();

        let handler = RarHandler;
        let result = handler.extract(&rar_file, &output_dir).await;

        // 在没有 rar-support feature 时应该返回错误
        #[cfg(not(feature = "rar-support"))]
        {
            assert!(result.is_err());
            let err_msg = result.unwrap_err().to_string();
            assert!(
                err_msg.contains("RAR support is not enabled"),
                "错误消息应该提示 RAR 支持未启用: {}",
                err_msg
            );
        }

        // 在有 rar-support feature 时，由于文件内容无效，也会返回错误
        #[cfg(feature = "rar-support")]
        {
            assert!(result.is_err());
        }
    }

    #[tokio::test]
    async fn test_extract_nonexistent_rar() {
        let temp_dir = tempfile::TempDir::new().expect("创建临时目录失败");
        let rar_file = temp_dir.path().join("nonexistent.rar");
        let output_dir = temp_dir.path().join("output");

        let handler = RarHandler;
        let result = handler.extract(&rar_file, &output_dir).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    #[cfg(feature = "rar-support")]
    async fn test_extract_rar_writes_to_the_sanitized_output_path() {
        let Some(rar_file) = find_unrar_fixture("version.rar") else {
            return;
        };

        let temp_dir = tempfile::TempDir::new().expect("创建临时目录失败");
        let output_dir = temp_dir.path().join("output");

        let handler = RarHandler;
        let summary = handler
            .extract(&rar_file, &output_dir)
            .await
            .expect("RAR 提取应成功");

        assert_eq!(summary.files_extracted, 1);
        let extracted_file = output_dir.join(Path::new("VERSION"));
        assert!(
            extracted_file.exists(),
            "RAR 条目应落到计算后的目标文件路径，而不是仅把目录传给 extract_to"
        );
        assert_eq!(
            std::fs::read_to_string(extracted_file).expect("读取已解压文件"),
            "unrar-0.4.0"
        );
    }
}

// 无 RAR 支持时的空实现
#[cfg(not(feature = "rar-support"))]
use crate::archive_handler::{ArchiveHandler, ExtractionSummary};
#[cfg(not(feature = "rar-support"))]
use async_trait::async_trait;
#[cfg(not(feature = "rar-support"))]
use la_core::error::{AppError, Result};
#[cfg(not(feature = "rar-support"))]
use std::path::Path;

#[cfg(not(feature = "rar-support"))]
pub struct RarHandler;

#[cfg(not(feature = "rar-support"))]
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
        _source: &Path,
        _target_dir: &Path,
        _max_file_size: u64,
        _max_total_size: u64,
        _max_file_count: usize,
    ) -> Result<ExtractionSummary> {
        Err(AppError::archive_error(
            "RAR support is not enabled. Enable the 'rar-support' feature to extract RAR files.",
            None,
        ))
    }

    fn file_extensions(&self) -> Vec<&str> {
        vec!["rar"]
    }
}
