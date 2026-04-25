//! Content-Addressable Storage (CAS) Module
//!
//! This module implements industry-standard content-addressable storage
//! based on Git and Docker patterns. It provides:
//!
//! - SHA-256 based file identification
//! - Flat storage structure to avoid path length limits
//! - Automatic deduplication
//! - SQLite metadata management
//!
//! ## Architecture
//!
//! ```text
//! workspace/
//! ├── metadata.db          # SQLite database
//! └── objects/             # Content storage (flat)
//!     ├── a3/
//!     │   └── f2e1d4c5...  # SHA-256 hash (first 2 chars as dir)
//!     └── b7/
//!         └── e145a3b2...
//! ```

// Re-export all types from la-storage crate
pub use la_storage::{
    verify_after_import, verify_file_integrity, verify_workspace_integrity, CacheHealthMetrics,
    CacheMonitor, CacheMonitorConfig, ContentAddressableStorage, GCConfig, GCManager, GCStats,
    GarbageCollector, IndexState, IndexedFile, InvalidFileInfo, MetadataStore, StorageCoordinator,
    ValidationReport,
};

// FileMetadata 和 ArchiveMetadata 的定义来自 la-core::storage_types，
// 通过 la-storage 的 re-export 保持公开 API 不变。
pub use la_storage::{ArchiveMetadata, FileMetadata};

#[cfg(test)]
mod integration_tests;
