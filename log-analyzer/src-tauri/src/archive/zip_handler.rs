use crate::archive::archive_handler::{ArchiveEntry, ArchiveHandler, ExtractionSummary};
use crate::archive::archive_handler_base::{ArchiveHandlerBase, ExtractionContext};
use crate::archive::extraction_config::SecurityConfig;
use crate::archive::extraction_error::{ExtractionError, ExtractionResult};
use crate::error::{AppError, Result};
use async_trait::async_trait;
use std::io::Read;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, trace, warn};

use zip::ZipArchive;

/**
 * ZIP文件处理器 (重构版本 - 使用 ArchiveHandlerBase)
 */
pub struct ZipHandler {}

impl ZipHandler {
    /// 静态路径验证方法（用于 spawn_blocking 内部）
    ///
    /// 检查路径是否包含：
    /// - 绝对路径（根据配置）
    /// - 父目录遍历 (..)（根据配置）
    /// - 黑名单路径
    fn validate_path_static(path: &Path, config: &SecurityConfig) -> ExtractionResult<()> {
        let path_str = path.to_string_lossy();

        // 检查绝对路径
        if !config.allow_absolute_paths && path.is_absolute() {
            return Err(ExtractionError::AbsolutePathNotAllowed {
                path: path_str.to_string(),
            });
        }

        // 检查父目录遍历
        if !config.allow_parent_traversal {
            let components: Vec<_> = path.components().collect();
            let mut depth = 0i32;
            for component in &components {
                match component {
                    std::path::Component::ParentDir => {
                        depth -= 1;
                        if depth < 0 {
                            return Err(ExtractionError::ParentTraversalNotAllowed {
                                path: path_str.to_string(),
                            });
                        }
                    }
                    std::path::Component::Normal(_) => depth += 1,
                    _ => {}
                }
            }
        }

        // 检查黑名单
        for blacklisted in &config.path_blacklist {
            if path_str.contains(blacklisted) {
                return Err(ExtractionError::PathBlacklisted {
                    path: path_str.to_string(),
                });
            }
        }

        Ok(())
    }
}

