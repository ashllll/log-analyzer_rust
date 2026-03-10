//! HTTP API 适配层
//!
//! 提供 RESTful HTTP API 供 Flutter 调用，复用现有的 Tauri 命令逻辑
//!
//! # 设计原则
//!
//! - 复用现有 Tauri 命令的业务逻辑
//! - 使用 JSON 格式进行请求/响应
//! - 支持 CORS 跨域请求
//! - 使用标准 HTTP 状态码
//! - 使用内置全局状态管理访问 AppState

use axum::{
    extract::State,
    http::{HeaderValue, StatusCode},
    response::Json,
    routing::{delete, get, post, put},
    Router,
};
use once_cell::sync::OnceCell;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tracing::{info, warn};

use crate::models::AppState;
use crate::utils::validation::validate_workspace_id;

// ============ 全局状态管理（独立于 FFI） ============

/// HTTP API 上下文
#[derive(Clone)]
pub struct HttpApiContext {
    pub app_state: AppState,
    pub app_data_dir: std::path::PathBuf,
}

/// 全局 HTTP API 上下文存储
static HTTP_API_CONTEXT: OnceCell<Mutex<Option<HttpApiContext>>> = OnceCell::new();

/// 初始化 HTTP API 上下文
pub fn init_http_api_context(app_state: AppState, app_data_dir: std::path::PathBuf) {
    let context = HttpApiContext {
        app_state,
        app_data_dir,
    };
    if let Some(inner) = HTTP_API_CONTEXT.get() {
        let mut guard = inner.lock();
        *guard = Some(context);
    } else {
        HTTP_API_CONTEXT.set(Mutex::new(Some(context))).ok();
    }
    info!("HTTP API 上下文已初始化");
}

/// 获取 HTTP API 上下文
pub fn get_http_api_context() -> Option<HttpApiContext> {
    HTTP_API_CONTEXT
        .get()
        .and_then(|inner| inner.lock().clone())
}

// ============ HTTP API 状态 ============

/// HTTP API 状态
pub struct HttpApiState {
    /// 应用数据目录
    pub app_data_dir: std::path::PathBuf,
    /// 服务器地址
    pub bind_addr: String,
}

impl HttpApiState {
    pub fn new(app_data_dir: std::path::PathBuf, bind_addr: String) -> Self {
        Self {
            app_data_dir,
            bind_addr,
        }
    }
}

// ============ 通用响应类型 ============

/// API 响应包装器
#[derive(Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(message: &str) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message.to_string()),
        }
    }
}

/// 健康检查响应
#[derive(Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

// ============ 请求类型 ============

/// 搜索请求
#[derive(Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub workspace_id: Option<String>,
    pub max_results: Option<usize>,
    #[allow(dead_code)]
    pub filters: Option<serde_json::Value>,
}

/// 创建工作区请求
#[derive(Deserialize)]
pub struct CreateWorkspaceRequest {
    pub name: String,
    pub path: String,
}

/// 导入文件夹请求
#[derive(Deserialize)]
pub struct ImportFolderRequest {
    pub path: String,
    pub workspace_id: String,
}

/// 监听请求
#[derive(Deserialize)]
pub struct WatchRequest {
    pub workspace_id: String,
    pub path: Option<String>,
}

/// 取消任务请求
#[derive(Deserialize)]
pub struct CancelTaskRequest {
    pub task_id: String,
}

/// 关键词组请求
#[derive(Deserialize)]
pub struct KeywordGroupRequest {
    pub name: String,
    pub patterns: Vec<String>,
    pub color: Option<String>,
    pub enabled: Option<bool>,
}

/// 保存配置请求
#[derive(Deserialize)]
pub struct SaveConfigRequest {
    pub config: serde_json::Value,
}

// ============ 响应类型 ============

/// 搜索响应
#[derive(Serialize)]
pub struct SearchResponse {
    pub search_id: String,
    pub total_results: usize,
    pub message: Option<String>,
}

/// 工作区信息
#[derive(Serialize)]
pub struct WorkspaceInfo {
    pub id: String,
    pub name: String,
    pub file_count: usize,
    pub status: String,
}

/// 任务信息
#[derive(Serialize)]
pub struct TaskInfo {
    pub task_id: String,
    pub status: String,
    pub progress: u32,
    pub message: String,
}

// ============ 路由处理器 ============

/// 健康检查
async fn health_check() -> Json<ApiResponse<HealthResponse>> {
    Json(ApiResponse::success(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }))
}

/// 获取工作区列表
async fn list_workspaces(
    State(state): State<Arc<RwLock<HttpApiState>>>,
) -> Json<ApiResponse<Vec<WorkspaceInfo>>> {
    let state = state.read().await;
    let extracted_dir = state.app_data_dir.join("extracted");

    if !extracted_dir.exists() {
        return Json(ApiResponse::success(vec![]));
    }

    let mut workspaces = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&extracted_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let id = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();

                // 读取工作区元数据
                let name = if let Some(metadata) =
                    crate::commands::workspace::WorkspaceMetadata::load(&path).await
                {
                    metadata.name
                } else {
                    id.clone()
                };

                // 检查是否为 CAS 格式
                let metadata_db = path.join("metadata.db");
                let objects_dir = path.join("objects");
                let status = if metadata_db.exists() && objects_dir.exists() {
                    "ready"
                } else {
                    "incomplete"
                };

                // 获取文件数量
                let file_count = if status == "ready" {
                    // 尝试从数据库获取文件数量
                    if let Ok(store) =
                        crate::storage::metadata_store::MetadataStore::new(&path).await
                    {
                        store.count_files().await.unwrap_or(0) as usize
                    } else {
                        0
                    }
                } else {
                    0
                };

                workspaces.push(WorkspaceInfo {
                    id,
                    name,
                    file_count,
                    status: status.to_string(),
                });
            }
        }
    }

    Json(ApiResponse::success(workspaces))
}

