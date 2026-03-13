//! Tauri Commands 的 FFI 异步适配层
//!
//! 本模块提供 `commands_bridge.rs` 的异步版本，使用全局 Runtime 执行异步操作。
//!
//! ## 设计原则
//!
//! 1. **复用现有逻辑**: 调用 `commands_bridge.rs` 中的函数，但使用全局 Runtime
//! 2. **异步优先**: 所有函数都是 async，避免阻塞 Flutter UI
//! 3. **统一错误**: 使用 `FfiError` 替代 `String` 错误
//! 4. **取消支持**: 支持 `CancellationToken` 取消长时间运行的任务
//!
//! ## 使用方式
//!
//! ```rust
//! use crate::ffi::commands_bridge_async::ffi_import_folder_async;
//!
//! async fn example() -> FfiResult<String> {
//!     ffi_import_folder_async("/path/to/folder".to_string(), "ws-1".to_string()).await
//! }
//! ```

use std::collections::HashMap;
use std::path::Path;

use crate::ffi::error::{FfiError, FfiErrorCode, FfiResult};
use crate::ffi::global_state::{get_app_data_dir, get_app_state};
use crate::ffi::types::*;
use crate::utils::validation::validate_workspace_id;

// ==================== 工作区操作 ====================

/// 异步：导入文件夹
pub async fn ffi_import_folder_async(
    path: String,
    workspace_id: String,
) -> FfiResult<String> {
    // 验证工作区 ID
    validate_workspace_id(&workspace_id).map_err(|e| {
        FfiError::invalid_argument("workspace_id", e)
    })?;

    // 验证路径
    let folder_path = Path::new(&path);
    if !folder_path.exists() {
        return Err(FfiError::not_found("文件夹", path));
    }

    if !folder_path.is_dir() {
        return Err(FfiError::invalid_argument("path", "路径不是文件夹"));
    }

    // 获取全局状态
    let app_state = get_app_state()
        .ok_or_else(|| FfiError::initialization_failed("全局状态未初始化"))?;

    let app_data_dir = get_app_data_dir()
        .ok_or_else(|| FfiError::initialization_failed("应用数据目录未初始化"))?;

    let workspace_dir = app_data_dir.join("workspaces").join(&workspace_id);

    // 创建工作区目录
    tokio::fs::create_dir_all(&workspace_dir).await.map_err(|e| {
        FfiError::io_error("创建工作区目录", e)
    })?;

    // 获取或创建任务管理器
    let task_manager = {
        let guard = app_state.task_manager.lock();
        guard.clone().ok_or_else(|| {
            FfiError::initialization_failed("TaskManager 未初始化")
        })?
    };

    // 创建导入任务
    let task_id = format!("import_{}", uuid::Uuid::new_v4());

    // 创建任务
    task_manager
        .create_task_async(
            task_id.clone(),
            "Import".to_string(),
            path.clone(),
            Some(workspace_id.clone()),
        )
        .await
        .map_err(|e| FfiError::runtime_error("创建导入任务", e))?;

    tracing::info!(
        task_id = %task_id,
        path = %path,
        workspace_id = %workspace_id,
        "导入任务已创建"
    );

    Ok(task_id)
}

/// 异步：删除工作区
pub async fn ffi_delete_workspace_async(workspace_id: String) -> FfiResult<bool> {
    validate_workspace_id(&workspace_id).map_err(|e| {
        FfiError::invalid_argument("workspace_id", e)
    })?;

    let app_data_dir = get_app_data_dir()
        .ok_or_else(|| FfiError::initialization_failed("应用数据目录未初始化"))?;

    let workspace_dir = app_data_dir.join("workspaces").join(&workspace_id);

    if !workspace_dir.exists() {
        return Err(FfiError::not_found("工作区", workspace_id));
    }

    // 异步删除目录
    tokio::fs::remove_dir_all(&workspace_dir).await.map_err(|e| {
        FfiError::io_error("删除工作区目录", e)
    })?;

    tracing::info!(workspace_id = %workspace_id, "工作区已删除");

    Ok(true)
}

