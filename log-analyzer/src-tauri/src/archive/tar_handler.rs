use crate::archive::archive_handler::{ArchiveEntry, ArchiveHandler, ExtractionSummary};
use crate::error::{AppError, Result};
use crate::utils::path_security::{
    validate_and_sanitize_archive_path, PathValidationResult, SecurityConfig,
};
use async_trait::async_trait;
use std::io::Read;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::warn;

use flate2::read::GzDecoder;
use tar::Archive;

/**
 * TAR文件处理器 (稳定版本)
 */
pub struct TarHandler {}

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

    async fn list_contents(&self, path: &Path) -> Result<Vec<ArchiveEntry>> {
        let path_owned = path.to_path_buf();
        let entries = tokio::task::spawn_blocking(move || {
            let file = std::fs::File::open(&path_owned)?;
            let is_gzipped = path_owned
                .extension()
                .and_then(|s| s.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("gz") || ext.eq_ignore_ascii_case("tgz"))
                .unwrap_or(false);

            let mut entries = Vec::new();

            if is_gzipped {
                let decoder = GzDecoder::new(file);
                let mut archive = Archive::new(decoder);
                Self::list_contents_sync(&mut archive, &mut entries)?;
            } else {
                let mut archive = Archive::new(file);
                Self::list_contents_sync(&mut archive, &mut entries)?;
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
            let file = std::fs::File::open(&path_owned)?;
            let is_gzipped = path_owned
                .extension()
                .and_then(|s| s.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("gz") || ext.eq_ignore_ascii_case("tgz"))
                .unwrap_or(false);

            // 大小限制：10MB
            const MAX_SIZE: u64 = 10 * 1024 * 1024;

            if is_gzipped {
                let decoder = GzDecoder::new(file);
                let mut archive = Archive::new(decoder);
                Self::read_file_sync(&mut archive, &file_name_owned, MAX_SIZE)
            } else {
                let mut archive = Archive::new(file);
                Self::read_file_sync(&mut archive, &file_name_owned, MAX_SIZE)
            }
        })
        .await
        .map_err(|e| AppError::archive_error(e.to_string(), None))?
    }
}

impl TarHandler {
    fn list_contents_sync<R: std::io::Read>(
        archive: &mut Archive<R>,
        entries: &mut Vec<ArchiveEntry>,
    ) -> Result<()> {
        let tar_entries = archive
            .entries()
            .map_err(|e| AppError::archive_error(e.to_string(), None))?;

        for entry_result in tar_entries {
            let entry = match entry_result {
                Ok(e) => e,
                Err(e) => {
                    warn!("Failed to read tar entry: {}", e);
                    continue;
                }
            };

            let path_buf = match entry.path() {
                Ok(p) => p.to_path_buf(),
                Err(e) => {
                    warn!("Failed to get entry path: {}", e);
                    continue;
                }
            };

            let name = path_buf
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| path_buf.to_string_lossy().to_string());

            let path_str = path_buf.to_string_lossy().to_string();
            let is_dir = entry.header().entry_type().is_dir();
            let size = entry.header().size().unwrap_or(0);

            entries.push(ArchiveEntry {
                name,
                path: path_str,
                is_dir,
                size,
                compressed_size: size, // TAR 不提供压缩大小
            });
        }

        Ok(())
    }

    fn read_file_sync<R: std::io::Read>(
        archive: &mut Archive<R>,
        file_name: &str,
        max_size: u64,
    ) -> Result<String> {
        let tar_entries = archive
            .entries()
            .map_err(|e| AppError::archive_error(e.to_string(), None))?;

        for entry_result in tar_entries {
            let entry = match entry_result {
                Ok(e) => e,
                Err(e) => {
                    warn!("Failed to read tar entry: {}", e);
                    continue;
                }
            };

            let path_buf = match entry.path() {
                Ok(p) => p.to_path_buf(),
                Err(e) => {
                    warn!("Failed to get entry path: {}", e);
                    continue;
                }
            };

            let path_str = path_buf.to_string_lossy().to_string();

            // 匹配文件名（支持完整路径或文件名）
            if path_str == file_name
                || path_buf
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    == Some(file_name.to_string())
            {
                let size = entry.header().size().unwrap_or(0);

                if size > max_size {
                    // 大文件截断读取
                    let mut buffer = vec![0u8; max_size as usize];
                    let mut entry = entry;
                    let bytes_read = entry.read(&mut buffer)?;
                    let mut content = String::from_utf8_lossy(&buffer[..bytes_read]).to_string();
                    content.push_str(&format!(
                        "\n\n[文件过大，已截断显示. 完整大小: {} bytes]",
                        size
                    ));
                    return Ok(content);
                } else {
                    let mut contents = String::new();
                    let mut entry = entry;
                    entry.read_to_string(&mut contents)?;
                    return Ok(contents);
                }
            }
        }

        Err(AppError::archive_error(
            format!("File not found in archive: {}", file_name),
            None,
        ))
    }

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