/// 获取工作区状态
async fn get_workspace_status(
    State(state): State<Arc<RwLock<HttpApiState>>>,
    axum::extract::Path(workspace_id): axum::extract::Path<String>,
) -> Json<ApiResponse<WorkspaceInfo>> {
    // 安全验证: 验证 workspace_id 格式
    if let Err(e) = validate_workspace_id(&workspace_id) {
        return Json(ApiResponse::error(&format!("Invalid workspace_id: {}", e)));
    }

    let state = state.read().await;
    let workspace_dir = state.app_data_dir.join("extracted").join(&workspace_id);

    if !workspace_dir.exists() {
        return Json(ApiResponse::error(&format!(
            "Workspace not found: {}",
            workspace_id
        )));
    }

    // 读取工作区元数据
    let name = if let Some(metadata) =
        crate::commands::workspace::WorkspaceMetadata::load(&workspace_dir).await
    {
        metadata.name
    } else {
        workspace_id.clone()
    };

    // 检查是否为 CAS 格式
    let metadata_db = workspace_dir.join("metadata.db");
    let objects_dir = workspace_dir.join("objects");
    let status = if metadata_db.exists() && objects_dir.exists() {
        "ready"
    } else {
        "incomplete"
    };

    // 获取文件数量
    let file_count = if status == "ready" {
        if let Ok(store) = crate::storage::metadata_store::MetadataStore::new(&workspace_dir).await
        {
            store.count_files().await.unwrap_or(0) as usize
        } else {
            0
        }
    } else {
        0
    };

    Json(ApiResponse::success(WorkspaceInfo {
        id: workspace_id,
        name,
        file_count,
        status: status.to_string(),
    }))
}

/// 搜索日志 - 调用实际的 Tauri 命令
async fn search_logs(
    State(_state): State<Arc<RwLock<HttpApiState>>>,
    Json(req): Json<SearchRequest>,
) -> Json<ApiResponse<SearchResponse>> {
    if req.query.is_empty() {
        return Json(ApiResponse::error("Query cannot be empty"));
    }

    info!("HTTP API: search_logs called with query: {}", req.query);

    // 获取全局状态
    let ctx = match get_http_api_context() {
        Some(ctx) => ctx,
        None => {
            return Json(ApiResponse::error(
                "HTTP API context not initialized. Search requires app to be running.",
            ));
        }
    };

    // 调用实际的 search_logs Tauri 命令
    // 注意：Tauri 命令需要 AppHandle，我们在 HTTP API 模式下使用模拟的方式
    // 这里直接调用搜索逻辑，类似于 Tauri 命令的实现
    match http_search_logs(&ctx, &req).await {
        Ok(response) => Json(ApiResponse::success(response)),
        Err(e) => Json(ApiResponse::error(&e)),
    }
}

/// HTTP API 版本的搜索逻辑（调用实际的业务代码）
async fn http_search_logs(
    ctx: &HttpApiContext,
    req: &SearchRequest,
) -> Result<SearchResponse, String> {
    use crate::models::search::{QueryMetadata, QueryOperator, SearchTerm, TermSource};
    use crate::services::QueryExecutor;

    // 生成搜索ID
    let search_id = uuid::Uuid::new_v4().to_string();

    // 获取工作区 ID
    let workspace_id = match &req.workspace_id {
        Some(id) => id.clone(),
        None => {
            let dirs = ctx.app_state.workspace_dirs.lock();
            dirs.keys()
                .next()
                .cloned()
                .ok_or("No workspaces available")?
        }
    };

    // 安全验证: 验证 workspace_id 格式
    if let Err(e) = validate_workspace_id(&workspace_id) {
        return Err(format!("Invalid workspace_id: {}", e));
    }

    // 获取工作区目录
    let workspace_dir = {
        let dirs = ctx.app_state.workspace_dirs.lock();
        dirs.get(&workspace_id)
            .cloned()
            .ok_or_else(|| format!("Workspace not found: {}", workspace_id))?
    };

    // 获取或创建 CAS 实例
    let cas = {
        let mut instances = ctx.app_state.cas_instances.lock();
        if let Some(cas) = instances.get(&workspace_id) {
            Arc::clone(cas)
        } else {
            let cas_arc = Arc::new(crate::storage::ContentAddressableStorage::new(
                workspace_dir.clone(),
            ));
            instances.insert(workspace_id.clone(), Arc::clone(&cas_arc));
            cas_arc
        }
    };

    // 获取或创建 MetadataStore
    let metadata_store = {
        let existing_store = {
            let stores = ctx.app_state.metadata_stores.lock();
            stores.get(&workspace_id).cloned()
        };

        match existing_store {
            Some(store) => store,
            None => {
                let store = crate::storage::metadata_store::MetadataStore::new(&workspace_dir)
                    .await
                    .map_err(|e| format!("Failed to create metadata store: {}", e))?;
                let store_arc = Arc::new(store);
                let mut stores = ctx.app_state.metadata_stores.lock();
                stores.insert(workspace_id.clone(), Arc::clone(&store_arc));
                store_arc
            }
        }
    };

    // 执行搜索
    let max_results = req.max_results.unwrap_or(50000).min(100_000);

    // 获取所有文件
    let files = metadata_store
        .get_all_files()
        .await
        .map_err(|e| format!("Failed to get files: {}", e))?;

    // 构建查询
    let raw_terms: Vec<String> = req
        .query
        .split('|')
        .map(|t| t.trim())
        .filter(|t| !t.is_empty())
        .map(|t| t.to_string())
        .collect();

    if raw_terms.is_empty() {
        return Err("Search query is empty after processing".to_string());
    }

    let search_terms: Vec<SearchTerm> = raw_terms
        .iter()
        .enumerate()
        .map(|(i, term)| SearchTerm {
            id: format!("term_{}", i),
            value: term.clone(),
            operator: QueryOperator::Or,
            source: TermSource::User,
            preset_group_id: None,
            is_regex: false,
            priority: 1,
            enabled: true,
            case_sensitive: false,
        })
        .collect();

    let structured_query = crate::models::SearchQuery {
        id: "http_search_query".to_string(),
        terms: search_terms,
        global_operator: QueryOperator::Or,
        filters: None,
        metadata: QueryMetadata {
            created_at: 0,
            last_modified: 0,
            execution_count: 0,
            label: None,
        },
    };

    let mut executor = QueryExecutor::new(100);
    let plan = executor
        .execute(&structured_query)
        .map_err(|e| format!("Query execution error: {}", e))?;

    // 执行搜索
    let mut total_results = 0;
    let max_results = max_results.min(1000);

    for file_metadata in files.iter().take(100) {
        if total_results >= max_results {
            break;
        }

        if !cas.exists(&file_metadata.sha256_hash) {
            continue;
        }

        let content = match cas.read_content(&file_metadata.sha256_hash).await {
            Ok(bytes) => bytes,
            Err(_) => continue,
        };

        let content_str = String::from_utf8_lossy(&content);

        for line in content_str.lines() {
            if total_results >= max_results {
                break;
            }

            if executor.matches_line(&plan, line) {
                total_results += 1;
            }
        }
    }

    info!("HTTP API: search completed with {} results", total_results);

    Ok(SearchResponse {
        search_id,
        total_results,
        message: None,
    })
}