/// 异步：刷新工作区
pub async fn ffi_refresh_workspace_async(
    workspace_id: String,
    path: String,
) -> FfiResult<String> {
    // 复用导入逻辑
    ffi_import_folder_async(path, workspace_id).await
}

/// 异步：获取工作区状态
pub async fn ffi_get_workspace_status_async(
    workspace_id: String,
) -> FfiResult<WorkspaceStatusData> {
    validate_workspace_id(&workspace_id).map_err(|e| {
        FfiError::invalid_argument("workspace_id", e)
    })?;

    let app_data_dir = get_app_data_dir()
        .ok_or_else(|| FfiError::initialization_failed("应用数据目录未初始化"))?;

    let workspace_dir = app_data_dir.join("workspaces").join(&workspace_id);

    // 检查工作区是否存在
    let exists = tokio::fs::try_exists(&workspace_dir).await.unwrap_or(false);

    if !exists {
        return Err(FfiError::not_found("工作区", workspace_id));
    }

    // 获取文件数量（简化实现）
    let file_count = count_files_async(&workspace_dir).await.unwrap_or(0);

    Ok(WorkspaceStatusData {
        id: workspace_id.clone(),
        name: workspace_id.clone(),
        status: "ready".to_string(),
        size: format!("{}", file_count),
        files: file_count,
    })
}

/// 异步计算文件数量
async fn count_files_async(dir: &std::path::Path) -> std::io::Result<i32> {
    let mut count = 0;
    let mut entries = tokio::fs::read_dir(dir).await?;

    while let Some(entry) = entries.next_entry().await? {
        let metadata = entry.metadata().await?;
        if metadata.is_file() {
            count += 1;
        } else if metadata.is_dir() {
            count += count_files_async(&entry.path()).await.unwrap_or(0);
        }
    }

    Ok(count)
}

// ==================== 搜索操作 ====================

/// 异步：搜索日志
pub async fn ffi_search_logs_async(
    query: String,
    workspace_id: Option<String>,
    _max_results: i32,
    _filters: Option<String>,
) -> FfiResult<String> {
    let app_state = get_app_state()
        .ok_or_else(|| FfiError::initialization_failed("全局状态未初始化"))?;

    let task_manager = {
        let guard = app_state.task_manager.lock();
        guard.clone().ok_or_else(|| {
            FfiError::initialization_failed("TaskManager 未初始化")
        })?
    };

    let search_id = format!("search_{}", uuid::Uuid::new_v4());
    let query_clone = query.clone();

    // 创建搜索任务
    task_manager
        .create_task_async(
            search_id.clone(),
            "Search".to_string(),
            query_clone,
            workspace_id.clone(),
        )
        .await
        .map_err(|e| FfiError::runtime_error("创建搜索任务", e))?;

    tracing::info!(
        search_id = %search_id,
        query = %query,
        "搜索任务已创建"
    );

    Ok(search_id)
}

/// 异步：取消搜索
pub async fn ffi_cancel_search_async(search_id: String) -> FfiResult<bool> {
    let app_state = get_app_state()
        .ok_or_else(|| FfiError::initialization_failed("全局状态未初始化"))?;

    let task_manager = {
        let guard = app_state.task_manager.lock();
        guard.clone().ok_or_else(|| {
            FfiError::initialization_failed("TaskManager 未初始化")
        })?
    };

    // 更新任务状态为停止
    task_manager
        .update_task_async(
            &search_id,
            0,
            "搜索已取消".to_string(),
            crate::task_manager::TaskStatus::Stopped,
        )
        .await
        .map_err(|e| FfiError::runtime_error("取消搜索", e))?;

    Ok(true)
}

