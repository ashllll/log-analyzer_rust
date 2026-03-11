use crate::archive::archive_handler::{ArchiveEntry, ArchiveHandler, ExtractionSummary};
use crate::archive::archive_handler_base::{ArchiveHandlerBase, ExtractionContext};
use crate::archive::extraction_error::{ExtractionError, ExtractionResult};
use crate::error::{AppError, Result};
use async_trait::async_trait;
use std::path::Path;
use tracing::{debug, trace, warn};

use tokio::fs;

use unrar::Archive;

/**
 * RAR文件处理器 (重构版本 - 使用 ArchiveHandlerBase)
 */
pub struct RarHandler {}

#[async_trait]
impl ArchiveHandlerBase for RarHandler {
    fn handler_name(&self) -> &'static str {
        "RarHandler"
    }

    fn supported_formats(&self) -> &[&'static str] {
        &["rar"]
    }

    async fn extract_with_context(
        &self,
        source: &Path,
        target_dir: &Path,
        context: &mut ExtractionContext,
    ) -> ExtractionResult<ExtractionSummary> {
        debug!("开始提取 RAR 文件: {:?}", source);

        // 创建目标目录
        fs::create_dir_all(target_dir).await.map_err(|e| {
            ExtractionError::DirectoryCreationFailed {
                path: target_dir.to_path_buf(),
                reason: e.to_string(),
            }
        })?;

        let source_path = source.to_path_buf();
        let target_path = target_dir.to_path_buf();

        // 提取限制值到局部变量，以便在 spawn_blocking 中使用
        let max_file_size = context.config.limits.max_file_size;
        let max_total_size = context.config.limits.max_total_size;
        let max_file_count = context.config.limits.max_file_count;

        // unrar crate uses libunrar, which is synchronous
        let summary = tokio::task::spawn_blocking(move || {
            let mut summary = ExtractionSummary::new();

            Self::extract_sync(
                &source_path,
                &target_path,
                &mut summary,
                max_file_size,
                max_total_size,
                max_file_count,
            )?;

            Ok::<ExtractionSummary, ExtractionError>(summary)
        })
        .await
        .map_err(|e| ExtractionError::IoError {
            operation: "spawn_blocking".to_string(),
            reason: format!("Task join error: {}", e),
        })?;

        // 更新上下文统计信息
        if let Ok(ref summary) = summary {
            for file_path in &summary.extracted_files {
                let full_path = target_dir.join(file_path);
                if let Ok(metadata) = std::fs::metadata(&full_path) {
                    context.record_extraction(&full_path, metadata.len());
                }
            }
        }

        summary
    }
}

#[async_trait]
impl ArchiveHandler for RarHandler {
    fn can_handle(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|s| s.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("rar"))
            .unwrap_or(false)
    }

    #[allow(deprecated)]
    async fn extract_with_limits(
        &self,
        source: &Path,
        target_dir: &Path,
        max_file_size: u64,
        max_total_size: u64,
        max_file_count: usize,
    ) -> Result<ExtractionSummary> {
        self.extract_with_limits_default(source, target_dir, max_file_size, max_total_size, max_file_count)
            .await
    }

    fn file_extensions(&self) -> Vec<&str> {
        vec!["rar"]
    }

    async fn list_contents(&self, path: &Path) -> Result<Vec<ArchiveEntry>> {
        let path_owned = path.to_path_buf();
        let entries = tokio::task::spawn_blocking(move || {
            let mut archive = Archive::new(&path_owned)
                .open_for_listing()
                .map_err(|e| AppError::archive_error(e.to_string(), None))?;

            let mut entries = Vec::new();

            while let Some(header) = archive
                .read_header()
                .map_err(|e| AppError::archive_error(e.to_string(), None))?
            {
                let entry = header.entry();
                let name = entry.filename.to_string_lossy().to_string();

                let path_buf = Path::new(&name);
                let name_only = path_buf
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| name.clone());

                entries.push(ArchiveEntry {
                    name: name_only,
                    path: name,
                    is_dir: entry.is_directory(),
                    size: entry.unpacked_size,
                    compressed_size: entry.unpacked_size, // RAR 不直接提供压缩大小
                });

                archive = header
                    .skip()
                    .map_err(|e| AppError::archive_error(e.to_string(), None))?;
            }

            Ok::<Vec<ArchiveEntry>, AppError>(entries)
        })
        .await
        .map_err(|e| AppError::archive_error(e.to_string(), None))??;

        Ok(entries)
    }

    async fn read_file(&self, path: &Path, file_name: &str) -> Result<String> {
        let path_owned = path.to_path_buf();
        let file_name_owned = file_name.to_string();

        tokio::task::spawn_blocking(move || {
            // 大小限制：10MB
            const MAX_SIZE: u64 = 10 * 1024 * 1024;

            let mut archive = Archive::new(&path_owned)
                .open_for_processing()
                .map_err(|e| AppError::archive_error(e.to_string(), None))?;

            while let Some(header) = archive
                .read_header()
                .map_err(|e| AppError::archive_error(e.to_string(), None))?
            {
                let entry = header.entry();
                let name = entry.filename.to_string_lossy().to_string();

                // 匹配文件名
                if name == file_name_owned
                    || Path::new(&name)
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        == Some(file_name_owned.clone())
                {
                    if entry.is_directory() {
                        return Err(AppError::archive_error(
                            "Cannot read directory".to_string(),
                            None,
                        ));
                    }

                    let size = entry.unpacked_size;

                    // 创建临时目录
                    let temp_dir = std::env::temp_dir();
                    let temp_path = temp_dir.join(format!("rar_extract_{}", std::process::id()));

                    // 提取到临时目录
                    let _archive = header
                        .extract_to(&temp_path)
                        .map_err(|e| AppError::archive_error(e.to_string(), None))?;

                    // 读取提取的文件
                    let extract_path = temp_path.join(&name);
                    if extract_path.exists() {
                        let content = if size > MAX_SIZE {
                            // 大文件截断读取
                            let bytes = std::fs::read(&extract_path)?;
                            let truncated =
                                String::from_utf8_lossy(&bytes[..MAX_SIZE as usize]).to_string();
                            format!(
                                "{}\n\n[文件过大，已截断显示. 完整大小: {} bytes]",
                                truncated, size
                            )
                        } else {
                            std::fs::read_to_string(&extract_path)?
                        };

                        // 清理临时文件
                        let _ = std::fs::remove_dir_all(&temp_path);

                        return Ok(content);
                    }

                    // 清理临时文件
                    let _ = std::fs::remove_dir_all(&temp_path);

                    return Err(AppError::archive_error(
                        format!("File not found after extraction: {}", file_name_owned),
                        None,
                    ));
                }

                archive = header
                    .skip()
                    .map_err(|e| AppError::archive_error(e.to_string(), None))?;
            }

            Err(AppError::archive_error(
                format!("File not found in archive: {}", file_name_owned),
                None,
            ))
        })
        .await
        .map_err(|e| AppError::archive_error(e.to_string(), None))?
    }
}

