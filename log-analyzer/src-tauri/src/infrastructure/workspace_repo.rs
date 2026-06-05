//! WorkspaceRepo — 工作区存储层组件的聚合容器。
//!
//! P9: 将 cas / metadata_store / search_engine / disk_result_store 打包为一个
//! 结构体，减少 WorkspaceServiceImpl 构造函数参数从 9 到 6，并显式标注"存储层"边界。
//!
//! 所有组件在导入时一起创建、一起传递、一起清理，自然形成聚合关系。

use std::sync::Arc;

use la_search::{DiskResultStore, SearchEngineManager};
use la_storage::{ContentAddressableStorage, MetadataStore};

/// 工作区的持久化存储 + 搜索索引聚合。
#[derive(Clone)]
pub struct WorkspaceRepo {
    pub cas: Arc<ContentAddressableStorage>,
    pub metadata_store: Arc<MetadataStore>,
    pub search_engine: Arc<SearchEngineManager>,
    pub disk_result_store: Arc<DiskResultStore>,
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
}
