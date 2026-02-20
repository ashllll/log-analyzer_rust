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

use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{delete, get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

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
    pub filters: Option<serde_json::Value>,
}

/// 创建工作区请求
#[derive(Deserialize)]
pub struct CreateWorkspaceRequest {
    pub name: String,
    pub path: String,
}

/// 刷新工作区请求
#[derive(Deserialize)]
pub struct RefreshWorkspaceRequest {
    pub path: String,
}

/// 取消任务请求
#[derive(Deserialize)]
pub struct CancelTaskRequest {
    pub task_id: String,
}

// ============ 响应类型 ============

/// 搜索响应
#[derive(Serialize)]
pub struct SearchResponse {
    pub search_id: String,
    pub total_results: usize,
}

/// 工作区信息
#[derive(Serialize)]
pub struct WorkspaceInfo {
    pub id: String,
    pub name: String,
    pub file_count: usize,
    pub status: String,
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
                let id = path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();

                // 读取工作区元数据
                let name = if let Some(metadata) = crate::commands::workspace::WorkspaceMetadata::load(&path).await {
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
                    if let Ok(store) = crate::storage::metadata_store::MetadataStore::new(&path).await {
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
    State(_state): State<Arc<RwLock<HttpApiState>>>,
    axum::extract::Path(workspace_id): axum::extract::Path<String>,
) -> Json<ApiResponse<WorkspaceInfo>> {
    // 简化实现，返回基本信息
    // 实际实现可以复用 workspace.rs 中的逻辑
    Json(ApiResponse::success(WorkspaceInfo {
        id: workspace_id,
        name: "Workspace".to_string(),
        file_count: 0,
        status: "ready".to_string(),
    }))
}

/// 搜索日志
async fn search_logs(
    State(_state): State<Arc<RwLock<HttpApiState>>>,
    Json(req): Json<SearchRequest>,
) -> Json<ApiResponse<SearchResponse>> {
    if req.query.is_empty() {
        return Json(ApiResponse::error("Query cannot be empty"));
    }

    // 生成搜索ID
    let search_id = uuid::Uuid::new_v4().to_string();

    info!("HTTP API: search_logs called with query: {}", req.query);

    // 这里需要调用实际的搜索逻辑
    // 由于涉及复杂的异步状态管理，实际实现需要集成 AppState
    // 暂时返回模拟响应
    Json(ApiResponse::success(SearchResponse {
        search_id,
        total_results: 0,
    }))
}

/// 创建工作区
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

    // 生成工作区ID
    let workspace_id = uuid::Uuid::new_v4().to_string();

    Json(ApiResponse::success(workspace_id))
}

/// 删除工作区
async fn delete_workspace(
    State(_state): State<Arc<RwLock<HttpApiState>>>,
    axum::extract::Path(workspace_id): axum::extract::Path<String>,
) -> Json<ApiResponse<bool>> {
    info!("HTTP API: delete_workspace called for: {}", workspace_id);

    // 暂时返回成功
    Json(ApiResponse::success(true))
}

/// 取消任务
async fn cancel_task(
    State(_state): State<Arc<RwLock<HttpApiState>>>,
    Json(req): Json<CancelTaskRequest>,
) -> Json<ApiResponse<bool>> {
    info!("HTTP API: cancel_task called for: {}", req.task_id);

    // 暂时返回成功
    Json(ApiResponse::success(true))
}

/// 加载配置
async fn load_config(
    State(_state): State<Arc<RwLock<HttpApiState>>>,
) -> Json<ApiResponse<serde_json::Value>> {
    // 返回默认配置
    Json(ApiResponse::success(serde_json::json!({
        "search": {
            "max_results": 50000,
            "cache_enabled": true
        },
        "cache": {
            "max_size_mb": 512,
            "ttl_seconds": 3600
        }
    })))
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
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/health", get(health_check))
        .route("/api/workspaces", get(list_workspaces))
        .route("/api/workspace", post(create_workspace))
        .route("/api/workspace/:id/status", get(get_workspace_status))
        .route("/api/workspace/:id", delete(delete_workspace))
        .route("/api/search", post(search_logs))
        .route("/api/task/cancel", post(cancel_task))
        .route("/api/config", get(load_config))
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
