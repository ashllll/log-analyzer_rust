use crate::archive::archive_handler::{ArchiveEntry, ArchiveHandler, ExtractionSummary};
use crate::archive::archive_handler_base::{ArchiveHandlerBase, ExtractionContext};
use crate::archive::extraction_error::{ExtractionError, ExtractionResult};
use crate::error::{AppError, Result};
use async_compression::tokio::bufread::GzipDecoder;
use async_trait::async_trait;
use flate2::read::GzDecoder as SyncGzDecoder;
use std::io::Read;
use std::path::Path;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tracing::{debug, trace};

/**
 * GZ文件处理器 (重构版本 - 使用 ArchiveHandlerBase)
 *
 * 处理单个gzip压缩文件，解压为单个文件
 */
pub struct GzHandler {}

#[async_trait]
impl ArchiveHandlerBase for GzHandler {
    fn handler_name(&self) -> &'static str {
        "GzHandler"
    }

    fn supported_formats(&self) -> &[&'static str] {
        &["gz"]
    }

    async fn extract_with_context(
        &self,
        source: &Path,
        target_dir: &Path,
        context: &mut ExtractionContext,
    ) -> ExtractionResult<ExtractionSummary> {
        debug!("开始提取 GZ 文件: {:?}", source);

        // 创建目标目录
        fs::create_dir_all(target_dir).await.map_err(|e| {
            ExtractionError::DirectoryCreationFailed {
                path: target_dir.to_path_buf(),
                reason: e.to_string(),
            }
        })?;

        // 确定输出文件名（去掉.gz扩展名）
        let output_name = source
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output");

        let output_path = target_dir.join(output_name);

        // 使用流式解压
        const BUFFER_SIZE: usize = 64 * 1024; // 64KB buffer
        let file = fs::File::open(source)
            .await
            .map_err(|e| ExtractionError::IoError {
                operation: "打开GZ文件".to_string(),
                reason: e.to_string(),
            })?;

        let reader = BufReader::with_capacity(BUFFER_SIZE, file);
        let mut decoder = GzipDecoder::new(reader);
        let output_file =
            fs::File::create(&output_path)
                .await
                .map_err(|e| ExtractionError::IoError {
                    operation: "创建输出文件".to_string(),
                    reason: e.to_string(),
                })?;

        let mut writer = BufWriter::with_capacity(BUFFER_SIZE, output_file);
        let mut buffer = vec![0u8; BUFFER_SIZE];
        let mut total_bytes = 0u64;

        loop {
            let bytes_read =
                decoder
                    .read(&mut buffer)
                    .await
                    .map_err(|e| ExtractionError::IoError {
                        operation: "解压GZ数据".to_string(),
                        reason: e.to_string(),
                    })?;

            if bytes_read == 0 {
                break;
            }

            total_bytes += bytes_read as u64;

            writer.write_all(&buffer[..bytes_read]).await.map_err(|e| {
                ExtractionError::IoError {
                    operation: "写入解压数据".to_string(),
                    reason: e.to_string(),
                }
            })?;
        }

        writer.flush().await.map_err(|e| ExtractionError::IoError {
            operation: "刷新输出文件".to_string(),
            reason: e.to_string(),
        })?;

        let mut summary = ExtractionSummary::new();
        summary.add_file(output_path.clone(), total_bytes);

        // 更新上下文
        context.record_extraction(&output_path, total_bytes);

        trace!("已提取 GZ 文件: {:?}, 大小: {}", output_path, total_bytes);

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
        vec!["gz"]
    }

    async fn list_contents(&self, path: &Path) -> Result<Vec<ArchiveEntry>> {
        let path_owned = path.to_path_buf();
        let entries = tokio::task::spawn_blocking(move || {
            let file = std::fs::File::open(&path_owned)?;
            let mut decoder = SyncGzDecoder::new(file);
            let mut decompressed = Vec::new();
            decoder.read_to_end(&mut decompressed)?;

            // GZ 文件只包含一个文件，文件名是去掉 .gz 扩展名
            let name = path_owned
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".to_string());

            let size = decompressed.len() as u64;

            Ok::<Vec<ArchiveEntry>, AppError>(vec![ArchiveEntry {
                name: name.clone(),
                path: name,
                is_dir: false,
                size,
                compressed_size: path_owned.metadata()?.len(),
            }])
        })
        .await
        .map_err(|e| AppError::archive_error(e.to_string(), None))??;

        Ok(entries)
    }

    async fn read_file(&self, path: &Path, _file_name: &str) -> Result<String> {
        let path_owned = path.to_path_buf();

        tokio::task::spawn_blocking(move || {
            let file = std::fs::File::open(&path_owned)?;
            let mut decoder = SyncGzDecoder::new(file);

            // 大小限制：10MB
            const MAX_SIZE: u64 = 10 * 1024 * 1024;

            let mut decompressed = Vec::new();
            decoder.read_to_end(&mut decompressed)?;
            let size = decompressed.len() as u64;

            if size > MAX_SIZE {
                let truncated: String =
                    String::from_utf8_lossy(&decompressed[..MAX_SIZE as usize]).to_string();
                Ok(format!(
                    "{}\n\n[文件过大，已截断显示. 完整大小: {} bytes]",
                    truncated, size
                ))
            } else {
                String::from_utf8(decompressed)
                    .map_err(|e| AppError::archive_error(format!("Invalid UTF-8: {}", e), None))
            }
        })
        .await
        .map_err(|e| AppError::archive_error(e.to_string(), None))?
    }
}

/**
 * 判断是否为tar.gz文件
 */
fn is_tar_gz(path: &Path) -> bool {
    if let Some(stem) = path.file_stem() {
        if let Some(stem_str) = stem.to_str() {
            return stem_str.to_lowercase().ends_with(".tar");
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use tempfile::TempDir;

    #[test]
    fn test_gz_handler_base_impl() {
        let handler = GzHandler {};
        assert_eq!(handler.handler_name(), "GzHandler");
        assert_eq!(handler.supported_formats(), &["gz"]);
    }

    #[test]
    fn test_gz_handler_can_handle() {
        let handler = GzHandler {};
        assert!(handler.can_handle(Path::new("test.gz")));
        assert!(!handler.can_handle(Path::new("test.tar.gz")));
        assert!(!handler.can_handle(Path::new("test.zip")));
        assert!(!handler.can_handle(Path::new("test.txt")));
    }

    #[test]
    fn test_gz_handler_file_extensions() {
        let handler = GzHandler {};
        let extensions = handler.file_extensions();
        assert_eq!(extensions, vec!["gz"]);
    }

    #[test]
    fn test_is_tar_gz() {
        assert!(is_tar_gz(Path::new("test.tar.gz")));
        assert!(is_tar_gz(Path::new("archive.tar.GZ")));
        assert!(!is_tar_gz(Path::new("test.gz")));
        assert!(!is_tar_gz(Path::new("document.txt")));
    }
}
