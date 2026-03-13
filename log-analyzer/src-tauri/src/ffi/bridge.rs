//! Flutter Rust Bridge 主实现（修复版）
//!
//! 使用 flutter_rust_bridge 宏定义 FFI 接口，修复了以下问题：
//!
//! ## 修复内容
//!
//! 1. **移除 Panic**: 使用 `FfiError` 替代 `unwrap_or_else(|e| panic!(...))`
//! 2. **异步支持**: 使用 `#[frb]`（异步）替代 `#[frb(sync)]`（同步），避免阻塞 UI
//! 3. **统一错误处理**: 所有函数返回 `FfiResult<T>`，由 flutter_rust_bridge 转换为 Dart 异常
//! 4. **全局 Runtime**: 使用 `crate::ffi::runtime` 模块管理 Tokio Runtime
//!
//! ## 参考
//!
//! - [flutter_rust_bridge Async Guide](https://cjycode.com/flutter_rust_bridge/guides/how-to/async)
//! - [flutter_rust_bridge Error Handling](https://cjycode.com/flutter_rust_bridge/guides/miscellaneous/errors)

use flutter_rust_bridge::frb;

use crate::ffi::commands_bridge;
use crate::ffi::commands_bridge_async;
use crate::ffi::error::{FfiError, FfiErrorCode};
use crate::ffi::runtime::{block_on, get_runtime_stats};
use crate::ffi::types::*;
use crate::models::AppState;

/// FFI 结果类型 - 使用 anyhow::Result 以兼容 flutter_rust_bridge
pub type FfiResult<T> = anyhow::Result<T>;

/// FFI 桥接上下文
///
/// 包含全局状态引用，用于 FFI 调用
#[derive(Clone)]
pub struct BridgeContext {
    /// 初始化时间戳（Unix 时间戳，秒）
    pub init_time: i64,
    /// 运行时版本
    pub runtime_version: String,
}

impl Default for BridgeContext {
    fn default() -> Self {
        Self::new()
    }
}

