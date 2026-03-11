use crate::archive::archive_handler::{ArchiveEntry, ArchiveHandler, ExtractionSummary};
use crate::archive::archive_handler_base::{ArchiveHandlerBase, ExtractionContext};
use crate::archive::extraction_error::{ExtractionError, ExtractionResult};
use crate::error::{AppError, Result};
use async_trait::async_trait;
use std::path::Path;
use tracing::{debug, trace, warn};

use tokio::fs;

use sevenz_rust::SevenZReader;

/**
 * 7z文件处理器 (重构版本 - 使用 ArchiveHandlerBase)
 */
pub struct SevenZHandler {}

#[async_trait]
impl ArchiveHandlerBase for SevenZHandler {
    fn handler_name(&self) -> &'static str {
        "SevenZHandler"
    }

    fn supported_formats(&self) -> &[&'static str] {
        &["7z"]
    }

    async fn extract_with_context(
        &self,
        source: &Path,
        target_dir: &Path,
        context: &mut ExtractionContext,
    ) -> ExtractionResult<ExtractionSummary> {
        debug!("开始提取 7Z 文件: {:?}", source);

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

        let summary = tokio::task::spawn_blocking(move || {
            let mut summary = ExtractionSummary::new();
            let mut reader = SevenZReader::open(&source_path, sevenz_rust::Password::empty())
                .map_err(|e| ExtractionError::ArchiveCorrupted {
                    path: source_path.clone(),
                    reason: format!("Failed to open 7z: {}", e),
                })?;

            use crate::utils::path_security::{
                validate_and_sanitize_archive_path, PathValidationResult, SecurityConfig,
            };
            let security_config = SecurityConfig::default();

            reader
                .for_each_entries(|entry, file_reader| {
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
                            return Ok(true);
                        }

                        if let Some(parent) = out_path.parent() {
                            let _ = std::fs::create_dir_all(parent);
                        }

                        match std::fs::File::create(&out_path) {
                            Ok(mut out_file) => {
                                if let Err(e) = std::io::copy(file_reader, &mut out_file) {
                                    warn!("Failed to extract 7z entry {:?}: {}", out_path, e);
                                    summary.add_error(format!(
                                        "Extraction failed for {}: {}",
                                        name, e
                                    ));
                                } else {
                                    summary.add_file(safe_path, size);
                                    trace!("已提取 7Z 文件: {:?}, 大小: {}", out_path, size);
                                }
                                // 显式释放文件句柄
                                drop(out_file);
                            }
                            Err(e) => {
                                warn!("Failed to create file {:?}: {}", out_path, e);
                                summary
                                    .add_error(format!("File creation failed for {}: {}", name, e));
                            }
                        }
                    }
                    Ok(true)
                })
                .map_err(|e| ExtractionError::IoError {
                    operation: "提取7Z条目".to_string(),
                    reason: e.to_string(),
                })?;

            // 显式释放归档读取器句柄
            drop(reader);
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
impl ArchiveHandler for SevenZHandler {
    fn can_handle(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|s| s.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("7z"))
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
        vec!["7z"]
    }

    async fn list_contents(&self, path: &Path) -> Result<Vec<ArchiveEntry>> {
        let path_owned = path.to_path_buf();
        let entries = tokio::task::spawn_blocking(move || {
            let mut reader = SevenZReader::open(&path_owned, sevenz_rust::Password::empty())
                .map_err(|e| AppError::archive_error(e.to_string(), None))?;

            let mut entries = Vec::new();

            // Use for_each_entries to iterate through all entries
            reader
                .for_each_entries(|entry, _reader| {
                    let name = entry.name().to_string();
                    let path_buf = Path::new(&name);
                    let name_only = path_buf
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| name.clone());

                    entries.push(ArchiveEntry {
                        name: name_only,
                        path: name,
                        is_dir: entry.is_directory(),
                        size: entry.size(),
                        compressed_size: entry.size(), // 7z 不直接提供压缩大小，使用解压后大小
                    });
                    Ok(true)
                })
                .map_err(|e| AppError::archive_error(e.to_string(), None))?;

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

            // 首先遍历一次找到目标文件的元数据
            let mut target_size: Option<u64> = None;
            let mut target_name: Option<String> = None;

            {
                let mut reader = SevenZReader::open(&path_owned, sevenz_rust::Password::empty())
                    .map_err(|e| AppError::archive_error(e.to_string(), None))?;

                reader
                    .for_each_entries(|entry, _reader| {
                        let name = entry.name().to_string();
                        if name == file_name_owned
                            || Path::new(&name)
                                .file_name()
                                .map(|s| s.to_string_lossy().to_string())
                                == Some(file_name_owned.clone())
                        {
                            target_size = Some(entry.size());
                            target_name = Some(name);
                        }
                        Ok(true)
                    })
                    .map_err(|e| AppError::archive_error(e.to_string(), None))?;
            }

            let Some(size) = target_size else {
                return Err(AppError::archive_error(
                    format!("File not found in archive: {}", file_name_owned),
                    None,
                ));
            };

            let target_name = target_name.ok_or_else(|| {
                AppError::archive_error(
                    format!("Failed to find file '{}' in archive", file_name_owned),
                    None,
                )
            })?;

            // 如果是目录，返回错误
            if size == 0 && target_name.ends_with('/') {
                return Err(AppError::archive_error(
                    "Cannot read directory".to_string(),
                    None,
                ));
            }

            // 创建临时文件
            let temp_dir = std::env::temp_dir();
            let temp_path = temp_dir.join(format!("7z_extract_{}", std::process::id()));

            // 提取单个文件
            {
                let mut reader = SevenZReader::open(&path_owned, sevenz_rust::Password::empty())
                    .map_err(|e| AppError::archive_error(e.to_string(), None))?;

                reader
                    .for_each_entries(|entry, file_reader| {
                        if entry.name() == target_name {
                            if let Some(parent) = temp_path.parent() {
                                let _ = std::fs::create_dir_all(parent);
                            }
                            if let Ok(mut out_file) = std::fs::File::create(&temp_path) {
                                let _ = std::io::copy(file_reader, &mut out_file);
                            }
                        }
                        Ok(true)
                    })
                    .map_err(|e| AppError::archive_error(e.to_string(), None))?;
            }

            // 读取提取的文件
            if temp_path.exists() {
                let content = if size > MAX_SIZE {
                    // 大文件截断读取
                    let bytes = std::fs::read(&temp_path)?;
                    let truncated =
                        String::from_utf8_lossy(&bytes[..MAX_SIZE as usize]).to_string();
                    format!(
                        "{}\n\n[文件过大，已截断显示. 完整大小: {} bytes]",
                        truncated, size
                    )
                } else {
                    std::fs::read_to_string(&temp_path)?
                };

                // 清理临时文件
                let _ = std::fs::remove_file(&temp_path);

                return Ok(content);
            }

            // 清理临时文件
            let _ = std::fs::remove_file(&temp_path);

            Err(AppError::archive_error(
                format!("File not found after extraction: {}", file_name_owned),
                None,
            ))
        })
        .await
        .map_err(|e| AppError::archive_error(e.to_string(), None))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use tempfile::TempDir;

    #[test]
    fn test_sevenz_handler_base_impl() {
        let handler = SevenZHandler {};
        assert_eq!(handler.handler_name(), "SevenZHandler");
        assert_eq!(handler.supported_formats(), &["7z"]);
    }

    #[test]
    fn test_sevenz_handler_can_handle() {
        let handler = SevenZHandler {};
        assert!(handler.can_handle(Path::new("test.7z")));
        assert!(handler.can_handle(Path::new("test.7Z")));
        assert!(!handler.can_handle(Path::new("test.zip")));
        assert!(!handler.can_handle(Path::new("test.txt")));
    }

    #[test]
    fn test_sevenz_handler_file_extensions() {
        let handler = SevenZHandler {};
        let extensions = handler.file_extensions();
        assert_eq!(extensions, vec!["7z"]);
    }
}
