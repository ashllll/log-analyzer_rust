/**
 * 压缩文件处理模块
 *
 * 提供统一的接口处理各种压缩格式（ZIP、RAR、TAR、GZ）
 */
pub mod archive_handler;
pub mod gz_handler;
pub mod processor;
pub mod rar_handler;
pub mod tar_handler;
pub mod zip_handler;

pub use archive_handler::{ArchiveHandler, ExtractionSummary};
pub use gz_handler::GzHandler;
pub use processor::process_path_recursive_with_metadata;
pub use rar_handler::RarHandler;
pub use tar_handler::TarHandler;
pub use zip_handler::ZipHandler;

use crate::error::Result;
use std::path::Path;

/**
 * 压缩处理器管理器
 *
 * 管理所有支持的压缩格式处理器
 */
pub struct ArchiveManager {
    handlers: Vec<Box<dyn ArchiveHandler>>,
}

impl ArchiveManager {
    /**
     * 创建新的压缩处理器管理器
     */
    pub fn new() -> Self {
        let handlers: Vec<Box<dyn ArchiveHandler>> = vec![
            Box::new(TarHandler),  // 先检查TAR（包括tar.gz等）
            Box::new(GzHandler),   // 再检查纯GZ
            Box::new(ZipHandler),  // 然后ZIP
            Box::new(RarHandler),  // 最后RAR
        ];

        Self { handlers }
    }

    /**
     * 提取压缩文件
     *
     * 自动检测文件类型并使用合适的处理器
     */
    pub async fn extract_archive(
        &self,
        source: &Path,
        target_dir: &Path,
    ) -> Result<ExtractionSummary> {
        // 查找合适的处理器
        let handler = self.find_handler(source).ok_or_else(|| {
            crate::error::AppError::archive_error(
                format!("Unsupported archive format: {:?}", source.extension()),
                Some(source.to_path_buf()),
            )
        })?;

        // 使用处理器提取文件
        handler.extract(source, target_dir).await
    }

    /**
     * 查找支持该文件的处理器
     */
    fn find_handler(&self, path: &Path) -> Option<&dyn ArchiveHandler> {
        self.handlers
            .iter()
            .find(|handler| handler.can_handle(path))
            .map(|handler| handler.as_ref())
    }

    /**
     * 获取所有支持的文件扩展名
     */
    pub fn supported_extensions(&self) -> Vec<String> {
        self.handlers
            .iter()
            .flat_map(|handler| handler.file_extensions())
            .map(|ext| ext.to_string())
            .collect()
    }
}

impl Default for ArchiveManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_archive_manager_supported_extensions() {
        let manager = ArchiveManager::new();
        let extensions = manager.supported_extensions();

        assert!(extensions.contains(&"zip".to_string()));
        assert!(extensions.contains(&"rar".to_string()));
        assert!(extensions.contains(&"tar".to_string()));
        assert!(extensions.contains(&"tar.gz".to_string()));
        assert!(extensions.contains(&"gz".to_string()));
    }

    #[test]
    fn test_archive_manager_find_handler() {
        let manager = ArchiveManager::new();

        assert!(manager.find_handler(Path::new("test.zip")).is_some());
        assert!(manager.find_handler(Path::new("test.rar")).is_some());
        assert!(manager.find_handler(Path::new("test.tar")).is_some());
        assert!(manager.find_handler(Path::new("test.tar.gz")).is_some());
        assert!(manager.find_handler(Path::new("test.gz")).is_some());
        assert!(manager.find_handler(Path::new("test.txt")).is_none());
    }

    #[tokio::test]
    async fn test_extract_unsupported_format() {
        let manager = ArchiveManager::new();
        let temp_dir = TempDir::new().unwrap();

        let source_file = temp_dir.path().join("test.txt");
        let output_dir = temp_dir.path().join("output");

        // 创建测试文件
        fs::write(&source_file, "test content").unwrap();

        // 尝试提取不支持的格式
        let result = manager.extract_archive(&source_file, &output_dir).await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unsupported archive format"));
    }
}
