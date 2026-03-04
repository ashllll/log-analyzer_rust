//! Flutter Rust Bridge 主实现
//!
//! 使用 flutter_rust_bridge 宏定义 FFI 接口
//!
//! ## 宏说明
//!
//! - `#[frb(init)]`: 初始化函数，返回全局上下文
//! - `#[frb(sync)]`: 同步函数
//! - `pub extern "C"`: 导出为 C ABI
//!
//! ## 事件流说明
//!
//! flutter_rust_bridge 2.x 不支持 `#[frb(stream)]`，事件流需要通过以下方式实现：
//! 1. Flutter 端使用 Tauri invoke + 事件监听
//! 2. 实现 HTTP 长轮询 API
//! 3. WebSocket 连接
//!
//! ## 错误处理说明
//!
//! flutter_rust_bridge 2.x 会自动将 Rust panic 转换为 Dart 异常。
//! 因此，本模块的函数直接返回值，错误时使用 `.expect()` 或 `?` + panic。

use crate::ffi::commands_bridge;
use crate::ffi::types::*;
use flutter_rust_bridge::frb;

/// FFI 专用结果类型
///
/// 在 flutter_rust_bridge 2.x 中，`Result<T, String>` 会被映射为不透明类型，
/// Dart 端无法直接访问其内容。
///
/// 因此，我们采用以下策略：
/// - 对于简单返回值，直接返回值，错误时 panic（FRB 会将其转为 Dart 异常）
/// - 对于复杂返回值，使用 Option<T> 包装，None 表示错误
///
/// 注意：这个类型定义保留是为了向后兼容，新代码不应使用。
pub type FfiResult<T> = std::result::Result<T, String>;

/// 将 FfiResult 转换为直接值，错误时 panic
///
/// FRB 2.x 会将 panic 转换为 Dart 异常
#[inline]
fn unwrap_result<T>(result: FfiResult<T>, context: &str) -> T {
    result.unwrap_or_else(|e| panic!("{}: {}", context, e))
}

/// FFI 桥接上下文
///
/// 包含全局状态引用，用于 FFI 调用
#[derive(Clone)]
pub struct BridgeContext {
    /// 初始化时间戳（Unix 时间戳，秒）
    pub init_time: i64,
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
        }
    }

    /// 获取运行时间（秒）
    pub fn uptime_seconds(&self) -> i64 {
        chrono::Utc::now().timestamp() - self.init_time
    }
}

// Flutter Rust Bridge 宏定义
// 注意：这里的 #[frb()] 会由 flutter_rust_codegen 生成对应的 Dart 代码

/// 初始化桥接
///
/// 这是 FFI 的入口点，返回全局上下文
#[frb(init)]
pub fn init_bridge() -> BridgeContext {
    tracing::info!("Flutter FFI Bridge 初始化");
    BridgeContext::new()
}

/// 健康检查
///
/// 用于验证 FFI 连接是否正常工作
#[frb(sync)]
pub fn health_check() -> String {
    tracing::debug!("FFI 健康检查调用");
    "OK".to_string()
}

// ==================== 工作区操作 ====================

/// 获取工作区列表
///
/// 返回所有可用的工作区
#[frb(sync)]
pub fn get_workspaces() -> Vec<WorkspaceData> {
    tracing::debug!("FFI: get_workspaces 调用");
    // 从应用数据目录获取工作区列表
    // 目前返回空列表，实际实现需要扫描 workspaces 目录
    vec![]
}

/// 创建工作区
///
/// 创建新的工作区并返回其 ID
#[frb(sync)]
pub fn create_workspace(name: String, path: String) -> String {
    tracing::info!(name = %name, path = %path, "FFI: create_workspace 调用");
    // 使用导入功能创建工作区
    let workspace_id = format!("ws-{}", name.to_lowercase().replace([' ', '/', '\\'], "-"));
    unwrap_result(
        commands_bridge::ffi_import_folder(path, workspace_id),
        "创建工作区失败",
    )
}

/// 删除工作区
#[frb(sync)]
pub fn delete_workspace(workspace_id: String) -> bool {
    unwrap_result(
        commands_bridge::ffi_delete_workspace(workspace_id),
        "删除工作区失败",
    )
}

