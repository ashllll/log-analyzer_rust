//! 转码管道模块
//!
//! 实现流式转码管道，将非 UTF-8 编码的文件转码为 UTF-8 临时文件。
//!
//! PRD 2.4 要求：
//! - 遭遇 UTF-16 等导致 SIMD 失效的编码时，立刻中断 Mmap
//! - 退化至流式 UTF-8 临时文件转码管道
//!
//! # 设计
//!
//! 1. 检测源文件编码
//! 2. 创建临时文件
//! 3. 流式读取源文件，转码后写入临时文件
//! 4. 返回临时文件路径供后续处理

use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tempfile::NamedTempFile;
use thiserror::Error;

use super::encoding_detector::{EncodingDetectionResult, EncodingDetector};

/// 转码管道错误类型
#[derive(Error, Debug)]
pub enum TranscodingError {
    /// IO 错误
    #[error("IO错误: {0}")]
    IoError(#[from] std::io::Error),

    /// 编码检测失败
    #[error("编码检测失败: {0}")]
    EncodingDetectionFailed(String),

    /// 转码失败
    #[error("转码失败: {0}")]
    TranscodingFailed(String),

    /// 临时文件创建失败
    #[error("临时文件创建失败: {0}")]
    TempFileCreationFailed(String),

    /// 不支持的编码
    #[error("不支持的编码: {0}")]
    UnsupportedEncoding(String),
}

/// 转码统计信息
#[derive(Debug, Clone, Default)]
pub struct TranscodingStats {
    /// 源文件大小（字节）
    pub source_size: u64,
    /// 转码后大小（字节）
    pub transcoded_size: u64,
    /// 源编码
    pub source_encoding: String,
    /// 是否有 BOM
    pub had_bom: bool,
    /// 处理时间（毫秒）
    pub processing_time_ms: u64,
}

/// 转码管道
///
/// 将非 UTF-8 编码的文件转码为 UTF-8 临时文件。
/// 支持流式处理大文件，避免一次性加载整个文件到内存。
pub struct TranscodingPipe {
    /// 转码后的临时文件
    temp_file: NamedTempFile,
    /// 临时文件路径
    temp_path: PathBuf,
    /// 源编码
    source_encoding: &'static encoding_rs::Encoding,
    /// 转码统计
    stats: TranscodingStats,
}

impl TranscodingPipe {
    /// 缓冲区大小（64KB）
    const BUFFER_SIZE: usize = 64 * 1024;

    /// 创建转码管道
    ///
    /// 检测源文件编码，如果需要转码则创建临时文件并执行转码。
    ///
    /// # 参数
    ///
    /// - `source_path`: 源文件路径
    ///
    /// # 返回值
    ///
    /// 返回转码管道实例，包含转码后的临时文件路径
    ///
    /// # 示例
    ///
    /// ```ignore
    /// use log_analyzer::utils::transcoding_pipe::TranscodingPipe;
    ///
    /// let pipe = TranscodingPipe::create(Path::new("log.txt")).await?;
    /// println!("转码后文件: {:?}", pipe.path());
    /// ```
    pub async fn create(source_path: &Path) -> Result<Self, TranscodingError> {
        let start_time = std::time::Instant::now();

        // 检测编码
        let detection = EncodingDetector::detect_from_file_async(source_path)
            .await
            .map_err(|e| TranscodingError::EncodingDetectionFailed(e.to_string()))?;

        Self::create_with_encoding(source_path, detection, start_time).await
    }

    /// 使用已知编码创建转码管道
    ///
    /// # 参数
    ///
    /// - `source_path`: 源文件路径
    /// - `detection`: 编码检测结果
    /// - `start_time`: 开始时间（用于统计）
    pub async fn create_with_encoding(
        source_path: &Path,
        detection: EncodingDetectionResult,
        start_time: std::time::Instant,
    ) -> Result<Self, TranscodingError> {
        // 获取源文件大小
        let source_metadata = tokio::fs::metadata(source_path)
            .await
            .map_err(TranscodingError::IoError)?;
        let source_size = source_metadata.len();

        // 如果不需要转码，仍然创建临时文件（可能是为了跳过 BOM）
        let needs_transcoding = detection.needs_transcoding || detection.has_bom;

        if !needs_transcoding {
            // 不需要转码，但可能需要跳过 BOM
            return Self::copy_without_bom(source_path, source_size, start_time).await;
        }

        tracing::info!(
            source = %source_path.display(),
            encoding = %detection.encoding_name,
            needs_transcoding = detection.needs_transcoding,
            "开始转码文件"
        );

        // 创建临时文件
        let temp_file = NamedTempFile::new()
            .map_err(|e| TranscodingError::TempFileCreationFailed(e.to_string()))?;
        let temp_path = temp_file.path().to_path_buf();

        // 执行转码
        let (transcoded_size, _stats) = Self::transcode_file(
            source_path,
            &temp_file,
            detection.encoding,
            detection.has_bom,
            source_size,
        )?;

        let elapsed = start_time.elapsed();

        tracing::info!(
            source = %source_path.display(),
            temp = %temp_path.display(),
            source_size = source_size,
            transcoded_size = transcoded_size,
            elapsed_ms = elapsed.as_millis() as u64,
            "转码完成"
        );

        Ok(Self {
            temp_file,
            temp_path,
            source_encoding: detection.encoding,
            stats: TranscodingStats {
                source_size,
                transcoded_size,
                source_encoding: detection.encoding_name,
                had_bom: detection.has_bom,
                processing_time_ms: elapsed.as_millis() as u64,
            },
        })
    }

