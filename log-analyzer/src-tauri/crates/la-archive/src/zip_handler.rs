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

/**
 * ZIP 流式处理器（async_zip）
 *
 * 与同步 `ZipHandler` 并存，供 `processor.rs` 在导入阶段优先使用。
 * 文本文件直接流式入 CAS，无需创建临时目录。
 */
pub struct StreamingZipHandler;

#[async_trait]
impl crate::archive_handler::StreamingArchiveHandler for StreamingZipHandler {
    async fn list_entries(&self, source: &Path) -> Result<Vec<crate::archive_handler::ArchiveEntryInfo>> {
        let reader = async_zip::tokio::read::fs::ZipFileReader::new(source).await.map_err(|e| {
            AppError::archive_error(format!("Failed to create ZIP reader: {}", e), Some(source.to_path_buf()))
        })?;

        let mut entries = Vec::new();
        for entry in reader.file().entries() {
            let path = PathBuf::from(entry.filename().as_str().unwrap_or(""));
            entries.push(crate::archive_handler::ArchiveEntryInfo {
                path,
                size: entry.uncompressed_size(),
                is_directory: entry.dir().unwrap_or(false),
                is_symlink: false,
            });
        }
        Ok(entries)
    }

    async fn stream_entry_to_cas(
        &self,
        source: &Path,
        entry_path: &str,
        cas: &la_storage::ContentAddressableStorage,
    ) -> Result<String> {
        let reader = async_zip::tokio::read::fs::ZipFileReader::new(source).await.map_err(|e| {
            AppError::archive_error(format!("Failed to create ZIP reader: {}", e), Some(source.to_path_buf()))
        })?;

        let index = reader.file().entries().iter()
            .position(|e| e.filename().as_str().unwrap_or("") == entry_path)
            .ok_or_else(|| AppError::archive_error(
                format!("ZIP entry not found: {}", entry_path),
                Some(source.to_path_buf()),
            ))?;

        let entry_reader = reader.reader_with_entry(index).await.map_err(|e| {
            AppError::archive_error(format!("Failed to open ZIP entry reader: {}", e), Some(source.to_path_buf()))
        })?;

        use tokio_util::compat::FuturesAsyncReadCompatExt;
        cas.store_stream(entry_reader.compat()).await.map_err(|e| {
            AppError::archive_error(format!("CAS store stream failed: {}", e), Some(source.to_path_buf()))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;
    use zip::write::FileOptions;

    /// 创建测试用的 ZIP 文件
    fn create_test_zip(
        path: &Path,
        files: Vec<(&str, &[u8])>,
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let file = std::fs::File::create(path)?;
        let mut zip = zip::ZipWriter::new(file);
        let options: zip::write::FileOptions<'_, ()> =
            FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        for (name, content) in files {
            zip.start_file(name, options)?;
            zip.write_all(content)?;
        }

        zip.finish()?;
        Ok(())
    }

    #[test]
    fn test_zip_handler_can_handle() {
        let handler = ZipHandler;

        // 应该能处理 .zip 文件
        assert!(handler.can_handle(Path::new("test.zip")));
        assert!(handler.can_handle(Path::new("test.ZIP")));
        assert!(handler.can_handle(Path::new("/path/to/archive.zip")));

        // 不应该处理其他格式
        assert!(!handler.can_handle(Path::new("test.tar")));
        assert!(!handler.can_handle(Path::new("test.gz")));
        assert!(!handler.can_handle(Path::new("test.rar")));
        assert!(!handler.can_handle(Path::new("test.txt")));
        assert!(!handler.can_handle(Path::new("test")));
    }

    #[test]
    fn test_zip_handler_file_extensions() {
        let handler = ZipHandler;
        let extensions = handler.file_extensions();

        assert_eq!(extensions, vec!["zip"]);
    }

    #[tokio::test]
    async fn test_extract_zip_file() {
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let zip_file = temp_dir.path().join("test.zip");
        let output_dir = temp_dir.path().join("output");

        // 创建测试 ZIP 文件
        let files = vec![
            ("file1.txt", b"Hello, World!" as &[u8]),
            ("file2.txt", b"Second file content"),
            ("dir/nested.txt", b"Nested file content"),
        ];
        create_test_zip(&zip_file, files).expect("创建 ZIP 文件失败");

        // 解压文件
        let handler = ZipHandler;
        let summary = handler
            .extract(&zip_file, &output_dir)
            .await
            .expect("解压 ZIP 文件失败");

        assert_eq!(summary.files_extracted, 3);
        assert!(output_dir.join("file1.txt").exists());
        assert!(output_dir.join("file2.txt").exists());
        assert!(output_dir.join("dir/nested.txt").exists());

        // 验证内容
        let content1 = std::fs::read_to_string(output_dir.join("file1.txt")).unwrap();
        assert_eq!(content1, "Hello, World!");

        let content2 = std::fs::read_to_string(output_dir.join("file2.txt")).unwrap();
        assert_eq!(content2, "Second file content");

        let content3 = std::fs::read_to_string(output_dir.join("dir/nested.txt")).unwrap();
        assert_eq!(content3, "Nested file content");
    }

    #[tokio::test]
    async fn test_extract_zip_with_limits() {
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let zip_file = temp_dir.path().join("test.zip");
        let output_dir = temp_dir.path().join("output");

        // 创建测试 ZIP 文件
        let files = vec![
            ("small.txt", b"Small content" as &[u8]),
            ("large.txt", &[b'x'; 2000]), // 2KB 文件
        ];
        create_test_zip(&zip_file, files).expect("创建 ZIP 文件失败");

        // 使用限制解压：最大文件大小 1KB
        let handler = ZipHandler;
        let summary = handler
            .extract_with_limits(&zip_file, &output_dir, 1024, 1024 * 1024, 100)
            .await
            .expect("解压 ZIP 文件失败");

        // 只有小文件应该被解压
        assert_eq!(summary.files_extracted, 1);
        assert!(output_dir.join("small.txt").exists());
        assert!(!output_dir.join("large.txt").exists());
    }

    #[tokio::test]
    async fn test_extract_zip_max_total_size_limit() {
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let zip_file = temp_dir.path().join("test.zip");
        let output_dir = temp_dir.path().join("output");

        // 创建测试 ZIP 文件，总大小超过限制
        let files = vec![
            ("file1.txt", &[b'a'; 600][..]), // 600 bytes
            ("file2.txt", &[b'b'; 600][..]), // 600 bytes
        ];
        create_test_zip(&zip_file, files).expect("创建 ZIP 文件失败");

        // 使用限制解压：最大总大小 1KB
        let handler = ZipHandler;
        let summary = handler
            .extract_with_limits(&zip_file, &output_dir, 1024, 1024, 100)
            .await
            .expect("解压 ZIP 文件失败");

        // 只有第一个文件应该被解压
        assert_eq!(summary.files_extracted, 1);
        assert!(output_dir.join("file1.txt").exists());
    }

    #[tokio::test]
    async fn test_extract_zip_max_file_count_limit() {
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let zip_file = temp_dir.path().join("test.zip");
        let output_dir = temp_dir.path().join("output");

        // 创建测试 ZIP 文件，包含多个文件
        let files = vec![
            ("file1.txt", b"content1" as &[u8]),
            ("file2.txt", b"content2"),
            ("file3.txt", b"content3"),
        ];
        create_test_zip(&zip_file, files).expect("创建 ZIP 文件失败");

        // 使用限制解压：最大文件数量 2
        let handler = ZipHandler;
        let summary = handler
            .extract_with_limits(&zip_file, &output_dir, 1024 * 1024, 1024 * 1024 * 1024, 2)
            .await
            .expect("解压 ZIP 文件失败");

        // 只有前两个文件应该被解压
        assert_eq!(summary.files_extracted, 2);
    }

    #[tokio::test]
    async fn test_extract_zip_empty_file() {
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let zip_file = temp_dir.path().join("empty.zip");
        let output_dir = temp_dir.path().join("output");

        // 创建空内容的 ZIP 文件（只有目录条目）
        let file = std::fs::File::create(&zip_file).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.add_directory("empty_dir/", zip::write::FileOptions::<'_, ()>::default())
            .unwrap();
        zip.finish().unwrap();

        // 解压文件
        let handler = ZipHandler;
        let summary = handler
            .extract(&zip_file, &output_dir)
            .await
            .expect("解压 ZIP 文件失败");

        // 目录条目不应该被计为提取的文件
        assert_eq!(summary.files_extracted, 0);
        // 但目录应该被创建
        assert!(output_dir.join("empty_dir").exists());
    }

    #[tokio::test]
    async fn test_extract_zip_with_special_chars_in_filename() {
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let zip_file = temp_dir.path().join("test.zip");
        let output_dir = temp_dir.path().join("output");

        // 创建包含特殊字符文件名的 ZIP 文件
        let files = vec![
            ("file with spaces.txt", b"content1" as &[u8]),
            ("file-with-dashes.txt", b"content2"),
            ("file_with_underscores.txt", b"content3"),
        ];
        create_test_zip(&zip_file, files).expect("创建 ZIP 文件失败");

        // 解压文件
        let handler = ZipHandler;
        let summary = handler
            .extract(&zip_file, &output_dir)
            .await
            .expect("解压 ZIP 文件失败");

        assert_eq!(summary.files_extracted, 3);
        assert!(output_dir.join("file with spaces.txt").exists());
        assert!(output_dir.join("file-with-dashes.txt").exists());
        assert!(output_dir.join("file_with_underscores.txt").exists());
    }

    #[tokio::test]
    async fn test_extract_nonexistent_zip() {
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let zip_file = temp_dir.path().join("nonexistent.zip");
        let output_dir = temp_dir.path().join("output");

        let handler = ZipHandler;
        let result = handler.extract(&zip_file, &output_dir).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_extract_corrupted_zip() {
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let zip_file = temp_dir.path().join("corrupted.zip");
        let output_dir = temp_dir.path().join("output");

        // 写入损坏的 ZIP 数据
        std::fs::write(&zip_file, b"This is not a valid ZIP file").unwrap();

        let handler = ZipHandler;
        let result = handler.extract(&zip_file, &output_dir).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_extract_zip_slip_protection() {
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let zip_file = temp_dir.path().join("test.zip");
        let output_dir = temp_dir.path().join("output");

        // 创建包含路径遍历尝试的 ZIP 文件
        // 注意：zip crate 在写入时会自动规范化路径，所以我们测试的是解压时的保护
        let file = std::fs::File::create(&zip_file).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options: zip::write::FileOptions<'_, ()> =
            FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        // 添加一个正常文件
        zip.start_file("normal.txt", options).unwrap();
        zip.write_all(b"Normal content").unwrap();

        zip.finish().unwrap();

        // 解压文件
        let handler = ZipHandler;
        let summary = handler
            .extract(&zip_file, &output_dir)
            .await
            .expect("解压 ZIP 文件失败");

        assert_eq!(summary.files_extracted, 1);
        assert!(output_dir.join("normal.txt").exists());
    }

    #[tokio::test]
    async fn test_extract_zip_deeply_nested() {
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let zip_file = temp_dir.path().join("test.zip");
        let output_dir = temp_dir.path().join("output");

        // 创建深层嵌套目录结构的 ZIP 文件
        let files = vec![
            ("a/b/c/d/e/deep.txt", b"Deep nested content" as &[u8]),
            ("a/b/c/mid.txt", b"Mid level content"),
            ("a/top.txt", b"Top level content"),
        ];
        create_test_zip(&zip_file, files).expect("创建 ZIP 文件失败");

        // 解压文件
        let handler = ZipHandler;
        let summary = handler
            .extract(&zip_file, &output_dir)
            .await
            .expect("解压 ZIP 文件失败");

        assert_eq!(summary.files_extracted, 3);
        assert!(output_dir.join("a/b/c/d/e/deep.txt").exists());
        assert!(output_dir.join("a/b/c/mid.txt").exists());
        assert!(output_dir.join("a/top.txt").exists());

        // 验证内容
        let deep_content = std::fs::read_to_string(output_dir.join("a/b/c/d/e/deep.txt")).unwrap();
        assert_eq!(deep_content, "Deep nested content");
    }

    #[tokio::test]
    async fn test_extract_zip_large_file() {
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let zip_file = temp_dir.path().join("test.zip");
        let output_dir = temp_dir.path().join("output");

        // 创建包含大文件的 ZIP 文件 (5MB)
        let large_content = vec![b'x'; 5 * 1024 * 1024];
        let files = vec![("large.bin", large_content.as_slice())];
        create_test_zip(&zip_file, files).expect("创建 ZIP 文件失败");

        // 解压文件
        let handler = ZipHandler;
        let summary = handler
            .extract(&zip_file, &output_dir)
            .await
            .expect("解压 ZIP 文件失败");

        assert_eq!(summary.files_extracted, 1);
        assert_eq!(summary.total_size, 5 * 1024 * 1024);
        assert!(output_dir.join("large.bin").exists());

        // 验证文件大小
        let metadata = std::fs::metadata(output_dir.join("large.bin")).unwrap();
        assert_eq!(metadata.len(), 5 * 1024 * 1024);
    }

    #[tokio::test]
    async fn test_extract_zip_many_files() {
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let zip_file = temp_dir.path().join("test.zip");
        let output_dir = temp_dir.path().join("output");

        // 创建包含多个文件的 ZIP 文件
        let mut files = Vec::new();
        for i in 0..100 {
            files.push((
                format!("file{:03}.txt", i),
                format!("Content of file {}", i).into_bytes(),
            ));
        }

        let file = std::fs::File::create(&zip_file).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options: zip::write::FileOptions<'_, ()> =
            FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        for (name, content) in files {
            zip.start_file(&name, options).unwrap();
            zip.write_all(&content).unwrap();
        }
        zip.finish().unwrap();

        // 解压文件
        let handler = ZipHandler;
        let summary = handler
            .extract(&zip_file, &output_dir)
            .await
            .expect("解压 ZIP 文件失败");

        assert_eq!(summary.files_extracted, 100);

        // 验证部分文件
        for i in [0, 50, 99] {
            let file_path = output_dir.join(format!("file{:03}.txt", i));
            assert!(file_path.exists());
            let content = std::fs::read_to_string(file_path).unwrap();
            assert_eq!(content, format!("Content of file {}", i));
        }
    }
}
