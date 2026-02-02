/**
 * 压缩文件处理模块
 *
 * 提供统一的接口处理各种压缩格式（ZIP、RAR、TAR、GZ）
 */
pub mod actors;
pub mod fault_tolerance;
pub mod streaming;

pub mod archive_handler;
pub mod resource_manager;
pub mod security_detector;

pub mod audit_logger;
pub mod checkpoint_manager;
pub mod edge_case_handlers;
pub mod extraction_context;
pub mod extraction_engine;
pub mod extraction_orchestrator;
pub mod gz_handler;
pub mod parallel_processor;
pub mod path_manager;
pub mod path_validator; // 新增：路径安全验证器
pub mod processor;
pub mod progress_tracker;
pub mod public_api;
pub mod rar_handler;
pub mod sevenz_handler;
pub mod tar_handler;
pub mod traversal; // 新增：统一遍历模块
pub mod zip_handler;

pub use archive_handler::{ArchiveHandler, ExtractionSummary};
pub use extraction_context::{ExtractionContext, ExtractionItem, ExtractionStack};
pub use extraction_engine::{ExtractionEngine, ExtractionPolicy};
pub use extraction_orchestrator::ExtractionOrchestrator;
pub use gz_handler::GzHandler;
pub use parallel_processor::{ParallelConfig, ParallelProcessor};
pub use path_manager::{PathConfig, PathManager};
pub use path_validator::{PathValidator, PathValidatorConfig}; // 导出验证器
#[allow(unused_imports)]
#[allow(deprecated)]
pub use processor::process_path_recursive_with_metadata;
pub use processor::{process_path_with_cas, CasProcessingContext}; // Export CAS-based processing function and context
pub use public_api::{extract_archive_async, extract_archive_sync, ExtractionResult};
pub use rar_handler::RarHandler;
pub use security_detector::{SecurityDetector, SecurityPolicy};
pub use sevenz_handler::SevenZHandler;

pub use tar_handler::TarHandler;
pub use zip_handler::ZipHandler;
pub use traversal::{
    DirectoryTraverser, PathNodeIterator, TraversalConfig, TraversalEntry, TraversalError,
    TraversalStats, TraversalStatsSnapshot,
};
// 导出 checkpoint 相关类型供测试使用
pub use checkpoint_manager::{Checkpoint, CheckpointConfig, CheckpointManager};
// 导出 audit logger 相关类型供测试使用
pub use audit_logger::{AuditLogEntry, AuditLogger};
// 导出 edge case handlers 相关类型供测试使用
pub use edge_case_handlers::EdgeCaseHandler;
// 导出 progress tracker 相关类型供测试使用
pub use progress_tracker::ProgressTracker;
// 导出 resource manager 相关类型供测试使用
pub use resource_manager::ResourceManager as ArchiveResourceManager;

use crate::error::Result;
use crate::models::config::ArchiveConfig;
use std::path::Path;

/**
 * 压缩处理器管理器
 *
 * 管理所有支持的压缩格式处理器
 * 配置使用统一的配置系统，支持从配置文件或环境变量加载
 */
pub struct ArchiveManager {
    handlers: Vec<Box<dyn ArchiveHandler>>,
    // 安全配置（来自 ArchiveConfig）
    max_file_size: u64,    // 单个文件最大大小（字节）
    max_total_size: u64,   // 解压后总大小限制（字节）
    max_file_count: usize, // 解压文件数量限制
}

impl ArchiveManager {
    /**
     * 创建新的压缩处理器管理器（使用默认配置）
     */
    pub fn new() -> Self {
        Self::with_config(ArchiveConfig::default())
    }

    /**
     * 使用自定义配置创建压缩处理器管理器
     *
     * # Arguments
     *
     * * `config` - 压缩包处理配置
     */
    pub fn with_config(config: ArchiveConfig) -> Self {
        let handlers: Vec<Box<dyn ArchiveHandler>> = vec![
            Box::new(TarHandler),    // 先检查TAR（包括tar.gz等）
            Box::new(GzHandler),     // 再检查纯GZ
            Box::new(ZipHandler),    // 然后ZIP
            Box::new(RarHandler),    // 然后RAR
            Box::new(SevenZHandler), // 最后7z支持
        ];

        Self {
            handlers,
            max_file_size: config.max_file_size,
            max_total_size: config.max_total_size,
            max_file_count: config.max_file_count,
        }
    }

    /**
     * 获取当前配置引用
     */
    pub fn get_config(&self) -> ArchiveConfig {
        ArchiveConfig {
            max_file_size: self.max_file_size,
            max_total_size: self.max_total_size,
            max_file_count: self.max_file_count,
            ..Default::default()
        }
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

        // 使用处理器提取文件，传递安全限制参数
        handler
            .extract_with_limits(
                source,
                target_dir,
                self.max_file_size,
                self.max_total_size,
                self.max_file_count,
            )
            .await
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
