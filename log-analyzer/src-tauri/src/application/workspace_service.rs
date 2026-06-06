//! WorkspaceService — 按工作区预组装的服务接口。
//!
//! 将 AppState 的"运行时映射容器"模式重构为"预组装服务"模式。
//! 每个工作区导入完成后，创建 WorkspaceServiceImpl 实例，包含该工作区的
//! 全部运行时依赖（CAS、MetadataStore、SearchEngine 等）。
//!
//! # 设计决策（来自架构审查 grilling loop）
//!
//! - **方向 A**：WorkspaceService 模式（每个工作区一套预组装服务）
//! - **接口拆分**：A2（按用例拆 trait：SearchService / ImportService / WatchService）
//! - **事件处理**：方案 1（注入 EventPublisher trait 对象）
//! - **创建时机**：导入完成时预创建（非懒加载）
//! - **全局资源**：DiskResultStore + ThreadPool 作为创建参数传入
//!
//! # 迁移状态
//!
//! - [x] P0: trait 定义
//! - [x] P1: SearchService 实现
//! - [x] P2: AppState 集成（workspace services HashMap）
//! - [x] P3: search_logs 命令迁移（取消令牌由 WorkspaceServiceImpl 内部管理）
//! - [x] P4: ImportService 实现
//! - [x] P5: WatchService 实现（watcher 状态移入 WorkspaceServiceImpl）
//! - [x] P6: 清理旧 HashMap（移除全局 cancellation_tokens，仅保留 services HashMap）

use async_trait::async_trait;
use la_core::error::Result;
use la_core::models::{SearchFilters, SearchQuery};
use la_core::traits::AppConfigProvider;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

// 保留 re-exports 以保持向后兼容（workspace_repo、cleanup_workspace_resources 等引用）
pub use la_storage::ContentAddressableStorage;
pub use la_search::SearchEngineManager;

// ============================================================================
// SearchService — 工作区搜索能力
// ============================================================================

/// 工作区搜索服务接口。
///
/// 封装了单个工作区的全部搜索能力，包括：
/// - 执行搜索（生成 search_id，内部管理 CancellationToken）
/// - 取消搜索
/// - 获取搜索结果页
///
/// 实现者负责内部组装 SearchUseCase 及其所有依赖，调用方无需了解底层细节。
#[async_trait]
pub trait SearchService: Send + Sync {
    /// 执行搜索查询。
    ///
    /// 内部生成唯一的 search_id，使用传入的 CancellationToken，调用 SearchUseCase 执行。
    /// 进度和结果通过注入的 EventPublisher 发射。
    ///
    /// # 参数
    /// - `query`: 结构化查询（已由命令层解析）
    /// - `raw_terms`: 原始搜索词（用于高亮显示）
    /// - `filters`: 搜索过滤器（时间范围、日志级别、文件路径等）
    /// - `max_results`: 最大结果数上限
    /// - `cancellation_token`: 取消令牌（由命令层创建和管理生命周期）
    ///
    /// # 返回
    /// 搜索会话 ID，前端可用此 ID 获取结果和进度。
    async fn search(
        &self,
        query: SearchQuery,
        raw_terms: Vec<String>,
        filters: SearchFilters,
        max_results: usize,
        cancellation_token: CancellationToken,
    ) -> Result<String>;

    /// 获取搜索结果分页。
    ///
    /// 从 DiskResultStore 读取指定搜索会话的结果页。
    async fn fetch_search_page(
        &self,
        search_id: &str,
        offset: usize,
        limit: usize,
    ) -> Result<la_search::SearchPageResult>;

    /// 取消正在执行的搜索。
    ///
    /// 取消指定 search_id 的搜索会话（如果存在）。
    async fn cancel_search(&self, search_id: &str) -> Result<()>;
}

// ============================================================================
// ImportService — 工作区导入能力
// ============================================================================

