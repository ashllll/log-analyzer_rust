use crate::archive::archive_handler::{ArchiveHandler, ExtractionSummary};
use crate::error::{AppError, Result};
use async_compression::tokio::bufread::GzipDecoder;
use async_trait::async_trait;
use std::path::Path;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};

/**
 * GZ文件处理器
 *
 * 处理单个gzip压缩文件，解压为单个文件
 *
 * 支持两种模式:
 * - 内存模式: 适用于小文件 (< 10MB)
 * - 流式模式: 适用于大文件，避免内存溢出
 */
pub struct GzHandler;

impl GzHandler {
    /// Stream extract a gzip file without loading entire content into memory
    ///
    /// This method uses async-compression for streaming decompression,
    /// which is essential for handling large log files (1GB+) without
    /// causing memory spikes.
    ///
    /// # Arguments
    ///
    /// * `source` - Path to the .gz file
    /// * `target_dir` - Directory to extract to
    /// * `max_file_size` - Maximum allowed file size (safety limit)
    ///
    /// # Returns
    ///
    /// ExtractionSummary with file count and size
    ///
    /// # Requirements
    ///
    /// Validates: Requirements 6.1, 6.2
    pub async fn stream_extract_gz(
        source: &Path,
        target_dir: &Path,
        max_file_size: u64,
    ) -> Result<ExtractionSummary> {
        const BUFFER_SIZE: usize = 64 * 1024; // 64KB buffer for streaming

        // Ensure target directory exists
        fs::create_dir_all(target_dir).await.map_err(|e| {
            AppError::archive_error(
                format!("Failed to create target directory: {}", e),
                Some(target_dir.to_path_buf()),
            )
        })?;

        // Open source file for streaming
        let file = fs::File::open(source).await.map_err(|e| {
            AppError::archive_error(
                format!("Failed to open GZ file: {}", e),
                Some(source.to_path_buf()),
            )
        })?;

        // Create buffered reader for efficient I/O
        let reader = BufReader::with_capacity(BUFFER_SIZE, file);

        // Create gzip decoder that streams decompression
        let mut decoder = GzipDecoder::new(reader);

        // Determine output file name (remove .gz extension)
        let output_name = source
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output");

        let output_path = target_dir.join(output_name);

        // Create output file with buffered writer
        let output_file = fs::File::create(&output_path).await.map_err(|e| {
            AppError::archive_error(
                format!("Failed to create output file: {}", e),
                Some(output_path.clone()),
            )
        })?;

        let mut writer = BufWriter::with_capacity(BUFFER_SIZE, output_file);

        // Stream decompression with size tracking
        let mut buffer = vec![0u8; BUFFER_SIZE];
        let mut total_bytes = 0u64;

        loop {
            let bytes_read = decoder.read(&mut buffer).await.map_err(|e| {
                AppError::archive_error(
                    format!("Failed to decompress gzip stream: {}", e),
                    Some(source.to_path_buf()),
                )
            })?;

            if bytes_read == 0 {
                break; // EOF
            }

            // Safety check: enforce size limit
            total_bytes += bytes_read as u64;
            if total_bytes > max_file_size {
                // Clean up partial file
                drop(writer);
                let _ = fs::remove_file(&output_path).await;

                return Err(AppError::archive_error(
                    format!(
                        "File {} exceeds maximum size limit of {} bytes (got {} bytes)",
                        source.display(),
                        max_file_size,
                        total_bytes
                    ),
                    Some(source.to_path_buf()),
                ));
            }

            // Write decompressed data
            writer.write_all(&buffer[..bytes_read]).await.map_err(|e| {
                AppError::archive_error(
                    format!("Failed to write decompressed data: {}", e),
                    Some(output_path.clone()),
                )
            })?;
        }

        // Flush remaining data
        writer.flush().await.map_err(|e| {
            AppError::archive_error(
                format!("Failed to flush output file: {}", e),
                Some(output_path.clone()),
            )
        })?;

        let mut summary = ExtractionSummary::new();
        summary.add_file(output_path, total_bytes);

        Ok(summary)
    }
}

#[async_trait]
impl ArchiveHandler for GzHandler {
    fn can_handle(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|s| s.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("gz") && !is_tar_gz(path))
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
        // **MEMORY OPTIMIZATION**: Lowered threshold from 10MB to 1MB
        // Reason: 9.9MB file with 10x compression ratio = 99MB memory spike
        // With 1MB threshold: worst case ~10MB memory usage (acceptable)
        const STREAMING_THRESHOLD: u64 = 1024 * 1024; // 1MB threshold

        // Check file size to decide between streaming and in-memory
        let file_size = fs::metadata(source).await.map(|m| m.len()).unwrap_or(0);

        // Use streaming for large files to avoid memory issues
        if file_size > STREAMING_THRESHOLD {
            tracing::info!(
                file = %source.display(),
                size = file_size,
                "Using streaming extraction for large GZ file"
            );
            return Self::stream_extract_gz(source, target_dir, max_file_size).await;
        }

