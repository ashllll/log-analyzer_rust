//! WorkspaceServiceImpl — WorkspaceService trait 的具体实现。
//!
//! 每个实例对应一个工作区，持有该工作区的全部运行时依赖。
//! 在导入完成时由命令层创建，存入 AppState.workspace_services。
//!
//! # 架构位置
//!
//! - **接口层**：`WorkspaceService` trait（application/workspace_service.rs）
//! - **实现层**：`WorkspaceServiceImpl`（本文件，infrastructure 层）
//! - **组装点**：导入命令完成时，由命令层调用 `WorkspaceServiceImpl::new` 创建
//!
//! # 当前实现状态
//!
//! - [x] SearchService（search / cancel_search / fetch_search_page）
//! - [ ] ImportService（占位，P4 实现）
//! - [ ] WatchService（占位，P5 实现）

use async_trait::async_trait;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

use la_core::domain::event::EventPublisher;
// use la_core::domain::SearchResultRepository; // 不需要直接导入，SearchUseCase 内部使用
use la_core::error::{AppError, Result};
use la_core::models::{SearchFilters, SearchQuery};

use crate::application::workspace_service::{
    ImportOptions, ImportService, SearchService, WatchService, WorkspaceService,
};
use crate::application::SearchUseCase;
use crate::infrastructure::{
    CasLogFileRepository, DiskResultStoreRepo, QueryEngineLogSearcher,
};

// ============================================================================
// WorkspaceServiceImpl
// ============================================================================

/// 工作区服务的具体实现。
///
/// 每个实例持有单个工作区的全部运行时依赖：
/// - CAS（内容寻址存储）
/// - MetadataStore（SQLite 元数据索引）
/// - SearchEngineManager（Tantivy 搜索索引）
/// - DiskResultStore（磁盘结果缓存）
/// - EventPublisher（事件发射器，trait 对象）
/// - ThreadPool（Rayon 线程池，共享引用）
///
/// 内部管理搜索会话的生命周期（CancellationToken 存储与清理）。
pub struct WorkspaceServiceImpl {
    workspace_id: String,
    workspace_dir: PathBuf,
    cas: Arc<la_storage::ContentAddressableStorage>,
    metadata_store: Arc<la_storage::MetadataStore>,
    search_engine: Arc<la_search::SearchEngineManager>,
    disk_result_store: Arc<la_search::DiskResultStore>,
    event_publisher: Arc<dyn EventPublisher>,
    thread_pool: Arc<rayon::ThreadPool>,
    /// FIX(P1-03): 缓存 QueryEngineLogSearcher，避免每次搜索都新建实例导致正则缓存失效
    searcher: Arc<QueryEngineLogSearcher>,
    /// 活跃的搜索会话 —— search_id → CancellationToken
    search_sessions: Arc<Mutex<HashMap<String, CancellationToken>>>,
}

