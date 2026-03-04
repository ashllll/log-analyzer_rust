//! 搜索历史命令实现
//!
//! 提供搜索历史的增删查功能
//!
//! # 前后端集成规范
//!
//! 为保持与 JavaScript camelCase 惯例一致，Tauri 命令参数使用 camelCase 命名。

use tauri::{command, State};
use tracing::{debug, info};

use crate::models::{AppState, SearchHistoryEntry};

/// 添加搜索历史记录
///
/// # Arguments
/// * `query` - 搜索查询内容
/// * `workspace_id` - 工作区ID
/// * `result_count` - 搜索结果数量
#[command]
pub async fn add_search_history(
    query: String,
    #[allow(non_snake_case)] workspaceId: String,
    #[allow(non_snake_case)] resultCount: usize,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // 验证输入
    if query.trim().is_empty() {
        return Err("Search query cannot be empty".to_string());
    }

    if workspaceId.trim().is_empty() {
        return Err("Workspace ID cannot be empty".to_string());
    }

    debug!(
        query = %query,
        workspace_id = %workspaceId,
        result_count = resultCount,
        "Adding search history entry"
    );

    // 创建历史条目
    let entry = SearchHistoryEntry::new(query.trim().to_string(), workspaceId.clone(), resultCount);

    // 添加到历史管理器
    let mut history = state.search_history.lock();
    history.add_entry(entry);

    info!(
        workspace_id = %workspaceId,
        total_entries = history.total_count(),
        "Search history entry added"
    );

    Ok(())
}

/// 获取搜索历史记录
///
/// # Arguments
/// * `workspace_id` - 可选的工作区ID，如果不提供则返回所有历史
/// * `limit` - 可选的返回数量限制
#[command]
pub async fn get_search_history(
    #[allow(non_snake_case)] workspaceId: Option<String>,
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<SearchHistoryEntry>, String> {
    let history = state.search_history.lock();

    let entries: Vec<SearchHistoryEntry> = if let Some(ws_id) = workspaceId {
        debug!(
            workspace_id = %ws_id,
            limit = ?limit,
            "Getting search history for workspace"
        );
        history
            .get_history(&ws_id, limit)
            .into_iter()
            .cloned()
            .collect()
    } else {
        debug!(
            limit = ?limit,
            "Getting all search history"
        );
        history
            .get_all_history(limit)
            .into_iter()
            .cloned()
            .collect()
    };

    debug!(
        entries_count = entries.len(),
        "Returning search history entries"
    );

    Ok(entries)
}

/// 清除搜索历史记录
///
/// # Arguments
/// * `workspace_id` - 可选的工作区ID，如果不提供则清除所有历史
#[command]
pub async fn clear_search_history(
    #[allow(non_snake_case)] workspaceId: Option<String>,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    let mut history = state.search_history.lock();

    let removed_count = if let Some(ws_id) = workspaceId {
        info!(
            workspace_id = %ws_id,
            "Clearing search history for workspace"
        );
        history.clear_workspace_history(&ws_id)
    } else {
        info!("Clearing all search history");
        let count = history.total_count();
        history.clear_all_history();
        count
    };

    info!(removed_count = removed_count, "Search history cleared");

    Ok(removed_count)
}

#[cfg(test)]
mod tests {
    // 注意：Tauri 命令测试需要 mock State，这里只做编译检查
    // 实际测试在 models/search_history.rs 中进行
}
