use crate::archive::archive_handler::{ArchiveHandler, ExtractionSummary};
use crate::error::{AppError, Result};
use async_trait::async_trait;
use std::path::Path;
use tokio::fs;
use tracing::info;

/**
 * RAR文件处理器
 *
 * 使用 unrar crate（基于 rarlab 官方 C 库）处理RAR文件
 */
pub struct RarHandler;

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
        // 确保目标目录存在
        fs::create_dir_all(target_dir).await.map_err(|e| {
            AppError::archive_error(
                format!("Failed to create target directory: {}", e),
                Some(target_dir.to_path_buf()),
            )
        })?;

        let source_str = source.to_string_lossy().to_string();
        let target_str = target_dir.to_string_lossy().to_string();

        // 使用 tokio::task::spawn_blocking 在阻塞型上下文中运行 unrar
        // 因为 unrar crate 是同步的（底层是 C 库调用）
        let result = tokio::task::spawn_blocking(move || {
            extract_rar_sync(&source_str, &target_str, max_file_size, max_total_size, max_file_count)
        })
        .await
        .map_err(|e| {
            AppError::archive_error(
                format!("RAR extraction task failed: {}", e),
                Some(source.to_path_buf()),
            )
        })?;

        result
    }

    #[allow(dead_code)]
    async fn extract(&self, source: &Path, target_dir: &Path) -> Result<ExtractionSummary> {
        // 默认使用安全限制：单个文件100MB，总大小1GB，文件数1000
        self.extract_with_limits(
            source,
            target_dir,
            100 * 1024 * 1024,
            1024 * 1024 * 1024, // 1GB
            1000,
        )
        .await
    }

    fn file_extensions(&self) -> Vec<&str> {
        vec!["rar"]
    }
}

/**
 * 同步提取 RAR 文件
 *
 * 使用 unrar crate 的 Archive API，需要配合 unrar 库文件（dll/so/dylib）
 */
fn extract_rar_sync(
    source: &str,
    target_dir: &str,
    max_file_size: u64,
    max_total_size: u64,
    max_file_count: usize,
) -> Result<ExtractionSummary> {
    let mut summary = ExtractionSummary::new();

    // 使用 unrar crate 打开归档进行提取处理
    // 模式：Process 模式允许提取文件内容
    // 状态：CursorBeforeHeader（光标在文件头之前）
    let result = unrar::Archive::new(source).open_for_processing();

    let mut archive = match result {
        Ok(a) => {
            info!("Successfully opened RAR archive: {}", source);
            a
        }
        Err(e) => {
            // 如果打开失败，检查是否缺少库文件
            let error_msg = e.to_string();
            if error_msg.contains("cannot open shared object file")
                || error_msg.contains("The specified module could not be found")
                || error_msg.contains("dll")
            {
                summary.add_error(format!(
                    "RAR library not found. Please ensure unrar library (unrar.dll/libunrar.so) is available. Error: {}",
                    error_msg
                ));
            } else {
                summary.add_error(format!("Failed to open RAR archive: {}", error_msg));
            }
            return Ok(summary);
        }
    };

    // 循环处理每个文件
    // Pattern: read_header() -> CursorBeforeFile -> extract() -> loop
    loop {
        // 读取下一个文件头
        // read_header() 返回 Option<OpenArchive<Process, CursorBeforeFile>>
        // None 表示没有更多文件
        let archive_option = match archive.read_header() {
            Ok(a) => a,
            Err(e) => {
                summary.add_error(format!("Failed to read archive header: {}", e));
                break;
            }
        };

        // 如果没有更多文件，退出循环
        let archive_before_file = match archive_option {
            Some(a) => a,
            None => {
                info!("End of archive reached");
                break;
            }
        };

        // 获取当前文件头信息
        let file_header = archive_before_file.entry();
        let entry_name = file_header.filename.to_string_lossy().to_string();
        let entry_size: u64 = file_header.unpacked_size;

        // 安全检查：单个文件大小限制
        if entry_size > max_file_size {
            summary.add_error(format!(
                "File {} exceeds maximum size limit of {} bytes, skipping",
                entry_name,
                max_file_size
            ));
            // 跳过这个文件，继续下一个
            match archive_before_file.skip() {
                Ok(a) => {
                    archive = a;
                    continue;
                }
                Err(e) => {
                    summary.add_error(format!("Failed to skip file {}: {}", entry_name, e));
                    break;
                }
            }
        }

        // 安全检查：总大小限制
        if summary.total_size + entry_size as u64 > max_total_size {
            summary.add_error(format!(
                "Extraction would exceed total size limit of {} bytes, stopping",
                max_total_size
            ));
            break;
        }

        // 安全检查：文件数量限制
        if summary.files_extracted + 1 > max_file_count {
            summary.add_error(format!(
                "Extraction would exceed file count limit of {} files, stopping",
                max_file_count
            ));
            break;
        }

        // 提取文件到目标目录（保留目录结构）
        match archive_before_file.extract_with_base(target_dir) {
            Ok(a) => {
                summary.add_file(
                    std::path::PathBuf::from(&entry_name),
                    entry_size as u64,
                );
                info!("Extracted: {}", entry_name);
                archive = a;
            }
            Err(e) => {
                summary.add_error(format!(
                    "Failed to extract {}: {}",
                    entry_name,
                    e
                ));
                break;
            }
        }
    }

    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_rar_handler_can_handle() {
        let handler = RarHandler;

        assert!(handler.can_handle(Path::new("test.rar")));
        assert!(handler.can_handle(Path::new("test.RAR")));
        assert!(!handler.can_handle(Path::new("test.zip")));
        assert!(!handler.can_handle(Path::new("test.txt")));
    }

    #[test]
    fn test_rar_handler_file_extensions() {
        let handler = RarHandler;
        let extensions = handler.file_extensions();

        assert_eq!(extensions, vec!["rar"]);
    }
}
