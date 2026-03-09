//! Tauri Commands 的 FFI 适配层
//!
//! 本模块将现有的 Tauri commands 适配为 FFI 调用。
//! 这样可以复用现有的业务逻辑，避免重复实现。
//!
//! ## 架构
//!
//! ```
//! FFI Call → commands_bridge → Global State → Business Logic
//! ```
//!
//! ## 注意事项
//!
//! - FFI 函数是同步的（`#[frb(sync)]`）
//! - Tauri commands 通常是异步的
//! - 使用全局状态管理器访问 AppState 和应用目录

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use crate::ffi::global_state::{get_app_data_dir, get_app_state};
use crate::ffi::types::*;
use crate::utils::validation::validate_workspace_id;

// ==================== Typestate Session FFI 命令 ====================

/// FFI 适配：创建 Session
///
/// 创建一个新的 Typestate Session 实例
///
/// # 参数
///
/// * `path` - 要打开的文件路径
///
/// # 返回
///
/// 返回 Session ID 和初始信息
pub fn ffi_open_session(path: String) -> Result<SessionInfo, String> {
    tracing::info!(path = %path, "FFI: open_session 调用");

    // 验证路径
    let file_path = Path::new(&path);
    if !file_path.exists() {
        return Err(format!("文件不存在: {}", path));
    }

    if !file_path.is_file() {
        return Err(format!("路径不是文件: {}", path));
    }

    // 生成 Session ID
    let session_id = format!("session_{}", uuid::Uuid::new_v4());

    // 创建 Session
    crate::ffi::global_state::create_session(session_id.clone(), path)
}

/// FFI 适配：映射 Session
///
/// 将 Session 从 Unmapped 状态转换为 Mapped 状态
///
/// # 参数
///
/// * `session_id` - Session ID
///
/// # 返回
///
/// 成功返回 true
pub fn ffi_map_session(session_id: String) -> Result<bool, String> {
    tracing::info!(session_id = %session_id, "FFI: map_session 调用");

    crate::ffi::global_state::map_session(&session_id)
}

/// FFI 适配：索引 Session
///
/// 将 Session 从 Mapped 状态转换为 Indexed 状态
///
/// # 参数
///
/// * `session_id` - Session ID
///
/// # 返回
///
/// 返回索引条目数量
pub fn ffi_index_session(session_id: String) -> Result<usize, String> {
    tracing::info!(session_id = %session_id, "FFI: index_session 调用");

    crate::ffi::global_state::index_session(&session_id)
}

/// FFI 适配：获取 Session 信息
///
/// 获取指定 Session 的详细信息
///
/// # 参数
///
/// * `session_id` - Session ID
///
/// # 返回
///
/// 返回 Session 信息
pub fn ffi_get_session_info(session_id: String) -> Result<SessionInfo, String> {
    tracing::debug!(session_id = %session_id, "FFI: get_session_info 调用");

    crate::ffi::global_state::get_session_info(&session_id)
        .ok_or_else(|| format!("Session 不存在: {}", session_id))
}

/// FFI 适配：获取索引条目
///
/// 从已索引的 Session 中获取所有索引条目
///
/// # 参数
///
/// * `session_id` - Session ID
///
/// # 返回
///
/// 返回索引条目列表
pub fn ffi_get_index_entries(session_id: String) -> Result<Vec<IndexEntryData>, String> {
    tracing::debug!(session_id = %session_id, "FFI: get_index_entries 调用");

    crate::ffi::global_state::get_session_entries(&session_id)
}

/// FFI 适配：关闭 Session
///
/// 关闭并删除 Session
///
/// # 参数
///
/// * `session_id` - Session ID
///
/// # 返回
///
/// 成功返回 true
pub fn ffi_close_session(session_id: String) -> Result<bool, String> {
    tracing::info!(session_id = %session_id, "FFI: close_session 调用");

    crate::ffi::global_state::remove_session(&session_id)
}

/// FFI 适配：获取活跃 Session 数量
///
/// 返回当前活跃的 Session 数量
pub fn ffi_get_session_count() -> Result<i32, String> {
    Ok(crate::ffi::global_state::get_session_count() as i32)
}

/// FFI 适配：获取所有 Session ID
///
/// 返回所有活跃 Session 的 ID 列表
pub fn ffi_get_all_sessions() -> Result<Vec<String>, String> {
    Ok(crate::ffi::global_state::get_all_session_ids())
}

// ==================== PageManager FFI 命令 ====================

use std::sync::OnceLock;

use parking_lot::RwLock;

/// 全局 PageManager 存储
type PageManagerStore = RwLock<HashMap<String, crate::services::typestate::SharedPageManager>>;

static PAGE_MANAGER_STORE: OnceLock<PageManagerStore> = OnceLock::new();

fn get_page_manager_store() -> &'static PageManagerStore {
    PAGE_MANAGER_STORE.get_or_init(|| RwLock::new(HashMap::new()))
}

/// FFI 适配：创建 PageManager
///
/// 为指定文件创建 PageManager 实例
///
/// # 参数
///
/// * `file_path` - 文件路径
///
/// # 返回
///
/// 返回 PageManager ID
pub fn ffi_create_page_manager(file_path: String) -> Result<String, String> {
    tracing::info!(file_path = %file_path, "FFI: create_page_manager 调用");

    use crate::services::typestate::PageManager;

    let pm = PageManager::new(&file_path).map_err(|e| format!("创建 PageManager 失败: {}", e))?;

    let pm_id = format!("pm_{}", uuid::Uuid::new_v4());

    {
        let store = get_page_manager_store();
        let mut guard = store.write();
        guard.insert(pm_id.clone(), pm.into_arc());
    }

    tracing::info!(pm_id = %pm_id, "PageManager 已创建");
    Ok(pm_id)
}

/// FFI 适配：获取视口数据
///
/// 从 PageManager 获取指定范围的数据
///
/// # 参数
///
/// * `pm_id` - PageManager ID
/// * `start` - 起始偏移
/// * `size` - 数据大小
///
/// # 返回
///
/// 返回视口数据
pub fn ffi_get_viewport(pm_id: String, start: u64, size: usize) -> Result<ViewportData, String> {
    tracing::debug!(
        pm_id = %pm_id,
        start = start,
        size = size,
        "FFI: get_viewport 调用"
    );

    let store = get_page_manager_store();
    let guard = store.read();

    let pm = guard
        .get(&pm_id)
        .ok_or_else(|| format!("PageManager 不存在: {}", pm_id))?;

    // 设置视口
    pm.set_viewport(start, size)
        .map_err(|e| format!("设置视口失败: {}", e))?;

    // 读取视口数据
    let data = pm
        .read_viewport()
        .ok_or_else(|| "读取视口数据失败".to_string())?;

    // 转换为 Base64
    let data_base64 = base64_encode(data);

    // 检查是否有更多数据
    let has_more = (start + size as u64) < pm.file_size();

    Ok(ViewportData {
        start_offset: start,
        data: data_base64,
        data_len: data.len(),
        has_more,
    })
}

/// FFI 适配：读取一行数据
///
/// 从 PageManager 读取一行数据
///
/// # 参数
///
/// * `pm_id` - PageManager ID
/// * `offset` - 起始偏移
///
/// # 返回
///
/// 返回行数据
pub fn ffi_get_line(pm_id: String, offset: u64) -> Result<LineData, String> {
    tracing::debug!(
        pm_id = %pm_id,
        offset = offset,
        "FFI: get_line 调用"
    );

    let store = get_page_manager_store();
    let guard = store.read();

    let pm = guard
        .get(&pm_id)
        .ok_or_else(|| format!("PageManager 不存在: {}", pm_id))?;

    // 读取一行
    let (line_bytes, next_offset) = pm
        .read_line(offset)
        .map_err(|e| format!("读取行失败: {}", e))?;

    // 转换为字符串
    let content = String::from_utf8_lossy(line_bytes).to_string();

    // 计算行号（简化处理，从索引获取）
    let line_number = 0; // 需要从 Session 索引获取

    Ok(LineData {
        line_number,
        content,
        byte_offset: offset,
        next_offset,
    })
}

/// FFI 适配：获取 PageManager 信息
///
/// 获取 PageManager 的文件大小和页面信息
///
/// # 参数
///
/// * `pm_id` - PageManager ID
///
/// # 返回
///
/// 返回 (文件大小, 页面数量, 内存使用量)
pub fn ffi_get_page_manager_info(pm_id: String) -> Result<(u64, usize, usize), String> {
    let store = get_page_manager_store();
    let guard = store.read();

    let pm = guard
        .get(&pm_id)
        .ok_or_else(|| format!("PageManager 不存在: {}", pm_id))?;

    Ok((pm.file_size(), pm.page_count(), pm.memory_usage()))
}

/// FFI 适配：销毁 PageManager
///
/// 销毁指定的 PageManager
///
/// # 参数
///
/// * `pm_id` - PageManager ID
///
/// # 返回
///
/// 成功返回 true
pub fn ffi_destroy_page_manager(pm_id: String) -> Result<bool, String> {
    tracing::info!(pm_id = %pm_id, "FFI: destroy_page_manager 调用");

    let store = get_page_manager_store();
    let mut guard = store.write();

    if guard.remove(&pm_id).is_some() {
        tracing::info!(pm_id = %pm_id, "PageManager 已销毁");
        Ok(true)
    } else {
        Err(format!("PageManager 不存在: {}", pm_id))
    }
}

/// Base64 编码辅助函数
fn base64_encode(data: &[u8]) -> String {
    use base64::{engine::general_purpose, Engine as _};
    general_purpose::STANDARD.encode(data)
}

// ==================== 工作区命令适配 ====================