/// 创建工作区 - 调用实际的 Tauri 命令
async fn create_workspace(
    State(_state): State<Arc<RwLock<HttpApiState>>>,
    Json(req): Json<CreateWorkspaceRequest>,
) -> Json<ApiResponse<String>> {
    if req.name.is_empty() {
        return Json(ApiResponse::error("Workspace name cannot be empty"));
    }
    if req.path.is_empty() {
        return Json(ApiResponse::error("Workspace path cannot be empty"));
    }

    info!("HTTP API: create_workspace called with name: {}", req.name);

    // 获取全局状态
    let ctx = match get_http_api_context() {
        Some(ctx) => ctx,
        None => {
            return Json(ApiResponse::error(
                "HTTP API context not initialized. Create workspace requires app to be running.",
            ));
        }
    };

    // 调用实际的 create_workspace 业务逻辑
    match http_create_workspace(&ctx, &req).await {
        Ok(workspace_id) => {
            info!("Workspace created successfully: {}", workspace_id);
            Json(ApiResponse::success(workspace_id))
        }
        Err(e) => Json(ApiResponse::error(&e)),
    }
}

/// HTTP API 版本的工作区创建逻辑（调用实际的业务代码）
async fn http_create_workspace(
    ctx: &HttpApiContext,
    req: &CreateWorkspaceRequest,
) -> Result<String, String> {
    use crate::commands::workspace::WorkspaceMetadata;

    // 验证路径存在
    let source_path = std::path::Path::new(&req.path);
    if !source_path.exists() {
        return Err(format!("Path does not exist: {}", req.path));
    }

    // 生成 workspace ID
    let workspace_id = format!(
        "ws-{}",
        req.name.to_lowercase().replace([' ', '/', '\\'], "-")
    );

    // 获取应用数据目录
    let workspace_dir = ctx.app_data_dir.join("extracted").join(&workspace_id);

    // 创建工作区目录
    tokio::fs::create_dir_all(&workspace_dir)
        .await
        .map_err(|e| format!("Failed to create workspace dir: {}", e))?;

    // 保存工作区元数据
    let metadata = WorkspaceMetadata::new(req.name.clone(), Some(req.path.clone()));
    metadata
        .save(&workspace_dir)
        .await
        .map_err(|e| format!("Failed to save workspace metadata: {}", e))?;

    // 初始化工作区资源
    // 存储工作区目录映射
    {
        let mut workspace_dirs = ctx.app_state.workspace_dirs.lock();
        workspace_dirs.insert(workspace_id.clone(), workspace_dir.clone());
    }

    // 初始化 CAS
    let cas = Arc::new(crate::storage::ContentAddressableStorage::new(
        workspace_dir.clone(),
    ));
    {
        let mut cas_instances = ctx.app_state.cas_instances.lock();
        cas_instances.insert(workspace_id.clone(), cas);
    }

    // 初始化 MetadataStore
    let metadata_store = crate::storage::metadata_store::MetadataStore::new(&workspace_dir)
        .await
        .map_err(|e| format!("Failed to create metadata store: {}", e))?;
    let metadata_store = Arc::new(metadata_store);
    {
        let mut stores = ctx.app_state.metadata_stores.lock();
        stores.insert(workspace_id.clone(), metadata_store);
    }

    Ok(workspace_id)
}

