use async_trait::async_trait;
use std::path::{Path, PathBuf};
use crate::archive::archive_handler::{ArchiveHandler, ExtractionSummary};
use crate::error::{AppError, Result};
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::io::Cursor;

/**
 * GZ文件处理器
 * 
 * 处理单个gzip压缩文件，解压为单个文件
 */
pub struct GzHandler;

#[async_trait]
impl ArchiveHandler for GzHandler {
    fn can_handle(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|s| s.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("gz") && !is_tar_gz(path))
            .unwrap_or(false)
    }
    
    async fn extract(
        &self,
        source: &Path,
        target_dir: &Path,
    ) -> Result<ExtractionSummary> {
        // 确保目标目录存在
        fs::create_dir_all(target_dir)
            .await
            .map_err(|e| AppError::archive_error(
                format!("Failed to create target directory: {}", e),
                Some(target_dir.to_path_buf())
            ))?;
        
        // 读取压缩文件
        let compressed_data = fs::read(source)
            .await
            .map_err(|e| AppError::archive_error(
                format!("Failed to read GZ file: {}", e),
                Some(source.to_path_buf())
            ))?;
        
        // 解压数据
        let decompressed_data = decompress_gzip(&compressed_data)?;
        
        // 确定输出文件名（去掉.gz扩展名）
        let output_name = source.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output");
        
        let output_path = target_dir.join(output_name);
        
        // 写入解压后的文件
        fs::write(&output_path, decompressed_data)
            .await
            .map_err(|e| AppError::archive_error(
                format!("Failed to write decompressed file: {}", e),
                Some(output_path.clone())
            ))?;
        
        let mut summary = ExtractionSummary::new();
        summary.add_file(output_path, decompressed_data.len() as u64);
        
        Ok(summary)
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
    
    decoder.read_to_end(&mut decompressed)
        .map_err(|e| AppError::archive_error(
            format!("Failed to decompress gzip: {}", e),
            None
        ))?;
    
    Ok(decompressed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;

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
        encoder.write_all(original_data).unwrap();
        let compressed = encoder.finish().unwrap();
        
        // 解压数据
        let decompressed = decompress_gzip(&compressed).unwrap();
        
        assert_eq!(decompressed, original_data);
    }

    #[tokio::test]
    async fn test_extract_gz_file() {
        let temp_dir = TempDir::new().unwrap();
        let source_file = temp_dir.path().join("test.txt.gz");
        let output_dir = temp_dir.path().join("output");
        
        // 创建测试数据
        let original_data = b"This is test content for gzip file.";
        
        // 压缩并写入文件
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(original_data).unwrap();
        let compressed = encoder.finish().unwrap();
        
        fs::write(&source_file, compressed).await.unwrap();
        
        // 提取文件
        let handler = GzHandler;
        let summary = handler.extract(&source_file, &output_dir).await.unwrap();
        
        assert_eq!(summary.files_extracted, 1);
        assert!(output_dir.join("test.txt").exists());
        
        // 验证内容
        let extracted_content = fs::read(output_dir.join("test.txt")).await.unwrap();
        assert_eq!(extracted_content, original_data);
    }
}