    /// 复制文件（跳过 BOM）
    ///
    /// 用于 UTF-8 文件但有 BOM 的情况
    async fn copy_without_bom(
        source_path: &Path,
        source_size: u64,
        start_time: std::time::Instant,
    ) -> Result<Self, TranscodingError> {
        // 使用 spawn_blocking 执行同步 IO
        let source_path_owned = source_path.to_path_buf();

        let result = tokio::task::spawn_blocking(move || {
            // 使用同步 API
            let mut source_file = std::fs::File::open(&source_path_owned)?;

            // 读取文件开头检测 BOM
            let mut header = [0u8; 3];
            let header_len = source_file.read(&mut header[..])?;

            // 检查 UTF-8 BOM
            let bom_size = if header_len >= 3 && header[..3] == [0xEF, 0xBB, 0xBF] {
                3 // UTF-8 BOM 大小
            } else {
                0 // 无 BOM
            };

            if bom_size == 0 {
                // 无 BOM，不需要处理
                return Ok((
                    None as Option<(NamedTempFile, PathBuf)>,
                    0u64,
                    source_path_owned,
                ));
            }

            // 创建临时文件
            let mut temp_file = NamedTempFile::new()
                .map_err(|e| TranscodingError::TempFileCreationFailed(e.to_string()))?;
            let temp_path = temp_file.path().to_path_buf();

            // 复制剩余内容（跳过 BOM）
            source_file.seek(std::io::SeekFrom::Start(bom_size as u64))?;

            // 使用 std::io::copy 复制剩余内容
            let copied = std::io::copy(&mut source_file, &mut temp_file)?;

            Ok::<_, TranscodingError>((Some((temp_file, temp_path)), copied, source_path_owned))
        })
        .await
        .map_err(|e| TranscodingError::TranscodingFailed(e.to_string()))??;

        let elapsed = start_time.elapsed();

        match result {
            (None, _, original_path) => {
                // 无 BOM
                let _ = original_path; // 避免未使用警告
                Ok(Self {
                    temp_file: NamedTempFile::new()?,
                    temp_path: PathBuf::new(),
                    source_encoding: encoding_rs::UTF_8,
                    stats: TranscodingStats {
                        source_size,
                        transcoded_size: source_size,
                        source_encoding: "UTF-8".to_string(),
                        had_bom: false,
                        processing_time_ms: elapsed.as_millis() as u64,
                    },
                })
            }
            (Some((temp_file, temp_path)), transcoded_size, original_path) => {
                tracing::info!(
                    source = %original_path.display(),
                    temp = %temp_path.display(),
                    bom_skipped = 3,
                    transcoded_size = transcoded_size,
                    elapsed_ms = elapsed.as_millis() as u64,
                    "BOM 跳过完成"
                );

                Ok(Self {
                    temp_file,
                    temp_path,
                    source_encoding: encoding_rs::UTF_8,
                    stats: TranscodingStats {
                        source_size,
                        transcoded_size,
                        source_encoding: "UTF-8".to_string(),
                        had_bom: true,
                        processing_time_ms: elapsed.as_millis() as u64,
                    },
                })
            }
        }
    }

