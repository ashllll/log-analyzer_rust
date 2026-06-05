//! la-archive: ZIP/TAR/GZ/RAR/7Z 递归解压模块
//!
//! 提供统一的接口处理各种压缩格式，支持递归解压嵌套压缩包。
//!
//! # Feature Gates（已澄清）
//!
//! `enhanced-extraction`: 默认开启（见 Cargo.toml `default-features`）。
//! 该 feature 控制核心提取引擎、安全检查、路径管理等模块的编译。
//! 当前无禁用场景，保留仅用于将来的最小化构建评估。

pub mod archive_handler;
#[cfg(feature = "enhanced-extraction")]
pub mod checkpoint_manager;
#[cfg(feature = "enhanced-extraction")]
pub mod extraction_engine; // P10: extraction_context types merged into extraction_engine
#[cfg(feature = "enhanced-extraction")]
pub mod extraction_orchestrator;
pub mod gz_handler;
pub mod internal;
#[cfg(feature = "enhanced-extraction")]
pub mod path_manager;
pub mod processor;
#[cfg(feature = "enhanced-extraction")]
pub mod public_api;
pub mod rar_handler;
#[cfg(feature = "enhanced-extraction")]
pub mod security_detector;
pub mod sevenz_handler;
pub mod stats;
mod symlink_guard;
pub mod tar_handler;
pub mod zip_handler;

// 重新导出核心类型
pub use archive_handler::{ArchiveHandler, ExtractionSummary};
#[cfg(feature = "enhanced-extraction")]
pub use checkpoint_manager::{Checkpoint, CheckpointConfig, CheckpointManager};
#[cfg(feature = "enhanced-extraction")]
#[cfg(feature = "enhanced-extraction")]
pub use extraction_engine::{ExtractionContext, ExtractionEngine, ExtractionItem, ExtractionPolicy, ExtractionStack};
#[cfg(feature = "enhanced-extraction")]
pub use extraction_orchestrator::ExtractionOrchestrator;
pub use gz_handler::GzHandler;
#[cfg(feature = "enhanced-extraction")]
pub use path_manager::{PathConfig, PathManager};
pub use processor::{process_path_with_cas, CasProcessingContext};
#[cfg(feature = "enhanced-extraction")]
pub use public_api::{extract_archive_async, extract_archive_sync, ExtractionResult};
pub use rar_handler::RarHandler;
#[cfg(feature = "enhanced-extraction")]
pub use security_detector::{SecurityDetector, SecurityPolicy};
pub use sevenz_handler::SevenZHandler;
pub use tar_handler::TarHandler;
pub use zip_handler::ZipHandler;

use la_core::error::Result;
use la_core::models::config::ArchiveConfig;
use std::path::Path;

/// 压缩处理器管理器
///
/// 管理所有支持的压缩格式处理器
pub struct ArchiveManager {
    handlers: Vec<Box<dyn ArchiveHandler>>,
    max_file_size: u64,
    max_total_size: u64,
    max_file_count: usize,
}

impl ArchiveManager {
    pub fn new() -> Self {
        Self::with_config(ArchiveConfig::default())
    }

    pub fn with_config(config: ArchiveConfig) -> Self {
        // Build handler list with full set enabled by default
        let handlers: Vec<Box<dyn ArchiveHandler>> = vec![
            Box::new(TarHandler),
            Box::new(GzHandler),
            Box::new(ZipHandler),
            Box::new(RarHandler),
            Box::new(SevenZHandler),
        ];

        Self {
            handlers,
            max_file_size: config.max_file_size,
            max_total_size: config.max_total_size,
            max_file_count: config.max_file_count,
        }
    }

    /// Build with handler toggles from `HandlersConfig`.
    ///
    /// Only handlers whose corresponding toggle is `true` are registered.
    /// This allows runtime control over which archive formats are supported.
    pub fn with_handlers_config(
        config: ArchiveConfig,
        handlers_cfg: &la_core::models::extraction_policy::HandlersConfig,
    ) -> Self {
        let mut handlers: Vec<Box<dyn ArchiveHandler>> = Vec::with_capacity(5);

        if handlers_cfg.tar {
            handlers.push(Box::new(TarHandler));
        }
        if handlers_cfg.gz {
            handlers.push(Box::new(GzHandler));
        }
        if handlers_cfg.zip {
            handlers.push(Box::new(ZipHandler));
        }
        if handlers_cfg.rar {
            handlers.push(Box::new(RarHandler));
        }
        if handlers_cfg.sevenz {
            handlers.push(Box::new(SevenZHandler));
        }

        Self {
            handlers,
            max_file_size: config.max_file_size,
            max_total_size: config.max_total_size,
            max_file_count: config.max_file_count,
        }
    }

    pub fn get_config(&self) -> ArchiveConfig {
        ArchiveConfig {
            max_file_size: self.max_file_size,
            max_total_size: self.max_total_size,
            max_file_count: self.max_file_count,
            ..Default::default()
        }
    }

    pub async fn extract_archive(
        &self,
        source: &Path,
        target_dir: &Path,
    ) -> Result<ExtractionSummary> {
        let handler = self.find_handler(source).ok_or_else(|| {
            la_core::error::AppError::archive_error(
                format!("Unsupported archive format: {:?}", source.extension()),
                Some(source.to_path_buf()),
            )
        })?;

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

    fn find_handler(&self, path: &Path) -> Option<&dyn ArchiveHandler> {
        self.handlers
            .iter()
            .find(|handler| handler.can_handle(path))
            .map(|handler| handler.as_ref())
    }

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

        fs::write(&source_file, "test content").unwrap();

        let result = manager.extract_archive(&source_file, &output_dir).await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unsupported archive format"));
    }
}