/// 删除工作区
async fn delete_workspace(
    State(state): State<Arc<RwLock<HttpApiState>>>,
    axum::extract::Path(workspace_id): axum::extract::Path<String>,
) -> Json<ApiResponse<bool>> {
    info!("HTTP API: delete_workspace called for: {}", workspace_id);

    // 获取工作区目录
    let state = state.read().await;
    let workspace_dir = state.app_data_dir.join("extracted").join(&workspace_id);

    if !workspace_dir.exists() {
        return Json(ApiResponse::error(&format!(
            "Workspace not found: {}",
            workspace_id
        )));
    }

    // 清理全局状态中的相关资源
    if let Some(ctx) = get_http_api_context() {
        // 停止文件监听器
        {
            let mut watchers = ctx.app_state.watchers.lock();
            if let Some(mut watcher_state) = watchers.remove(&workspace_id) {
                watcher_state.is_active = false;
            }
        }

        // 移除 CAS 实例
        {
            let mut cas_instances = ctx.app_state.cas_instances.lock();
            cas_instances.remove(&workspace_id);
        }

        // 移除 MetadataStore
        {
            let mut stores = ctx.app_state.metadata_stores.lock();
            stores.remove(&workspace_id);
        }

        // 移除工作区目录映射
        {
            let mut workspace_dirs = ctx.app_state.workspace_dirs.lock();
            workspace_dirs.remove(&workspace_id);
        }

        // 清除搜索缓存
        {
            let cache = ctx.app_state.cache_manager.lock();
            cache.clear();
        }
    }

    // 删除工作区目录
    if let Err(e) = tokio::fs::remove_dir_all(&workspace_dir).await {
        warn!("Failed to remove workspace directory: {}", e);
        // 不返回错误，因为资源已清理
    }

    info!("Workspace {} deleted successfully", workspace_id);
    Json(ApiResponse::success(true))
}

/// 刷新工作区 - 调用实际的 Tauri 命令
async fn refresh_workspace(
    State(_state): State<Arc<RwLock<HttpApiState>>>,
    axum::extract::Path(workspace_id): axum::extract::Path<String>,
) -> Json<ApiResponse<String>> {
    info!("HTTP API: refresh_workspace called for: {}", workspace_id);

    // 获取全局状态
    let ctx = match get_http_api_context() {
        Some(ctx) => ctx,
        None => {
            return Json(ApiResponse::error(
                "HTTP API context not initialized. Refresh workspace requires app to be running.",
            ));
        }
    };

    // 获取工作区目录
    let workspace_dir = {
        let dirs = ctx.app_state.workspace_dirs.lock();
        match dirs.get(&workspace_id) {
            Some(dir) => dir.clone(),
            None => {
                return Json(ApiResponse::error(&format!(
                    "Workspace not found: {}",
                    workspace_id
                )));
            }
        }
    };

    // 获取原始路径（从元数据中读取）
    let source_path =
        match crate::commands::workspace::WorkspaceMetadata::load(&workspace_dir).await {
            Some(metadata) => metadata.source_path.unwrap_or_default(),
            None => String::new(),
        };

    if source_path.is_empty() || !std::path::Path::new(&source_path).exists() {
        return Json(ApiResponse::error("Source path not found for refresh"));
    }

    // 调用实际的 refresh 业务逻辑（重新导入）
    match http_refresh_workspace(&ctx, &workspace_id, &source_path).await {
        Ok(task_id) => {
            info!("Workspace {} refreshed successfully", workspace_id);
            Json(ApiResponse::success(task_id))
        }
        Err(e) => Json(ApiResponse::error(&e)),
    }
}

/// HTTP API 版本的工作区刷新逻辑（调用实际的业务代码）
async fn http_refresh_workspace(
    ctx: &HttpApiContext,
    workspace_id: &str,
    source_path: &str,
) -> Result<String, String> {
    // 生成任务 ID
    let task_id = uuid::Uuid::new_v4().to_string();

    // 获取工作区目录
    let workspace_dir = ctx.app_data_dir.join("extracted").join(workspace_id);

    // 清理现有的 CAS 和 MetadataStore
    {
        let mut cas_instances = ctx.app_state.cas_instances.lock();
        cas_instances.remove(workspace_id);
    }
    {
        let mut stores = ctx.app_state.metadata_stores.lock();
        stores.remove(workspace_id);
    }

    // 重新初始化 CAS
    let cas = Arc::new(crate::storage::ContentAddressableStorage::new(
        workspace_dir.clone(),
    ));
    {
        let mut cas_instances = ctx.app_state.cas_instances.lock();
        cas_instances.insert(workspace_id.to_string(), Arc::clone(&cas));
    }

    // 重新初始化 MetadataStore
    let metadata_store = crate::storage::metadata_store::MetadataStore::new(&workspace_dir)
        .await
        .map_err(|e| format!("Failed to create metadata store: {}", e))?;
    let metadata_store = Arc::new(metadata_store);
    {
        let mut stores = ctx.app_state.metadata_stores.lock();
        stores.insert(workspace_id.to_string(), Arc::clone(&metadata_store));
    }

    // 执行重新导入
    let path = std::path::Path::new(source_path);
    let file_count = simplified_import(path, "", &workspace_dir, &cas, metadata_store).await?;

    info!(
        "Workspace {} refreshed with {} files",
        workspace_id, file_count
    );
    Ok(task_id)
}

