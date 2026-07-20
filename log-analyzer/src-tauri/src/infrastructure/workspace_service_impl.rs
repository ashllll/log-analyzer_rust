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
//! - [x] ImportService（P4 完整实现）
//! - [x] WatchService（P5 完整实现，watcher 状态内嵌于实例中）

use async_trait::async_trait;
use parking_lot::Mutex;
use std::path::PathBuf;
use std::sync::Arc;

use crate::application::search_session::SearchSessionManager;
use la_core::domain::event::EventPublisher;

use crate::application::workspace_service::WorkspaceService;
use crate::infrastructure::{QueryEngineLogSearcher, WorkspaceRepo};
use crate::services::file_watcher::WatcherState;

mod import;
mod search;
mod watch;

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
    /// P9: 存储层聚合（cas + metadata_store + search_engine + disk_result_store）
    repo: WorkspaceRepo,
    event_publisher: Arc<dyn EventPublisher>,
    thread_pool: Arc<rayon::ThreadPool>,
    /// FIX(P1-03): 缓存 QueryEngineLogSearcher，避免每次搜索都新建实例导致正则缓存失效
    searcher: Arc<QueryEngineLogSearcher>,
    /// P8: 搜索会话生命周期由后端全局 SearchSessionManager 统一管理
    search_session_manager: SearchSessionManager,
    /// 文件监听器状态（P5：从 AppState::watchers 移入实例）
    watcher_state: Arc<Mutex<Option<WatcherState>>>,
    /// Watch 模式 FilesUpdated 广播用（传递给 WatcherRunner）
    app_handle: tauri::AppHandle,
}

impl WorkspaceServiceImpl {
    /// 创建新的 WorkspaceServiceImpl 实例。
    ///
    /// # 参数
    /// - `workspace_id`: 工作区唯一标识
    /// - `workspace_dir`: 工作区目录路径
    /// - `repo`: 预组装的存储层聚合（CAS + MetadataStore + SearchEngine + DiskResultStore）
    /// - `event_publisher`: 事件发射器 trait 对象
    /// - `thread_pool`: 全局共享的 Rayon 线程池
    /// - `regex_cache_size`: 正则缓存大小（传递给 QueryEngineLogSearcher）
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        workspace_id: String,
        workspace_dir: PathBuf,
        repo: WorkspaceRepo,
        event_publisher: Arc<dyn EventPublisher>,
        thread_pool: Arc<rayon::ThreadPool>,
        regex_cache_size: usize,
        search_session_manager: SearchSessionManager,
        app_handle: tauri::AppHandle,
    ) -> Self {
        Self {
            workspace_id,
            workspace_dir,
            repo,
            event_publisher,
            thread_pool,
            searcher: Arc::new(QueryEngineLogSearcher::new(regex_cache_size)),
            search_session_manager,
            watcher_state: Arc::new(Mutex::new(None)),
            app_handle,
        }
    }
}

// ============================================================================
// WorkspaceService 实现
// ============================================================================

#[async_trait]
impl WorkspaceService for WorkspaceServiceImpl {
    fn workspace_id(&self) -> &str {
        &self.workspace_id
    }

    fn workspace_dir(&self) -> &PathBuf {
        &self.workspace_dir
    }

    fn cas(&self) -> &Arc<la_storage::ContentAddressableStorage> {
        self.repo.cas()
    }

    fn metadata_store(&self) -> &Arc<la_storage::MetadataStore> {
        self.repo.metadata_store()
    }

    fn search_engine(&self) -> &Arc<la_search::SearchEngineManager> {
        self.repo.search_engine()
    }

    async fn close_databases(&self) {
        self.repo.metadata_store().close().await;
        self.repo.search_engine().close().await;
    }
}