impl RarHandler {
    fn extract_sync(
        source_path: &Path,
        target_path: &Path,
        summary: &mut ExtractionSummary,
        max_file_size: u64,
        max_total_size: u64,
        max_file_count: usize,
    ) -> ExtractionResult<()> {
        use crate::utils::path_security::{
            validate_and_sanitize_archive_path, PathValidationResult, SecurityConfig,
        };

        let mut archive = Archive::new(&source_path)
            .open_for_processing()
            .map_err(|e| ExtractionError::ArchiveCorrupted {
                path: source_path.to_path_buf(),
                reason: format!("Failed to open RAR: {}", e),
            })?;

        let security_config = SecurityConfig::default();

        // 使用 loop 避免借用冲突
        loop {
            let header = match archive.read_header() {
                Ok(Some(h)) => h,
                Ok(None) => break,
                Err(e) => {
                    return Err(ExtractionError::IoError {
                        operation: "读取RAR头".to_string(),
                        reason: e.to_string(),
                    });
                }
            };

            // 在一个单独的作用域中获取 entry 的信息
            let (name, is_dir, size, should_skip, safe_path) = {
                let entry = header.entry();
                let name = entry.filename.to_string_lossy().to_string();
                let is_dir = entry.is_directory();
                let size = entry.unpacked_size;

                let validation = validate_and_sanitize_archive_path(&name, &security_config);
                let (should_skip, safe_path) = match validation {
                    PathValidationResult::Unsafe(_) => (true, std::path::PathBuf::new()),
                    PathValidationResult::Valid(p) => (false, std::path::PathBuf::from(p)),
                    PathValidationResult::RequiresSanitization(_, p) => {
                        (false, std::path::PathBuf::from(p))
                    }
                };

                (name, is_dir, size, should_skip, safe_path)
            };

            if should_skip {
                archive = header.skip().map_err(|e| ExtractionError::IoError {
                    operation: "跳过RAR条目".to_string(),
                    reason: e.to_string(),
                })?;
                continue;
            }

            let out_path = target_path.join(&safe_path);

            if is_dir {
                let _ = std::fs::create_dir_all(&out_path);
                archive = header.skip().map_err(|e| ExtractionError::IoError {
                    operation: "跳过RAR目录条目".to_string(),
                    reason: e.to_string(),
                })?;
            } else {
                // 限制检查
                let would_exceed_limits = size > max_file_size
                    || summary.total_size + size > max_total_size
                    || summary.files_extracted + 1 > max_file_count;

                if would_exceed_limits {
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
                    summary.add_error(format!("File skipped (limits exceeded): {}", name));
                    archive = header.skip().map_err(|e| ExtractionError::IoError {
                        operation: "跳过超出限制的RAR条目".to_string(),
                        reason: e.to_string(),
                    })?;
                    continue;
                }

                if let Some(parent) = out_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }

                match header.extract_to(target_path) {
                    Ok(new_archive) => {
                        archive = new_archive;
                        summary.add_file(safe_path, size);
                        trace!("已提取 RAR 文件: {:?}, 大小: {}", out_path, size);
                    }
                    Err(e) => {
                        warn!("Failed to extract RAR entry {:?}: {}", out_path, e);
                        summary.add_error(format!("Extraction failed for {}: {}", name, e));
                        return Err(ExtractionError::IoError {
                            operation: "提取RAR条目".to_string(),
                            reason: e.to_string(),
                        });
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rar_handler_base_impl() {
        let handler = RarHandler {};
        assert_eq!(handler.handler_name(), "RarHandler");
        assert_eq!(handler.supported_formats(), &["rar"]);
    }

    #[test]
    fn test_rar_handler_can_handle() {
        let handler = RarHandler {};
        assert!(handler.can_handle(Path::new("test.rar")));
        assert!(handler.can_handle(Path::new("test.RAR")));
        assert!(!handler.can_handle(Path::new("test.zip")));
        assert!(!handler.can_handle(Path::new("test.txt")));
    }

    #[test]
    fn test_rar_handler_file_extensions() {
        let handler = RarHandler {};
        let extensions = handler.file_extensions();
        assert_eq!(extensions, vec!["rar"]);
    }
}