/// 刷新工作区
///
/// 返回任务 ID 用于跟踪进度
#[frb(sync)]
pub fn refresh_workspace(workspace_id: String, path: String) -> String {
    unwrap_result(
        commands_bridge::ffi_refresh_workspace(workspace_id, path),
        "刷新工作区失败",
    )
}

/// 获取工作区状态
#[frb(sync)]
pub fn get_workspace_status(workspace_id: String) -> WorkspaceStatusData {
    unwrap_result(
        commands_bridge::ffi_get_workspace_status(workspace_id),
        "获取工作区状态失败",
    )
}

// ==================== 搜索操作 ====================

/// 执行日志搜索
///
/// 返回搜索 ID 用于获取结果
#[frb(sync)]
pub fn search_logs(
    query: String,
    workspace_id: Option<String>,
    max_results: i32,
    filters: Option<String>,
) -> String {
    unwrap_result(
        commands_bridge::ffi_search_logs(query, workspace_id, max_results, filters),
        "搜索失败",
    )
}

/// 取消搜索
#[frb(sync)]
pub fn cancel_search(search_id: String) -> bool {
    unwrap_result(
        commands_bridge::ffi_cancel_search(search_id),
        "取消搜索失败",
    )
}

/// 获取活跃搜索数量
#[frb(sync)]
pub fn get_active_searches_count() -> i32 {
    commands_bridge::ffi_get_active_searches_count().unwrap_or(0)
}

// ==================== 关键词操作 ====================

/// 获取关键词列表
#[frb(sync)]
pub fn get_keywords() -> Vec<KeywordGroupData> {
    commands_bridge::ffi_get_keywords().unwrap_or_default()
}

/// 添加关键词组
#[frb(sync)]
pub fn add_keyword_group(group: KeywordGroupInput) -> bool {
    unwrap_result(
        commands_bridge::ffi_add_keyword_group(group),
        "添加关键词组失败",
    )
}

/// 更新关键词组
#[frb(sync)]
pub fn update_keyword_group(group_id: String, group: KeywordGroupInput) -> bool {
    unwrap_result(
        commands_bridge::ffi_update_keyword_group(group_id, group),
        "更新关键词组失败",
    )
}

/// 删除关键词组
#[frb(sync)]
pub fn delete_keyword_group(group_id: String) -> bool {
    unwrap_result(
        commands_bridge::ffi_delete_keyword_group(group_id),
        "删除关键词组失败",
    )
}

// ==================== 任务操作 ====================

/// 获取任务指标
#[frb(sync)]
pub fn get_task_metrics() -> TaskMetricsData {
    unwrap_result(commands_bridge::ffi_get_task_metrics(), "获取任务指标失败")
}

/// 取消任务
#[frb(sync)]
pub fn cancel_task(task_id: String) -> bool {
    unwrap_result(commands_bridge::ffi_cancel_task(task_id), "取消任务失败")
}

// ==================== 配置操作 ====================

/// 加载配置
#[frb(sync)]
pub fn load_config() -> ConfigData {
    commands_bridge::ffi_load_config().unwrap_or_default()
}

/// 保存配置
#[frb(sync)]
pub fn save_config(config: ConfigData) -> bool {
    unwrap_result(commands_bridge::ffi_save_config(config), "保存配置失败")
}

// ==================== 性能监控 ====================

/// 获取性能指标
#[frb(sync)]
pub fn get_performance_metrics(time_range: String) -> PerformanceMetricsData {
    commands_bridge::ffi_get_performance_metrics(time_range).unwrap_or_default()
}

// ==================== 文件监听 ====================

/// 启动文件监听
#[frb(sync)]
pub fn start_watch(workspace_id: String, paths: Vec<String>, recursive: bool) -> bool {
    unwrap_result(
        commands_bridge::ffi_start_watch(workspace_id, paths, recursive),
        "启动文件监听失败",
    )
}

/// 停止文件监听
#[frb(sync)]
pub fn stop_watch(workspace_id: String) -> bool {
    unwrap_result(
        commands_bridge::ffi_stop_watch(workspace_id),
        "停止文件监听失败",
    )
}

/// 检查是否正在监听
#[frb(sync)]
pub fn is_watching(workspace_id: String) -> bool {
    commands_bridge::ffi_is_watching(workspace_id).unwrap_or(false)
}

// ==================== 导入操作 ====================