/// 导入文件夹 - 简化版本，不依赖 AppHandle
async fn import_folder(
    State(_state): State<Arc<RwLock<HttpApiState>>>,
    Json(req): Json<ImportFolderRequest>,
) -> Json<ApiResponse<TaskInfo>> {
    info!(
        "HTTP API: import_folder called for path: {}, workspace: {}",
        req.path, req.workspace_id
    );

    // 验证路径存在
    let source_path = std::path::Path::new(&req.path);
    if !source_path.exists() {
        return Json(ApiResponse::error(&format!(
            "Path does not exist: {}",
            req.path
        )));
    }

    // 获取全局状态
    let ctx = match get_http_api_context() {
        Some(ctx) => ctx,
        None => {
            return Json(ApiResponse::error(
                "HTTP API context not initialized. Import functionality requires app to be running.",
            ));
        }
    };

    // 生成任务 ID
    let task_id = uuid::Uuid::new_v4().to_string();

    // 获取工作区目录
    let workspace_dir = {
        let dirs = ctx.app_state.workspace_dirs.lock();
        match dirs.get(&req.workspace_id) {
            Some(dir) => dir.clone(),
            None => {
                return Json(ApiResponse::error(&format!(
                    "Workspace not found: {}",
                    req.workspace_id
                )));
            }
        }
    };

    // 获取或创建 CAS 实例
    let cas = {
        let mut instances = ctx.app_state.cas_instances.lock();
        if let Some(cas) = instances.get(&req.workspace_id) {
            Arc::clone(cas)
        } else {
            let cas_arc = Arc::new(crate::storage::ContentAddressableStorage::new(
                workspace_dir.clone(),
            ));
            instances.insert(req.workspace_id.clone(), Arc::clone(&cas_arc));
            cas_arc
        }
    };

    // 获取或创建 MetadataStore
    let metadata_store = {
        // 先检查是否已存在
        let existing_store = {
            let stores = ctx.app_state.metadata_stores.lock();
            stores.get(&req.workspace_id).cloned()
        };

        match existing_store {
            Some(store) => store,
            None => {
                // 创建新的 MetadataStore
                match crate::storage::metadata_store::MetadataStore::new(&workspace_dir).await {
                    Ok(store) => {
                        let store_arc = Arc::new(store);
                        // 插入到存储
                        let mut stores = ctx.app_state.metadata_stores.lock();
                        stores.insert(req.workspace_id.clone(), Arc::clone(&store_arc));
                        store_arc
                    }
                    Err(e) => {
                        return Json(ApiResponse::error(&format!(
                            "Failed to create metadata store: {}",
                            e
                        )));
                    }
                }
            }
        }
    };

    // 执行导入（简化版本 - 不使用 process_path_with_cas 因为需要 AppHandle）
    let root_name = source_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    // 使用简化的导入逻辑
    let result = simplified_import(
        source_path,
        &root_name,
        &workspace_dir,
        &cas,
        metadata_store,
    )
    .await;

    match result {
        Ok(file_count) => {
            info!(
                "Import completed successfully for task {}, {} files imported",
                task_id, file_count
            );
            Json(ApiResponse::success(TaskInfo {
                task_id,
                status: "completed".to_string(),
                progress: 100,
                message: format!("Import completed, {} files imported", file_count),
            }))
        }
        Err(e) => {
            warn!("Import failed: {}", e);
            Json(ApiResponse::error(&format!("Import failed: {}", e)))
        }
    }
}

/// 简化的导入逻辑（不依赖 AppHandle）
async fn simplified_import(
    source_path: &std::path::Path,
    _root_name: &str,
    workspace_dir: &std::path::Path,
    cas: &crate::storage::ContentAddressableStorage,
    metadata_store: Arc<crate::storage::MetadataStore>,
) -> Result<usize, String> {
    use crate::storage::FileMetadata;
    use sha2::{Digest, Sha256};
    use walkdir::WalkDir;

    let mut file_count = 0;
    let objects_dir = workspace_dir.join("objects");

    // 确保对象目录存在
    tokio::fs::create_dir_all(&objects_dir)
        .await
        .map_err(|e| format!("Failed to create objects dir: {}", e))?;

    // 遍历源目录
    for entry in WalkDir::new(source_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();

        // 读取文件内容
        let content = match tokio::fs::read(path).await {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to read file {}: {}", path.display(), e);
                continue;
            }
        };

        // 计算 SHA256
        let mut hasher = Sha256::new();
        hasher.update(&content);
        let hash = format!("{:x}", hasher.finalize());

        // 存储到 CAS
        if let Err(e) = cas.store_content(&content).await {
            warn!("Failed to store content in CAS: {}", e);
            continue;
        }

        // 创建元数据
        let relative_path = path
            .strip_prefix(source_path)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();

        let original_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let file_metadata = FileMetadata {
            id: 0, // 自动生成
            sha256_hash: hash,
            virtual_path: relative_path.clone(),
            original_name,
            size: content.len() as i64,
            modified_time: chrono::Utc::now().timestamp(),
            mime_type: Some(
                mime_guess::from_path(path)
                    .first_or_text_plain()
                    .to_string(),
            ),
            parent_archive_id: None,
            depth_level: 0,
        };

        // 存储元数据
        if let Err(e) = metadata_store.insert_file(&file_metadata).await {
            warn!("Failed to insert file metadata: {}", e);
            continue;
        }

        file_count += 1;
    }

    Ok(file_count)
}