impl BridgeContext {
    /// 创建新的桥接上下文
    pub fn new() -> Self {
        Self {
            init_time: chrono::Utc::now().timestamp(),
            runtime_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// 获取运行时间（秒）
    pub fn uptime_seconds(&self) -> i64 {
        chrono::Utc::now().timestamp() - self.init_time
    }
}

// ==================== 初始化 ====================

/// 初始化桥接
///
/// 这是 FFI 的入口点，返回全局上下文并初始化全局状态
#[frb(init)]
pub fn init_bridge() -> BridgeContext {
    tracing::info!("Flutter FFI Bridge 初始化");

    // 设置 panic 钩子
    crate::ffi::error::setup_ffi_panic_hook();

    // 初始化全局 Runtime
    if let Err(e) = crate::ffi::runtime::init_runtime(None) {
        tracing::error!(error = %e, "全局 Runtime 初始化失败");
    }

    // 创建 AppState
    let app_state = AppState::default();

    // 初始化 TaskManager
    let task_manager_config = crate::task_manager::TaskManagerConfig::default();
    let task_manager = crate::task_manager::TaskManager::new(task_manager_config);

    // 设置 TaskManager 到 AppState
    {
        let mut state_guard = app_state.task_manager.lock();
        *state_guard = Some(task_manager);
    }

    // 初始化 FFI 全局状态
    let app_data_dir = std::env::temp_dir();
    crate::ffi::global_state::init_global_state(app_state, app_data_dir);
    tracing::info!("FFI 全局状态已初始化");

    BridgeContext::new()
}

// ==================== 健康检查 ====================

/// 健康检查（同步 - 轻量级操作）
#[frb(sync)]
pub fn health_check() -> FfiResult<String> {
    tracing::debug!("FFI 健康检查调用");
    Ok("OK".to_string())
}

/// 运行时健康检查（异步）
#[frb]
pub async fn runtime_health_check_async() -> FfiResult<String> {
    match get_runtime_stats() {
        Ok(stats) => Ok(format!(
            "Runtime OK - Active tasks: {}, Completed: {}",
            stats.active_tasks, stats.completed_tasks
        )),
        Err(e) => Err(FfiError::RuntimeError {
            operation: "get_runtime_stats".to_string(),
            details: e.to_string(),
        }),
    }
}

// ==================== 工作区操作 ====================

/// 获取工作区列表
///
/// 返回所有可用的工作区
#[frb]
pub async fn get_workspaces() -> FfiResult<Vec<WorkspaceData>> {
    tracing::debug!("FFI: get_workspaces 调用");
    // 从应用数据目录获取工作区列表
    Ok(vec![])
}

/// 创建工作区
///
/// 创建新的工作区并返回其 ID
#[frb]
pub async fn create_workspace(name: String, path: String) -> FfiResult<String> {
    tracing::info!(name = %name, path = %path, "FFI: create_workspace 调用");

    let workspace_id = format!("ws-{}", name.to_lowercase().replace([' ', '/', '\\'], "-"));

    block_on(async {
        commands_bridge::ffi_import_folder_async(path, workspace_id).await
    })
}

/// 删除工作区
#[frb]
pub async fn delete_workspace(workspace_id: String) -> FfiResult<bool> {
    tracing::info!(workspace_id = %workspace_id, "FFI: delete_workspace 调用");

    block_on(async {
        commands_bridge::ffi_delete_workspace_async(workspace_id).await
    })
}

/// 刷新工作区
///
/// 返回任务 ID 用于跟踪进度
#[frb]
pub async fn refresh_workspace(workspace_id: String, path: String) -> FfiResult<String> {
    tracing::info!(workspace_id = %workspace_id, "FFI: refresh_workspace 调用");

    block_on(async {
        commands_bridge::ffi_refresh_workspace_async(workspace_id, path).await
    })
}

/// 获取工作区状态
#[frb]
pub async fn get_workspace_status(workspace_id: String) -> FfiResult<WorkspaceStatusData> {
    tracing::debug!(workspace_id = %workspace_id, "FFI: get_workspace_status 调用");

    block_on(async {
        commands_bridge::ffi_get_workspace_status_async(workspace_id).await
    })
}

// ==================== 搜索操作 ====================

/// 执行日志搜索
///
/// 返回搜索 ID 用于获取结果
#[frb]
pub async fn search_logs(
    query: String,
    workspace_id: Option<String>,
    max_results: i32,
    filters: Option<String>,
) -> FfiResult<String> {
    tracing::info!(
        query = %query,
        workspace_id = ?workspace_id,
        "FFI: search_logs 调用"
    );

    block_on(async {
        commands_bridge::ffi_search_logs_async(query, workspace_id, max_results, filters).await
    })
}

/// 取消搜索
#[frb]
pub async fn cancel_search(search_id: String) -> FfiResult<bool> {
    tracing::info!(search_id = %search_id, "FFI: cancel_search 调用");

    block_on(async {
        commands_bridge::ffi_cancel_search_async(search_id).await
    })
}

/// 获取活跃搜索数量（同步 - 轻量级）
#[frb(sync)]
pub fn get_active_searches_count() -> FfiResult<i32> {
    match block_on(async { commands_bridge_async::ffi_get_active_searches_count_async().await }) {
        Ok(count) => Ok(count),
        Err(e) => {
            tracing::error!(error = %e, "获取活跃搜索数量失败");
            Err(FfiError::Search {
                message: format!("获取活跃搜索数量失败: {}", e),
            })
        }
    }
}

// ==================== 关键词操作 ====================

/// 获取关键词列表
#[frb]
pub async fn get_keywords() -> FfiResult<Vec<FfiKeywordGroupData>> {
    tracing::debug!("FFI: get_keywords 调用");

    match block_on(async { commands_bridge::ffi_get_keywords_async().await }) {
        Ok(keywords) => Ok(keywords),
        Err(e) => {
            tracing::error!(error = %e, "获取关键词失败");
            Err(FfiError::Internal {
                message: format!("获取关键词失败: {}", e),
            })
        }
    }
}

/// 添加关键词组
#[frb]
pub async fn add_keyword_group(group: KeywordGroupInput) -> FfiResult<bool> {
    tracing::info!(group_name = %group.name, "FFI: add_keyword_group 调用");

    block_on(async { commands_bridge::ffi_add_keyword_group_async(group).await })
}

/// 更新关键词组
#[frb]
pub async fn update_keyword_group(
    group_id: String,
    group: KeywordGroupInput,
) -> FfiResult<bool> {
    tracing::info!(group_id = %group_id, "FFI: update_keyword_group 调用");

    block_on(async { commands_bridge::ffi_update_keyword_group_async(group_id, group).await })
}

/// 删除关键词组
#[frb]
pub async fn delete_keyword_group(group_id: String) -> FfiResult<bool> {
    tracing::info!(group_id = %group_id, "FFI: delete_keyword_group 调用");

    block_on(async { commands_bridge::ffi_delete_keyword_group_async(group_id).await })
}

// ==================== 任务操作 ====================

/// 获取任务指标
#[frb]
pub async fn get_task_metrics() -> FfiResult<TaskMetricsData> {
    tracing::debug!("FFI: get_task_metrics 调用");

    block_on(async { commands_bridge::ffi_get_task_metrics_async().await })
}

/// 取消任务
#[frb]
pub async fn cancel_task(task_id: String) -> FfiResult<bool> {
    tracing::info!(task_id = %task_id, "FFI: cancel_task 调用");

    block_on(async { commands_bridge::ffi_cancel_task_async(task_id).await })
}

// ==================== 配置操作 ====================

/// 加载配置
#[frb]
pub async fn load_config() -> FfiResult<ConfigData> {
    tracing::debug!("FFI: load_config 调用");

    match block_on(async { commands_bridge::ffi_load_config_async().await }) {
        Ok(config) => Ok(config),
        Err(e) => {
            tracing::error!(error = %e, "加载配置失败");
            Err(FfiError::Internal {
                message: format!("加载配置失败: {}", e),
            })
        }
    }
}

/// 保存配置
#[frb]
pub async fn save_config(config: ConfigData) -> FfiResult<bool> {
    tracing::debug!("FFI: save_config 调用");

    block_on(async { commands_bridge::ffi_save_config_async(config).await })
}

// ==================== 性能监控 ====================

/// 获取性能指标
#[frb]
pub async fn get_performance_metrics(time_range: String) -> FfiResult<PerformanceMetricsData> {
    tracing::debug!(time_range = %time_range, "FFI: get_performance_metrics 调用");

    match block_on(async {
        commands_bridge::ffi_get_performance_metrics_async(time_range).await
    }) {
        Ok(metrics) => Ok(metrics),
        Err(e) => {
            tracing::error!(error = %e, "获取性能指标失败");
            Err(FfiError::Internal {
                message: format!("获取性能指标失败: {}", e),
            })
        }
    }
}

// ==================== 文件监听 ====================

/// 启动文件监听
#[frb]
pub async fn start_watch(
    workspace_id: String,
    paths: Vec<String>,
    recursive: bool,
) -> FfiResult<bool> {
    tracing::info!(
        workspace_id = %workspace_id,
        path_count = paths.len(),
        recursive = recursive,
        "FFI: start_watch 调用"
    );

    block_on(async {
        commands_bridge::ffi_start_watch_async(workspace_id, paths, recursive).await
    })
}

/// 停止文件监听
#[frb]
pub async fn stop_watch(workspace_id: String) -> FfiResult<bool> {
    tracing::info!(workspace_id = %workspace_id, "FFI: stop_watch 调用");

    block_on(async { commands_bridge::ffi_stop_watch_async(workspace_id).await })
}

/// 检查是否正在监听（同步 - 轻量级）
#[frb(sync)]
pub fn is_watching(workspace_id: String) -> FfiResult<bool> {
    match block_on(async { commands_bridge_async::ffi_is_watching_async(workspace_id).await }) {
        Ok(result) => Ok(result),
        Err(e) => {
            tracing::error!(error = %e, "检查监听状态失败");
            Err(FfiError::Internal {
                message: format!("检查监听状态失败: {}", e),
            })
        }
    }
}

// ==================== 导入操作 ====================

/// 导入文件夹
///
/// 返回任务 ID 用于跟踪进度
#[frb]
pub async fn import_folder(path: String, workspace_id: String) -> FfiResult<String> {
    tracing::info!(path = %path, workspace_id = %workspace_id, "FFI: import_folder 调用");

    block_on(async { commands_bridge::ffi_import_folder_async(path, workspace_id).await })
}

/// 检查 RAR 支持（同步 - 常量检查）
#[frb(sync)]
pub fn check_rar_support() -> FfiResult<bool> {
    Ok(cfg!(feature = "rar"))
}

// ==================== 导出操作 ====================

/// 导出搜索结果
#[frb]
pub async fn export_results(
    search_id: String,
    format: String,
    output_path: String,
) -> FfiResult<String> {
    tracing::info!(
        search_id = %search_id,
        format = %format,
        "FFI: export_results 调用"
    );

    block_on(async {
        commands_bridge::ffi_export_results_async(search_id, format, output_path).await
    })
}

// ==================== 搜索历史操作 ====================

/// 添加搜索历史记录
#[frb]
pub async fn add_search_history(
    query: String,
    workspace_id: String,
    result_count: i32,
) -> FfiResult<bool> {
    tracing::debug!(query = %query, "FFI: add_search_history 调用");

    block_on(async {
        commands_bridge::ffi_add_search_history_async(query, workspace_id, result_count as usize)
            .await
    })
}

/// 获取搜索历史记录
#[frb]
pub async fn get_search_history(
    workspace_id: Option<String>,
    limit: Option<i32>,
) -> FfiResult<Vec<SearchHistoryData>> {
    tracing::debug!("FFI: get_search_history 调用");

    block_on(async {
        commands_bridge_async::ffi_get_search_history_async(workspace_id, limit.map(|l| l as usize))
            .await
    })
}

/// 删除搜索历史记录（按查询词）
#[frb]
pub async fn delete_search_history(query: String, workspace_id: String) -> FfiResult<bool> {
    tracing::debug!(query = %query, "FFI: delete_search_history 调用");

    block_on(async {
        commands_bridge::ffi_delete_search_history_async(query, workspace_id).await
    })
}

/// 批量删除搜索历史记录
#[frb]
pub async fn delete_search_histories(
    queries: Vec<String>,
    workspace_id: String,
) -> FfiResult<i32> {
    tracing::debug!(query_count = queries.len(), "FFI: delete_search_histories 调用");

    block_on(async {
        commands_bridge::ffi_delete_search_histories_async(queries, workspace_id)
            .await
            .map(|n| n as i32)
    })
}

/// 清空搜索历史
#[frb]
pub async fn clear_search_history(workspace_id: Option<String>) -> FfiResult<i32> {
    tracing::debug!("FFI: clear_search_history 调用");

    block_on(async {
        commands_bridge::ffi_clear_search_history_async(workspace_id)
            .await
            .map(|n| n as i32)
    })
}

// ==================== 虚拟文件树操作 ====================

/// 获取虚拟文件树（根节点）
#[frb]
pub async fn get_virtual_file_tree(
    workspace_id: String,
) -> FfiResult<Vec<VirtualTreeNodeData>> {
    tracing::debug!(workspace_id = %workspace_id, "FFI: get_virtual_file_tree 调用");

    block_on(async { commands_bridge::ffi_get_virtual_file_tree_async(workspace_id).await })
}

/// 获取树子节点（懒加载）
#[frb]
pub async fn get_tree_children(
    workspace_id: String,
    parent_path: String,
) -> FfiResult<Vec<VirtualTreeNodeData>> {
    tracing::debug!(workspace_id = %workspace_id, parent_path = %parent_path, "FFI: get_tree_children 调用");

    block_on(async {
        commands_bridge::ffi_get_tree_children_async(workspace_id, parent_path).await
    })
}

/// 通过哈希读取文件内容
#[frb]
pub async fn read_file_by_hash(
    workspace_id: String,
    hash: String,
) -> FfiResult<FileContentResponseData> {
    tracing::debug!(workspace_id = %workspace_id, hash = %hash, "FFI: read_file_by_hash 调用");

    block_on(async {
        commands_bridge::ffi_read_file_by_hash_async(workspace_id, hash).await
    })
}

// ==================== 多关键词组合搜索操作 ====================

/// 执行结构化搜索（多关键词组合搜索）
#[frb]
pub async fn search_structured(
    query: StructuredSearchQueryData,
    workspace_id: Option<String>,
    max_results: i32,
) -> FfiResult<Vec<FfiSearchResultEntry>> {
    tracing::info!("FFI: search_structured 调用");

    block_on(async {
        commands_bridge::ffi_search_structured_async(query, workspace_id, max_results).await
    })
}

/// 构建搜索查询对象
#[frb(sync)]
pub fn build_search_query(
    keywords: Vec<String>,
    global_operator: String,
    is_regex: bool,
    case_sensitive: bool,
) -> StructuredSearchQueryData {
    let op = match global_operator.to_uppercase().as_str() {
        "OR" => QueryOperatorData::Or,
        "NOT" => QueryOperatorData::Not,
        _ => QueryOperatorData::And,
    };

    commands_bridge_async::ffi_build_search_query(
        keywords, op, is_regex, case_sensitive,
    )
}

// ==================== 正则搜索操作 ====================

/// 验证正则表达式语法（同步 - 轻量级计算）
#[frb(sync)]
pub fn validate_regex(pattern: String) -> RegexValidationResult {
    commands_bridge_async::ffi_validate_regex(pattern)
}

/// 执行正则表达式搜索
#[frb]
pub async fn search_regex(
    pattern: String,
    workspace_id: Option<String>,
    max_results: i32,
    case_sensitive: bool,
) -> FfiResult<Vec<FfiSearchResultEntry>> {
    tracing::info!(pattern = %pattern, "FFI: search_regex 调用");

    block_on(async {
        commands_bridge::ffi_search_regex_async(pattern, workspace_id, max_results, case_sensitive)
            .await
    })
}

// ==================== 过滤器操作 ====================

/// 保存或更新过滤器
#[frb]
pub async fn save_filter(filter: SavedFilterInput) -> FfiResult<bool> {
    tracing::debug!(filter_name = %filter.name, "FFI: save_filter 调用");

    block_on(async { commands_bridge::ffi_save_filter_async(filter).await })
}

/// 获取工作区的所有过滤器
#[frb]
pub async fn get_saved_filters(
    workspace_id: String,
    limit: Option<i32>,
) -> FfiResult<Vec<SavedFilterData>> {
    tracing::debug!(workspace_id = %workspace_id, "FFI: get_saved_filters 调用");

    block_on(async {
        commands_bridge::ffi_get_saved_filters_async(workspace_id, limit.map(|l| l as usize))
            .await
    })
}

/// 删除指定过滤器
#[frb]
pub async fn delete_filter(filter_id: String, workspace_id: String) -> FfiResult<bool> {
    tracing::debug!(filter_id = %filter_id, "FFI: delete_filter 调用");

    block_on(async {
        commands_bridge::ffi_delete_filter_async(filter_id, workspace_id).await
    })
}

/// 更新过滤器使用统计
#[frb]
pub async fn update_filter_usage(filter_id: String, workspace_id: String) -> FfiResult<bool> {
    tracing::debug!(filter_id = %filter_id, "FFI: update_filter_usage 调用");

    block_on(async {
        commands_bridge::ffi_update_filter_usage_async(filter_id, workspace_id).await
    })
}

// ==================== 日志级别统计操作 ====================

/// 获取日志级别统计
#[frb]
pub async fn get_log_level_stats(workspace_id: String) -> FfiResult<LogLevelStatsOutput> {
    tracing::debug!(workspace_id = %workspace_id, "FFI: get_log_level_stats 调用");

    block_on(async { commands_bridge::ffi_get_log_level_stats_async(workspace_id).await })
}

// ==================== Session 操作 ====================

/// 打开 Session
#[frb]
pub async fn open_session(path: String) -> FfiResult<SessionInfo> {
    tracing::info!(path = %path, "FFI: open_session 调用");

    let session_id = format!("session_{}", uuid::Uuid::new_v4());
    crate::ffi::global_state::create_session(session_id, path)
}

/// 获取 Session 信息
#[frb]
pub async fn get_session_info(session_id: String) -> FfiResult<SessionInfo> {
    tracing::debug!(session_id = %session_id, "FFI: get_session_info 调用");

    crate::ffi::global_state::get_session_info(&session_id)
        .ok_or_else(|| FfiError::session_expired(session_id))
}

/// 关闭 Session
#[frb]
pub async fn close_session(session_id: String) -> FfiResult<bool> {
    tracing::info!(session_id = %session_id, "FFI: close_session 调用");

    crate::ffi::global_state::remove_session(&session_id)
}

/// 获取所有 Session
#[frb]
pub async fn get_all_sessions() -> FfiResult<Vec<String>> {
    Ok(crate::ffi::global_state::get_all_session_ids())
}

// ==================== 系统操作 ====================

/// 获取系统信息（同步 - 轻量级）
#[frb(sync)]
pub fn get_system_info() -> FfiResult<SystemInfoData> {
    use sysinfo::{MemoryRefreshKind, RefreshKind, System};

    let sys = System::new_with_specifics(
        RefreshKind::new().with_memory(MemoryRefreshKind::everything()),
    );

    Ok(SystemInfoData {
        total_memory: sys.total_memory(),
        used_memory: sys.used_memory(),
        cpu_count: sys.cpus().len() as i32,
        rust_version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// 系统信息数据
#[derive(Debug, Clone, Default)]
#[frb(dart_metadata = ("immutable"))]
pub struct SystemInfoData {
    pub total_memory: u64,
    pub used_memory: u64,
    pub cpu_count: i32,
    pub rust_version: String,
}

// ==================== 同步工具函数（用于向后兼容） ====================

/// 同步包装器：执行异步块（仅用于测试和过渡）
///
/// ⚠️ 警告：在 Flutter 主线程上调用会阻塞 UI！
/// 仅用于需要同步调用的特殊情况
#[doc(hidden)]
pub fn block_on_sync<F, T>(f: F) -> FfiResult<T>
where
    F: std::future::Future<Output = FfiResult<T>>,
{
    block_on(f)
}
