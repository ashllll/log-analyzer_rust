use crate::archive::archive_handler::{ArchiveHandler, ExtractionSummary};
use crate::error::{AppError, Result};
use async_trait::async_trait;
use std::io::Cursor;
use std::path::Path;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use zip::ZipArchive;

/**
 * ZIP文件处理器
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
        // 确保目标目录存在
        fs::create_dir_all(target_dir).await.map_err(|e| {
            AppError::archive_error(
                format!("Failed to create target directory: {}", e),
                Some(target_dir.to_path_buf()),
            )
        })?;

        // 读取ZIP文件内容
        let zip_data = fs::read(source).await.map_err(|e| {
            AppError::archive_error(
                format!("Failed to read ZIP file: {}", e),
                Some(source.to_path_buf()),
            )
        })?;

        // 在同步上下文中处理 ZIP 归档，提取所有文件数据
        let source_path = source.to_path_buf(); // Clone path to avoid lifetime issues
        let files_data = tokio::task::spawn_blocking(move || {
            let cursor = Cursor::new(zip_data);
            let mut archive = ZipArchive::new(cursor).map_err(|e| {
                AppError::archive_error(
                    format!("Failed to open ZIP archive: {}", e),
                    Some(source_path.clone()),
                )
            })?;

            let mut files = Vec::new();
            let mut total_size = 0;
            let mut file_count = 0;

            // 提取所有文件内容
            for i in 0..archive.len() {
                let mut file = archive.by_index(i).map_err(|e| {
                    AppError::archive_error(
                        format!("Failed to access file {} in archive: {}", i, e),
                        Some(source_path.clone()),
                    )
                })?;

                let file_name = file.name().to_string();
                let is_dir = file.is_dir();
                let file_size = file.size();

                // 安全检查：防止路径遍历
                if file_name.contains("..") {
                    files.push((file_name.clone(), None, true)); // 标记为错误
                    continue;
                }

                if is_dir {
                    files.push((file_name, None, false));
                } else {
                    // 安全检查：单个文件大小限制
                    if file_size > max_file_size {
                        return Err(AppError::archive_error(
                            format!(
                                "File {} exceeds maximum size limit of {} bytes",
                                file_name, max_file_size
                            ),
                            Some(source_path),
                        ));
                    }

                    // 安全检查：总大小限制
                    if total_size + file_size > max_total_size {
                        return Err(AppError::archive_error(
                            format!(
                                "Extraction would exceed total size limit of {} bytes",
                                max_total_size
                            ),
                            Some(source_path),
                        ));
                    }

                    // 安全检查：文件数量限制
                    if file_count + 1 > max_file_count {
                        return Err(AppError::archive_error(
                            format!(
                                "Extraction would exceed file count limit of {} files",
                                max_file_count
                            ),
                            Some(source_path),
                        ));
                    }

                    // 读取文件内容到内存
                    let mut buffer = Vec::new();
                    std::io::copy(&mut file, &mut buffer).map_err(|e| {
                        AppError::archive_error(
                            format!("Failed to read file content: {}", e),
                            Some(source_path.clone()),
                        )
                    })?;

                    // 更新统计
                    total_size += buffer.len() as u64;
                    file_count += 1;

                    files.push((file_name, Some(buffer), false));
                }
            }

            Ok::<Vec<(String, Option<Vec<u8>>, bool)>, AppError>(files)
        })
        .await
        .map_err(|e| {
            AppError::archive_error(
                format!("Failed to extract ZIP archive: {}", e),
                Some(source.to_path_buf()),
            )
        })??;

        let mut summary = ExtractionSummary::new();

        // 异步写入文件
        for (file_name, content, is_error) in files_data {
            if is_error {
                summary.add_error(format!("Unsafe file path detected: {}", file_name));
                continue;
            }

            let out_path = target_dir.join(&file_name);

            // 如果是目录，创建目录
            if content.is_none() {
                fs::create_dir_all(&out_path).await.map_err(|e| {
                    AppError::archive_error(
                        format!("Failed to create directory: {}", e),
                        Some(out_path.clone()),
                    )
                })?;
                continue;
            }

            // 确保父目录存在
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent).await.map_err(|e| {
                    AppError::archive_error(
                        format!("Failed to create parent directory: {}", e),
                        Some(parent.to_path_buf()),
                    )
                })?;
            }

            // 创建输出文件并写入内容
            if let Some(buffer) = content {
                let mut out_file = fs::File::create(&out_path).await.map_err(|e| {
                    AppError::archive_error(
                        format!("Failed to create output file: {}", e),
                        Some(out_path.clone()),
                    )
                })?;

                out_file.write_all(&buffer).await.map_err(|e| {
                    AppError::archive_error(
                        format!("Failed to write file content: {}", e),
                        Some(out_path.clone()),
                    )
                })?;

                summary.add_file(out_path, buffer.len() as u64);
            }
        }

        Ok(summary)
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
        vec!["zip"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;
    use zip::write::FileOptions;

    #[tokio::test]
    async fn test_zip_handler_can_handle() {
        let handler = ZipHandler;

        assert!(handler.can_handle(Path::new("test.zip")));
        assert!(handler.can_handle(Path::new("test.ZIP")));
        assert!(!handler.can_handle(Path::new("test.rar")));
        assert!(!handler.can_handle(Path::new("test.txt")));
    }

    #[tokio::test]
    async fn test_zip_handler_extract() {
        // 创建临时目录
        let temp_dir = TempDir::new().unwrap();
        let source_path = temp_dir.path().join("test.zip");
        let target_dir = temp_dir.path().join("extracted");

        // 创建测试ZIP文件
        create_test_zip(&source_path);

        // 提取ZIP文件
        let handler = ZipHandler;
        let summary = handler.extract(&source_path, &target_dir).await.unwrap();

        // 验证提取结果
        assert!(summary.files_extracted > 0);
        assert!(summary.total_size > 0);
        assert!(!summary.has_errors());

        // 验证文件存在
        let test_txt = target_dir.join("test.txt");
        assert!(test_txt.exists());

        // 验证文件内容
        let content = fs::read_to_string(&test_txt).await.unwrap();
        assert_eq!(content, "Hello, World!");
    }

    #[tokio::test]
    async fn test_zip_handler_extract_empty() {
        let temp_dir = TempDir::new().unwrap();
        let source_path = temp_dir.path().join("empty.zip");
        let target_dir = temp_dir.path().join("extracted");

        // 创建空ZIP文件
        create_empty_zip(&source_path);

        let handler = ZipHandler;
        let summary = handler.extract(&source_path, &target_dir).await.unwrap();

        assert_eq!(summary.files_extracted, 0);
        assert_eq!(summary.total_size, 0);
    }

    #[tokio::test]
    async fn test_zip_handler_file_extensions() {
        let handler = ZipHandler;
        let extensions = handler.file_extensions();

        assert_eq!(extensions, vec!["zip"]);
    }

    // 辅助函数：创建测试ZIP文件
    fn create_test_zip(path: &Path) {
        let file = std::fs::File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = FileOptions::default().compression_method(zip::CompressionMethod::Stored);

        // 添加一个文本文件
        zip.start_file("test.txt", options).unwrap();
        zip.write_all(b"Hello, World!").unwrap();

        // 添加一个目录
        zip.add_directory("test_dir/", options).unwrap();

        zip.finish().unwrap();
    }

    // 辅助函数：创建空ZIP文件
    fn create_empty_zip(path: &Path) {
        let file = std::fs::File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.finish().unwrap();
    }
}