/// 工作区导入服务接口。
///
/// 封装了单个工作区的全部导入能力，包括：
/// - 调用 process_path_with_cas 处理文件/压缩包
/// - 回退文件统计（处理 PENDING 文件）
/// - 重建搜索索引
///
/// 命令层负责 TaskManager 任务生命周期、完整性验证和 Tantivy 段合并。
#[async_trait]
pub trait ImportService: Send + Sync {
    /// 导入日志文件到工作区。
    ///
    /// # 参数
    /// - `source_path`: 要导入的文件或目录路径
    /// - `options`: 导入选项（extract_archives, skip_existing）
    /// - `config_provider`: 应用配置提供者（解耦 Tauri AppHandle）
    /// - `task_id`: 任务 ID（由命令层 TaskManager 创建，用于进度关联）
    /// - `cancellation_token`: 取消令牌（由命令层创建和管理生命周期）
    ///
    /// # 返回
    /// `ImportResult` 包含根名称和已导入文件数。
    async fn import_file(
        &self,
        source_path: &std::path::Path,
        options: ImportOptions,
        config_provider: &dyn AppConfigProvider,
        task_id: &str,
        cancellation_token: CancellationToken,
    ) -> Result<ImportResult>;
}

/// 导入返回结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    /// 导入根名称（文件名或目录名）
    pub root_name: String,
    /// 已导入的文件数
    pub files_imported: usize,
}

/// 导入选项。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImportOptions {
    /// 是否解压缩包
    pub extract_archives: bool,
    /// 是否跳过已存在的文件
    pub skip_existing: bool,
}

// ============================================================================
// WatchService — 工作区文件监听能力
// ============================================================================

/// 工作区文件监听服务接口。
///
/// 每个 WorkspaceServiceImpl 实例管理自己的 watcher 状态（Option<WatcherState>），
/// 不再依赖 AppState 的全局 HashMap。watch 命令变为薄层委托。
#[async_trait]
pub trait WatchService: Send + Sync {
    /// 启动文件监听。
    ///
    /// # 参数
    /// - `watch_path`: 要监听的文件系统路径（文件或目录）
    async fn start_watch(&self, watch_path: &str) -> Result<()>;

    /// 停止文件监听。
    async fn stop_watch(&self) -> Result<()>;

    /// 获取监听状态。
    async fn is_watching(&self) -> Result<bool>;
}

// ============================================================================
// WorkspaceService — 组合 trait
// ============================================================================

/// 工作区服务组合接口。
///
/// 一个 WorkspaceService 实例代表一个已加载的工作区的全部运行时能力。
/// 在导入完成时预创建，持有该工作区的全部依赖（CAS、MetadataStore、
/// SearchEngine、DiskResultStore 等）。
///
/// # 生命周期
///
/// 1. 导入命令完成文件导入后，调用 `WorkspaceServiceImpl::new(...)` 创建实例
/// 2. 实例存入 AppState.workspace_services HashMap
/// 3. 后续命令通过 `AppState.get_workspace_service(id)` 获取并调用
/// 4. 工作区删除时，从 HashMap 移除并释放资源
pub trait WorkspaceService: SearchService + ImportService + WatchService + Send + Sync {
    /// 获取工作区 ID。
    fn workspace_id(&self) -> &str;

    /// 获取工作区目录路径。
    fn workspace_dir(&self) -> &PathBuf;

    /// 获取 CAS 实例（供 workspace_repo、cleanup 等使用）。
    fn cas(&self) -> &Arc<la_storage::ContentAddressableStorage>;

    /// 获取 MetadataStore 实例（供 workspace_repo、cleanup 等使用）。
    fn metadata_store(&self) -> &Arc<la_storage::MetadataStore>;

    /// 获取 SearchEngineManager 实例（供 workspace_repo、cleanup 等使用）。
    fn search_engine(&self) -> &Arc<la_search::SearchEngineManager>;
}

// ============================================================================
// 类型别名 — 便于使用
// ============================================================================

/// WorkspaceService trait 对象的 Arc 包装。
pub type WorkspaceServiceRef = Arc<dyn WorkspaceService>;