/// 导入文件夹
///
/// 返回任务 ID 用于跟踪进度
#[frb(sync)]
pub fn import_folder(path: String, workspace_id: String) -> String {
    unwrap_result(
        commands_bridge::ffi_import_folder(path, workspace_id),
        "导入文件夹失败",
    )
}

/// 检查 RAR 支持
#[frb(sync)]
pub fn check_rar_support() -> bool {
    tracing::debug!("FFI: check_rar_support 调用");
    cfg!(feature = "rar")
}

// ==================== 导出操作 ====================

/// 导出搜索结果
#[frb(sync)]
pub fn export_results(search_id: String, format: String, output_path: String) -> String {
    unwrap_result(
        commands_bridge::ffi_export_results(search_id, format, output_path),
        "导出结果失败",
    )
}

// ==================== 搜索历史操作 ====================

/// 添加搜索历史记录
///
/// 将搜索查询添加到历史记录
#[frb(sync)]
pub fn add_search_history(query: String, workspace_id: String, result_count: i32) -> bool {
    unwrap_result(
        commands_bridge::ffi_add_search_history(query, workspace_id, result_count as usize),
        "添加搜索历史失败",
    )
}

/// 获取搜索历史记录
///
/// 获取指定工作区或所有工作区的搜索历史
#[frb(sync)]
pub fn get_search_history(workspace_id: Option<String>, limit: Option<i32>) -> Vec<SearchHistoryData> {
    unwrap_result(
        commands_bridge::ffi_get_search_history(workspace_id, limit.map(|l| l as usize)),
        "获取搜索历史失败",
    )
}

/// 删除搜索历史记录（按查询词）
///
/// 删除指定工作区中特定查询的历史记录
#[frb(sync)]
pub fn delete_search_history(query: String, workspace_id: String) -> bool {
    unwrap_result(
        commands_bridge::ffi_delete_search_history(query, workspace_id),
        "删除搜索历史失败",
    )
}

/// 批量删除搜索历史记录（按查询词列表）
///
/// 批量删除指定工作区中多个查询的历史记录
#[frb(sync)]
pub fn delete_search_histories(queries: Vec<String>, workspace_id: String) -> i32 {
    unwrap_result(
        commands_bridge::ffi_delete_search_histories(queries, workspace_id),
        "批量删除搜索历史失败",
    )
}

/// 清空搜索历史
///
/// 清空指定工作区或所有工作区的搜索历史
#[frb(sync)]
pub fn clear_search_history(workspace_id: Option<String>) -> i32 {
    unwrap_result(
        commands_bridge::ffi_clear_search_history(workspace_id),
        "清空搜索历史失败",
    )
}

// ==================== 虚拟文件树操作 ====================

/// 获取虚拟文件树（根节点）
///
/// 获取指定工作区的虚拟文件树结构
///
/// # 参数
///
/// * `workspace_id` - 工作区 ID
///
/// # 返回
///
/// 返回根节点列表
#[frb(sync)]
pub fn get_virtual_file_tree(workspace_id: String) -> Vec<VirtualTreeNodeData> {
    unwrap_result(
        commands_bridge::ffi_get_virtual_file_tree(workspace_id),
        "获取虚拟文件树失败",
    )
}

/// 获取树子节点（懒加载）
///
/// 获取指定父节点下的子节点
///
/// # 参数
///
/// * `workspace_id` - 工作区 ID
/// * `parent_path` - 父节点路径
///
/// # 返回
///
/// 返回子节点列表
#[frb(sync)]
pub fn get_tree_children(
    workspace_id: String,
    parent_path: String,
) -> Vec<VirtualTreeNodeData> {
    unwrap_result(
        commands_bridge::ffi_get_tree_children(workspace_id, parent_path),
        "获取子节点失败",
    )
}

/// 通过哈希读取文件内容
///
/// 从 CAS 存储读取指定哈希的文件内容
///
/// # 参数
///
/// * `workspace_id` - 工作区 ID
/// * `hash` - 文件 SHA-256 哈希
///
/// # 返回
///
/// 返回文件内容响应
#[frb(sync)]
pub fn read_file_by_hash(
    workspace_id: String,
    hash: String,
) -> FileContentResponseData {
    unwrap_result(
        commands_bridge::ffi_read_file_by_hash(workspace_id, hash),
        "读取文件失败",
    )
}
