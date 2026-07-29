use crate::archive_handler::{ArchiveHandler, ExtractionSummary};
use async_compression::tokio::bufread::GzipDecoder;
use async_trait::async_trait;
use la_core::error::{AppError, Result};
use la_core::ExtractionPolicy;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};

/**
 * GZ文件处理器
 *
 * 处理单个gzip压缩文件，解压为单个文件
 *
 * 所有输入统一使用流式解压，在写入过程中执行输出资源限制，
 * 避免按压缩文件大小分流时让高膨胀输入在校验前占满内存。
 *
 * ============================================================================
 * Phase 4: 并行解压技术评估报告
 * ============================================================================
 *
 * ## 评估方案对比
 *
 * ### 方案 1: 使用 rayon 并行处理多个 gzip 块
 * **可行性**: ⭐⭐⭐ (中等)
 * **复杂度**: 高
 * **适用场景**: 超大文件 (1GB+) 且需要极致解压速度
 *
 * **技术限制**:
 * - gzip 格式本身不是并行友好的，压缩数据是流式依赖的
 * - 需要将文件分割成独立块，每个块需要独立的压缩字典
 * - 实现复杂度极高，需要修改压缩库底层
 *
 * **性能预期**:
 * - 理论上可达到 2-4x 加速（取决于 CPU 核心数）
 * - 实际收益可能受 I/O 瓶颈限制
 *
 * ### 方案 2: 使用 `parallel-gzip` (pigz 的 Rust 移植)
 * **可行性**: ⭐⭐⭐⭐ (较高)
 * **复杂度**: 中
 * **适用场景**: 批量处理多个 gzip 文件
 *
 * **技术实现**:
 * ```ignore
 * use rayon::prelude::*;
 *
 * // 并行处理多个 gzip 文件
 * pub fn parallel_extract_files(files: &[PathBuf], output_dir: &Path) -> Vec<Result<()>> {
 *     files.par_iter()
 *         .map(|file| extract_single_file(file, output_dir))
 *         .collect()
 * }
 * ```
 *
 * **优点**:
 * - 实现简单，直接利用 rayon 的并行迭代器
 * - 适合日志分析场景（通常需要处理多个日志文件）
 *
 * ### 方案 3: 使用 `async-compression` + `tokio::task::spawn_blocking`
 * **可行性**: ⭐⭐⭐⭐⭐ (推荐)
 * **复杂度**: 低
 * **适用场景**: 当前架构的最佳选择
 *
 * **当前实现**: 已在使用 `async-compression` 进行流式解压
 * **优化建议**: 对多个文件使用并行流处理
 *
 * ## 适用场景总结
 *
 * | 场景 | 推荐方案 | 原因 |
 * |------|---------|------|
 * | 单个大文件 (>1GB) | 流式处理 (当前) | gzip 格式限制，并行收益有限 |
 * | 多个文件批量处理 | rayon 并行 | 文件间无依赖，并行收益高 |
 * | 实时解压需求 | async + 流式 | 内存友好，响应及时 |
 *
 * ## 建议实现
 *
 * 对于日志分析器的实际使用场景，建议在更高层（工作区导入）使用并行处理：
 * - 多个 gzip 文件并行解压
 * - 与索引构建并行化
 * - 使用 tokio 的 `spawn_blocking` 避免阻塞异步运行时
 *
 * 参见: `src/services/concurrent_import.rs` (如需要可实现)
 * ============================================================================
 */
pub struct GzHandler;

#[derive(Clone, Copy)]
struct GzExtractionLimits {
    max_file_size: u64,
    max_total_size: u64,
    max_file_count: usize,
    buffer_size: usize,
}

impl GzExtractionLimits {
    const DEFAULT_BUFFER_SIZE: usize = 1024 * 1024;