    /// 执行文件转码
    ///
    /// 使用同步 IO 进行转码（在 tokio::spawn_blocking 中执行）
    fn transcode_file(
        source_path: &Path,
        temp_file: &NamedTempFile,
        encoding: &'static encoding_rs::Encoding,
        has_bom: bool,
        source_size: u64,
    ) -> Result<(u64, TranscodingStats), TranscodingError> {
        use std::fs::File;

        // 打开源文件
        let mut source_file = File::open(source_path)?;

        // 如果有 BOM，跳过它
        if has_bom {
            let bom_size = Self::get_bom_size(encoding);
            if bom_size > 0 {
                source_file.seek(std::io::SeekFrom::Start(bom_size as u64))?;
            }
        }

        // 创建解码器
        let mut decoder = encoding.new_decoder();

        // 创建缓冲区
        let mut input_buffer = vec![0u8; Self::BUFFER_SIZE];
        let _output_buffer = vec![0u16; Self::BUFFER_SIZE * 2]; // UTF-16 中间缓冲区 (保留用于未来扩展)
        let mut utf8_buffer = vec![0u8; Self::BUFFER_SIZE * 4]; // UTF-8 输出缓冲区

        let mut temp_writer = std::io::BufWriter::new(temp_file);
        let mut total_written: u64 = 0;

        loop {
            // 读取源数据
            let bytes_read = source_file.read(&mut input_buffer)?;
            if bytes_read == 0 {
                break;
            }

            // 解码到 UTF-8
            // decode_to_utf8 返回 (CoderResult, read_bytes, written_bytes, had_errors)
            let (_result, _read, written, _had_errors) =
                decoder.decode_to_utf8(&input_buffer[..bytes_read], &mut utf8_buffer, false);

            // 写入临时文件
            temp_writer.write_all(&utf8_buffer[..written])?;
            total_written += written as u64;
        }

        // 刷新最后一字节
        let (_result, _read, written, _had_errors) =
            decoder.decode_to_utf8(&[], &mut utf8_buffer, true);
        if written > 0 {
            temp_writer.write_all(&utf8_buffer[..written])?;
            total_written += written as u64;
        }

        temp_writer.flush()?;

        Ok((
            total_written,
            TranscodingStats {
                source_size,
                transcoded_size: total_written,
                source_encoding: encoding.name().to_string(),
                had_bom: has_bom,
                processing_time_ms: 0, // 由调用者设置
            },
        ))
    }

    /// 获取 BOM 大小
    fn get_bom_size(encoding: &'static encoding_rs::Encoding) -> usize {
        let name = encoding.name();
        match name {
            "UTF-8" => 3,    // EF BB BF
            "UTF-16LE" => 2, // FF FE
            "UTF-16BE" => 2, // FE FF
            "UTF-32LE" => 4, // FF FE 00 00
            "UTF-32BE" => 4, // 00 00 FE FF
            _ => 0,
        }
    }

    /// 获取转码后的临时文件路径
    pub fn path(&self) -> &Path {
        &self.temp_path
    }

    /// 获取源编码
    pub fn source_encoding(&self) -> &'static encoding_rs::Encoding {
        self.source_encoding
    }

    /// 获取转码统计信息
    pub fn stats(&self) -> &TranscodingStats {
        &self.stats
    }

    /// 持久化临时文件到指定路径
    ///
    /// 将临时文件移动到指定位置，使其在 TranscodingPipe drop 后仍然存在
    pub fn persist(self, dest: &Path) -> Result<PathBuf, TranscodingError> {
        self.temp_file
            .persist(dest)
            .map_err(|e| TranscodingError::TempFileCreationFailed(e.to_string()))?;

        Ok(dest.to_path_buf())
    }

    /// 创建共享的转码管道
    pub fn into_arc(self) -> Arc<Self> {
        Arc::new(self)
    }
}

/// 异步创建转码管道
///
/// 在 tokio::spawn_blocking 中执行同步 IO 操作，避免阻塞异步运行时
pub async fn create_transcoding_pipe(
    source_path: &Path,
) -> Result<TranscodingPipe, TranscodingError> {
    let source_path = source_path.to_path_buf();

    tokio::task::spawn_blocking(move || {
        // 使用同步 API 检测编码
        let detection = EncodingDetector::detect_from_file(&source_path)
            .map_err(|e| TranscodingError::EncodingDetectionFailed(e.to_string()))?;

        // 创建运行时执行异步部分
        let start_time = std::time::Instant::now();

        // 获取源文件大小
        let source_metadata = std::fs::metadata(&source_path)?;
        let source_size = source_metadata.len();

        // 如果不需要转码
        if !detection.needs_transcoding && !detection.has_bom {
            let elapsed = start_time.elapsed();
            return Ok(TranscodingPipe {
                temp_file: NamedTempFile::new()?,
                temp_path: PathBuf::new(),
                source_encoding: encoding_rs::UTF_8,
                stats: TranscodingStats {
                    source_size,
                    transcoded_size: source_size,
                    source_encoding: "UTF-8".to_string(),
                    had_bom: false,
                    processing_time_ms: elapsed.as_millis() as u64,
                },
            });
        }

        // 创建临时文件
        let temp_file = NamedTempFile::new()
            .map_err(|e| TranscodingError::TempFileCreationFailed(e.to_string()))?;
        let temp_path = temp_file.path().to_path_buf();

        // 执行转码
        let (transcoded_size, _) = TranscodingPipe::transcode_file(
            &source_path,
            &temp_file,
            detection.encoding,
            detection.has_bom,
            source_size,
        )?;

        let elapsed = start_time.elapsed();

        Ok(TranscodingPipe {
            temp_file,
            temp_path,
            source_encoding: detection.encoding,
            stats: TranscodingStats {
                source_size,
                transcoded_size,
                source_encoding: detection.encoding_name,
                had_bom: detection.has_bom,
                processing_time_ms: elapsed.as_millis() as u64,
            },
        })
    })
    .await
    .map_err(|e| TranscodingError::TranscodingFailed(e.to_string()))?
}