impl WorkspaceServiceImpl {
    /// 创建新的 WorkspaceServiceImpl 实例。
    ///
    /// # 参数
    /// - `workspace_id`: 工作区唯一标识
    /// - `workspace_dir`: 工作区目录路径
    /// - `cas`: 已初始化的 CAS 实例
    /// - `metadata_store`: 已初始化的 MetadataStore 实例
    /// - `search_engine`: 已初始化的 SearchEngineManager 实例
    /// - `disk_result_store`: 全局共享的 DiskResultStore
    /// - `event_publisher`: 事件发射器 trait 对象
    /// - `thread_pool`: 全局共享的 Rayon 线程池
    /// - `regex_cache_size`: 正则缓存大小（传递给 QueryEngineLogSearcher）
    ///
    /// # 注意
    /// CAS、MetadataStore、SearchEngineManager 应在调用前已初始化。
    /// 导入命令的 `ensure_workspace_runtime_state` 函数负责创建这些组件。
    pub fn new(
        workspace_id: String,
        workspace_dir: PathBuf,
        cas: Arc<la_storage::ContentAddressableStorage>,
        metadata_store: Arc<la_storage::MetadataStore>,
        search_engine: Arc<la_search::SearchEngineManager>,
        disk_result_store: Arc<la_search::DiskResultStore>,
        event_publisher: Arc<dyn EventPublisher>,
        thread_pool: Arc<rayon::ThreadPool>,
        regex_cache_size: usize,
    ) -> Self {
        Self {
            workspace_id,
            workspace_dir,
            cas,
            metadata_store,
            search_engine,
            disk_result_store,
            event_publisher,
            thread_pool,
            searcher: Arc::new(QueryEngineLogSearcher::new(regex_cache_size)),
            search_sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

}

// ============================================================================
// WorkspaceService 实现
// ============================================================================

impl WorkspaceService for WorkspaceServiceImpl {
    fn workspace_id(&self) -> &str {
        &self.workspace_id
    }

    fn workspace_dir(&self) -> &PathBuf {
        &self.workspace_dir
    }

    fn cas(&self) -> &Arc<la_storage::ContentAddressableStorage> {
        &self.cas
    }

    fn metadata_store(&self) -> &Arc<la_storage::MetadataStore> {
        &self.metadata_store
    }

    fn search_engine(&self) -> &Arc<la_search::SearchEngineManager> {
        &self.search_engine
    }
}

// ============================================================================
// SearchService 实现
// ============================================================================

#[async_trait]
impl SearchService for WorkspaceServiceImpl {
    async fn search(
        &self,
        query: SearchQuery,
        raw_terms: Vec<String>,
        filters: SearchFilters,
        max_results: usize,
        cancellation_token: CancellationToken,
    ) -> Result<String> {
        // 1. 生成搜索会话 ID
        let search_id = uuid::Uuid::new_v4().to_string();

        // 2. 注册 CancellationToken 到会话表（供内部 cancel_search 使用）
        {
            self.search_sessions
                .lock()
                .insert(search_id.clone(), cancellation_token.clone());
        }

        // 3. 组装 SearchUseCase 的依赖
        let log_files = Arc::new(CasLogFileRepository {
            metadata: self.metadata_store.clone(),
            cas: self.cas.clone(),
        });
        let results = Arc::new(DiskResultStoreRepo {
            store: self.disk_result_store.clone(),
        });
        let searcher = Arc::clone(&self.searcher);

        // 4. 创建 SearchUseCase（泛型参数在此具体化）
        let use_case = SearchUseCase::new(
            log_files,
            results,
            self.event_publisher.clone(),
            searcher,
            self.thread_pool.clone(),
        );

        // 5. 执行搜索（spawn_blocking 内运行，立即返回）
        let workspace_id = self.workspace_id.clone();
        let search_id_clone = search_id.clone();
        let sessions = Arc::clone(&self.search_sessions);

        // 在后台执行搜索，完成后清理会话
        tokio::spawn(async move {
            let result = use_case
                .execute(
                    &workspace_id,
                    &query,
                    raw_terms,
                    &filters,
                    max_results,
                    search_id_clone.clone(),
                    cancellation_token,
                )
                .await;

            // 搜索完成后（成功/失败/取消），从会话表中移除
            sessions.lock().remove(&search_id_clone);

            if let Err(e) = result {
                tracing::warn!(
                    search_id = %search_id_clone,
                    error = %e,
                    "Search execution failed"
                );
            }
        });

        Ok(search_id)
    }

    async fn fetch_search_page(
        &self,
        search_id: &str,
        offset: usize,
        limit: usize,
    ) -> Result<la_search::SearchPageResult> {
        let limit = limit.min(10_000);

        if !self.disk_result_store.has_session(search_id) {
            return Err(AppError::not_found(format!(
                "Search session '{}' not found",
                search_id
            )));
        }

        self.disk_result_store
            .read_page(search_id, offset, limit)
            .map_err(|e| {
                AppError::io_error(
                    format!("Failed to read search page: {e}"),
                    None,
                )
            })
    }
}

// ============================================================================
// ImportService 实现（占位）
// ============================================================================

#[async_trait]
impl ImportService for WorkspaceServiceImpl {
    async fn import_file(
        &self,
        _source_path: &std::path::Path,
        _options: ImportOptions,
    ) -> Result<String> {
        // P4: 将 commands/import.rs 中的导入逻辑下沉到这里
        todo!("ImportService::import_file — 待 P4 实现")
    }
}

// ============================================================================
// WatchService 实现（占位）
// ============================================================================

#[async_trait]
impl WatchService for WorkspaceServiceImpl {
    async fn start_watch(&self) -> Result<()> {
        // P5: 将 commands/watch.rs 中的监听逻辑下沉到这里
        todo!("WatchService::start_watch — 待 P5 实现")
    }

    async fn stop_watch(&self) -> Result<()> {
        // P5: 将 commands/watch.rs 中的停止监听逻辑下沉到这里
        todo!("WatchService::stop_watch — 待 P5 实现")
    }

    async fn is_watching(&self) -> Result<bool> {
        // P5: 查询监听状态
        todo!("WatchService::is_watching — 待 P5 实现")
    }
}