/// 开始文件监听
async fn start_watch(
    State(_state): State<Arc<RwLock<HttpApiState>>>,
    Json(req): Json<WatchRequest>,
) -> Json<ApiResponse<bool>> {
    info!(
        "HTTP API: start_watch called for workspace: {}",
        req.workspace_id
    );

    // 获取路径
    let path = match req.path {
        Some(ref p) => p.clone(),
        None => {
            // 尝试从工作区获取路径
            if let Some(ctx) = get_http_api_context() {
                let dirs = ctx.app_state.workspace_dirs.lock();
                match dirs.get(&req.workspace_id) {
                    Some(dir) => dir.display().to_string(),
                    None => return Json(ApiResponse::error("Path is required")),
                }
            } else {
                return Json(ApiResponse::error("Path is required"));
            }
        }
    };

    // 验证路径存在
    let watch_path = std::path::Path::new(&path);
    if !watch_path.exists() {
        return Json(ApiResponse::error(&format!(
            "Path does not exist: {}",
            path
        )));
    }

    // 获取全局状态
    let ctx = match get_http_api_context() {
        Some(ctx) => ctx,
        None => {
            return Json(ApiResponse::error(
                "HTTP API context not initialized. Watch functionality requires app to be running.",
            ));
        }
    };

    // 检查是否已经在监听
    {
        let watchers = ctx.app_state.watchers.lock();
        if watchers.contains_key(&req.workspace_id) {
            return Json(ApiResponse::error("Workspace is already being watched"));
        }
    }

    // 创建文件监听器
    use crate::services::file_watcher::WatcherState;
    use notify::{recommended_watcher, RecursiveMode, Watcher};
    use std::collections::HashMap;

    let (tx, rx) = crossbeam::channel::unbounded::<Result<notify::Event, notify::Error>>();

    let mut watcher = match recommended_watcher(tx) {
        Ok(w) => w,
        Err(e) => {
            return Json(ApiResponse::error(&format!(
                "Failed to create file watcher: {}",
                e
            )));
        }
    };

    if let Err(e) = watcher.watch(watch_path, RecursiveMode::Recursive) {
        return Json(ApiResponse::error(&format!(
            "Failed to start watching path: {}",
            e
        )));
    }

    let watcher_state = WatcherState {
        workspace_id: req.workspace_id.clone(),
        watched_path: watch_path.to_path_buf(),
        file_offsets: HashMap::new(),
        is_active: true,
        thread_handle: Arc::new(std::sync::Mutex::new(None)),
        watcher: Arc::new(std::sync::Mutex::new(Some(watcher))),
    };

    // 启动监听线程
    let workspace_id_clone = req.workspace_id.clone();
    let watchers_arc = Arc::clone(&ctx.app_state.watchers);
    let thread_handle_arc = Arc::clone(&watcher_state.thread_handle);

    {
        let mut watchers = ctx.app_state.watchers.lock();
        watchers.insert(req.workspace_id.clone(), watcher_state);
    }

    let handle = std::thread::spawn(move || {
        for res in rx {
            match res {
                Ok(event) => {
                    // 处理文件变更事件
                    info!("File event: {:?}", event.kind);

                    let is_active = {
                        let watchers = watchers_arc.lock();
                        watchers
                            .get(&workspace_id_clone)
                            .map(|w| w.is_active)
                            .unwrap_or(false)
                    };

                    if !is_active {
                        break;
                    }
                }
                Err(e) => {
                    warn!("Watch error: {}", e);
                }
            }
        }
    });

    if let Ok(mut guard) = thread_handle_arc.lock() {
        *guard = Some(handle);
    }

    info!("Watch started for workspace {}", req.workspace_id);
    Json(ApiResponse::success(true))
}

/// 停止文件监听
async fn stop_watch(
    State(_state): State<Arc<RwLock<HttpApiState>>>,
    Json(req): Json<WatchRequest>,
) -> Json<ApiResponse<bool>> {
    info!(
        "HTTP API: stop_watch called for workspace: {}",
        req.workspace_id
    );

    // 获取全局状态
    let ctx = match get_http_api_context() {
        Some(ctx) => ctx,
        None => {
            return Json(ApiResponse::error(
                "HTTP API context not initialized. Watch functionality requires app to be running.",
            ));
        }
    };

    let mut watchers = ctx.app_state.watchers.lock();

    let (thread_handle, watcher) = if let Some(watcher_state) = watchers.get_mut(&req.workspace_id)
    {
        watcher_state.is_active = false;
        let h = watcher_state
            .thread_handle
            .lock()
            .ok()
            .and_then(|mut h| h.take());
        let w = watcher_state.watcher.lock().ok().and_then(|mut w| w.take());
        (h, w)
    } else {
        return Json(ApiResponse::error(
            "No active watcher found for this workspace",
        ));
    };

    watchers.remove(&req.workspace_id);
    drop(watchers);

    // 释放监听器
    drop(watcher);

    // 等待线程结束
    if let Some(handle) = thread_handle {
        let _ = handle.join();
    }

    info!("Watch stopped for workspace {}", req.workspace_id);
    Json(ApiResponse::success(true))
}

/// 取消任务
async fn cancel_task(
    State(_state): State<Arc<RwLock<HttpApiState>>>,
    Json(req): Json<CancelTaskRequest>,
) -> Json<ApiResponse<bool>> {
    info!("HTTP API: cancel_task called for: {}", req.task_id);

    // 获取全局状态
    if let Some(ctx) = get_http_api_context() {
        // 检查是否有取消令牌
        let tokens = ctx.app_state.search_cancellation_tokens.lock();
        if let Some(token) = tokens.get(&req.task_id) {
            token.cancel();
            return Json(ApiResponse::success(true));
        }
    }

    Json(ApiResponse::error(&format!(
        "Task not found: {}",
        req.task_id
    )))
}

/// 加载配置
async fn load_config(
    State(state): State<Arc<RwLock<HttpApiState>>>,
) -> Json<ApiResponse<serde_json::Value>> {
    info!("HTTP API: load_config called");

    let state = state.read().await;
    let config_path = state.app_data_dir.join("config.json");

    if config_path.exists() {
        match tokio::fs::read_to_string(&config_path).await {
            Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(config) => return Json(ApiResponse::success(config)),
                Err(e) => {
                    warn!("Failed to parse config file: {}", e);
                }
            },
            Err(e) => {
                warn!("Failed to read config file: {}", e);
            }
        }
    }

    // 返回默认配置
    let default_config = crate::models::config::AppConfig::default();
    Json(ApiResponse::success(
        serde_json::to_value(default_config).unwrap_or_default(),
    ))
}

