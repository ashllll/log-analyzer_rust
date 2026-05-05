//! 日志分析器 - 主入口
//!
//! 应用程序入口点，负责：
//! - 初始化日志系统
//! - 配置 Tauri 应用
//! - 注册所有命令处理器
//! - 管理应用状态

// 导入 log_analyzer 库的模块
use log_analyzer::commands::{
    async_search::*, config::*, export::*, import::*, search::*, state_sync::*, validation::*,
    virtual_tree::*, watch::*, workspace::*,
};
use log_analyzer::models::{AppState, CacheState, SearchState, WorkspaceState};
use log_analyzer::task_manager::TaskManager;
use log_analyzer::utils::cache_manager::{
    CacheConfig as RuntimeCacheConfig, CacheManager, CacheThresholds,
};
use moka::sync::Cache;
use std::{sync::Arc, time::Duration};
use tauri::Manager;
use tracing::info;

fn load_app_config(app: &tauri::AppHandle) -> Option<la_core::models::config::AppConfig> {
    let config_path = app.path().app_config_dir().ok()?.join("config.json");
    if !config_path.exists() {
        return None;
    }

    la_core::models::config::AppConfigLoader::load(Some(config_path))
        .ok()
        .map(|loader| loader.get_config().clone())
}

fn build_runtime_cache_manager(
    cache_config: &la_core::models::config::CacheConfig,
) -> CacheManager {
    let runtime_config = RuntimeCacheConfig {
        max_capacity: cache_config.max_cache_capacity as u64,
        ttl: Duration::from_secs(cache_config.cache_ttl_seconds),
        tti: Duration::from_secs(cache_config.cache_tti_seconds),
        compression_threshold: cache_config.compression_threshold as usize,
        enable_compression: cache_config.compression_enabled,
        access_pattern_window: cache_config.access_window_size,
        preload_threshold: cache_config.preload_threshold as u32,
    };

    let thresholds = CacheThresholds {
        min_hit_rate: cache_config.min_hit_rate_threshold,
        max_avg_access_time_ms: cache_config.max_avg_access_time_ms,
        max_avg_load_time_ms: cache_config.max_avg_load_time_ms,
        max_eviction_rate_per_minute: cache_config.max_eviction_rate_per_min,
    };

    let sync_cache = Cache::builder()
        .max_capacity(runtime_config.max_capacity)
        .time_to_live(runtime_config.ttl)
        .time_to_idle(runtime_config.tti)
        .build();

    CacheManager::with_thresholds(Arc::new(sync_cache), runtime_config, thresholds)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志系统
    // 使用条件编译：debug 模式启用 DEBUG 级别，release 模式启用 INFO 级别
    init_logging_with_profile();

    fn init_logging_with_profile() {
        use tracing_subscriber::EnvFilter;

        let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            #[cfg(debug_assertions)]
            {
                // Debug 模式：启用 DEBUG 级别日志
                EnvFilter::new("debug")
                    .add_directive("log_analyzer::task_manager=info".parse().unwrap())
                    .add_directive("log_analyzer::search_engine=info".parse().unwrap())
                    .add_directive("log_analyzer::cache_manager=info".parse().unwrap())
            }
            #[cfg(not(debug_assertions))]
            {
                // Release 模式：启用 INFO 级别日志，高频模块使用 WARN
                EnvFilter::new("info")
                    .add_directive("log_analyzer::task_manager=warn".parse().unwrap())
                    .add_directive("log_analyzer::search_engine=warn".parse().unwrap())
                    .add_directive("log_analyzer::cache_manager=warn".parse().unwrap())
                    .add_directive("log_analyzer::commands=info".parse().unwrap())
            }
        });

        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(true)
            .with_thread_ids(false)
            .with_thread_names(false)
            .with_file(false)
            .with_line_number(false)
            .init();
    }

    info!("🚀 Log Analyzer v{} - 启动中...", env!("CARGO_PKG_VERSION"));

    tauri::Builder::default()
        // 初始化 dialog 插件（供前端使用）
        .plugin(tauri_plugin_dialog::init())
        // 初始化 opener 插件（供前端打开外链使用）
        .plugin(tauri_plugin_opener::init())
        // 管理应用状态 - 领域驱动拆分后的独立状态
        .manage(AppState::default())
        .manage(WorkspaceState::default())
        .manage(SearchState::default())
        .manage(CacheState::default())
        // 初始化后设置 TaskManager
        .setup(|app| {
            use log_analyzer::models::AppState;

            let app_state: tauri::State<'_, AppState> = app.state();
            let app_config = load_app_config(app.app_handle());

            if let Some(config) = app_config.as_ref() {
                let cache_manager = build_runtime_cache_manager(&config.cache);
                let mut cache_guard = app_state.cache_manager.lock();
                *cache_guard = cache_manager;
                info!("✅ 已从配置文件加载运行时缓存参数");
            }

            let task_manager_config = app_config
                .as_ref()
                .map(|config| {
                    log_analyzer::task_manager::TaskManagerConfig::from_app_config(
                        &config.task_manager,
                    )
                })
                .unwrap_or_default();

            // 初始化 TaskManager
            let task_manager = TaskManager::new(app.handle().clone(), task_manager_config)?;

            // 设置到 AppState
            let mut state_guard = app_state.task_manager.lock();
            *state_guard = Some(task_manager);

            info!("✅ TaskManager 初始化成功");

            // M4 Fix: Initialize DiskResultStore at app data dir (persistent)
            // instead of the OS temp directory (volatile, may be cleaned by system)
            if let Ok(app_data_dir) = app.path().app_data_dir() {
                app_state.init_disk_result_store_at(app_data_dir);
            }

            info!("✅ 应用初始化完成");
            Ok(())
        })
        // 注册所有命令
        .invoke_handler(tauri::generate_handler![
            // ===== 配置管理 =====
            load_config,
            save_config,
            get_file_filter_config,
            save_file_filter_config,
            get_cache_config,
            save_cache_config,
            get_search_config,
            save_search_config,
            get_task_manager_config,
            save_task_manager_config,
            // ===== 工作区管理 =====
            load_workspace,
            refresh_workspace,
            delete_workspace,
            cancel_task,
            get_workspace_status,
            get_workspace_time_range,
            // ===== 文件监听 =====
            start_watch,
            stop_watch,
            // ===== 虚拟文件树 =====
            read_file_by_hash,
            // ===== 日志搜索 =====
            search_logs,
            cancel_search,
            fetch_search_page,
            // ===== 导入 =====
            import_folder,
            check_rar_support,
            // ===== 导出 =====
            export_results,
            // ===== 异步搜索 =====
            async_search_logs,
            cancel_async_search,
            get_active_searches_count,
            // ===== 状态同步 =====
            init_state_sync,
            // ===== 验证 =====
            validate_workspace_id_format,
            validate_path_security,
            validate_workspace_config_cmd,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::ExitRequested { .. } = event {
                info!("应用退出请求，执行清理");
                let state = app_handle.state::<AppState>();

                // 1. 清理 DiskResultStore（先执行，释放文件句柄）
                let disk_store = state.disk_result_store.read().clone();
                disk_store.cleanup_all();

                // 2. 清理异步组件（MetadataStore / SearchEngineManager / TaskManager）
                let metadata_stores = Arc::clone(&state.metadata_stores);
                let search_engine_managers = Arc::clone(&state.search_engine_managers);
                let task_manager = Arc::clone(&state.task_manager);

                let handle = std::thread::spawn(move || {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build();
                    match rt {
                        Ok(rt) => rt.block_on(async {
                            let stores: Vec<_> = metadata_stores.lock().values().cloned().collect();
                            for store in stores {
                                store.close().await;
                            }
                            let engines: Vec<_> =
                                search_engine_managers.lock().values().cloned().collect();
                            for mgr in engines {
                                let _ = tokio::time::timeout(
                                    std::time::Duration::from_secs(3),
                                    mgr.close(),
                                )
                                .await;
                            }
                            let tm_opt = task_manager.lock().take();
                            if let Some(tm) = tm_opt {
                                let _ = tokio::time::timeout(
                                    std::time::Duration::from_secs(5),
                                    tm.shutdown_async(),
                                )
                                .await;
                            }
                        }),
                        Err(e) => {
                            tracing::error!(error = %e, "创建清理用 tokio runtime 失败");
                        }
                    }
                });
                if let Err(e) = handle.join() {
                    tracing::error!(error = ?e, "应用退出清理线程 panic");
                }

                info!("应用退出清理完成");
            }
        });

    Ok(())
}
