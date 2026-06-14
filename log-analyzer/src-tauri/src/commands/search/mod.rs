//! 搜索命令实现 — Clean Architecture 路径（WorkspaceService）
//!
//! # 架构
//! - `query`: 查询解析
//! - `mod.rs` (本文件): Tauri 命令入口，纯委托给 WorkspaceService
//!
//! P6 后：命令层不再持有 cancellation_tokens HashMap。
//! 搜索取消通过 WorkspaceService::cancel_search() 路由，
//! CancellationToken 生命周期由 WorkspaceServiceImpl 内部管理。

pub(crate) mod query;

use serde::{Deserialize, Serialize};
use tauri::{command, AppHandle, State};

use la_core::error::{AppError, CommandError};
use la_core::models::{LogEntry, SearchFilters, SearchQuery};

use crate::application::workspace_service::WorkspaceServiceRef;
use crate::commands::search::query::resolve_search_query;
use crate::models::AppState;

// ============================================================================
// 公共类型
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinarySearchResult {
    pub search_id: String,
    pub entries: Vec<LogEntry>,
    pub total_count: usize,
    pub duration_ms: u64,
    pub was_truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinarySearchRequest {
    pub query: String,
    pub workspace_id: Option<String>,
    pub max_results: Option<usize>,
    pub filters: Option<SearchFilters>,
}

// ============================================================================
// 运行时配置
// ============================================================================

#[derive(Debug, Clone)]
pub(crate) struct SearchRuntimeConfig {
    pub(crate) default_max_results: usize,
    pub(crate) case_sensitive: bool,
}

impl Default for SearchRuntimeConfig {
    fn default() -> Self {
        Self {
            default_max_results: 100_000,
            case_sensitive: false,
        }
    }
}

pub(crate) fn load_search_runtime_config(app: &AppHandle) -> SearchRuntimeConfig {
    let config = crate::utils::load_app_config(app);
    match config {
        Some(c) => SearchRuntimeConfig {
            default_max_results: c.search.max_results,
            case_sensitive: c.search.case_sensitive,
        },
        None => SearchRuntimeConfig::default(),
    }
}

// ============================================================================
// WorkspaceService 获取/创建（渐进式迁移兼容层）
// ============================================================================

/// 获取工作区服务（纯查找模式，P6 清理后）。
///
/// 不再自动创建服务。如果工作区未找到，返回错误提示用户重新导入。
async fn get_workspace_service_or_error(
    state: &AppState,
    workspace_id: &str,
) -> Result<WorkspaceServiceRef, CommandError> {
    state.get_workspace_service(workspace_id).ok_or_else(|| {
        CommandError::new("NOT_FOUND", format!("Workspace {workspace_id} not found"))
            .with_help("Try reloading the workspace")
    })
}

// ============================================================================
// 辅助函数
// ============================================================================

pub(crate) fn validate_search_params(query: &str) -> Result<(), CommandError> {
    if query.is_empty() {
        return Err(
            CommandError::new("VALIDATION_ERROR", "Search query cannot be empty")
                .with_help("Please enter at least one search term"),
        );
    }
    if query.len() > 1000 {
        return Err(CommandError::new(
            "VALIDATION_ERROR",
            "Search query too long (max 1000 characters)",
        )
        .with_help("Try reducing the number of search terms"));
    }
    Ok(())
}

pub(crate) fn resolve_workspace_id(
    id_arg: Option<String>,
    state: &AppState,
) -> Result<String, CommandError> {
    if let Some(id) = id_arg {
        return Ok(id);
    }
    let ids = state.workspace_ids();
    if let Some(first) = ids.first() {
        Ok(first.clone())
    } else {
        Err(CommandError::new("NOT_FOUND", "No workspaces available")
            .with_help("Create a workspace first"))
    }
}

// ============================================================================
// Tauri 命令 — 搜索管理
// ============================================================================

#[command]
pub async fn cancel_search(
    #[allow(non_snake_case)] searchId: String,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    // Cancel via WorkspaceService — search sessions are managed internally by each service instance.
    // (P6: removed global cancellation_tokens HashMap; cancel_search is purely delegated.)
    for svc in state.all_workspace_services() {
        if svc.cancel_search(&searchId).await.is_ok() {
            return Ok(());
        }
    }

    Err(
        CommandError::new("NOT_FOUND", format!("Search {searchId} not found"))
            .with_help("Search may have already finished"),
    )
}

#[command]
pub async fn fetch_search_page(
    state: State<'_, AppState>,
    #[allow(non_snake_case)] searchId: String,
    offset: usize,
    limit: usize,
) -> Result<la_search::SearchPageResult, CommandError> {
    // P3 迁移：DiskResultStore 全局共享，任意 workspace service 皆可服务
    let services = state.all_workspace_services();
    if let Some(svc) = services.first() {
        return svc
            .fetch_search_page(&searchId, offset, limit)
            .await
            .map_err(|e| {
                CommandError::from(AppError::io_error(
                    format!("Failed to read page: {e}"),
                    None,
                ))
                .with_help("Results may have been cleared. Try searching again")
            });
    }
    Err(CommandError::new("NOT_FOUND", "No workspace available")
        .with_help("Import a workspace first"))
}

// ============================================================================
// 搜索命令入口 — 使用 SearchUseCase（Clean Architecture 路径）
// ============================================================================

#[tauri::command]
#[allow(non_snake_case)]
pub async fn search_logs(
    app: AppHandle,
    query: String,
    structuredQuery: Option<SearchQuery>,
    workspaceId: Option<String>,
    maxResults: Option<usize>,
    filters: Option<SearchFilters>,
    state: State<'_, AppState>,
) -> Result<String, CommandError> {
    // ── 1. Validate ──
    validate_search_params(&query)?;

    // ── 2. Load config ──
    let rc = load_search_runtime_config(&app);

    // ── 3. Resolve params ──
    let mr = maxResults.unwrap_or(rc.default_max_results).min(100_000);
    let f = filters.unwrap_or_default();
    let (raw_terms, sq) =
        resolve_search_query(&query, structuredQuery, rc.case_sensitive, "search_logs")?;
    let ws_id = resolve_workspace_id(workspaceId, &state)?;

    // ── 4. Get WorkspaceService (pure lookup — workspace must be pre-created at import time) ──
    let workspace = get_workspace_service_or_error(&state, &ws_id).await?;

    // ── 5. Execute search via WorkspaceService ──
    // CancellationToken lifecycle is managed by WorkspaceServiceImpl internally;
    // cancel_search goes through service.cancel_search() — no global HashMap needed.
    let search_id = workspace.search(sq, raw_terms, f, mr).await.map_err(|e| {
        CommandError::new("SEARCH_ERROR", format!("Failed to start search: {e}"))
            .with_help("Try again with a simpler query")
    })?;

    Ok(search_id)
}