/// FFI 适配：加载工作区
///
/// 检查工作区是否存在并返回基本信息
pub fn ffi_load_workspace(workspace_id: String) -> Result<WorkspaceLoadResponseData, String> {
    tracing::info!(workspace_id = %workspace_id, "FFI: load_workspace 调用");

    // 验证工作区 ID
    validate_workspace_id(&workspace_id)?;

    // 获取全局状态
    let app_data_dir = get_app_data_dir().ok_or_else(|| "FFI 全局状态未初始化".to_string())?;

    // 构建工作区目录路径
    let workspace_dir = app_data_dir.join("extracted").join(&workspace_id);

    // 检查工作区是否存在
    if !workspace_dir.exists() {
        return Err(format!("工作区不存在: {}", workspace_id));
    }

    // 检查是否为 CAS 格式
    let metadata_db = workspace_dir.join("metadata.db");
    let objects_dir = workspace_dir.join("objects");

    if !metadata_db.exists() || !objects_dir.exists() {
        return Err(format!(
            "工作区 {} 不是 CAS 格式。请创建新工作区。",
            workspace_id
        ));
    }

    // 计算目录大小和文件数
    let mut file_count = 0usize;
    let mut total_size = 0u64;

    if let Ok(entries) = std::fs::read_dir(&objects_dir) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    file_count += 1;
                    total_size += metadata.len();
                }
            }
        }
    }

    // 格式化大小
    let size_mb = total_size / (1024 * 1024);
    let total_size_str = if size_mb > 1024 {
        format!("{:.1}GB", size_mb as f64 / 1024.0)
    } else {
        format!("{}MB", size_mb)
    };

    Ok(WorkspaceLoadResponseData {
        workspace_id: workspace_id.clone(),
        status: "READY".to_string(),
        file_count: file_count as i32,
        total_size: total_size_str,
    })
}

/// FFI 适配：删除工作区
///
/// 删除工作区及其所有相关资源
pub fn ffi_delete_workspace(workspace_id: String) -> Result<bool, String> {
    tracing::info!(workspace_id = %workspace_id, "FFI: delete_workspace 调用");

    // 验证工作区 ID
    validate_workspace_id(&workspace_id)?;

    // 获取全局状态
    let app_state = get_app_state().ok_or_else(|| "FFI 全局状态未初始化".to_string())?;
    let app_data_dir = get_app_data_dir().ok_or_else(|| "FFI 全局状态未初始化".to_string())?;

    // 停止文件监听器
    {
        let mut watchers = app_state.watchers.lock();
        if let Some(mut watcher_state) = watchers.remove(&workspace_id) {
            watcher_state.is_active = false;
            tracing::info!(workspace_id = %workspace_id, "文件监听器已停止");
        }
    }

    // 清除缓存
    {
        let cache = app_state.cache_manager.lock();
        cache.clear();
    }

    // 删除工作区目录
    let workspace_dir = app_data_dir.join("extracted").join(&workspace_id);
    if workspace_dir.exists() {
        std::fs::remove_dir_all(&workspace_dir)
            .map_err(|e| format!("删除工作区目录失败: {}", e))?;
        tracing::info!(path = %workspace_dir.display(), "工作区目录已删除");
    }

    // 删除索引文件
    let index_dir = app_data_dir.join("indices");
    for ext in &[".idx.gz", ".idx"] {
        let index_file = index_dir.join(format!("{}{}", workspace_id, ext));
        if index_file.exists() {
            let _ = std::fs::remove_file(&index_file);
        }
    }

    tracing::info!(workspace_id = %workspace_id, "工作区删除成功");
    Ok(true)
}

/// FFI 适配：刷新工作区
///
/// 刷新工作区索引（对于 CAS 架构等同于重新导入）
pub fn ffi_refresh_workspace(workspace_id: String, path: String) -> Result<String, String> {
    tracing::info!(
        workspace_id = %workspace_id,
        path = %path,
        "FFI: refresh_workspace 调用"
    );

    // 验证输入
    validate_workspace_id(&workspace_id)?;

    let source_path = Path::new(&path);
    if !source_path.exists() {
        return Err(format!("路径不存在: {}", path));
    }

    // 刷新操作返回任务 ID（实际导入由前端通过 import_folder 触发）
    let task_id = format!("task_{}", uuid::Uuid::new_v4());

    tracing::info!(
        workspace_id = %workspace_id,
        task_id = %task_id,
        "刷新任务已创建"
    );

    Ok(task_id)
}

/// FFI 适配：获取工作区状态
///
/// 返回工作区的详细状态信息
pub fn ffi_get_workspace_status(workspace_id: String) -> Result<WorkspaceStatusData, String> {
    tracing::debug!(workspace_id = %workspace_id, "FFI: get_workspace_status 调用");

    // 验证工作区 ID
    validate_workspace_id(&workspace_id)?;

    // 获取全局状态
    let app_data_dir = get_app_data_dir().ok_or_else(|| "FFI 全局状态未初始化".to_string())?;

    // 构建工作区目录路径
    let workspace_dir = app_data_dir.join("extracted").join(&workspace_id);

    // 检查工作区是否存在
    if !workspace_dir.exists() {
        return Err(format!("工作区不存在: {}", workspace_id));
    }

    // 检查是否为 CAS 格式
    let metadata_db = workspace_dir.join("metadata.db");
    let objects_dir = workspace_dir.join("objects");

    let is_cas = metadata_db.exists() && objects_dir.exists();

    if !is_cas {
        return Err(format!("工作区 {} 不是 CAS 格式", workspace_id));
    }

    // 计算目录大小
    let total_size: u64 = walkdir::WalkDir::new(&workspace_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter_map(|e| e.metadata().ok())
        .filter(|m| m.is_file())
        .map(|m| m.len())
        .sum();

    let size_mb = total_size / (1024 * 1024);
    let size_str = if size_mb > 1024 {
        format!("{:.1}GB", size_mb as f64 / 1024.0)
    } else {
        format!("{}MB", size_mb)
    };

    // 计算文件数
    let file_count = if objects_dir.exists() {
        std::fs::read_dir(&objects_dir)
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().is_file())
                    .count()
            })
            .unwrap_or(0)
    } else {
        0
    };

    // 尝试读取工作区元数据
    let workspace_name = workspace_id
        .strip_prefix("ws-")
        .unwrap_or(&workspace_id)
        .replace('-', " ");

    Ok(WorkspaceStatusData {
        id: workspace_id.clone(),
        name: workspace_name,
        status: "READY".to_string(),
        size: size_str,
        files: file_count as i32,
    })
}

// ==================== 搜索命令适配 ====================

/// FFI 适配：执行搜索
///
/// 创建搜索任务并返回搜索 ID。实际搜索结果通过事件流推送。
///
/// # 参数
///
/// * `query` - 搜索查询字符串
/// * `workspace_id` - 工作区 ID（可选，默认使用第一个可用工作区）
/// * `max_results` - 最大结果数量
/// * `filters_json` - 过滤器 JSON（可选）
///
/// # 返回
///
/// 返回搜索 ID，用于取消搜索或追踪结果
pub fn ffi_search_logs(
    query: String,
    workspace_id: Option<String>,
    max_results: i32,
    _filters_json: Option<String>,
) -> Result<String, String> {
    tracing::info!(
        query = %query,
        workspace_id = ?workspace_id,
        max_results,
        "FFI: search_logs 调用"
    );

    // 验证查询
    if query.is_empty() {
        return Err("搜索查询不能为空".to_string());
    }
    if query.len() > 1000 {
        return Err("搜索查询过长（最大 1000 字符）".to_string());
    }

    // 获取全局状态
    let app_state = get_app_state().ok_or_else(|| "FFI 全局状态未初始化".to_string())?;

    // 确定工作区 ID
    let workspace_id = if let Some(id) = workspace_id {
        id
    } else {
        // 获取第一个可用的工作区
        let dirs = app_state.workspace_dirs.lock();
        if let Some(first_id) = dirs.keys().next() {
            first_id.clone()
        } else {
            return Err("没有可用的工作区。请先创建工作区。".to_string());
        }
    };

    // 生成搜索 ID
    let search_id = format!("search_{}", uuid::Uuid::new_v4());

    // 创建取消令牌
    let cancellation_token = tokio_util::sync::CancellationToken::new();
    {
        let mut tokens = app_state.search_cancellation_tokens.lock();
        tokens.insert(search_id.clone(), cancellation_token);
    }

    // 增加搜索计数
    {
        let mut total = app_state.total_searches.lock();
        *total += 1;
    }

    tracing::info!(
        search_id = %search_id,
        workspace_id = %workspace_id,
        "搜索任务已创建"
    );

    // 注意：实际搜索在 Tauri 环境中异步执行
    // Flutter 端通过事件流接收结果
    Ok(search_id)
}

/// FFI 适配：取消搜索
///
/// 取消正在进行的搜索任务
///
/// # 参数
///
/// * `search_id` - 要取消的搜索 ID
///
/// # 返回
///
/// 成功返回 true，失败返回错误信息
pub fn ffi_cancel_search(search_id: String) -> Result<bool, String> {
    tracing::info!(search_id = %search_id, "FFI: cancel_search 调用");

    // 获取全局状态
    let app_state = get_app_state().ok_or_else(|| "FFI 全局状态未初始化".to_string())?;

    // 获取并移除取消令牌
    let cancellation_token = {
        let mut tokens = app_state.search_cancellation_tokens.lock();
        tokens.remove(&search_id)
    };

    if let Some(token) = cancellation_token {
        // 触发取消
        token.cancel();
        tracing::info!(search_id = %search_id, "搜索已取消");
        Ok(true)
    } else {
        tracing::warn!(search_id = %search_id, "未找到搜索任务");
        Err(format!("未找到搜索任务: {}", search_id))
    }
}

/// FFI 适配：获取活跃搜索数量
///
/// 返回当前正在进行的搜索任务数量
pub fn ffi_get_active_searches_count() -> Result<i32, String> {
    let app_state = get_app_state().ok_or_else(|| "FFI 全局状态未初始化".to_string())?;

    let tokens = app_state.search_cancellation_tokens.lock();
    Ok(tokens.len() as i32)
}

// ==================== 导入命令适配 ====================

