use crate::models::search_history::{SearchHistory, SearchHistoryItem};
use std::sync::{Arc, Mutex};
use tauri::State;

/// 搜索历史全局状态类型
pub type HistoryState = Arc<Mutex<SearchHistory>>;

/**
 * 添加搜索历史记录
 *
 * # 参数
 * * `query` - 搜索查询字符串
 * * `workspace_id` - 工作区 ID
 * * `result_count` - 搜索结果数量（可选）
 * * `history` - 全局搜索历史状态
 */
#[tauri::command]
pub async fn add_search_history(
    query: String,
    workspace_id: String,
    result_count: Option<usize>,
    history: State<'_, HistoryState>,
) -> Result<(), String> {
    let item = SearchHistoryItem {
        id: uuid::Uuid::new_v4().to_string(),
        query,
        timestamp: chrono::Utc::now().timestamp(),
        result_count,
        workspace_id,
    };

    let mut h = history
        .lock()
        .map_err(|e| format!("Failed to lock history: {}", e))?;
    h.add(item);
    Ok(())
}

/**
 * 获取指定工作区的所有搜索历史
 *
 * # 参数
 * * `workspace_id` - 工作区 ID
 * * `history` - 全局搜索历史状态
 *
 * # 返回
 * 该工作区的所有历史记录（按时间倒序）
 */
#[tauri::command]
pub async fn get_search_history(
    workspace_id: String,
    history: State<'_, HistoryState>,
) -> Result<Vec<SearchHistoryItem>, String> {
    let h = history
        .lock()
        .map_err(|e| format!("Failed to lock history: {}", e))?;
    Ok(h.get_by_workspace(&workspace_id))
}

/**
 * 搜索匹配的历史记录（用于自动补全）
 *
 * # 参数
 * * `prefix` - 搜索前缀
 * * `workspace_id` - 工作区 ID
 * * `history` - 全局搜索历史状态
 *
 * # 返回
 * 匹配的历史记录（按时间倒序，大小写不敏感）
 */
#[tauri::command]
pub async fn search_history_items(
    prefix: String,
    workspace_id: String,
    history: State<'_, HistoryState>,
) -> Result<Vec<SearchHistoryItem>, String> {
    let h = history
        .lock()
        .map_err(|e| format!("Failed to lock history: {}", e))?;
    Ok(h.search(&prefix, &workspace_id))
}

/**
 * 删除单条搜索历史记录
 *
 * # 参数
 * * `id` - 历史记录 ID
 * * `history` - 全局搜索历史状态
 */
#[tauri::command]
pub async fn delete_search_history(
    id: String,
    history: State<'_, HistoryState>,
) -> Result<(), String> {
    let mut h = history
        .lock()
        .map_err(|e| format!("Failed to lock history: {}", e))?;
    h.remove(&id);
    Ok(())
}

/**
 * 清空所有搜索历史记录
 *
 * # 参数
 * * `history` - 全局搜索历史状态
 */
#[tauri::command]
pub async fn clear_search_history(history: State<'_, HistoryState>) -> Result<(), String> {
    let mut h = history
        .lock()
        .map_err(|e| format!("Failed to lock history: {}", e))?;
    h.clear();
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_commands_exist() {
        // 测试命令存在性（编译测试）
        // 实际功能测试在 search_history.rs 中完成
        // 如果编译通过，说明命令模块存在
    }
}