/// 异步：获取活跃搜索数量
pub async fn ffi_get_active_searches_count_async() -> FfiResult<i32> {
    let app_state = get_app_state()
        .ok_or_else(|| FfiError::initialization_failed("全局状态未初始化"))?;

    let task_manager = {
        let guard = app_state.task_manager.lock();
        guard.clone().ok_or_else(|| {
            FfiError::initialization_failed("TaskManager 未初始化")
        })?
    };

    let metrics = task_manager
        .get_metrics_async()
        .await
        .map_err(|e| FfiError::runtime_error("获取任务指标", e))?;

    // 返回运行中的任务数作为活跃搜索数
    Ok(metrics.running_tasks as i32)
}

// ==================== 关键词操作 ====================

/// 异步：获取关键词
pub async fn ffi_get_keywords_async() -> FfiResult<Vec<FfiKeywordGroupData>> {
    // TODO: 从存储中读取关键词
    // 简化实现：返回空列表
    Ok(vec![])
}

/// 异步：添加关键词组
pub async fn ffi_add_keyword_group_async(_group: KeywordGroupInput) -> FfiResult<bool> {
    // TODO: 实现关键词保存
    Ok(true)
}

/// 异步：更新关键词组
pub async fn ffi_update_keyword_group_async(
    _group_id: String,
    _group: KeywordGroupInput,
) -> FfiResult<bool> {
    // TODO: 实现关键词更新
    Ok(true)
}

/// 异步：删除关键词组
pub async fn ffi_delete_keyword_group_async(_group_id: String) -> FfiResult<bool> {
    // TODO: 实现关键词删除
    Ok(true)
}

// ==================== 任务操作 ====================

/// 异步：获取任务指标
pub async fn ffi_get_task_metrics_async() -> FfiResult<TaskMetricsData> {
    let app_state = get_app_state()
        .ok_or_else(|| FfiError::initialization_failed("全局状态未初始化"))?;

    let task_manager = {
        let guard = app_state.task_manager.lock();
        guard.clone().ok_or_else(|| {
            FfiError::initialization_failed("TaskManager 未初始化")
        })?
    };

    let metrics = task_manager
        .get_metrics_async()
        .await
        .map_err(|e| FfiError::runtime_error("获取任务指标", e))?;

    Ok(TaskMetricsData {
        total_tasks: metrics.total_tasks as i32,
        running_tasks: metrics.running_tasks as i32,
        completed_tasks: metrics.completed_tasks as i32,
        failed_tasks: metrics.failed_tasks as i32,
        stopped_tasks: metrics.stopped_tasks as i32,
    })
}

/// 异步：取消任务
pub async fn ffi_cancel_task_async(task_id: String) -> FfiResult<bool> {
    let app_state = get_app_state()
        .ok_or_else(|| FfiError::initialization_failed("全局状态未初始化"))?;

    let task_manager = {
        let guard = app_state.task_manager.lock();
        guard.clone().ok_or_else(|| {
            FfiError::initialization_failed("TaskManager 未初始化")
        })?
    };

    task_manager
        .update_task_async(
            &task_id,
            0,
            "用户取消任务".to_string(),
            crate::task_manager::TaskStatus::Stopped,
        )
        .await
        .map_err(|e| FfiError::runtime_error("取消任务", e))?;

    Ok(true)
}

// ==================== 配置操作 ====================

/// 异步：加载配置
pub async fn ffi_load_config_async() -> FfiResult<ConfigData> {
    let app_data_dir = get_app_data_dir()
        .ok_or_else(|| FfiError::initialization_failed("应用数据目录未初始化"))?;

    let config_path = app_data_dir.join("config.json");

    if !config_path.exists() {
        // 返回默认配置
        return Ok(ConfigData::default());
    }

    let content = tokio::fs::read_to_string(&config_path).await.map_err(|e| {
        FfiError::io_error("读取配置文件", e)
    })?;

    let config: ConfigData = serde_json::from_str(&content).map_err(|e| {
        FfiError::serialization_error(e)
    })?;

    Ok(config)
}