/// FFI 适配：导入文件夹
///
/// 创建导入任务并返回任务 ID。实际导入操作异步执行，
/// 进度通过事件流推送。
///
/// # 参数
///
/// * `path` - 要导入的文件夹路径
/// * `workspace_id` - 目标工作区 ID
///
/// # 返回
///
/// 返回任务 ID，用于追踪导入进度
pub fn ffi_import_folder(path: String, workspace_id: String) -> Result<String, String> {
    tracing::info!(
        path = %path,
        workspace_id = %workspace_id,
        "FFI: import_folder 调用"
    );

    // 验证路径
    let source_path = Path::new(&path);
    if !source_path.exists() {
        return Err(format!("路径不存在: {}", path));
    }

    // 验证工作区 ID
    validate_workspace_id(&workspace_id)?;

    // 获取全局状态
    let app_state = get_app_state().ok_or_else(|| "FFI 全局状态未初始化".to_string())?;
    let app_data_dir = get_app_data_dir().ok_or_else(|| "FFI 全局状态未初始化".to_string())?;

    // 生成任务 ID
    let task_id = format!("task_{}", uuid::Uuid::new_v4());

    // 创建工作区目录
    let workspace_dir = app_data_dir.join("workspaces").join(&workspace_id);
    std::fs::create_dir_all(&workspace_dir).map_err(|e| format!("创建工作区目录失败: {}", e))?;

    // 获取目标名称
    let target_name = source_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&path)
        .to_string();

    // 创建任务（异步操作）
    let task_manager = {
        let guard = app_state.task_manager.lock();
        guard
            .as_ref()
            .ok_or_else(|| "任务管理器未初始化".to_string())?
            .clone()
    };

    let task_id_clone = task_id.clone();
    let workspace_id_clone = workspace_id.clone();

    let result = tokio::runtime::Runtime::new()
        .map_err(|e| format!("创建运行时失败: {}", e))?
        .block_on(async {
            task_manager
                .create_task_async(
                    task_id_clone.clone(),
                    "Import".to_string(),
                    target_name.clone(),
                    Some(workspace_id_clone.clone()),
                )
                .await
        });

    match result {
        Ok(_) => {
            tracing::info!(
                task_id = %task_id,
                workspace_id = %workspace_id,
                "导入任务已创建"
            );
            Ok(task_id)
        }
        Err(e) => {
            tracing::error!(error = %e, "创建导入任务失败");
            Err(format!("创建导入任务失败: {}", e))
        }
    }
}

// ==================== 文件监听命令适配 ====================

/// FFI 适配：启动文件监听
///
/// 启动对指定路径的文件监听。实际监听通过 Tauri 事件系统实现，
/// FFI 版本仅管理监听状态。
///
/// # 参数
///
/// * `workspace_id` - 工作区 ID
/// * `paths` - 要监听的路径列表
/// * `recursive` - 是否递归监听子目录
///
/// # 返回
///
/// 成功返回 true
pub fn ffi_start_watch(
    workspace_id: String,
    paths: Vec<String>,
    recursive: bool,
) -> Result<bool, String> {
    tracing::info!(
        workspace_id = %workspace_id,
        paths = ?paths,
        recursive,
        "FFI: start_watch 调用"
    );

    // 验证工作区 ID
    validate_workspace_id(&workspace_id)?;

    // 验证路径
    if paths.is_empty() {
        return Err("必须提供至少一个监听路径".to_string());
    }

    for path in &paths {
        let watch_path = Path::new(path);
        if !watch_path.exists() {
            return Err(format!("路径不存在: {}", path));
        }
    }

    // 获取全局状态
    let app_state = get_app_state().ok_or_else(|| "FFI 全局状态未初始化".to_string())?;

    // 检查是否已经在监听
    {
        let watchers = app_state.watchers.lock();
        if watchers.contains_key(&workspace_id) {
            return Err("工作区已在监听中".to_string());
        }
    }

    // 创建监听状态
    let watcher_state = crate::services::file_watcher::WatcherState {
        workspace_id: workspace_id.clone(),
        watched_path: Path::new(&paths[0]).to_path_buf(),
        file_offsets: std::collections::HashMap::new(),
        is_active: true,
        thread_handle: Arc::new(std::sync::Mutex::new(None)),
        watcher: Arc::new(std::sync::Mutex::new(None)),
    };

    // 添加到监听器映射
    {
        let mut watchers = app_state.watchers.lock();
        watchers.insert(workspace_id.clone(), watcher_state);
    }

    tracing::info!(
        workspace_id = %workspace_id,
        "文件监听已启动（状态管理）"
    );

    // 注意：实际的文件监听线程需要在有 Tauri AppHandle 的环境中启动
    // FFI 版本只管理状态，实际监听需要通过 Tauri 命令或 Flutter 端实现

    Ok(true)
}

/// FFI 适配：停止文件监听
///
/// 停止指定工作区的文件监听
///
/// # 参数
///
/// * `workspace_id` - 工作区 ID
///
/// # 返回
///
/// 成功返回 true
pub fn ffi_stop_watch(workspace_id: String) -> Result<bool, String> {
    tracing::info!(workspace_id = %workspace_id, "FFI: stop_watch 调用");

    // 获取全局状态
    let app_state = get_app_state().ok_or_else(|| "FFI 全局状态未初始化".to_string())?;

    // 停止并移除监听器
    {
        let mut watchers = app_state.watchers.lock();
        if let Some(mut watcher_state) = watchers.remove(&workspace_id) {
            watcher_state.is_active = false;
            tracing::info!(workspace_id = %workspace_id, "文件监听已停止");
            Ok(true)
        } else {
            tracing::warn!(workspace_id = %workspace_id, "未找到监听器");
            Err(format!("未找到工作区的监听器: {}", workspace_id))
        }
    }
}

/// FFI 适配：检查是否正在监听
///
/// 检查指定工作区是否正在监听文件变化
pub fn ffi_is_watching(workspace_id: String) -> Result<bool, String> {
    let app_state = get_app_state().ok_or_else(|| "FFI 全局状态未初始化".to_string())?;

    let watchers = app_state.watchers.lock();
    Ok(watchers.contains_key(&workspace_id)
        && watchers
            .get(&workspace_id)
            .map(|w| w.is_active)
            .unwrap_or(false))
}

// ==================== 关键词命令适配 ====================

/// 读取配置文件中的关键词组
fn read_keyword_groups_from_config() -> Result<Vec<KeywordGroupData>, String> {
    let app_data_dir = get_app_data_dir().ok_or_else(|| "FFI 全局状态未初始化".to_string())?;

    let config_path = app_data_dir.join("config.json");
    if !config_path.exists() {
        return Ok(vec![]);
    }

    let config_content =
        std::fs::read_to_string(&config_path).map_err(|e| format!("读取配置文件失败: {}", e))?;

    let config: serde_json::Value =
        serde_json::from_str(&config_content).map_err(|e| format!("解析配置文件失败: {}", e))?;

    let groups = config
        .get("keyword_groups")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| serde_json::from_value(v.clone()).ok())
                .collect()
        })
        .unwrap_or_default();

    Ok(groups)
}

/// 保存关键词组到配置文件
fn save_keyword_groups_to_config(groups: &[KeywordGroupData]) -> Result<(), String> {
    let app_data_dir = get_app_data_dir().ok_or_else(|| "FFI 全局状态未初始化".to_string())?;

    let config_path = app_data_dir.join("config.json");

    // 读取现有配置
    let mut config: serde_json::Value = if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)
            .map_err(|e| format!("读取配置文件失败: {}", e))?;
        serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    // 更新关键词组
    config["keyword_groups"] =
        serde_json::to_value(groups).map_err(|e| format!("序列化关键词组失败: {}", e))?;

    // 保存配置
    let content =
        serde_json::to_string_pretty(&config).map_err(|e| format!("序列化配置失败: {}", e))?;

    std::fs::write(&config_path, content).map_err(|e| format!("写入配置文件失败: {}", e))?;

    Ok(())
}

/// FFI 适配：获取关键词列表
///
/// 从配置文件读取所有关键词组
pub fn ffi_get_keywords() -> Result<Vec<KeywordGroupData>, String> {
    tracing::debug!("FFI: get_keywords 调用");
    read_keyword_groups_from_config()
}

/// FFI 适配：添加关键词组
///
/// 添加新的关键词组到配置文件
pub fn ffi_add_keyword_group(group: KeywordGroupInput) -> Result<bool, String> {
    tracing::info!(name = %group.name, "FFI: add_keyword_group 调用");

    let mut groups = read_keyword_groups_from_config()?;

    // 创建新的关键词组
    let new_group = KeywordGroupData {
        id: format!("kw-{}", uuid::Uuid::new_v4()),
        name: group.name,
        color: group.color,
        patterns: group.patterns,
        enabled: group.enabled,
    };

    groups.push(new_group);
    save_keyword_groups_to_config(&groups)?;

    tracing::info!("关键词组已添加");
    Ok(true)
}

/// FFI 适配：更新关键词组
///
/// 更新现有的关键词组
pub fn ffi_update_keyword_group(
    group_id: String,
    group: KeywordGroupInput,
) -> Result<bool, String> {
    tracing::info!(group_id = %group_id, "FFI: update_keyword_group 调用");

    let mut groups = read_keyword_groups_from_config()?;

    // 查找并更新
    let found = groups.iter_mut().find(|g| g.id == group_id);
    if let Some(existing) = found {
        existing.name = group.name;
        existing.color = group.color;
        existing.patterns = group.patterns;
        existing.enabled = group.enabled;

        save_keyword_groups_to_config(&groups)?;
        tracing::info!(group_id = %group_id, "关键词组已更新");
        Ok(true)
    } else {
        Err(format!("未找到关键词组: {}", group_id))
    }
}

/// FFI 适配：删除关键词组
///
/// 从配置文件删除关键词组
pub fn ffi_delete_keyword_group(group_id: String) -> Result<bool, String> {
    tracing::info!(group_id = %group_id, "FFI: delete_keyword_group 调用");

    let mut groups = read_keyword_groups_from_config()?;

    let initial_len = groups.len();
    groups.retain(|g| g.id != group_id);

    if groups.len() < initial_len {
        save_keyword_groups_to_config(&groups)?;
        tracing::info!(group_id = %group_id, "关键词组已删除");
        Ok(true)
    } else {
        Err(format!("未找到关键词组: {}", group_id))
    }
}

// ==================== 任务命令适配 ====================

