//! WorkspaceServiceFactory — 工作区服务创建工厂。
//!
//! 从 `commands/import.rs` 提取（P7），消除跨命令文件导入。
//! 集中管理 WorkspaceServiceImpl 的组装逻辑，命令层通过工厂获取服务。

use std::path::Path;
use std::sync::Arc;

use tauri::AppHandle;
use tracing::info;

use crate::application::workspace_service::WorkspaceServiceRef;
use crate::infrastructure::{TauriEventPublisher, WorkspaceRepo, WorkspaceServiceImpl};
use crate::models::AppState;
use la_storage::{ContentAddressableStorage, MetadataStore};

const SEARCH_INDEX_DIR_NAME: &str = "search_index";
const SEARCH_INDEX_WRITER_HEAP_BYTES: usize = 50_000_000;

// ============================================================================
// 配置加载
// ============================================================================

/// 加载工作区搜索配置（用于 SearchEngineManager 初始化）。
pub(crate) fn load_workspace_search_config(
    app: &AppHandle,
) -> la_core::models::config::SearchConfig {
    crate::utils::load_app_config(app)
        .map(|c| c.search)
        .unwrap_or_default()
}

// ============================================================================
// 搜索引擎管理器创建
// ============================================================================

/// 确保 SearchEngineManager 已初始化。
///
/// 优先从已有的 workspace service 获取，否则新建。
/// 接受预加载的 SearchConfig 以避免重复读取 config.json。
pub(crate) fn ensure_search_engine_manager_with_config(
    state: &AppState,
    workspace_id: &str,
    workspace_dir: &Path,
    search_config: &la_core::models::config::SearchConfig,
) -> Result<Arc<la_search::SearchEngineManager>, String> {
    if let Some(service) = state.get_workspace_service(workspace_id) {
        return Ok(Arc::clone(service.search_engine()));
    }

    let index_path = workspace_dir.join(SEARCH_INDEX_DIR_NAME);
    let manager = Arc::new(
        la_search::SearchEngineManager::with_app_config(
            search_config.clone(),
            index_path,
            SEARCH_INDEX_WRITER_HEAP_BYTES,
        )
        .map_err(|e| format!("Failed to initialize search engine: {e}"))?,
    );

    Ok(manager)
}

// ============================================================================
// 服务工厂
// ============================================================================

/// 获取或创建工作区服务实例。
///
/// 如果服务已存在于 AppState 中则直接返回，否则创建新实例并存储。
pub(crate) async fn get_or_create_workspace_service(
    app: &AppHandle,
    state: &AppState,
    workspace_id: &str,
    workspace_dir: &Path,
) -> Result<WorkspaceServiceRef, String> {
    // 优先返回已存在的服务
    if let Some(service) = state.get_workspace_service(workspace_id) {
        return Ok(service);
    }

    // 创建各运行时组件
    let cas = Arc::new(ContentAddressableStorage::new(workspace_dir.to_path_buf()));

    let metadata_store = Arc::new(
        MetadataStore::new(workspace_dir)
            .await
            .map_err(|e| format!("Failed to open metadata store: {e}"))?,
    );

    // 预加载配置以避免 ensure_search_engine_manager 重复读取 config.json
    let search_config = load_workspace_search_config(app);
    let search_manager = ensure_search_engine_manager_with_config(
        state,
        workspace_id,
        workspace_dir,
        &search_config,
    )?;

    let disk_result_store = state
        .get_disk_result_store()
        .ok_or("Disk result store not initialized")?;
    let thread_pool = state.get_search_thread_pool();
    let regex_cache_size = search_config.regex_cache_size.max(1);

    let repo = WorkspaceRepo::new(cas, metadata_store, search_manager, disk_result_store);

    let service = Arc::new(WorkspaceServiceImpl::new(
        workspace_id.to_string(),
        workspace_dir.to_path_buf(),
        repo,
        Arc::new(TauriEventPublisher {
            app_handle: app.clone(),
        }),
        thread_pool,
        regex_cache_size,
    ));

    state.set_workspace_service(
        workspace_id.to_string(),
        service.clone() as WorkspaceServiceRef,
    );
    info!(
        workspace_id = %workspace_id,
        "WorkspaceService created and registered"
    );

    Ok(service as WorkspaceServiceRef)
}
