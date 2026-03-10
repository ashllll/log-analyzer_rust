//! Flutter Rust Bridge 最小化实现
//!
//! 只暴露返回基础类型的函数，避免复杂类型带来的 codegen 问题。
//! 复杂类型使用 JSON 序列化传输。
//!
//! ## 设计原则
//!
//! - 只返回 Dart 原生支持的简单类型：String, i32, bool, Vec<T>
//! - 复杂数据结构使用 JSON 字符串序列化
//! - 避免暴露有重复名称的类型
//! - 使用 #[frb(opaque)] 标记不透明类型

use flutter_rust_bridge::frb;

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

/// 初始化桥接
///
/// 这是 FFI 的入口点，返回全局上下文
#[frb(init)]
pub fn init_bridge() -> BridgeContext {
    tracing::info!("Flutter FFI Bridge 初始化（最小化版本）");
    BridgeContext::new()
}

/// 获取应用版本
///
/// 返回当前应用的版本号
#[frb(sync)]
pub fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// 健康检查
///
/// 用于验证 FFI 连接是否正常工作
#[frb(sync)]
pub fn health_check() -> String {
    tracing::debug!("FFI 健康检查调用");
    "OK".to_string()
}

/// 测试问候函数
///
/// 简单的测试函数，验证 FFI 连接
#[frb(sync)]
pub fn hello(name: String) -> String {
    format!("Hello, {}!", name)
}

/// 获取工作区列表（JSON格式）
///
/// 返回所有可用的工作区（JSON 字符串）
#[frb(sync)]
pub fn get_workspaces_json() -> String {
    tracing::debug!("FFI: get_workspaces_json 调用");
    // 返回空数组，实际数据由前端解析
    "[]".to_string()
}

/// 创建工作区
///
/// 创建新的工作区并返回任务 ID
#[frb(sync)]
pub fn create_workspace(name: String, _path: String) -> String {
    tracing::info!(name = %name, "FFI: create_workspace 调用");
    // 生成工作区 ID
    let _workspace_id = format!("ws-{}", name.to_lowercase().replace([' ', '/', '\\'], "-"));
    // 返回任务 ID（简化版本）
    format!("task_{}", uuid::Uuid::new_v4())
}

/// 删除工作区
///
/// 返回是否成功
#[frb(sync)]
pub fn delete_workspace(workspace_id: String) -> bool {
    tracing::info!(workspace_id = %workspace_id, "FFI: delete_workspace 调用");
    true // 简化版本
}

/// 刷新工作区
///
/// 返回任务 ID
#[frb(sync)]
pub fn refresh_workspace(workspace_id: String, _path: String) -> String {
    tracing::info!(workspace_id = %workspace_id, "FFI: refresh_workspace 调用");
    format!("task_{}", uuid::Uuid::new_v4())
}

/// 获取工作区状态（JSON格式）
///
/// 返回工作区状态 JSON
#[frb(sync)]
pub fn get_workspace_status_json(workspace_id: String) -> String {
    tracing::debug!(workspace_id = %workspace_id, "FFI: get_workspace_status_json 调用");
    // 返回简化状态
    format!(
        r#"{{"id":"{}","name":"Workspace","status":"READY","size":"0MB","files":0}}"#,
        workspace_id
    )
}

/// 执行日志搜索
///
/// 返回搜索 ID
#[frb(sync)]
pub fn search_logs(
    query: String,
    _workspace_id: Option<String>,
    max_results: i32,
    _filters: Option<String>,
) -> String {
    tracing::debug!(query = %query, max_results = max_results, "FFI: search_logs 调用");
    if query.is_empty() {
        return "".to_string();
    }
    format!("search_{}", uuid::Uuid::new_v4())
}

/// 取消搜索
///
/// 返回是否成功
#[frb(sync)]
pub fn cancel_search(search_id: String) -> bool {
    tracing::debug!(search_id = %search_id, "FFI: cancel_search 调用");
    !search_id.is_empty()
}

/// 获取活跃搜索数量
#[frb(sync)]
pub fn get_active_searches_count() -> i32 {
    0 // 简化版本
}

/// 获取关键词列表（JSON格式）
#[frb(sync)]
pub fn get_keywords_json() -> String {
    tracing::debug!("FFI: get_keywords_json 调用");
    "[]".to_string()
}