#[async_trait]
impl ArchiveHandlerBase for ZipHandler {
    fn handler_name(&self) -> &'static str {
        "ZipHandler"
    }

    fn supported_formats(&self) -> &[&'static str] {
        &["zip"]
    }

    async fn extract_with_context(
        &self,
        source: &Path,
        target_dir: &Path,
        context: &mut ExtractionContext,
    ) -> ExtractionResult<ExtractionSummary> {
        debug!("开始提取 ZIP 文件: {:?}", source);

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
        let security_config = context.config.security.clone();

        // 使用 spawn_blocking 进行同步 ZIP 操作
        let summary = tokio::task::spawn_blocking(move || {
            let file = std::fs::File::open(&source_path).map_err(|e| ExtractionError::IoError {
                operation: "打开ZIP文件".to_string(),
                reason: e.to_string(),
            })?;

            let mut archive =
                ZipArchive::new(file).map_err(|e| ExtractionError::ArchiveCorrupted {
                    path: source_path.clone(),
                    reason: e.to_string(),
                })?;

            let mut summary = ExtractionSummary::new();

            for i in 0..archive.len() {
                let mut file = match archive.by_index(i) {
                    Ok(f) => f,
                    Err(e) => {
                        warn!("Failed to read zip entry {}: {}", i, e);
                        summary.add_error(format!("Entry {}: {}", i, e));
                        continue;
                    }
                };

                let name = file.name().to_string();
                let size = file.size();

                // 路径安全验证
                let path = Path::new(&name);
                if let Err(e) = Self::validate_path_static(path, &security_config) {
                    warn!("Skipping unsafe path: {} - {}", name, e);
                    summary.add_error(format!("Path validation failed for {}: {}", name, e));
                    continue;
                }

                // 限制检查
                let would_exceed_limits = size > max_file_size
                    || summary.total_size + size > max_total_size
                    || summary.files_extracted + 1 > max_file_count;

                if would_exceed_limits {
                    warn!(
                        "Skipping file exceeding limits: {} (size: {}, total: {}, count: {})",
                        name, size, summary.total_size, summary.files_extracted
                    );
                    summary.add_error(format!("File skipped (limits exceeded): {}", name));
                    continue;
                }

                let out_path = target_path.join(&name);

                if file.is_dir() {
                    let _ = std::fs::create_dir_all(&out_path);
                    continue;
                }

                // 创建父目录
                if let Some(parent) = out_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }

                // 提取文件
                match std::fs::File::create(&out_path) {
                    Ok(mut out_file) => {
                        if let Err(e) = std::io::copy(&mut file, &mut out_file) {
                            warn!("Failed to extract file {:?}: {}", out_path, e);
                            summary.add_error(format!("Extraction failed for {}: {}", name, e));
                        } else {
                            summary.add_file(PathBuf::from(&name), size);
                            trace!("已提取 ZIP 文件: {:?}, 大小: {}", out_path, size);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to create file {:?}: {}", out_path, e);
                        summary.add_error(format!("File creation failed for {}: {}", name, e));
                    }
                }
            }

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
impl ArchiveHandler for ZipHandler {
    fn can_handle(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|s| s.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("zip"))
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
        vec!["zip"]
    }

    async fn list_contents(&self, path: &Path) -> Result<Vec<ArchiveEntry>> {
        let path_owned = path.to_path_buf();
        let entries = tokio::task::spawn_blocking(move || {
            let file = std::fs::File::open(&path_owned)
                .map_err(|e| AppError::archive_error(e.to_string(), Some(path_owned.clone())))?;
            let mut archive = ZipArchive::new(file)
                .map_err(|e| AppError::archive_error(e.to_string(), Some(path_owned.clone())))?;
            let mut entries = Vec::new();

            for i in 0..archive.len() {
                let file = match archive.by_index(i) {
                    Ok(f) => f,
                    Err(e) => {
                        warn!("Failed to read zip entry {}: {}", i, e);
                        continue;
                    }
                };

                let name = PathBuf::from(file.name())
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();

                entries.push(ArchiveEntry {
                    name,
                    path: file.name().to_string(),
                    is_dir: file.is_dir(),
                    size: file.size(),
                    compressed_size: file.compressed_size(),
                });
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
            let file = std::fs::File::open(&path_owned)
                .map_err(|e| AppError::archive_error(e.to_string(), Some(path_owned.clone())))?;
            let mut archive = ZipArchive::new(file)
                .map_err(|e| AppError::archive_error(e.to_string(), Some(path_owned.clone())))?;
            let mut zip_file = archive
                .by_name(&file_name_owned)
                .map_err(|e| AppError::archive_error(e.to_string(), None))?;

            // 大小限制：10MB
            const MAX_SIZE: u64 = 10 * 1024 * 1024;
            let size = zip_file.size();

            if size > MAX_SIZE {
                // 大文件截断读取
                let mut buffer = vec![0u8; MAX_SIZE as usize];
                let bytes_read = zip_file.read(&mut buffer)?;
                let mut content = String::from_utf8_lossy(&buffer[..bytes_read]).to_string();
                content.push_str(&format!(
                    "\n\n[文件过大，已截断显示. 完整大小: {} bytes]",
                    size
                ));
                Ok(content)
            } else {
                let mut contents = String::new();
                zip_file.read_to_string(&mut contents)?;
                Ok(contents)
            }
        })
        .await
        .map_err(|e| AppError::archive_error(e.to_string(), None))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_zip_handler_base_impl() {
        let handler = ZipHandler {};
        assert_eq!(handler.handler_name(), "ZipHandler");
        assert_eq!(handler.supported_formats(), &["zip"]);
    }

    #[test]
    fn test_zip_handler_can_handle() {
        let handler = ZipHandler {};
        assert!(handler.can_handle(Path::new("test.zip")));
        assert!(handler.can_handle(Path::new("test.ZIP")));
        assert!(!handler.can_handle(Path::new("test.rar")));
        assert!(!handler.can_handle(Path::new("test.txt")));
    }

    #[test]
    fn test_zip_handler_file_extensions() {
        let handler = ZipHandler {};
        let extensions = handler.file_extensions();
        assert_eq!(extensions, vec!["zip"]);
    }

    #[tokio::test]
    async fn test_zip_handler_extract_with_context_basic() {
        use crate::archive::extraction_config::ExtractionConfig;
        use std::io::Write;

        let temp_dir = TempDir::new().unwrap();
        let zip_path = temp_dir.path().join("test.zip");
        let output_dir = temp_dir.path().join("output");

        // 创建测试 ZIP 文件
        {
            let file = std::fs::File::create(&zip_path).unwrap();
            let mut zip = zip::ZipWriter::new(file);
            let options = zip::write::FileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);

            zip.start_file("test.txt", options).unwrap();
            zip.write_all(b"Hello, World!").unwrap();
            zip.finish().unwrap();
        }

        let config = ExtractionConfig::default();
        let mut context = ExtractionContext::new(config);
        let handler = ZipHandler {};

        let result = handler
            .extract_with_context(&zip_path, &output_dir, &mut context)
            .await;

        assert!(result.is_ok());
        let summary = result.unwrap();
        assert_eq!(summary.files_extracted, 1);
        assert!(output_dir.join("test.txt").exists());
    }
}