    fn from_policy(policy: &ExtractionPolicy) -> Self {
        Self {
            max_file_size: policy.max_file_size,
            max_total_size: policy.max_total_size,
            max_file_count: policy.max_file_count,
            buffer_size: policy.buffer_size,
        }
    }
}

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
        Self::stream_extract_gz_with_limits(
            source,
            target_dir,
            GzExtractionLimits {
                max_file_size,
                max_total_size: max_file_size,
                max_file_count: 1,
                buffer_size: GzExtractionLimits::DEFAULT_BUFFER_SIZE,
            },
        )
        .await
    }

    async fn stream_extract_gz_with_limits(
        source: &Path,
        target_dir: &Path,
        limits: GzExtractionLimits,
    ) -> Result<ExtractionSummary> {
        if limits.max_file_count < 1 {
            return Err(AppError::archive_error(
                format!(
                    "Extraction would exceed file count limit of {} files",
                    limits.max_file_count
                ),
                Some(source.to_path_buf()),
            ));
        }

        fs::create_dir_all(target_dir).await.map_err(|e| {
            AppError::archive_error(
                format!("Failed to create target directory: {e}"),
                Some(target_dir.to_path_buf()),
            )
        })?;

        let file = fs::File::open(source).await.map_err(|e| {
            AppError::archive_error(
                format!("Failed to open GZ file: {e}"),
                Some(source.to_path_buf()),
            )
        })?;

        let reader = BufReader::with_capacity(limits.buffer_size, file);
        let mut decoder = GzipDecoder::new(reader);

        let output_name = source
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("output");
        let output_name = if output_name.eq_ignore_ascii_case("gz") {
            "output"
        } else if let Some(stem) = output_name.strip_suffix(".gz") {
            stem
        } else if let Some(stem) = output_name.strip_suffix(".GZ") {
            stem
        } else {
            output_name
        };

        let output_path = target_dir.join(output_name);
        Self::publish_decoded_stream(&mut decoder, source, target_dir, output_path, limits).await
    }

    async fn publish_decoded_stream<R>(
        decoder: &mut R,
        source: &Path,
        target_dir: &Path,
        output_path: PathBuf,
        limits: GzExtractionLimits,
    ) -> Result<ExtractionSummary>
    where
        R: AsyncRead + Unpin,
    {
        let existing_permissions = match fs::metadata(&output_path).await {
            Ok(metadata) => Some(metadata.permissions()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => {
                return Err(AppError::archive_error(
                    format!("Failed to inspect existing output file: {error}"),
                    Some(output_path),
                ));
            }
        };

        let mut temporary_file_builder = tempfile::Builder::new();
        temporary_file_builder.prefix(".gz-extract-");
        #[cfg(unix)]
        if existing_permissions.is_none() {
            use std::os::unix::fs::PermissionsExt;
            temporary_file_builder.permissions(std::fs::Permissions::from_mode(0o666));
        }
        let temporary_file = temporary_file_builder
            .tempfile_in(target_dir)
            .map_err(|e| {
                AppError::archive_error(
                    format!("Failed to create temporary output file: {e}"),
                    Some(target_dir.to_path_buf()),
                )
            })?;
        let (output_file, temporary_path) = temporary_file.into_parts();
        let output_file = fs::File::from_std(output_file);

        let mut writer = BufWriter::with_capacity(limits.buffer_size, output_file);
        let mut buffer = vec![0u8; limits.buffer_size];
        let mut total_bytes = 0u64;

        loop {
            let bytes_read = decoder.read(&mut buffer).await.map_err(|e| {
                AppError::archive_error(
                    format!("Failed to decompress gzip stream: {e}"),
                    Some(source.to_path_buf()),
                )
            })?;

            if bytes_read == 0 {
                break; // EOF
            }

            total_bytes = total_bytes.saturating_add(bytes_read as u64);
            if total_bytes > limits.max_file_size {
                return Err(AppError::archive_error(
                    format!(
                        "File {} exceeds maximum size limit of {} bytes (got {} bytes)",
                        source.display(),
                        limits.max_file_size,
                        total_bytes
                    ),
                    Some(source.to_path_buf()),
                ));
            }

            if total_bytes > limits.max_total_size {
                return Err(AppError::archive_error(
                    format!(
                        "Extraction would exceed total size limit of {} bytes \
                         (got {total_bytes} bytes)",
                        limits.max_total_size
                    ),
                    Some(source.to_path_buf()),
                ));
            }

            writer.write_all(&buffer[..bytes_read]).await.map_err(|e| {
                AppError::archive_error(
                    format!("Failed to write decompressed data: {e}"),
                    Some(output_path.clone()),
                )
            })?;
        }

        writer.flush().await.map_err(|e| {
            AppError::archive_error(
                format!("Failed to flush output file: {e}"),
                Some(output_path.clone()),
            )
        })?;
        drop(writer);
        if let Some(permissions) = existing_permissions {
            let temporary_file_path: &Path = temporary_path.as_ref();
            fs::set_permissions(temporary_file_path, permissions)
                .await
                .map_err(|error| {
                    AppError::archive_error(
                        format!("Failed to preserve output permissions: {error}"),
                        Some(output_path.clone()),
                    )
                })?;
        }
        temporary_path.persist(&output_path).map_err(|error| {
            AppError::archive_error(
                format!("Failed to publish extracted output: {error}"),
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
        Self::stream_extract_gz_with_limits(
            source,
            target_dir,
            GzExtractionLimits {
                max_file_size,
                max_total_size,
                max_file_count,
                buffer_size: GzExtractionLimits::DEFAULT_BUFFER_SIZE,
            },
        )
        .await
    }

    async fn extract_with_policy(
        &self,
        source: &Path,
        target_dir: &Path,
        policy: &ExtractionPolicy,
    ) -> Result<ExtractionSummary> {
        policy.validate()?;
        Self::stream_extract_gz_with_limits(
            source,
            target_dir,
            GzExtractionLimits::from_policy(policy),
        )
        .await
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use la_core::ExtractionPolicy;
    use std::io::Write;
    use std::pin::Pin;
    use std::task::{Context, Poll};
    use tempfile::TempDir;

    #[test]
    fn extraction_policy_controls_streaming_buffer_size() {
        let policy = ExtractionPolicy {
            buffer_size: 4096,
            ..ExtractionPolicy::default()
        };

        assert_eq!(GzExtractionLimits::from_policy(&policy).buffer_size, 4096);
    }
    use tokio::io::ReadBuf;
    use tokio::sync::oneshot;

    struct PendingAfterChunk {
        chunk: Option<Vec<u8>>,
        pending: Option<oneshot::Sender<()>>,
    }

    impl AsyncRead for PendingAfterChunk {
        fn poll_read(
            mut self: Pin<&mut Self>,
            _context: &mut Context<'_>,
            buffer: &mut ReadBuf<'_>,
        ) -> Poll<std::io::Result<()>> {
            if let Some(chunk) = self.chunk.take() {
                buffer.put_slice(&chunk);
                return Poll::Ready(Ok(()));
            }

            if let Some(pending) = self.pending.take() {
                let _ = pending.send(());
            }
            Poll::Pending
        }
    }

    async fn write_low_compression_gzip_fixture(source_file: &Path) {
        let original_data = vec![b'x'; 2 * 1024 * 1024];
        let mut encoder = GzEncoder::new(Vec::new(), Compression::none());
        encoder
            .write_all(&original_data)
            .expect("Failed to write to gzip encoder");
        let compressed = encoder
            .finish()
            .expect("Failed to finalize gzip compression");
        assert!(
            compressed.len() > 1024 * 1024,
            "fixture must span multiple decoder buffers"
        );
        fs::write(source_file, compressed)
            .await
            .expect("Failed to write compressed data");
    }

    async fn write_gzip_fixture(source_file: &Path, content: &[u8]) {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(content)
            .expect("Failed to write to gzip encoder");
        fs::write(
            source_file,
            encoder
                .finish()
                .expect("Failed to finalize gzip compression"),
        )
        .await
        .expect("Failed to write compressed data");
    }

    async fn assert_no_temporary_outputs(output_dir: &Path) {
        let mut entries = fs::read_dir(output_dir)
            .await
            .expect("Failed to read output directory");
        while let Some(entry) = entries.next_entry().await.expect("Failed to read entry") {
            let name = entry.file_name();
            assert!(
                !name.to_string_lossy().starts_with(".gz-extract-"),
                "temporary extraction output must be cleaned up"
            );
        }
    }

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

        // Highly compressible content must still be streamed and bounded by output limits.
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

        // A small compressed input with large output must use the bounded streaming path.
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

    #[tokio::test]
    async fn streaming_path_enforces_total_size_limit() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let source_file = temp_dir.path().join("large.txt.gz");
        let output_dir = temp_dir.path().join("output");
        write_low_compression_gzip_fixture(&source_file).await;

        let result = GzHandler
            .extract_with_limits(&source_file, &output_dir, 4 * 1024 * 1024, 1024 * 1024, 1)
            .await;

        assert!(result.is_err(), "streaming must enforce max_total_size");
        assert!(
            !output_dir.join("large.txt").exists(),
            "a rejected extraction must not leave a partial file"
        );
    }

    #[tokio::test]
    async fn streaming_path_enforces_file_count_limit() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let source_file = temp_dir.path().join("large.txt.gz");
        let output_dir = temp_dir.path().join("output");
        write_low_compression_gzip_fixture(&source_file).await;

        let result = GzHandler
            .extract_with_limits(
                &source_file,
                &output_dir,
                4 * 1024 * 1024,
                4 * 1024 * 1024,
                0,
            )
            .await;

        assert!(result.is_err(), "streaming must enforce max_file_count");
        assert!(
            !output_dir.join("large.txt").exists(),
            "a rejected extraction must not leave a partial file"
        );
    }

    #[tokio::test]
    async fn corrupted_stream_does_not_leave_partial_output() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let source_file = temp_dir.path().join("corrupted.txt.gz");
        let output_dir = temp_dir.path().join("output");
        let original_data = vec![b'x'; 2 * 1024 * 1024];
        let mut encoder = GzEncoder::new(Vec::new(), Compression::none());
        encoder
            .write_all(&original_data)
            .expect("Failed to write to gzip encoder");
        let mut compressed = encoder
            .finish()
            .expect("Failed to finalize gzip compression");
        compressed.truncate(compressed.len() / 2);
        fs::write(&source_file, compressed)
            .await
            .expect("Failed to write corrupted gzip data");

        let result = GzHandler
            .extract_with_limits(
                &source_file,
                &output_dir,
                4 * 1024 * 1024,
                4 * 1024 * 1024,
                1,
            )
            .await;

        assert!(result.is_err(), "a truncated gzip stream must be rejected");
        assert!(
            !output_dir.join("corrupted.txt").exists(),
            "a rejected stream must not leave a partial output"
        );
        assert_no_temporary_outputs(&output_dir).await;
    }

    #[tokio::test]
    async fn corrupted_stream_preserves_existing_output() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let source_file = temp_dir.path().join("existing.txt.gz");
        let output_dir = temp_dir.path().join("output");
        fs::create_dir_all(&output_dir)
            .await
            .expect("Failed to create output directory");
        let output_file = output_dir.join("existing.txt");
        fs::write(&output_file, b"previous complete content")
            .await
            .expect("Failed to seed existing output");

        let original_data = vec![b'x'; 2 * 1024 * 1024];
        let mut encoder = GzEncoder::new(Vec::new(), Compression::none());
        encoder
            .write_all(&original_data)
            .expect("Failed to write to gzip encoder");
        let mut compressed = encoder
            .finish()
            .expect("Failed to finalize gzip compression");
        compressed.truncate(compressed.len() / 2);
        fs::write(&source_file, compressed)
            .await
            .expect("Failed to write corrupted gzip data");

        let result = GzHandler
            .extract_with_limits(
                &source_file,
                &output_dir,
                4 * 1024 * 1024,
                4 * 1024 * 1024,
                1,
            )
            .await;

        assert!(result.is_err(), "a truncated gzip stream must be rejected");
        assert_eq!(
            fs::read(&output_file)
                .await
                .expect("existing output must remain readable"),
            b"previous complete content"
        );
        assert_no_temporary_outputs(&output_dir).await;
    }

    #[tokio::test]
    async fn size_limit_preserves_existing_output_and_cleans_temporary_file() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let source_file = temp_dir.path().join("existing.txt.gz");
        let output_dir = temp_dir.path().join("output");
        fs::create_dir_all(&output_dir)
            .await
            .expect("Failed to create output directory");
        let output_file = output_dir.join("existing.txt");
        fs::write(&output_file, b"previous complete content")
            .await
            .expect("Failed to seed existing output");
        write_low_compression_gzip_fixture(&source_file).await;

        let result = GzHandler
            .extract_with_limits(&source_file, &output_dir, 4 * 1024 * 1024, 1024 * 1024, 1)
            .await;

        assert!(result.is_err(), "the total size limit must reject output");
        assert_eq!(
            fs::read(&output_file)
                .await
                .expect("existing output must remain readable"),
            b"previous complete content"
        );
        assert_no_temporary_outputs(&output_dir).await;
    }

    #[tokio::test]
    async fn cancellation_preserves_existing_output_and_cleans_temporary_file() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let source_file = temp_dir.path().join("existing.txt.gz");
        let output_dir = temp_dir.path().join("output");
        fs::create_dir_all(&output_dir)
            .await
            .expect("Failed to create output directory");
        let output_file = output_dir.join("existing.txt");
        fs::write(&output_file, b"previous complete content")
            .await
            .expect("Failed to seed existing output");
        let (pending_tx, pending_rx) = oneshot::channel();
        let mut reader = PendingAfterChunk {
            chunk: Some(vec![b'x'; 128]),
            pending: Some(pending_tx),
        };
        let task_output_dir = output_dir.clone();
        let task_output_file = output_file.clone();

        let extraction = tokio::spawn(async move {
            GzHandler::publish_decoded_stream(
                &mut reader,
                &source_file,
                &task_output_dir,
                task_output_file,
                GzExtractionLimits {
                    max_file_size: 1024,
                    max_total_size: 1024,
                    max_file_count: 1,
                    buffer_size: GzExtractionLimits::DEFAULT_BUFFER_SIZE,
                },
            )
            .await
        });
        pending_rx
            .await
            .expect("reader must reach its pending state");
        extraction.abort();
        let join_error = extraction
            .await
            .expect_err("cancelled extraction must not complete");
        assert!(join_error.is_cancelled());

        assert_eq!(
            fs::read(&output_file)
                .await
                .expect("existing output must remain readable"),
            b"previous complete content"
        );
        assert_no_temporary_outputs(&output_dir).await;
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn new_output_uses_regular_file_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let source_file = temp_dir.path().join("new.txt.gz");
        let output_dir = temp_dir.path().join("output");
        fs::create_dir_all(&output_dir)
            .await
            .expect("Failed to create output directory");
        let control_file = output_dir.join("control.txt");
        fs::write(&control_file, b"control")
            .await
            .expect("Failed to create control file");
        write_gzip_fixture(&source_file, b"new content").await;

        GzHandler
            .extract(&source_file, &output_dir)
            .await
            .expect("Failed to extract gzip");

        let expected_mode = fs::metadata(control_file)
            .await
            .expect("Failed to inspect control file")
            .permissions()
            .mode()
            & 0o777;
        let actual_mode = fs::metadata(output_dir.join("new.txt"))
            .await
            .expect("Failed to inspect extracted file")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(actual_mode, expected_mode);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn replacing_output_preserves_existing_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let source_file = temp_dir.path().join("existing.txt.gz");
        let output_dir = temp_dir.path().join("output");
        fs::create_dir_all(&output_dir)
            .await
            .expect("Failed to create output directory");
        let output_file = output_dir.join("existing.txt");
        fs::write(&output_file, b"old content")
            .await
            .expect("Failed to create existing output");
        fs::set_permissions(&output_file, std::fs::Permissions::from_mode(0o640))
            .await
            .expect("Failed to set existing permissions");
        write_gzip_fixture(&source_file, b"replacement content").await;

        GzHandler
            .extract(&source_file, &output_dir)
            .await
            .expect("Failed to replace gzip output");

        let metadata = fs::metadata(&output_file)
            .await
            .expect("Failed to inspect replaced output");
        assert_eq!(metadata.permissions().mode() & 0o777, 0o640);
        assert_eq!(
            fs::read(output_file)
                .await
                .expect("Failed to read replaced output"),
            b"replacement content"
        );
    }

    #[tokio::test]
    async fn test_gz_zipslip_prevention() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        // Simulate a malicious path like ../../etc/passwd.gz inside the temp dir
        let malicious_dir = temp_dir
            .path()
            .join("malicious")
            .join("..")
            .join("..")
            .join("etc");
        fs::create_dir_all(&malicious_dir)
            .await
            .expect("Failed to create malicious dir");
        let source_file = malicious_dir.join("passwd.gz");
        let output_dir = temp_dir.path().join("output");

        let original_data = b"secret data";

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

        let handler = GzHandler;
        let summary = handler
            .extract(&source_file, &output_dir)
            .await
            .expect("Failed to extract gz file");

        assert_eq!(summary.files_extracted, 1);
        // The extracted file should be named "passwd" (basename only), NOT "etc/passwd"
        let expected_file = output_dir.join("passwd");
        assert!(
            expected_file.exists(),
            "Extracted file should be at {expected_file:?}, not inside a traversal path"
        );
        // Ensure no path traversal occurred
        assert!(
            !temp_dir.path().join("etc").exists(),
            "Path traversal should not create 'etc' directory outside output_dir"
        );

        let extracted_content = fs::read(&expected_file)
            .await
            .expect("Failed to read extracted file");
        assert_eq!(extracted_content, original_data);
    }
}