/// FFI 适配：获取任务指标
///
/// 返回任务管理器的统计信息
pub fn ffi_get_task_metrics() -> Result<TaskMetricsData, String> {
    tracing::debug!("FFI: get_task_metrics 调用");

    // 获取全局状态
    let app_state = get_app_state().ok_or_else(|| "FFI 全局状态未初始化".to_string())?;

    // 获取 TaskManager
    let task_manager_opt = app_state.task_manager.lock();
    let task_manager = task_manager_opt
        .as_ref()
        .ok_or_else(|| "任务管理器未初始化".to_string())?
        .clone();

    // 获取 metrics（异步操作）
    let metrics = tokio::runtime::Runtime::new()
        .map_err(|e| format!("创建运行时失败: {}", e))?
        .block_on(async { task_manager.get_metrics_async().await });

    match metrics {
        Ok(m) => Ok(TaskMetricsData {
            total_tasks: m.total_tasks as i32,
            running_tasks: m.running_tasks as i32,
            completed_tasks: m.completed_tasks as i32,
            failed_tasks: m.failed_tasks as i32,
            stopped_tasks: m.stopped_tasks as i32,
        }),
        Err(e) => {
            tracing::error!(error = %e, "获取任务指标失败");
            Err(format!("获取任务指标失败: {}", e))
        }
    }
}

/// FFI 适配：取消任务
///
/// 取消正在执行的任务
///
/// # 参数
///
/// * `task_id` - 任务 ID
///
/// # 返回
///
/// 成功返回 true
pub fn ffi_cancel_task(task_id: String) -> Result<bool, String> {
    tracing::info!(task_id = %task_id, "FFI: cancel_task 调用");

    // 获取全局状态
    let app_state = get_app_state().ok_or_else(|| "FFI 全局状态未初始化".to_string())?;

    // 获取 TaskManager
    let task_manager = {
        let task_manager_opt = app_state.task_manager.lock();
        task_manager_opt
            .as_ref()
            .ok_or_else(|| "任务管理器未初始化".to_string())?
            .clone()
    };

    // 更新任务状态为 Stopped（异步操作）
    let result = tokio::runtime::Runtime::new()
        .map_err(|e| format!("创建运行时失败: {}", e))?
        .block_on(async {
            task_manager
                .update_task_async(
                    &task_id,
                    0,
                    "用户取消任务".to_string(),
                    crate::task_manager::TaskStatus::Stopped,
                )
                .await
        });

    match result {
        Ok(_) => {
            tracing::info!(task_id = %task_id, "任务已取消");
            Ok(true)
        }
        Err(e) => {
            tracing::error!(task_id = %task_id, error = %e, "取消任务失败");
            Err(format!("取消任务失败: {}", e))
        }
    }
}

// ==================== 配置命令适配 ====================

/// FFI 适配：加载配置
///
/// 从配置文件加载应用配置
pub fn ffi_load_config() -> Result<ConfigData, String> {
    tracing::debug!("FFI: load_config 调用");

    let app_data_dir = get_app_data_dir().ok_or_else(|| "FFI 全局状态未初始化".to_string())?;

    let config_path = app_data_dir.join("config.json");

    // 如果配置文件不存在，返回默认配置
    if !config_path.exists() {
        tracing::info!("配置文件不存在，返回默认配置");
        return Ok(ConfigData {
            file_filter: FileFilterConfigData {
                enabled: false,
                binary_detection_enabled: false,
                mode: "whitelist".to_string(),
                filename_patterns: vec![],
                allowed_extensions: vec![],
                forbidden_extensions: vec![],
            },
            advanced_features: AdvancedFeaturesConfigData {
                enable_filter_engine: false,
                enable_regex_engine: true,
                enable_time_partition: false,
                enable_autocomplete: true,
                regex_cache_size: 1000,
                autocomplete_limit: 100,
                time_partition_size_secs: 3600,
            },
        });
    }

    // 读取配置文件
    let config_content =
        std::fs::read_to_string(&config_path).map_err(|e| format!("读取配置文件失败: {}", e))?;

    let config: serde_json::Value =
        serde_json::from_str(&config_content).map_err(|e| format!("解析配置文件失败: {}", e))?;

    // 解析文件过滤器配置
    let file_filter = config.get("file_filter");
    let file_filter_data = FileFilterConfigData {
        enabled: file_filter
            .and_then(|f| f.get("enabled").and_then(|v| v.as_bool()))
            .unwrap_or(false),
        binary_detection_enabled: file_filter
            .and_then(|f| f.get("binary_detection_enabled").and_then(|v| v.as_bool()))
            .unwrap_or(false),
        mode: file_filter
            .and_then(|f| f.get("mode").and_then(|v| v.as_str()))
            .unwrap_or("whitelist")
            .to_string(),
        filename_patterns: file_filter
            .and_then(|f| f.get("filename_patterns").and_then(|v| v.as_array()))
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default(),
        allowed_extensions: file_filter
            .and_then(|f| f.get("allowed_extensions").and_then(|v| v.as_array()))
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default(),
        forbidden_extensions: file_filter
            .and_then(|f| f.get("forbidden_extensions").and_then(|v| v.as_array()))
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default(),
    };

    // 解析高级功能配置
    let advanced = config.get("advanced_features");
    let advanced_data = AdvancedFeaturesConfigData {
        enable_filter_engine: advanced
            .and_then(|a| a.get("enable_filter_engine").and_then(|v| v.as_bool()))
            .unwrap_or(false),
        enable_regex_engine: advanced
            .and_then(|a| a.get("enable_regex_engine").and_then(|v| v.as_bool()))
            .unwrap_or(true),
        enable_time_partition: advanced
            .and_then(|a| a.get("enable_time_partition").and_then(|v| v.as_bool()))
            .unwrap_or(false),
        enable_autocomplete: advanced
            .and_then(|a| a.get("enable_autocomplete").and_then(|v| v.as_bool()))
            .unwrap_or(true),
        regex_cache_size: advanced
            .and_then(|a| a.get("regex_cache_size").and_then(|v| v.as_i64()))
            .unwrap_or(1000) as i32,
        autocomplete_limit: advanced
            .and_then(|a| a.get("autocomplete_limit").and_then(|v| v.as_i64()))
            .unwrap_or(100) as i32,
        time_partition_size_secs: advanced
            .and_then(|a| a.get("time_partition_size_secs").and_then(|v| v.as_i64()))
            .unwrap_or(3600) as i32,
    };

    tracing::info!("配置加载成功");
    Ok(ConfigData {
        file_filter: file_filter_data,
        advanced_features: advanced_data,
    })
}

/// FFI 适配：保存配置
///
/// 保存配置到文件
pub fn ffi_save_config(config: ConfigData) -> Result<bool, String> {
    tracing::info!("FFI: save_config 调用");

    let app_data_dir = get_app_data_dir().ok_or_else(|| "FFI 全局状态未初始化".to_string())?;

    let config_path = app_data_dir.join("config.json");

    // 读取现有配置（如果存在）
    let mut existing_config: serde_json::Value = if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)
            .map_err(|e| format!("读取配置文件失败: {}", e))?;
        serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    // 更新文件过滤器配置
    existing_config["file_filter"] = serde_json::json!({
        "enabled": config.file_filter.enabled,
        "binary_detection_enabled": config.file_filter.binary_detection_enabled,
        "mode": config.file_filter.mode,
        "filename_patterns": config.file_filter.filename_patterns,
        "allowed_extensions": config.file_filter.allowed_extensions,
        "forbidden_extensions": config.file_filter.forbidden_extensions,
    });

    // 更新高级功能配置
    existing_config["advanced_features"] = serde_json::json!({
        "enable_filter_engine": config.advanced_features.enable_filter_engine,
        "enable_regex_engine": config.advanced_features.enable_regex_engine,
        "enable_time_partition": config.advanced_features.enable_time_partition,
        "enable_autocomplete": config.advanced_features.enable_autocomplete,
        "regex_cache_size": config.advanced_features.regex_cache_size,
        "autocomplete_limit": config.advanced_features.autocomplete_limit,
        "time_partition_size_secs": config.advanced_features.time_partition_size_secs,
    });

    // 保存配置
    let content = serde_json::to_string_pretty(&existing_config)
        .map_err(|e| format!("序列化配置失败: {}", e))?;

    std::fs::write(&config_path, content).map_err(|e| format!("写入配置文件失败: {}", e))?;

    tracing::info!("配置保存成功");
    Ok(true)
}

// ==================== 性能监控命令适配 ====================

/// FFI 适配：获取性能指标
///
/// 返回性能监控数据
pub fn ffi_get_performance_metrics(_time_range: String) -> Result<PerformanceMetricsData, String> {
    tracing::debug!("FFI: get_performance_metrics 调用");

    // 获取全局状态
    let app_state = get_app_state().ok_or_else(|| "FFI 全局状态未初始化".to_string())?;

    // 获取缓存统计
    let cache_stats = app_state.get_cache_statistics();

    // 获取搜索统计
    let total_searches = *app_state.total_searches.lock();
    let cache_hits = *app_state.cache_hits.lock();
    let last_duration = *app_state.last_search_duration.lock();

    // 计算缓存命中率
    let cache_hit_rate = if total_searches > 0 {
        (cache_hits as f64 / total_searches as f64) * 100.0
    } else {
        0.0
    };

    // 获取延迟（毫秒）
    let search_latency = last_duration.as_secs_f64() * 1000.0;

    Ok(PerformanceMetricsData {
        search_latency,
        search_throughput: if search_latency > 0.0 {
            1000.0 / search_latency
        } else {
            0.0
        },
        cache_hit_rate,
        cache_size: cache_stats.entry_count as i32,
        total_queries: total_searches as i32,
        cache_hits: cache_hits as i32,
        latency_history: vec![], // 历史记录需要单独存储
        avg_latency: search_latency,
    })
}

// ==================== 导出命令适配 ====================

/// FFI 适配：导出结果
///
/// 导出搜索结果到文件
///
/// # 参数
///
/// * `search_id` - 搜索 ID
/// * `format` - 导出格式 (json, csv, txt)
/// * `output_path` - 输出文件路径
///
/// # 返回
///
/// 返回输出文件路径
pub fn ffi_export_results(
    search_id: String,
    format: String,
    output_path: String,
) -> Result<String, String> {
    tracing::info!(
        search_id = %search_id,
        format = %format,
        output_path = %output_path,
        "FFI: export_results 调用"
    );

    // 验证格式
    let valid_formats = ["json", "csv", "txt"];
    if !valid_formats.contains(&format.to_lowercase().as_str()) {
        return Err(format!(
            "不支持的导出格式: {}。支持: json, csv, txt",
            format
        ));
    }

    // 验证输出路径
    let output = Path::new(&output_path);
    if let Some(parent) = output.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| format!("创建输出目录失败: {}", e))?;
        }
    }

    // 注意：实际的搜索结果需要从缓存或存储中获取
    // 这里返回任务 ID，实际导出通过事件流处理
    let task_id = format!("export_{}", uuid::Uuid::new_v4());

    tracing::info!(
        task_id = %task_id,
        output_path = %output_path,
        "导出任务已创建"
    );

    // 返回输出路径作为确认
    Ok(output_path)
}

