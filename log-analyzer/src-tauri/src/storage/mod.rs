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

pub mod cas;
pub mod integrity;
pub mod metadata_store;
pub mod metrics_store;

#[cfg(test)]
mod integration_tests;

pub use cas::ContentAddressableStorage;
pub use integrity::{
    verify_after_import, verify_file_integrity, verify_workspace_integrity, InvalidFileInfo,
    ValidationReport,
};
pub use metadata_store::{ArchiveMetadata, FileMetadata, IndexState, IndexedFile, MetadataStore};
pub use metrics_store::{
    MetricsSnapshot, MetricsSnapshotScheduler, MetricsStore, MetricsStoreStats, SearchEvent,
    TimeRange,
};
