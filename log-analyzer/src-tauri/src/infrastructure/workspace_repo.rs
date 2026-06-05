//! WorkspaceRepo — workspace storage layer aggregate.
//!
//! Created by P9 and hardened in P13: fields are now private with accessor
//! methods so callers depend on the interface, not the layout.

use std::sync::Arc;

use la_search::{DiskResultStore, SearchEngineManager};
use la_storage::{ContentAddressableStorage, MetadataStore};

/// Persistent storage + search index for a single workspace.
#[derive(Clone)]
pub struct WorkspaceRepo {
    cas: Arc<ContentAddressableStorage>,
    metadata_store: Arc<MetadataStore>,
    search_engine: Arc<SearchEngineManager>,
    disk_result_store: Arc<DiskResultStore>,
}

impl WorkspaceRepo {
    pub fn new(
        cas: Arc<ContentAddressableStorage>,
        metadata_store: Arc<MetadataStore>,
        search_engine: Arc<SearchEngineManager>,
        disk_result_store: Arc<DiskResultStore>,
    ) -> Self {
        Self {
            cas,
            metadata_store,
            search_engine,
            disk_result_store,
        }
    }

    pub fn cas(&self) -> &Arc<ContentAddressableStorage> {
        &self.cas
    }

    pub fn metadata_store(&self) -> &Arc<MetadataStore> {
        &self.metadata_store
    }

    pub fn search_engine(&self) -> &Arc<SearchEngineManager> {
        &self.search_engine
    }

    pub fn disk_result_store(&self) -> &Arc<DiskResultStore> {
        &self.disk_result_store
    }
}
