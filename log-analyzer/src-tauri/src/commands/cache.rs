//! 缓存管理命令
//!
//! 提供工作区缓存清理功能
//!
//! # 前后端集成规范
//!
//! 为保持与 JavaScript camelCase 惯例一致，Tauri 命令参数使用 camelCase 命名。

use crate::models::AppState;
use tauri::{command, State};

/// 清理工作区缓存
///
/// 清理指定工作区的所有缓存条目，释放内存和磁盘空间。
///
/// # 参数
///
/// * `workspaceId` - 工作区 ID
///
/// # 返回
///
/// 返回被清理的缓存条目数量
///
/// # 示例
///
/// ```typescript
/// await invoke('invalidate_workspace_cache', { workspaceId: 'workspace-123' });
/// ```
#[command]
pub async fn invalidate_workspace_cache(
    #[allow(non_snake_case)] workspaceId: String, // 对应前端 invoke('invalidate_workspace_cache', { workspaceId })
    state: State<'_, AppState>,
) -> Result<usize, String> {
    state.invalidate_workspace_cache(&workspaceId)
}