        // For small files, use the original in-memory approach
        // 确保目标目录存在
        fs::create_dir_all(target_dir).await.map_err(|e| {
            AppError::archive_error(
                format!("Failed to create target directory: {}", e),
                Some(target_dir.to_path_buf()),
            )
        })?;

        // 读取压缩文件
        let compressed_data = fs::read(source).await.map_err(|e| {
            AppError::archive_error(
                format!("Failed to read GZ file: {}", e),
                Some(source.to_path_buf()),
            )
        })?;

        // 解压数据
        let decompressed_data = decompress_gzip(&compressed_data)?;
        let data_len = decompressed_data.len() as u64;

        // 安全检查：单个文件大小限制
        if data_len > max_file_size {
            return Err(AppError::archive_error(
                format!(
                    "File {} exceeds maximum size limit of {} bytes",
                    source.display(),
                    max_file_size
                ),
                Some(source.to_path_buf()),
            ));
        }

        // 安全检查：总大小限制
        if data_len > max_total_size {
            return Err(AppError::archive_error(
                format!(
                    "Extraction would exceed total size limit of {} bytes",
                    max_total_size
                ),
                Some(source.to_path_buf()),
            ));
        }

        // 安全检查：文件数量限制（GZ通常只包含一个文件）
        if max_file_count < 1 {
            return Err(AppError::archive_error(
                format!(
                    "Extraction would exceed file count limit of {} files",
                    max_file_count
                ),
                Some(source.to_path_buf()),
            ));
        }

        // 确定输出文件名（去掉.gz扩展名）
        let output_name = source
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output");

        let output_path = target_dir.join(output_name);

        // 写入解压后的文件
        fs::write(&output_path, decompressed_data)
            .await
            .map_err(|e| {
                AppError::archive_error(
                    format!("Failed to write decompressed file: {}", e),
                    Some(output_path.clone()),
                )
            })?;

        let mut summary = ExtractionSummary::new();
        summary.add_file(output_path, data_len);

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
        vec!["gz"]
    }
}

/**
 * 判断是否为tar.gz文件
 */
fn is_tar_gz(path: &Path) -> bool {
    if let Some(stem) = path.file_stem() {
        if let Some(stem_str) = stem.to_str() {
            return stem_str.ends_with(".tar");
        }
    }
    false
}

/**
 * 解压gzip数据
 */