/// 保存配置
async fn save_config(
    State(state): State<Arc<RwLock<HttpApiState>>>,
    Json(req): Json<SaveConfigRequest>,
) -> Json<ApiResponse<bool>> {
    info!("HTTP API: save_config called");

    let state = state.read().await;
    let config_path = state.app_data_dir.join("config.json");

    // 确保目录存在
    if let Some(parent) = config_path.parent() {
        if let Err(e) = tokio::fs::create_dir_all(parent).await {
            return Json(ApiResponse::error(&format!(
                "Failed to create config directory: {}",
                e
            )));
        }
    }

    // 保存配置
    let content = match serde_json::to_string_pretty(&req.config) {
        Ok(c) => c,
        Err(e) => {
            return Json(ApiResponse::error(&format!(
                "Failed to serialize config: {}",
                e
            )));
        }
    };

    if let Err(e) = tokio::fs::write(&config_path, content).await {
        return Json(ApiResponse::error(&format!(
            "Failed to write config file: {}",
            e
        )));
    }

    info!("Config saved successfully");
    Json(ApiResponse::success(true))
}

/// 获取性能指标
async fn get_performance_metrics() -> Json<ApiResponse<serde_json::Value>> {
    info!("HTTP API: get_performance_metrics called");

    // 获取全局状态
    if let Some(ctx) = get_http_api_context() {
        // 获取缓存统计
        let cache_stats = ctx.app_state.get_cache_statistics();

        // 获取搜索统计
        let total_searches = *ctx.app_state.total_searches.lock();
        let cache_hits = *ctx.app_state.cache_hits.lock();
        let last_duration = *ctx.app_state.last_search_duration.lock();

        // 计算缓存命中率
        let total_requests = cache_hits + cache_stats.l1_miss_count;
        let hit_rate = if total_requests > 0 {
            (cache_hits as f64 / total_requests as f64) * 100.0
        } else {
            0.0
        };

        let metrics = serde_json::json!({
            "search": {
                "total_searches": total_searches,
                "cache_hits": cache_hits,
                "last_duration_ms": last_duration.as_millis(),
            },
            "cache": {
                "hit_rate": hit_rate,
                "miss_count": cache_stats.l1_miss_count,
                "hit_count": cache_hits,
                "size": cache_stats.estimated_size,
                "evictions": cache_stats.eviction_count,
            },
            "memory": {
                "used_mb": 0,
                "total_mb": 0,
            }
        });

        return Json(ApiResponse::success(metrics));
    }

    // 返回默认值
    Json(ApiResponse::success(serde_json::json!({
        "search": {
            "total_searches": 0,
            "cache_hits": 0,
            "last_duration_ms": 0,
        },
        "cache": {
            "hit_rate": 0.0,
            "miss_count": 0,
            "hit_count": 0,
            "size": 0,
            "evictions": 0,
        },
        "memory": {
            "used_mb": 0,
            "total_mb": 0,
        }
    })))
}

/// 获取关键词组列表
async fn list_keyword_groups(
    State(state): State<Arc<RwLock<HttpApiState>>>,
) -> Json<ApiResponse<Vec<serde_json::Value>>> {
    info!("HTTP API: list_keyword_groups called");

    let state = state.read().await;
    let config_path = state.app_data_dir.join("config.json");

    if config_path.exists() {
        if let Ok(content) = tokio::fs::read_to_string(&config_path).await {
            if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(keyword_groups) =
                    config.get("keyword_groups").and_then(|v| v.as_array())
                {
                    return Json(ApiResponse::success(keyword_groups.clone()));
                }
            }
        }
    }

    Json(ApiResponse::success(vec![]))
}

/// 添加关键词组
async fn add_keyword_group(
    State(state): State<Arc<RwLock<HttpApiState>>>,
    Json(req): Json<KeywordGroupRequest>,
) -> Json<ApiResponse<String>> {
    info!("HTTP API: add_keyword_group called with name: {}", req.name);

    // 生成 ID
    let id = uuid::Uuid::new_v4().to_string();

    let state = state.read().await;
    let config_path = state.app_data_dir.join("config.json");

    // 读取现有配置
    let mut config: serde_json::Value = if config_path.exists() {
        if let Ok(content) = tokio::fs::read_to_string(&config_path).await {
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            serde_json::json!({})
        }
    } else {
        serde_json::json!({})
    };

    // 添加新的关键词组
    let keyword_group = serde_json::json!({
        "id": id,
        "name": req.name,
        "patterns": req.patterns,
        "color": req.color.unwrap_or_else(|| "#FF5722".to_string()),
        "enabled": req.enabled.unwrap_or(true),
    });

    if let Some(keyword_groups) = config
        .get_mut("keyword_groups")
        .and_then(|v| v.as_array_mut())
    {
        keyword_groups.push(keyword_group);
    } else {
        config["keyword_groups"] = serde_json::json!([keyword_group]);
    }

    // 保存配置
    if let Ok(content) = serde_json::to_string_pretty(&config) {
        let _ = tokio::fs::write(&config_path, content).await;
    }

    Json(ApiResponse::success(id))
}