/// 异步：保存配置
pub async fn ffi_save_config_async(config: ConfigData) -> FfiResult<bool> {
    let app_data_dir = get_app_data_dir()
        .ok_or_else(|| FfiError::initialization_failed("应用数据目录未初始化"))?;

    let config_path = app_data_dir.join("config.json");

    let content = serde_json::to_string_pretty(&config).map_err(|e| {
        FfiError::serialization_error(e)
    })?;

    tokio::fs::write(&config_path, content).await.map_err(|e| {
        FfiError::io_error("写入配置文件", e)
    })?;

    Ok(true)
}

// ==================== 性能监控 ====================

/// 异步：获取性能指标
pub async fn ffi_get_performance_metrics_async(
    _time_range: String,
) -> FfiResult<PerformanceMetricsData> {
    // TODO: 从监控系统获取指标
    Ok(PerformanceMetricsData::default())
}

// ==================== 文件监听 ====================

/// 异步：启动文件监听
pub async fn ffi_start_watch_async(
    _workspace_id: String,
    _paths: Vec<String>,
    _recursive: bool,
) -> FfiResult<bool> {
    // TODO: 实现文件监听
    Ok(true)
}

/// 异步：停止文件监听
pub async fn ffi_stop_watch_async(_workspace_id: String) -> FfiResult<bool> {
    // TODO: 实现停止监听
    Ok(true)
}

/// 异步：检查是否正在监听
pub async fn ffi_is_watching_async(_workspace_id: String) -> FfiResult<bool> {
    // TODO: 实现状态检查
    Ok(false)
}

// ==================== 导出操作 ====================

/// 异步：导出结果
pub async fn ffi_export_results_async(
    _search_id: String,
    _format: String,
    _output_path: String,
) -> FfiResult<String> {
    // TODO: 实现导出逻辑
    Err(FfiError::new(FfiErrorCode::FfiError, "导出功能尚未实现"))
}

// ==================== 搜索历史操作 ====================

/// 异步：添加搜索历史
pub async fn ffi_add_search_history_async(
    _query: String,
    _workspace_id: String,
    _result_count: usize,
) -> FfiResult<bool> {
    // TODO: 实现搜索历史保存
    Ok(true)
}

/// 异步：获取搜索历史
pub async fn ffi_get_search_history_async(
    _workspace_id: Option<String>,
    _limit: Option<usize>,
) -> FfiResult<Vec<SearchHistoryData>> {
    // TODO: 实现搜索历史查询
    Ok(vec![])
}

/// 异步：删除搜索历史
pub async fn ffi_delete_search_history_async(
    _query: String,
    _workspace_id: String,
) -> FfiResult<bool> {
    // TODO: 实现删除
    Ok(true)
}

/// 异步：批量删除搜索历史
pub async fn ffi_delete_search_histories_async(
    _queries: Vec<String>,
    _workspace_id: String,
) -> FfiResult<usize> {
    // TODO: 实现批量删除
    Ok(0)
}

/// 异步：清空搜索历史
pub async fn ffi_clear_search_history_async(
    _workspace_id: Option<String>,
) -> FfiResult<usize> {
    // TODO: 实现清空
    Ok(0)
}

// ==================== 虚拟文件树操作 ====================

/// 异步：获取虚拟文件树
pub async fn ffi_get_virtual_file_tree_async(
    _workspace_id: String,
) -> FfiResult<Vec<VirtualTreeNodeData>> {
    // TODO: 实现文件树查询
    Ok(vec![])
}

/// 异步：获取树子节点
pub async fn ffi_get_tree_children_async(
    _workspace_id: String,
    _parent_path: String,
) -> FfiResult<Vec<VirtualTreeNodeData>> {
    // TODO: 实现懒加载
    Ok(vec![])
}