// ==================== 搜索历史命令适配 ====================

use crate::ffi::types::SearchHistoryData;

/// FFI 适配：添加搜索历史
///
/// 将搜索记录添加到历史管理器
///
/// # 参数
///
/// * `query` - 搜索查询字符串
/// * `workspace_id` - 工作区 ID
/// * `result_count` - 搜索结果数量
///
/// # 返回
///
/// 成功返回 true
pub fn ffi_add_search_history(
    query: String,
    workspace_id: String,
    result_count: usize,
) -> Result<bool, String> {
    tracing::debug!(
        query = %query,
        workspace_id = %workspace_id,
        result_count = result_count,
        "FFI: add_search_history 调用"
    );

    // 获取全局状态
    let app_state = get_app_state().ok_or_else(|| "FFI 全局状态未初始化".to_string())?;

    // 创建历史条目
    let entry = crate::models::SearchHistoryEntry::new(query, workspace_id.clone(), result_count);

    // 添加到历史管理器
    {
        let mut history = app_state.search_history.lock();
        history.add_entry(entry);
    }

    tracing::info!(workspace_id = %workspace_id, "搜索历史已添加");
    Ok(true)
}

/// FFI 适配：获取搜索历史
///
/// 获取指定工作区或所有工作区的搜索历史
///
/// # 参数
///
/// * `workspace_id` - 工作区 ID（可选，None 表示获取所有）
/// * `limit` - 最大返回数量（可选）
///
/// # 返回
///
/// 返回搜索历史列表
pub fn ffi_get_search_history(
    workspace_id: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<SearchHistoryData>, String> {
    tracing::debug!(
        workspace_id = ?workspace_id,
        limit = ?limit,
        "FFI: get_search_history 调用"
    );

    // 获取全局状态
    let app_state = get_app_state().ok_or_else(|| "FFI 全局状态未初始化".to_string())?;

    let history = app_state.search_history.lock();

    let entries: Vec<SearchHistoryData> = if let Some(ws_id) = workspace_id {
        history
            .get_history(&ws_id, limit)
            .into_iter()
            .map(|e| SearchHistoryData::from(e.clone()))
            .collect()
    } else {
        history
            .get_all_history(limit)
            .into_iter()
            .map(|e| SearchHistoryData::from(e.clone()))
            .collect()
    };

    Ok(entries)
}

/// FFI 适配：删除搜索历史（按查询词）
///
/// 删除指定工作区中特定查询的历史记录
///
/// # 参数
///
/// * `query` - 查询字符串
/// * `workspace_id` - 工作区 ID
///
/// # 返回
///
/// 成功返回 true（即使没有删除任何记录）
pub fn ffi_delete_search_history(query: String, workspace_id: String) -> Result<bool, String> {
    tracing::debug!(
        query = %query,
        workspace_id = %workspace_id,
        "FFI: delete_search_history 调用"
    );

    // 获取全局状态
    let app_state = get_app_state().ok_or_else(|| "FFI 全局状态未初始化".to_string())?;

    let mut history = app_state.search_history.lock();

    // 手动删除匹配的条目
    let initial_count = history.total_count();
    history
        .get_entries_mut()
        .retain(|e| !(e.query == query && e.workspace_id == workspace_id));
    let deleted = initial_count > history.total_count();

    if deleted {
        tracing::info!(
            query = %query,
            workspace_id = %workspace_id,
            "搜索历史已删除"
        );
    }

    Ok(deleted)
}

/// FFI 适配：批量删除搜索历史（按查询词列表）
///
/// 批量删除指定工作区中多个查询的历史记录
///
/// # 参数
///
/// * `queries` - 查询字符串列表
/// * `workspace_id` - 工作区 ID
///
/// # 返回
///
/// 返回实际删除的数量
pub fn ffi_delete_search_histories(
    queries: Vec<String>,
    workspace_id: String,
) -> Result<i32, String> {
    tracing::debug!(
        queries_count = queries.len(),
        workspace_id = %workspace_id,
        "FFI: delete_search_histories 调用"
    );

    // 获取全局状态
    let app_state = get_app_state().ok_or_else(|| "FFI 全局状态未初始化".to_string())?;

    let mut history = app_state.search_history.lock();

    // 批量删除匹配的条目
    let initial_count = history.total_count();
    history
        .get_entries_mut()
        .retain(|e| !(queries.contains(&e.query) && e.workspace_id == workspace_id));
    let deleted_count = initial_count - history.total_count();

    tracing::info!(deleted_count = deleted_count, "批量删除搜索历史完成");
    Ok(deleted_count as i32)
}

/// FFI 适配：清空搜索历史
///
/// 清空指定工作区或所有工作区的搜索历史
///
/// # 参数
///
/// * `workspace_id` - 工作区 ID（可选，None 表示清空所有）
///
/// # 返回
///
/// 返回实际删除的数量
pub fn ffi_clear_search_history(workspace_id: Option<String>) -> Result<i32, String> {
    tracing::info!(
        workspace_id = ?workspace_id,
        "FFI: clear_search_history 调用"
    );

    // 获取全局状态
    let app_state = get_app_state().ok_or_else(|| "FFI 全局状态未初始化".to_string())?;

    let mut history = app_state.search_history.lock();

    let removed_count = if let Some(ws_id) = workspace_id {
        history.clear_workspace_history(&ws_id)
    } else {
        let count = history.total_count();
        history.clear_all_history();
        count
    };

    tracing::info!(removed_count = removed_count, "搜索历史已清空");
    Ok(removed_count as i32)
}

// ==================== 虚拟文件树命令适配 ====================

use crate::ffi::types::{FileContentResponseData, VirtualTreeNodeData};

// ==================== 多关键词组合搜索命令适配 ====================

use crate::ffi::types::{
    QueryOperatorData, SearchResultEntry, SearchTermData, StructuredSearchQueryData,
};

/// FFI 适配：执行结构化搜索（多关键词组合搜索）
///
/// 支持多个关键词的 AND/OR/NOT 组合搜索
///
/// # 参数
///
/// * `query` - 结构化搜索查询
/// * `workspace_id` - 工作区 ID（可选）
/// * `max_results` - 最大结果数量
///
/// # 返回
///
/// 返回匹配的搜索结果列表
pub fn ffi_search_structured(
    query: StructuredSearchQueryData,
    workspace_id: Option<String>,
    max_results: i32,
) -> Result<Vec<SearchResultEntry>, String> {
    tracing::info!(
        terms_count = query.terms.len(),
        global_operator = ?query.global_operator,
        workspace_id = ?workspace_id,
        max_results,
        "FFI: search_structured 调用"
    );

    // 获取全局状态
    let app_state = get_app_state().ok_or_else(|| "FFI 全局状态未初始化".to_string())?;

    // 确定工作区目录
    let workspace_id = if let Some(id) = workspace_id {
        id
    } else {
        let dirs = app_state.workspace_dirs.lock();
        if let Some(first_id) = dirs.keys().next() {
            first_id.clone()
        } else {
            return Err("没有可用的工作区".to_string());
        }
    };

    let app_data_dir = get_app_data_dir().ok_or_else(|| "FFI 全局状态未初始化".to_string())?;
    let workspace_dir = app_data_dir.join("workspaces").join(&workspace_id);

    if !workspace_dir.exists() {
        return Err(format!("工作区不存在: {}", workspace_id));
    }

    // 使用 tokio 运行时执行异步搜索
    let rt = tokio::runtime::Runtime::new().map_err(|e| format!("创建运行时失败: {}", e))?;

    rt.block_on(async {
        // 打开元数据存储
        let metadata_store = crate::storage::MetadataStore::new(&workspace_dir)
            .await
            .map_err(|e| format!("打开元数据存储失败: {}", e))?;

        // 获取所有文件
        let files = metadata_store
            .get_all_files()
            .await
            .map_err(|e| format!("获取文件失败: {}", e))?;

        // 提取启用关键词
        let keywords: Vec<String> = query
            .terms
            .iter()
            .filter(|t| t.enabled)
            .map(|t| t.value.clone())
            .collect();

        if keywords.is_empty() {
            return Ok(vec![]);
        }

        // 全局操作符
        let use_and = matches!(query.global_operator, QueryOperatorData::And);
        let use_not = matches!(query.global_operator, QueryOperatorData::Not);

        // 构建 Aho-Corasick 自动机
        let ac = aho_corasick::AhoCorasick::new(&keywords)
            .map_err(|e| format!("构建匹配器失败: {}", e))?;

        let mut results = Vec::new();
        let max_results = max_results as usize;

        // 遍历所有文件
        for file in files {
            if results.len() >= max_results {
                break;
            }

            // 跳过二进制文件
            if let Some(mime) = &file.mime_type {
                if mime.starts_with("application/") || mime.starts_with("image/") {
                    continue;
                }
            }

            // 读取文件内容
            let cas = crate::storage::ContentAddressableStorage::new(workspace_dir.clone());
            if !cas.exists(&file.sha256_hash) {
                continue;
            }

            let content_bytes = match cas.read_content(&file.sha256_hash).await {
                Ok(c) => c,
                Err(_) => continue,
            };

            let content = match String::from_utf8(content_bytes) {
                Ok(c) => c,
                Err(_) => continue,
            };

            // 搜索匹配行
            for (line_idx, line) in content.lines().enumerate() {
                if results.len() >= max_results {
                    break;
                }

                // 使用 Aho-Corasick 查找所有匹配
                let matches: Vec<&str> = ac.find_iter(line).map(|m| &line[m.start()..m.end()]).collect();

                let should_include = if keywords.len() == 1 {
                    !matches.is_empty()
                } else if use_and {
                    // AND: 所有关键词都必须匹配
                    keywords.iter().all(|kw| {
                        if kw.is_empty() {
                            true
                        } else {
                            line.contains(kw)
                        }
                    })
                } else if use_not {
                    // NOT: 排除包含任一关键词的行
                    matches.is_empty()
                } else {
                    // OR: 任一关键词匹配即可
                    !matches.is_empty()
                };

                if should_include {
                    // 计算匹配位置
                    let (match_start, match_end) = if !matches.is_empty() {
                        let first_match = ac.find_iter(line).next().unwrap();
                        (first_match.start() as i64, first_match.end() as i64)
                    } else {
                        (0, line.len() as i64)
                    };

                    results.push(SearchResultEntry {
                        line_number: (line_idx + 1) as i64,
                        content: line.to_string(),
                        match_start,
                        match_end,
                    });
                }
            }
        }

        tracing::info!(results_count = results.len(), "结构化搜索完成");
        Ok(results)
    })
}

/// FFI 适配：构建搜索查询对象
///
/// 从关键词列表构建结构化搜索查询
///
/// # 参数
///
/// * `keywords` - 关键词列表
/// * `global_operator` - 全局操作符
/// * `is_regex` - 是否使用正则表达式
/// * `case_sensitive` - 是否大小写敏感
///
/// # 返回
///
/// 返回构建的结构化搜索查询
pub fn ffi_build_search_query(
    keywords: Vec<String>,
    global_operator: QueryOperatorData,
    is_regex: bool,
    case_sensitive: bool,
) -> StructuredSearchQueryData {
    tracing::debug!(
        keywords_count = keywords.len(),
        global_operator = ?global_operator,
        is_regex,
        case_sensitive,
        "FFI: build_search_query 调用"
    );

    let terms: Vec<SearchTermData> = keywords
        .into_iter()
        .enumerate()
        .map(|(idx, value)| SearchTermData {
            id: format!("term_{}", idx),
            value,
            operator: QueryOperatorData::And, // 默认为 AND
            is_regex,
            priority: idx as u32,
            enabled: true,
            case_sensitive,
        })
        .collect();

    StructuredSearchQueryData {
        terms,
        global_operator,
        filters: None,
    }
}

/// FFI 适配：获取虚拟文件树
///
/// 获取工作区的虚拟文件树结构（根节点）
///
/// # 参数
///
/// * `workspace_id` - 工作区 ID
///
/// # 返回
///
/// 返回根节点列表
pub fn ffi_get_virtual_file_tree(workspace_id: String) -> Result<Vec<VirtualTreeNodeData>, String> {
    tracing::info!(workspace_id = %workspace_id, "FFI: get_virtual_file_tree 调用");

    // 获取应用数据目录
    let app_data_dir = get_app_data_dir().ok_or_else(|| "FFI 全局状态未初始化".to_string())?;

    // 构建工作区目录路径
    let workspace_dir = app_data_dir.join("workspaces").join(&workspace_id);

    if !workspace_dir.exists() {
        return Err(format!("工作区不存在: {}", workspace_id));
    }

    // 使用 tokio 运行时执行异步操作
    let rt = tokio::runtime::Runtime::new().map_err(|e| format!("创建运行时失败: {}", e))?;

    rt.block_on(async {
        // 打开元数据存储
        let metadata_store = crate::storage::MetadataStore::new(&workspace_dir)
            .await
            .map_err(|e| format!("打开元数据存储失败: {}", e))?;

        // 获取所有归档和文件
        let archives = metadata_store
            .get_all_archives()
            .await
            .map_err(|e| format!("获取归档失败: {}", e))?;

        let all_files = metadata_store
            .get_all_files()
            .await
            .map_err(|e| format!("获取文件失败: {}", e))?;

        // 构建树结构
        let tree = crate::commands::virtual_tree::build_tree_structure(&archives, &all_files, &metadata_store).await?;

        // 转换为 FFI 类型
        Ok(tree.into_iter().map(VirtualTreeNodeData::from).collect())
    })
}

/// FFI 适配：获取树子节点（懒加载）
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
pub fn ffi_get_tree_children(
    workspace_id: String,
    parent_path: String,
) -> Result<Vec<VirtualTreeNodeData>, String> {
    tracing::debug!(workspace_id = %workspace_id, parent_path = %parent_path, "FFI: get_tree_children 调用");

    // 获取应用数据目录
    let app_data_dir = get_app_data_dir().ok_or_else(|| "FFI 全局状态未初始化".to_string())?;

    // 构建工作区目录路径
    let workspace_dir = app_data_dir.join("workspaces").join(&workspace_id);

    if !workspace_dir.exists() {
        return Err(format!("工作区不存在: {}", workspace_id));
    }

    // 使用 tokio 运行时执行异步操作
    let rt = tokio::runtime::Runtime::new().map_err(|e| format!("创建运行时失败: {}", e))?;

    rt.block_on(async {
        // 打开元数据存储
        let metadata_store = crate::storage::MetadataStore::new(&workspace_dir)
            .await
            .map_err(|e| format!("打开元数据存储失败: {}", e))?;

        // 获取所有归档和文件
        let archives = metadata_store
            .get_all_archives()
            .await
            .map_err(|e| format!("获取归档失败: {}", e))?;

        let all_files = metadata_store
            .get_all_files()
            .await
            .map_err(|e| format!("获取文件失败: {}", e))?;

        // 查找父归档
        let parent_archive = archives.iter().find(|a| a.virtual_path == parent_path);

        if let Some(parent) = parent_archive {
            // 获取子归档
            let child_archives: Vec<_> = archives
                .iter()
                .filter(|a| a.parent_archive_id == Some(parent.id))
                .collect();

            // 获取子文件
            let child_files: Vec<_> = all_files
                .iter()
                .filter(|f| f.parent_archive_id == Some(parent.id))
                .collect();

            let mut children = Vec::new();

            // 添加子归档
            for archive in child_archives {
                children.push(VirtualTreeNodeData::from(
                    crate::commands::virtual_tree::VirtualTreeNode::Archive {
                        name: archive.original_name.clone(),
                        path: archive.virtual_path.clone(),
                        hash: archive.sha256_hash.clone(),
                        archive_type: archive.archive_type.clone(),
                        children: vec![], // 懒加载，不展开子节点
                    },
                ));
            }

            // 添加子文件
            for file in child_files {
                children.push(VirtualTreeNodeData::from(
                    crate::commands::virtual_tree::VirtualTreeNode::File {
                        name: file.original_name.clone(),
                        path: file.virtual_path.clone(),
                        hash: file.sha256_hash.clone(),
                        size: file.size,
                        mime_type: file.mime_type.clone(),
                    },
                ));
            }

            Ok(children)
        } else {
            Err(format!("未找到父路径: {}", parent_path))
        }
    })
}

/// FFI 适配：通过哈希读取文件内容
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
pub fn ffi_read_file_by_hash(
    workspace_id: String,
    hash: String,
) -> Result<FileContentResponseData, String> {
    tracing::debug!(workspace_id = %workspace_id, hash = %hash, "FFI: read_file_by_hash 调用");

    // 获取应用数据目录
    let app_data_dir = get_app_data_dir().ok_or_else(|| "FFI 全局状态未初始化".to_string())?;

    // 构建工作区目录路径
    let workspace_dir = app_data_dir.join("workspaces").join(&workspace_id);

    // 使用 tokio 运行时执行异步操作
    let rt = tokio::runtime::Runtime::new().map_err(|e| format!("创建运行时失败: {}", e))?;

    rt.block_on(async {
        // 初始化 CAS
        let cas = crate::storage::ContentAddressableStorage::new(workspace_dir);

        // 检查文件是否存在
        if !cas.exists(&hash) {
            return Err(format!("文件不存在: {}", hash));
        }

        // 读取内容
        let content_bytes = cas
            .read_content(&hash)
            .await
            .map_err(|e| format!("读取文件失败: {}", e))?;

        // 转换为 UTF-8 字符串
        let content = String::from_utf8(content_bytes.clone())
            .map_err(|e| format!("文件内容不是有效的 UTF-8: {}", e))?;

        Ok(FileContentResponseData {
            content,
            hash,
            size: content_bytes.len() as i64,
        })
    })
}

// ==================== 过滤器命令适配 ====================

use crate::ffi::types::{SavedFilterData, SavedFilterInput};

/// 读取过滤器配置文件
fn read_saved_filters_from_config(workspace_id: &str) -> Result<Vec<SavedFilterData>, String> {
    let app_data_dir = get_app_data_dir().ok_or_else(|| "FFI 全局状态未初始化".to_string())?;

    let config_path = app_data_dir.join("filters.json");
    if !config_path.exists() {
        return Ok(vec![]);
    }

    let config_content =
        std::fs::read_to_string(&config_path).map_err(|e| format!("读取过滤器配置文件失败: {}", e))?;

    let config: serde_json::Value =
        serde_json::from_str(&config_content).map_err(|e| format!("解析过滤器配置文件失败: {}", e))?;

    // 按工作区过滤
    let filters = config
        .get("filters")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    let ws_id = v.get("workspace_id")?.as_str()?;
                    if ws_id == workspace_id {
                        serde_json::from_value(v.clone()).ok()
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(filters)
}

/// 保存过滤器列表到配置文件
fn save_filters_to_config(filters: &[SavedFilterData]) -> Result<(), String> {
    let app_data_dir = get_app_data_dir().ok_or_else(|| "FFI 全局状态未初始化".to_string())?;

    let config_path = app_data_dir.join("filters.json");

    // 读取现有配置
    let mut config: serde_json::Value = if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)
            .map_err(|e| format!("读取过滤器配置文件失败: {}", e))?;
        serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    // 更新过滤器列表
    config["filters"] =
        serde_json::to_value(filters).map_err(|e| format!("序列化过滤器失败: {}", e))?;

    // 保存配置
    let content =
        serde_json::to_string_pretty(&config).map_err(|e| format!("序列化配置失败: {}", e))?;

    std::fs::write(&config_path, content).map_err(|e| format!("写入过滤器配置文件失败: {}", e))?;

    Ok(())
}

/// FFI 适配：保存或更新过滤器
///
/// 根据 workspace_id + name 唯一键保存或更新过滤器
///
/// # 参数
///
/// * `filter` - 过滤器输入数据
///
/// # 返回
///
/// 成功返回 true
pub fn ffi_save_filter(filter: SavedFilterInput) -> Result<bool, String> {
    tracing::info!(
        name = %filter.name,
        workspace_id = %filter.workspace_id,
        "FFI: save_filter 调用"
    );

    // 验证输入
    if filter.name.is_empty() {
        return Err("过滤器名称不能为空".to_string());
    }

    let mut filters = read_saved_filters_from_config(&filter.workspace_id)?;

    // 生成唯一 ID 或查找现有过滤器
    let now = chrono::Utc::now().to_rfc3339();
    let filter_id = format!("filter-{}", uuid::Uuid::new_v4());

    // 检查是否已存在同名过滤器（按工作区）
    let existing_index = filters
        .iter()
        .position(|f| f.name == filter.name && f.workspace_id == filter.workspace_id);

    let new_filter = if let Some(idx) = existing_index {
        // 更新现有过滤器
        let existing = &mut filters[idx];
        existing.description = filter.description;
        existing.terms_json = filter.terms_json;
        existing.global_operator = filter.global_operator;
        existing.time_range_start = filter.time_range_start;
        existing.time_range_end = filter.time_range_end;
        existing.levels_json = filter.levels_json;
        existing.file_pattern = filter.file_pattern;
        existing.is_default = filter.is_default;
        existing.sort_order = filter.sort_order;
        existing.id.clone()
    } else {
        // 创建新过滤器
        let new_filter_data = SavedFilterData {
            id: filter_id.clone(),
            name: filter.name,
            description: filter.description,
            workspace_id: filter.workspace_id.clone(),
            terms_json: filter.terms_json,
            global_operator: filter.global_operator,
            time_range_start: filter.time_range_start,
            time_range_end: filter.time_range_end,
            levels_json: filter.levels_json,
            file_pattern: filter.file_pattern,
            is_default: filter.is_default,
            sort_order: filter.sort_order,
            usage_count: 0,
            created_at: now.clone(),
            last_used_at: None,
        };
        filters.push(new_filter_data);
        filter_id.clone()
    };

    save_filters_to_config(&filters)?;

    tracing::info!(
        filter_id = %new_filter,
        workspace_id = %filter.workspace_id,
        "过滤器已保存"
    );
    Ok(true)
}

/// FFI 适配：获取工作区的所有过滤器
///
/// 获取指定工作区的所有已保存过滤器
///
/// # 参数
///
/// * `workspace_id` - 工作区 ID
/// * `limit` - 最大返回数量（可选）
///
/// # 返回
///
/// 返回过滤器列表
pub fn ffi_get_saved_filters(
    workspace_id: String,
    limit: Option<usize>,
) -> Result<Vec<SavedFilterData>, String> {
    tracing::debug!(
        workspace_id = %workspace_id,
        limit = ?limit,
        "FFI: get_saved_filters 调用"
    );

    let mut filters = read_saved_filters_from_config(&workspace_id)?;

    // 按使用次数排序（使用最多的在前）
    filters.sort_by(|a, b| b.usage_count.cmp(&a.usage_count));

    // 限制返回数量
    if let Some(l) = limit {
        filters.truncate(l);
    }

    Ok(filters)
}

/// FFI 适配：删除指定过滤器
///
/// 删除指定工作区中的过滤器
///
/// # 参数
///
/// * `filter_id` - 过滤器 ID
/// * `workspace_id` - 工作区 ID
///
/// # 返回
///
/// 成功返回 true
pub fn ffi_delete_filter(filter_id: String, workspace_id: String) -> Result<bool, String> {
    tracing::info!(
        filter_id = %filter_id,
        workspace_id = %workspace_id,
        "FFI: delete_filter 调用"
    );

    let mut filters = read_saved_filters_from_config(&workspace_id)?;

    let initial_len = filters.len();
    filters.retain(|f| f.id != filter_id || f.workspace_id != workspace_id);

    if filters.len() < initial_len {
        save_filters_to_config(&filters)?;
        tracing::info!(filter_id = %filter_id, "过滤器已删除");
        Ok(true)
    } else {
        Err(format!("未找到过滤器: {}", filter_id))
    }
}

/// FFI 适配：更新过滤器使用统计
///
/// 更新过滤器的使用次数和最后使用时间
///
/// # 参数
///
/// * `filter_id` - 过滤器 ID
/// * `workspace_id` - 工作区 ID
///
/// # 返回
///
/// 成功返回 true
pub fn ffi_update_filter_usage(filter_id: String, workspace_id: String) -> Result<bool, String> {
    tracing::debug!(
        filter_id = %filter_id,
        workspace_id = %workspace_id,
        "FFI: update_filter_usage 调用"
    );

    let mut filters = read_saved_filters_from_config(&workspace_id)?;

    // 查找并更新过滤器
    let filter_updated = {
        let found = filters
            .iter_mut()
            .find(|f| f.id == filter_id && f.workspace_id == workspace_id);

        if let Some(filter) = found {
            filter.usage_count += 1;
            filter.last_used_at = Some(chrono::Utc::now().to_rfc3339());
            true
        } else {
            false
        }
    };

    if filter_updated {
        save_filters_to_config(&filters)?;
        tracing::info!(
            filter_id = %filter_id,
            usage_count = filters.iter().find(|f| f.id == filter_id).map(|f| f.usage_count).unwrap_or(0),
            "过滤器使用统计已更新"
        );
        Ok(true)
    } else {
        Err(format!("未找到过滤器: {}", filter_id))
    }
}

// ==================== 日志级别统计命令适配 ====================

use crate::ffi::types::LogLevelStatsOutput;

/// FFI 适配：获取日志级别统计
///
/// 获取工作区中每个日志级别的记录数量
///
/// # 参数
///
/// * `workspace_id` - 工作区 ID
///
/// # 返回
///
/// 返回每个日志级别的数量统计
pub fn ffi_get_log_level_stats(workspace_id: String) -> Result<LogLevelStatsOutput, String> {
    tracing::info!(workspace_id = %workspace_id, "FFI: get_log_level_stats 调用");

    // 验证工作区 ID
    validate_workspace_id(&workspace_id)?;

    // 获取全局状态
    let app_state = get_app_state().ok_or_else(|| "FFI 全局状态未初始化".to_string())?;
    let app_data_dir = get_app_data_dir().ok_or_else(|| "FFI 全局状态未初始化".to_string())?;

    // 构建工作区目录路径
    let workspace_dir = app_data_dir.join("workspaces").join(&workspace_id);

    if !workspace_dir.exists() {
        return Err(format!("工作区不存在: {}", workspace_id));
    }

    // 使用 tokio 运行时执行异步操作
    let rt = tokio::runtime::Runtime::new().map_err(|e| format!("创建运行时失败: {}", e))?;

    rt.block_on(async {
        // 打开元数据存储
        let metadata_store = crate::storage::MetadataStore::new(&workspace_dir)
            .await
            .map_err(|e| format!("打开元数据存储失败: {}", e))?;

        // 获取所有文件
        let files = metadata_store
            .get_all_files()
            .await
            .map_err(|e| format!("获取文件失败: {}", e))?;

        // 统计每个级别的数量
        let mut fatal_count = 0u64;
        let mut error_count = 0u64;
        let mut warn_count = 0u64;
        let mut info_count = 0u64;
        let mut debug_count = 0u64;
        let mut trace_count = 0u64;
        let mut unknown_count = 0u64;

        // 获取日志级别统计（从文件内容中解析）
        for file in files {
            // 跳过二进制文件
            if let Some(mime) = &file.mime_type {
                if mime.starts_with("application/") || mime.starts_with("image/") {
                    continue;
                }
            }

            // 读取文件内容进行统计
            let cas = crate::storage::ContentAddressableStorage::new(workspace_dir.clone());
            if !cas.exists(&file.sha256_hash) {
                continue;
            }

            let content_bytes = match cas.read_content(&file.sha256_hash).await {
                Ok(c) => c,
                Err(_) => continue,
            };

            let content = match String::from_utf8(content_bytes) {
                Ok(c) => c,
                Err(_) => continue,
            };

            // 统计每行的日志级别
            for line in content.lines() {
                if let Some(level) = crate::domain::log_analysis::value_objects::LogLevel::parse_from_line(line) {
                    match level {
                        crate::domain::log_analysis::value_objects::LogLevel::Fatal => fatal_count += 1,
                        crate::domain::log_analysis::value_objects::LogLevel::Error => error_count += 1,
                        crate::domain::log_analysis::value_objects::LogLevel::Warn => warn_count += 1,
                        crate::domain::log_analysis::value_objects::LogLevel::Info => info_count += 1,
                        crate::domain::log_analysis::value_objects::LogLevel::Debug => debug_count += 1,
                        crate::domain::log_analysis::value_objects::LogLevel::Trace => trace_count += 1,
                        crate::domain::log_analysis::value_objects::LogLevel::Unknown(_) => unknown_count += 1,
                    }
                }
            }
        }

        let total = fatal_count + error_count + warn_count + info_count + debug_count + trace_count + unknown_count;

        tracing::info!(
            workspace_id = %workspace_id,
            fatal = fatal_count,
            error = error_count,
            warn = warn_count,
            info = info_count,
            debug = debug_count,
            trace = trace_count,
            unknown = unknown_count,
            total = total,
            "日志级别统计完成"
        );

        Ok(LogLevelStatsOutput {
            fatal_count,
            error_count,
            warn_count,
            info_count,
            debug_count,
            trace_count,
            unknown_count,
            total,
        })
    })
}

// ==================== 正则搜索命令适配 ====================

/// FFI 适配：验证正则表达式语法
///
/// 验证正则表达式是否有效
///
/// # 参数
///
/// * `pattern` - 正则表达式模式
///
/// # 返回
///
/// 返回验证结果
pub fn ffi_validate_regex(pattern: String) -> RegexValidationResult {
    tracing::debug!(pattern = %pattern, "FFI: validate_regex 调用");

    // 尝试编译正则表达式
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

/// FFI 适配：执行正则表达式搜索
///
/// 在工作区中搜索匹配正则表达式的行
///
/// # 参数
///
/// * `pattern` - 正则表达式模式
/// * `workspace_id` - 工作区 ID（可选）
/// * `max_results` - 最大结果数量
/// * `case_sensitive` - 是否大小写敏感
///
/// # 返回
///
/// 返回搜索结果列表
pub fn ffi_search_regex(
    pattern: String,
    workspace_id: Option<String>,
    max_results: i32,
    case_sensitive: bool,
) -> Result<Vec<SearchResultEntry>, String> {
    tracing::info!(
        pattern = %pattern,
        workspace_id = ?workspace_id,
        max_results,
        case_sensitive,
        "FFI: search_regex 调用"
    );

    // 验证正则表达式
    if let Err(e) = regex::Regex::new(&pattern) {
        return Err(format!("无效的正则表达式: {}", e));
    }

    // 获取全局状态
    let app_state = get_app_state().ok_or_else(|| "FFI 全局状态未初始化".to_string())?;

    // 确定工作区目录
    let workspace_id = if let Some(id) = workspace_id {
        id
    } else {
        let dirs = app_state.workspace_dirs.lock();
        if let Some(first_id) = dirs.keys().next() {
            first_id.clone()
        } else {
            return Err("没有可用的工作区".to_string());
        }
    };

    let app_data_dir = get_app_data_dir().ok_or_else(|| "FFI 全局状态未初始化".to_string())?;
    let workspace_dir = app_data_dir.join("workspaces").join(&workspace_id);

    if !workspace_dir.exists() {
        return Err(format!("工作区不存在: {}", workspace_id));
    }

    // 使用 tokio 运行时执行异步搜索
    let rt = tokio::runtime::Runtime::new().map_err(|e| format!("创建运行时失败: {}", e))?;

    rt.block_on(async {
        // 打开元数据存储
        let metadata_store = crate::storage::MetadataStore::new(&workspace_dir)
            .await
            .map_err(|e| format!("打开元数据存储失败: {}", e))?;

        // 获取所有文件
        let files = metadata_store
            .get_all_files()
            .await
            .map_err(|e| format!("获取文件失败: {}", e))?;

        // 创建正则表达式
        let regex_pattern = if case_sensitive {
            regex::Regex::new(&pattern).map_err(|e| format!("正则表达式错误: {}", e))?
        } else {
            regex::Regex::new(&format!("(?i){}", pattern))
                .map_err(|e| format!("正则表达式错误: {}", e))?
        };

        let mut results = Vec::new();
        let max_results = max_results as usize;

        // 遍历所有文件
        for file in files {
            if results.len() >= max_results {
                break;
            }

            // 跳过二进制文件
            if let Some(mime) = &file.mime_type {
                if mime.starts_with("application/") || mime.starts_with("image/") {
                    continue;
                }
            }

            // 读取文件内容
            let cas = crate::storage::ContentAddressableStorage::new(workspace_dir.clone());
            if !cas.exists(&file.sha256_hash) {
                continue;
            }

            let content_bytes = match cas.read_content(&file.sha256_hash).await {
                Ok(c) => c,
                Err(_) => continue,
            };

            let content = match String::from_utf8(content_bytes) {
                Ok(c) => c,
                Err(_) => continue,
            };

            // 搜索匹配行
            for (line_idx, line) in content.lines().enumerate() {
                if results.len() >= max_results {
                    break;
                }

                if let Some(m) = regex_pattern.find(line) {
                    results.push(SearchResultEntry {
                        line_number: (line_idx + 1) as i64,
                        content: line.to_string(),
                        match_start: m.start() as i64,
                        match_end: m.end() as i64,
                    });
                }
            }
        }

        tracing::info!(results_count = results.len(), "正则搜索完成");
        Ok(results)
    })
}

// ==================== 测试模块 ====================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// 创建测试用的临时文件
    fn create_test_log_file() -> NamedTempFile {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        writeln!(file, "Line 1: ERROR: Test error message").unwrap();
        writeln!(file, "Line 2: WARN: Test warning message").unwrap();
        writeln!(file, "Line 3: INFO: Test info message").unwrap();
        writeln!(file, "Line 4: DEBUG: Test debug message").unwrap();
        writeln!(file, "Line 5: TRACE: Test trace message").unwrap();
        file.flush().unwrap();
        file
    }

    #[test]
    fn test_ffi_open_session() {
        let temp_file = create_test_log_file();
        let path = temp_file.path().to_string_lossy().to_string();

        let result = ffi_open_session(path.clone());

        assert!(result.is_ok(), "创建 Session 应该成功");
        let info = result.unwrap();
        assert!(info.session_id.starts_with("session_"));
        assert_eq!(info.file_path, path);
        assert_eq!(info.state, SessionState::Unmapped);
        assert!(info.file_size > 0);

        // 清理
        let _ = ffi_close_session(info.session_id);
    }

    #[test]
    fn test_ffi_open_session_invalid_path() {
        let result = ffi_open_session("/nonexistent/path/file.log".to_string());
        assert!(result.is_err(), "打开不存在的文件应该失败");
    }

    #[test]
    fn test_ffi_session_lifecycle() {
        let temp_file = create_test_log_file();
        let path = temp_file.path().to_string_lossy().to_string();

        // 1. 创建 Session
        let info = ffi_open_session(path).unwrap();
        let session_id = info.session_id.clone();
        assert_eq!(info.state, SessionState::Unmapped);

        // 2. 映射 Session
        let map_result = ffi_map_session(session_id.clone());
        assert!(map_result.is_ok(), "映射 Session 应该成功");

        // 检查状态
        let info = ffi_get_session_info(session_id.clone()).unwrap();
        assert_eq!(info.state, SessionState::Mapped);

        // 3. 索引 Session
        let index_result = ffi_index_session(session_id.clone());
        assert!(index_result.is_ok(), "索引 Session 应该成功");
        let entry_count = index_result.unwrap();
        assert_eq!(entry_count, 5, "应该有 5 行");

        // 检查状态
        let info = ffi_get_session_info(session_id.clone()).unwrap();
        assert_eq!(info.state, SessionState::Indexed);

        // 4. 获取索引条目
        let entries = ffi_get_index_entries(session_id.clone()).unwrap();
        assert_eq!(entries.len(), 5);

        // 验证第一个条目
        let first_entry = &entries[0];
        assert_eq!(first_entry.line_number, 1);
        assert_eq!(first_entry.byte_offset, 0);

        // 5. 关闭 Session
        let close_result = ffi_close_session(session_id.clone());
        assert!(close_result.is_ok(), "关闭 Session 应该成功");

        // 6. 验证 Session 已删除
        let info_result = ffi_get_session_info(session_id);
        assert!(info_result.is_err(), "Session 应该已删除");
    }

    #[test]
    fn test_ffi_map_session_wrong_state() {
        let temp_file = create_test_log_file();
        let path = temp_file.path().to_string_lossy().to_string();

        // 创建并映射
        let info = ffi_open_session(path).unwrap();
        let session_id = info.session_id.clone();
        ffi_map_session(session_id.clone()).unwrap();

        // 尝试再次映射（应该失败）
        let result = ffi_map_session(session_id.clone());
        assert!(result.is_err(), "重复映射应该失败");

        // 清理
        let _ = ffi_close_session(session_id);
    }

    #[test]
    fn test_ffi_index_session_wrong_state() {
        let temp_file = create_test_log_file();
        let path = temp_file.path().to_string_lossy().to_string();

        // 创建但不映射
        let info = ffi_open_session(path).unwrap();
        let session_id = info.session_id.clone();

        // 尝试直接索引（应该失败）
        let result = ffi_index_session(session_id.clone());
        assert!(result.is_err(), "未映射时索引应该失败");

        // 清理
        let _ = ffi_close_session(session_id);
    }

    #[test]
    fn test_ffi_get_session_count() {
        // 获取初始数量
        let initial_count = ffi_get_session_count().unwrap();

        // 创建一个 Session
        let temp_file = create_test_log_file();
        let path = temp_file.path().to_string_lossy().to_string();
        let info = ffi_open_session(path).unwrap();

        // 检查数量增加
        let new_count = ffi_get_session_count().unwrap();
        assert_eq!(new_count, initial_count + 1);

        // 清理
        let _ = ffi_close_session(info.session_id);
    }

    #[test]
    fn test_ffi_create_page_manager() {
        let temp_file = create_test_log_file();
        let path = temp_file.path().to_string_lossy().to_string();

        let result = ffi_create_page_manager(path);

        assert!(result.is_ok(), "创建 PageManager 应该成功");
        let pm_id = result.unwrap();
        assert!(pm_id.starts_with("pm_"));

        // 清理
        let _ = ffi_destroy_page_manager(pm_id);
    }

    #[test]
    fn test_ffi_get_page_manager_info() {
        let temp_file = create_test_log_file();
        let path = temp_file.path().to_string_lossy().to_string();

        let pm_id = ffi_create_page_manager(path).unwrap();

        let result = ffi_get_page_manager_info(pm_id.clone());
        assert!(result.is_ok());

        let (file_size, page_count, memory_usage) = result.unwrap();
        assert!(file_size > 0);
        assert!(page_count >= 1);
        assert!(memory_usage > 0);

        // 清理
        let _ = ffi_destroy_page_manager(pm_id);
    }

    #[test]
    fn test_ffi_get_viewport() {
        let temp_file = create_test_log_file();
        let path = temp_file.path().to_string_lossy().to_string();

        let pm_id = ffi_create_page_manager(path).unwrap();

        // 读取前 50 字节
        let result = ffi_get_viewport(pm_id.clone(), 0, 50);
        assert!(result.is_ok());

        let viewport = result.unwrap();
        assert_eq!(viewport.start_offset, 0);
        assert!(viewport.data_len > 0);
        assert!(!viewport.data.is_empty()); // Base64 编码的数据

        // 清理
        let _ = ffi_destroy_page_manager(pm_id);
    }

    #[test]
    fn test_ffi_get_line() {
        let temp_file = create_test_log_file();
        let path = temp_file.path().to_string_lossy().to_string();

        let pm_id = ffi_create_page_manager(path).unwrap();

        // 读取第一行
        let result = ffi_get_line(pm_id.clone(), 0);
        assert!(result.is_ok());

        let line_data = result.unwrap();
        assert!(line_data.content.contains("Line 1"));
        assert!(line_data.next_offset > 0);

        // 读取第二行
        let result2 = ffi_get_line(pm_id.clone(), line_data.next_offset);
        assert!(result2.is_ok());

        let line_data2 = result2.unwrap();
        assert!(line_data2.content.contains("Line 2"));

        // 清理
        let _ = ffi_destroy_page_manager(pm_id);
    }
}