/// 更新关键词组
async fn update_keyword_group(
    State(state): State<Arc<RwLock<HttpApiState>>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(req): Json<KeywordGroupRequest>,
) -> Json<ApiResponse<bool>> {
    info!("HTTP API: update_keyword_group called for id: {}", id);

    let state = state.read().await;
    let config_path = state.app_data_dir.join("config.json");

    if config_path.exists() {
        if let Ok(content) = tokio::fs::read_to_string(&config_path).await {
            if let Ok(mut config) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(keyword_groups) = config
                    .get_mut("keyword_groups")
                    .and_then(|v| v.as_array_mut())
                {
                    for group in keyword_groups.iter_mut() {
                        if group.get("id").and_then(|v| v.as_str()) == Some(&id) {
                            group["name"] = serde_json::json!(req.name);
                            group["patterns"] = serde_json::json!(req.patterns);
                            if let Some(color) = &req.color {
                                group["color"] = serde_json::json!(color);
                            }
                            if let Some(enabled) = req.enabled {
                                group["enabled"] = serde_json::json!(enabled);
                            }
                            break;
                        }
                    }
                }

                // 保存配置
                if let Ok(content) = serde_json::to_string_pretty(&config) {
                    let _ = tokio::fs::write(&config_path, content).await;
                }

                return Json(ApiResponse::success(true));
            }
        }
    }

    Json(ApiResponse::error(&format!(
        "Keyword group not found: {}",
        id
    )))
}

/// 删除关键词组
async fn delete_keyword_group(
    State(state): State<Arc<RwLock<HttpApiState>>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Json<ApiResponse<bool>> {
    info!("HTTP API: delete_keyword_group called for id: {}", id);

    let state = state.read().await;
    let config_path = state.app_data_dir.join("config.json");

    if config_path.exists() {
        if let Ok(content) = tokio::fs::read_to_string(&config_path).await {
            if let Ok(mut config) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(keyword_groups) = config
                    .get_mut("keyword_groups")
                    .and_then(|v| v.as_array_mut())
                {
                    let initial_len = keyword_groups.len();
                    keyword_groups.retain(|g| g.get("id").and_then(|v| v.as_str()) != Some(&id));

                    if keyword_groups.len() < initial_len {
                        // 保存配置
                        if let Ok(content) = serde_json::to_string_pretty(&config) {
                            let _ = tokio::fs::write(&config_path, content).await;
                        }
                        return Json(ApiResponse::success(true));
                    }
                }
            }
        }
    }

    Json(ApiResponse::error(&format!(
        "Keyword group not found: {}",
        id
    )))
}

/// 404 处理
async fn not_found() -> (StatusCode, Json<ApiResponse<()>>) {
    (
        StatusCode::NOT_FOUND,
        Json(ApiResponse::error("Endpoint not found")),
    )
}

/// 创建路由器
pub fn create_router(state: Arc<RwLock<HttpApiState>>) -> Router {
    // 安全修复: 从环境变量读取允许的域名，而不是允许所有来源
    let allowed_origins: Vec<HeaderValue> = std::env::var("ALLOWED_ORIGINS")
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse::<HeaderValue>().ok())
        .collect();

    let cors = if allowed_origins.is_empty() {
        // 如果没有配置允许的域名，默认拒绝所有跨域请求
        warn!("ALLOWED_ORIGINS not configured, CORS requests will be denied");
        CorsLayer::new()
            .allow_origin(AllowOrigin::predicate(|_, _| false))
            .allow_methods(Any)
            .allow_headers(Any)
    } else {
        info!("CORS enabled for origins: {:?}", allowed_origins);
        CorsLayer::new()
            .allow_origin(AllowOrigin::list(allowed_origins))
            .allow_methods(Any)
            .allow_headers(Any)
    };

    Router::new()
        // 健康检查
        .route("/health", get(health_check))
        // 工作区 API
        .route("/api/workspaces", get(list_workspaces))
        .route("/api/workspace", post(create_workspace))
        .route("/api/workspace/:id/status", get(get_workspace_status))
        .route("/api/workspace/:id", delete(delete_workspace))
        .route("/api/workspace/:id/refresh", post(refresh_workspace))
        // 搜索 API
        .route("/api/search", post(search_logs))
        // 任务 API
        .route("/api/task/cancel", post(cancel_task))
        // 配置 API
        .route("/api/config", get(load_config).post(save_config))
        // 关键词 API
        .route(
            "/api/keywords",
            get(list_keyword_groups).post(add_keyword_group),
        )
        .route(
            "/api/keywords/:id",
            put(update_keyword_group).delete(delete_keyword_group),
        )
        // 监听 API
        .route("/api/watch/start", post(start_watch))
        .route("/api/watch/stop", post(stop_watch))
        // 导入 API
        .route("/api/import/folder", post(import_folder))
        // 性能 API
        .route("/api/performance/metrics", get(get_performance_metrics))
        .layer(cors)
        .fallback(not_found)
        .with_state(state)
}

/// 启动 HTTP 服务器
pub async fn start_http_server(
    app_data_dir: std::path::PathBuf,
    bind_addr: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let state = Arc::new(RwLock::new(HttpApiState::new(
        app_data_dir,
        bind_addr.clone(),
    )));

    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    info!("🚀 HTTP API 服务器启动: http://{}", bind_addr);

    let app = create_router(state);

    axum::serve(listener, app).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_response_success() {
        let response = ApiResponse::success("test".to_string());
        assert!(response.success);
        assert_eq!(response.data, Some("test".to_string()));
        assert!(response.error.is_none());
    }

    #[test]
    fn test_api_response_error() {
        let response: ApiResponse<String> = ApiResponse::error("test error");
        assert!(!response.success);
        assert!(response.data.is_none());
        assert_eq!(response.error, Some("test error".to_string()));
    }

    #[test]
    fn test_health_response_serialization() {
        let response = HealthResponse {
            status: "ok".to_string(),
            version: "1.0.0".to_string(),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("ok"));
        assert!(json.contains("1.0.0"));
    }
}