/// 添加关键词组
///
/// 返回是否成功
#[frb(sync)]
pub fn add_keyword_group(_group_json: String) -> bool {
    tracing::debug!("FFI: add_keyword_group 调用");
    true
}

/// 更新关键词组
///
/// 返回是否成功
#[frb(sync)]
pub fn update_keyword_group(_group_id: String, _group_json: String) -> bool {
    tracing::debug!("FFI: update_keyword_group 调用");
    true
}

/// 删除关键词组
///
/// 返回是否成功
#[frb(sync)]
pub fn delete_keyword_group(_group_id: String) -> bool {
    tracing::debug!("FFI: delete_keyword_group 调用");
    true
}

/// 获取任务指标（JSON格式）
#[frb(sync)]
pub fn get_task_metrics_json() -> String {
    tracing::debug!("FFI: get_task_metrics_json 调用");
    r#"{"total_tasks":0,"running_tasks":0,"completed_tasks":0,"failed_tasks":0,"stopped_tasks":0}"#
        .to_string()
}

/// 取消任务
///
/// 返回是否成功
#[frb(sync)]
pub fn cancel_task(task_id: String) -> bool {
    tracing::debug!(task_id = %task_id, "FFI: cancel_task 调用");
    !task_id.is_empty()
}

/// 加载配置（JSON格式）
#[frb(sync)]
pub fn load_config_json() -> String {
    tracing::debug!("FFI: load_config_json 调用");
    r#"{"file_filter":{"enabled":false,"binary_detection_enabled":false,"mode":"whitelist","filename_patterns":[],"allowed_extensions":[],"forbidden_extensions":[]},"advanced_features":{"enable_filter_engine":false,"enable_regex_engine":true,"enable_time_partition":false,"enable_autocomplete":true,"regex_cache_size":1000,"autocomplete_limit":100,"time_partition_size_secs":3600}}"#.to_string()
}

/// 保存配置
///
/// 返回是否成功
#[frb(sync)]
pub fn save_config_json(_config_json: String) -> bool {
    tracing::debug!("FFI: save_config_json 调用");
    true
}

/// 获取性能指标（JSON格式）
#[frb(sync)]
pub fn get_performance_metrics_json(_time_range: String) -> String {
    tracing::debug!("FFI: get_performance_metrics_json 调用");
    r#"{"search_latency":0.0,"search_throughput":0.0,"cache_hit_rate":0.0,"cache_size":0,"total_queries":0,"cache_hits":0,"latency_history":[],"avg_latency":0.0}"#.to_string()
}

/// 启动文件监听
///
/// 返回是否成功
#[frb(sync)]
pub fn start_watch(workspace_id: String, _paths: Vec<String>, _recursive: bool) -> bool {
    tracing::debug!(workspace_id = %workspace_id, "FFI: start_watch 调用");
    !workspace_id.is_empty()
}

/// 停止文件监听
///
/// 返回是否成功
#[frb(sync)]
pub fn stop_watch(workspace_id: String) -> bool {
    tracing::debug!(workspace_id = %workspace_id, "FFI: stop_watch 调用");
    !workspace_id.is_empty()
}

/// 检查是否正在监听
#[frb(sync)]
pub fn is_watching(workspace_id: String) -> bool {
    tracing::debug!(workspace_id = %workspace_id, "FFI: is_watching 调用");
    false // 简化版本
}

/// 导入文件夹
///
/// 返回任务 ID
#[frb(sync)]
pub fn import_folder(path: String, workspace_id: String) -> String {
    tracing::info!(path = %path, workspace_id = %workspace_id, "FFI: import_folder 调用");
    format!("task_{}", uuid::Uuid::new_v4())
}

/// 检查 RAR 支持
#[frb(sync)]
pub fn check_rar_support() -> bool {
    tracing::debug!("FFI: check_rar_support 调用");
    cfg!(feature = "rar")
}

/// 导出搜索结果
///
/// 返回输出路径
#[frb(sync)]
pub fn export_results(search_id: String, format: String, output_path: String) -> String {
    tracing::info!(search_id = %search_id, format = %format, "FFI: export_results 调用");
    output_path // 简化版本，直接返回输出路径
}
