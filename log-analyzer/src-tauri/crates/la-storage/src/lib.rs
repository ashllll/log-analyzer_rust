// la-storage: CAS 内容寻址存储 + SQLite 元数据
pub mod cache_monitor;
pub mod cas;
pub mod coordinator;
pub mod gc;
pub mod integrity;
pub mod metadata_store;

// 重新导出核心类型
pub use cache_monitor::{CacheHealthMetrics, CacheMonitor, CacheMonitorConfig};
pub use cas::ContentAddressableStorage;
pub use coordinator::StorageCoordinator;
pub use gc::{GCConfig, GCManager, GCStats, GarbageCollector};
pub use integrity::{
    verify_after_import, verify_file_integrity, verify_workspace_integrity, InvalidFileInfo,
    ValidationReport,
};
pub use metadata_store::{ArchiveMetadata, FileMetadata, IndexState, IndexedFile, MetadataStore};