/// 检查文件是否需要转码
///
/// 快速检查文件是否需要转码，不执行实际转码操作
pub async fn needs_transcoding(path: &Path) -> std::io::Result<bool> {
    let detection = EncodingDetector::detect_from_file_async(path).await?;
    Ok(detection.needs_transcoding || detection.has_bom)
}

/// 检查文件是否会破坏 SIMD 优化
///
/// 用于判断是否需要中断 Mmap 并退化到流式处理
pub async fn breaks_simd(path: &Path) -> std::io::Result<bool> {
    let detection = EncodingDetector::detect_from_file_async(path).await?;
    Ok(detection.breaks_simd)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_file_utf8() -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "Hello, World!你好，世界！").unwrap();
        file.flush().unwrap();
        file
    }

    fn create_test_file_utf8_with_bom() -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(&[0xEF, 0xBB, 0xBF]).unwrap(); // UTF-8 BOM
        writeln!(file, "Hello with BOM!").unwrap();
        file.flush().unwrap();
        file
    }

    fn create_test_file_utf16_le() -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(&[0xFF, 0xFE]).unwrap(); // UTF-16 LE BOM
                                                // 'H' 'i' in UTF-16 LE
        file.write_all(&[0x48, 0x00, 0x69, 0x00]).unwrap();
        file.flush().unwrap();
        file
    }

    fn create_test_file_gbk() -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        // GBK 编码的 "你好" = C4 E3 BA C3
        file.write_all(&[0xC4, 0xE3, 0xBA, 0xC3]).unwrap();
        file.flush().unwrap();
        file
    }

    #[tokio::test]
    async fn test_needs_transcoding_utf8() {
        let file = create_test_file_utf8();
        let result = needs_transcoding(file.path()).await.unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn test_needs_transcoding_utf8_with_bom() {
        let file = create_test_file_utf8_with_bom();
        let result = needs_transcoding(file.path()).await.unwrap();
        assert!(result); // 有 BOM 需要处理（跳过 BOM）
    }

    #[tokio::test]
    async fn test_needs_transcoding_utf16() {
        let file = create_test_file_utf16_le();
        let result = needs_transcoding(file.path()).await.unwrap();
        assert!(result);
    }

    #[tokio::test]
    async fn test_needs_transcoding_gbk() {
        let file = create_test_file_gbk();
        let result = needs_transcoding(file.path()).await.unwrap();
        assert!(result);
    }

    #[tokio::test]
    async fn test_breaks_simd_utf8() {
        let file = create_test_file_utf8();
        let result = breaks_simd(file.path()).await.unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn test_breaks_simd_utf16() {
        let file = create_test_file_utf16_le();
        let result = breaks_simd(file.path()).await.unwrap();
        assert!(result);
    }

    #[test]
    fn test_transcoding_stats_default() {
        let stats = TranscodingStats::default();
        assert_eq!(stats.source_size, 0);
        assert_eq!(stats.transcoded_size, 0);
        assert_eq!(stats.processing_time_ms, 0);
    }

    #[test]
    fn test_get_bom_size() {
        assert_eq!(TranscodingPipe::get_bom_size(encoding_rs::UTF_8), 3);
        assert_eq!(TranscodingPipe::get_bom_size(encoding_rs::UTF_16LE), 2);
        assert_eq!(TranscodingPipe::get_bom_size(encoding_rs::UTF_16BE), 2);
        // UTF-32 可能不存在于某些版本的 encoding_rs，测试 UTF-16 和 GBK
        assert_eq!(TranscodingPipe::get_bom_size(encoding_rs::GBK), 0);
    }
}
