use crate::archive::archive_handler::{ArchiveEntry, ArchiveHandler, ExtractionSummary};
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
pub struct RarHandler {}

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

    async fn list_contents(&self, path: &Path) -> Result<Vec<ArchiveEntry>> {
        let path_owned = path.to_path_buf();
        let entries = tokio::task::spawn_blocking(move || {
            let mut archive = Archive::new(&path_owned)
                .open_for_listing()
                .map_err(|e| AppError::archive_error(e.to_string(), None))?;

            let mut entries = Vec::new();

            while let Some(header) = archive.read_header().map_err(|e| AppError::archive_error(e.to_string(), None))? {
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

                archive = header.skip().map_err(|e| AppError::archive_error(e.to_string(), None))?;
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

            while let Some(header) = archive.read_header().map_err(|e| AppError::archive_error(e.to_string(), None))? {
                let entry = header.entry();
                let name = entry.filename.to_string_lossy().to_string();

                // 匹配文件名
                if name == file_name_owned || Path::new(&name).file_name().map(|s| s.to_string_lossy().to_string()) == Some(file_name_owned.clone()) {
                    if entry.is_directory() {
                        return Err(AppError::archive_error("Cannot read directory".to_string(), None));
                    }

                    let size = entry.unpacked_size;

                    // 创建临时目录
                    let temp_dir = std::env::temp_dir();
                    let temp_path = temp_dir.join(format!("rar_extract_{}", std::process::id()));

                    // 提取到临时目录
                    let _archive = header.extract_to(&temp_path)
                        .map_err(|e| AppError::archive_error(e.to_string(), None))?;

                    // 读取提取的文件
                    let extract_path = temp_path.join(&name);
                    if extract_path.exists() {
                        let content = if size > MAX_SIZE {
                            // 大文件截断读取
                            let bytes = std::fs::read(&extract_path)?;
                            let truncated = String::from_utf8_lossy(&bytes[..MAX_SIZE as usize]).to_string();
                            format!("{}\n\n[文件过大，已截断显示. 完整大小: {} bytes]", truncated, size)
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

                archive = header.skip().map_err(|e| AppError::archive_error(e.to_string(), None))?;
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