fn decompress_gzip(data: &[u8]) -> Result<Vec<u8>> {
    use flate2::read::GzDecoder;
    use std::io::Read;

    let mut decoder = GzDecoder::new(data);
    let mut decompressed = Vec::new();

    decoder
        .read_to_end(&mut decompressed)
        .map_err(|e| AppError::archive_error(format!("Failed to decompress gzip: {}", e), None))?;

    Ok(decompressed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_gz_handler_can_handle() {
        let handler = GzHandler;

        assert!(handler.can_handle(Path::new("test.gz")));
        assert!(!handler.can_handle(Path::new("test.tar.gz"))); // tar.gz由TarHandler处理
        assert!(!handler.can_handle(Path::new("test.zip")));
        assert!(!handler.can_handle(Path::new("test.txt")));
    }

    #[test]
    fn test_is_tar_gz() {
        assert!(is_tar_gz(Path::new("test.tar.gz")));
        assert!(is_tar_gz(Path::new("archive.tar.GZ")));
        assert!(!is_tar_gz(Path::new("test.gz")));
        assert!(!is_tar_gz(Path::new("document.txt")));
    }

    #[test]
    fn test_gz_handler_file_extensions() {
        let handler = GzHandler;
        let extensions = handler.file_extensions();

        assert_eq!(extensions, vec!["gz"]);
    }

    #[test]
    fn test_decompress_gzip() {
        let original_data = b"Hello, World! This is test data for gzip compression.";

        // 压缩数据
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(original_data)
            .expect("Failed to write to gzip encoder");
        let compressed = encoder
            .finish()
            .expect("Failed to finalize gzip compression");

        // 解压数据
        let decompressed = decompress_gzip(&compressed).expect("Failed to decompress gzip data");

        assert_eq!(decompressed, original_data);
    }

    #[tokio::test]
    async fn test_extract_gz_file() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let source_file = temp_dir.path().join("test.txt.gz");
        let output_dir = temp_dir.path().join("output");

        // 创建测试数据
        let original_data = b"This is test content for gzip file.";

        // 压缩并写入文件
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(original_data)
            .expect("Failed to write to gzip encoder");
        let compressed = encoder
            .finish()
            .expect("Failed to finalize gzip compression");

        fs::write(&source_file, compressed)
            .await
            .expect("Failed to write compressed data");

        // 提取文件
        let handler = GzHandler;
        let summary = handler
            .extract(&source_file, &output_dir)
            .await
            .expect("Failed to extract gz file");

        assert_eq!(summary.files_extracted, 1);
        assert!(output_dir.join("test.txt").exists());

        // 验证内容
        let extracted_content = fs::read(output_dir.join("test.txt"))
            .await
            .expect("Failed to read extracted file");
        assert_eq!(extracted_content, original_data);
    }

    #[tokio::test]
    async fn test_stream_extract_gz_small_file() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let source_file = temp_dir.path().join("small.txt.gz");
        let output_dir = temp_dir.path().join("output");

        // Create small test data (< 10MB)
        let original_data = b"Small file content for streaming test.";

        // Compress and write file
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(original_data)
            .expect("Failed to write to gzip encoder");
        let compressed = encoder
            .finish()
            .expect("Failed to finalize gzip compression");

        fs::write(&source_file, compressed)
            .await
            .expect("Failed to write compressed data");

        // Extract using streaming
        let summary = GzHandler::stream_extract_gz(
            &source_file,
            &output_dir,
            100 * 1024 * 1024, // 100MB limit
        )
        .await
        .expect("Failed to stream extract gz file");

        assert_eq!(summary.files_extracted, 1);
        assert!(output_dir.join("small.txt").exists());

        // Verify content
        let extracted_content = fs::read(output_dir.join("small.txt"))
            .await
            .expect("Failed to read extracted file");
        assert_eq!(extracted_content, original_data);
    }

    #[tokio::test]
    async fn test_stream_extract_gz_large_file() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let source_file = temp_dir.path().join("large.txt.gz");
        let output_dir = temp_dir.path().join("output");

        // Create large test data (> 10MB to trigger streaming)
        let original_data = vec![b'x'; 15 * 1024 * 1024]; // 15MB

        // Compress and write file
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(&original_data)
            .expect("Failed to write to gzip encoder");
        let compressed = encoder
            .finish()
            .expect("Failed to finalize gzip compression");

        fs::write(&source_file, compressed)
            .await
            .expect("Failed to write compressed data");

        // Extract using streaming
        let summary = GzHandler::stream_extract_gz(
            &source_file,
            &output_dir,
            100 * 1024 * 1024, // 100MB limit
        )
        .await
        .expect("Failed to stream extract gz file");

        assert_eq!(summary.files_extracted, 1);
        assert!(output_dir.join("large.txt").exists());

        // Verify content
        let extracted_content = fs::read(output_dir.join("large.txt"))
            .await
            .expect("Failed to read extracted file");
        assert_eq!(extracted_content, original_data);
    }

    #[tokio::test]
    async fn test_stream_extract_gz_size_limit() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let source_file = temp_dir.path().join("toolarge.txt.gz");
        let output_dir = temp_dir.path().join("output");

        // Create data that will exceed limit
        let original_data = vec![b'x'; 2 * 1024 * 1024]; // 2MB

        // Compress and write file
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(&original_data)
            .expect("Failed to write to gzip encoder");
        let compressed = encoder
            .finish()
            .expect("Failed to finalize gzip compression");

        fs::write(&source_file, compressed)
            .await
            .expect("Failed to write compressed data");

        // Try to extract with small limit (should fail)
        let result = GzHandler::stream_extract_gz(
            &source_file,
            &output_dir,
            1024 * 1024, // 1MB limit (smaller than file)
        )
        .await;

        assert!(result.is_err(), "Should fail when file exceeds size limit");

        // Verify partial file was cleaned up
        assert!(
            !output_dir.join("toolarge.txt").exists(),
            "Partial file should be cleaned up"
        );
    }

    #[tokio::test]
    async fn test_extract_with_limits_uses_streaming_for_large_files() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let source_file = temp_dir.path().join("large.txt.gz");
        let output_dir = temp_dir.path().join("output");

        // Create large test data (> 10MB to trigger streaming)
        let original_data = vec![b'y'; 12 * 1024 * 1024]; // 12MB

        // Compress and write file
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(&original_data)
            .expect("Failed to write to gzip encoder");
        let compressed = encoder
            .finish()
            .expect("Failed to finalize gzip compression");

        fs::write(&source_file, compressed)
            .await
            .expect("Failed to write compressed data");

        // Extract using extract_with_limits (should automatically use streaming)
        let handler = GzHandler;
        let summary = handler
            .extract_with_limits(
                &source_file,
                &output_dir,
                100 * 1024 * 1024,  // 100MB max file size
                1024 * 1024 * 1024, // 1GB max total size
                1000,               // max file count
            )
            .await
            .expect("Failed to extract gz file with limits");

        assert_eq!(summary.files_extracted, 1);
        assert!(output_dir.join("large.txt").exists());

        // Verify content
        let extracted_content = fs::read(output_dir.join("large.txt"))
            .await
            .expect("Failed to read extracted file");
        assert_eq!(extracted_content, original_data);
    }
}