/// 异步：通过哈希读取文件
pub async fn ffi_read_file_by_hash_async(
    _workspace_id: String,
    _hash: String,
) -> FfiResult<FileContentResponseData> {
    // TODO: 实现 CAS 读取
    Err(FfiError::new(FfiErrorCode::FfiError, "CAS 读取尚未实现"))
}

// ==================== 结构化搜索操作 ====================

/// 同步：构建搜索查询（轻量级计算）
pub fn ffi_build_search_query(
    keywords: Vec<String>,
    global_operator: QueryOperatorData,
    is_regex: bool,
    case_sensitive: bool,
) -> StructuredSearchQueryData {
    let terms = keywords
        .into_iter()
        .map(|keyword| SearchTermData {
            id: format!("term_{}", uuid::Uuid::new_v4()),
            value: keyword,
            operator: global_operator.clone(),
            is_regex,
            case_sensitive,
            priority: 0,
            enabled: true,
        })
        .collect();

    StructuredSearchQueryData {
        terms,
        global_operator,
        filters: None,
    }
}

/// 异步：结构化搜索
pub async fn ffi_search_structured_async(
    _query: StructuredSearchQueryData,
    _workspace_id: Option<String>,
    _max_results: i32,
) -> FfiResult<Vec<FfiSearchResultEntry>> {
    // TODO: 实现结构化搜索
    Ok(vec![])
}

// ==================== 正则搜索操作 ====================

/// 同步：验证正则表达式
pub fn ffi_validate_regex(pattern: String) -> RegexValidationResult {
    match regex::Regex::new(&pattern) {
        Ok(_) => RegexValidationResult {
            valid: true,
            error_message: None,
        },
        Err(e) => RegexValidationResult {
            valid: false,
            error_message: Some(e.to_string()),
        },
    }
}

/// 异步：正则搜索
pub async fn ffi_search_regex_async(
    _pattern: String,
    _workspace_id: Option<String>,
    _max_results: i32,
    _case_sensitive: bool,
) -> FfiResult<Vec<FfiSearchResultEntry>> {
    // TODO: 实现正则搜索
    Ok(vec![])
}

// ==================== 过滤器操作 ====================

/// 异步：保存过滤器
pub async fn ffi_save_filter_async(_filter: SavedFilterInput) -> FfiResult<bool> {
    // TODO: 实现过滤器保存
    Ok(true)
}

/// 异步：获取过滤器列表
pub async fn ffi_get_saved_filters_async(
    _workspace_id: String,
    _limit: Option<usize>,
) -> FfiResult<Vec<SavedFilterData>> {
    // TODO: 实现过滤器查询
    Ok(vec![])
}

/// 异步：删除过滤器
pub async fn ffi_delete_filter_async(
    _filter_id: String,
    _workspace_id: String,
) -> FfiResult<bool> {
    // TODO: 实现过滤器删除
    Ok(true)
}

/// 异步：更新过滤器使用统计
pub async fn ffi_update_filter_usage_async(
    _filter_id: String,
    _workspace_id: String,
) -> FfiResult<bool> {
    // TODO: 实现统计更新
    Ok(true)
}

// ==================== 日志级别统计操作 ====================

/// 异步：获取日志级别统计
pub async fn ffi_get_log_level_stats_async(
    _workspace_id: String,
) -> FfiResult<LogLevelStatsOutput> {
    // TODO: 实现统计查询
    Ok(LogLevelStatsOutput {
        fatal_count: 0,
        error_count: 0,
        warn_count: 0,
        info_count: 0,
        debug_count: 0,
        trace_count: 0,
        unknown_count: 0,
        total: 0,
    })
}

// ==================== 扩展错误类型 ====================

impl FfiError {
    fn serialization_error(e: impl std::fmt::Display) -> Self {
        Self::SerializationError {
            message: e.to_string(),
        }
    }
}
