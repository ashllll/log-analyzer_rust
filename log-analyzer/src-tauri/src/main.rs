//! 日志分析器 - 主入口
//!
//! 应用程序入口点，负责：
//! - 初始化日志系统
//! - 配置 Tauri 应用
//! - 注册所有命令处理器
//! - 管理应用状态

// 导入 log_analyzer 库的模块
use log_analyzer::commands::{
    async_search::*, cache::*, config::*, error_reporting::*, export::*, import::*, legacy::*,
    log_config::*, query::*, search::*, state_sync::*, validation::*, virtual_tree::*, watch::*,
    workspace::*,
};
use log_analyzer::models::{AppState, CacheState, SearchState, WorkspaceState};
use log_analyzer::task_manager::TaskManager;
use log_analyzer::utils::cache_manager::{
    CacheConfig as RuntimeCacheConfig, CacheManager, CacheThresholds,
};
use moka::sync::Cache;
use std::{sync::Arc, time::Duration};
use tauri::Manager;
use tracing::{error, info, warn};

fn load_app_config(app: &tauri::AppHandle) -> Option<la_core::models::config::AppConfig> {
    let config_path = app.path().app_config_dir().ok()?.join("config.json");
    if !config_path.exists() {
        return None;
    }

    log_analyzer::models::config::AppConfigLoader::load(Some(config_path))
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

            // 注册应用退出钩子（在 setup 外面注册）
            // 注意：这里不能在 setup 内注册，因为 app 生命周期有限
            // 实际的清理逻辑将在 window close 事件中处理

            info!("✅ 应用退出钩子已注册");
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
            // ===== 日志配置管理 =====
            get_current_log_config,
            set_log_level,
            set_module_level,
            reset_log_configuration,
            get_recommended_production_config,
            get_recommended_debug_config,
            load_log_config,
            save_log_config,
            get_available_log_levels,
            apply_log_preset,
            // ===== 工作区管理 =====
            load_workspace,
            refresh_workspace,
            delete_workspace,
            cancel_task,
            get_workspace_status,
            create_workspace,
            get_workspace_time_range,
            // ===== 文件监听 =====
            start_watch,
            stop_watch,
            // ===== 虚拟文件树 =====
            read_file_by_hash,
            get_virtual_file_tree,
            // ===== 结构化查询 =====
            execute_structured_query,
            validate_query,
            // ===== 传统格式检测 =====
            scan_legacy_formats,
            get_legacy_workspace_info,
            // ===== 日志搜索 =====
            search_logs,
            cancel_search,
            // ===== 流式搜索分页 (VirtualSearchManager) =====
            fetch_search_page,
            register_search_session,
            get_search_session_info,
            get_search_total_count,
            remove_search_session,
            cleanup_expired_search_sessions,
            get_virtual_search_stats,
            // ===== 导入 =====
            import_folder,
            check_rar_support,
            // ===== 错误报告 =====
            report_frontend_error,
            submit_user_feedback,
            get_error_statistics,
            // ===== 状态同步 =====
            init_state_sync,
            get_workspace_state,
            get_event_history,
            broadcast_test_event,
            // ===== 导出 =====
            export_results,
            // ===== 缓存管理 =====
            invalidate_workspace_cache,
            // ===== 数据验证 =====
            validate_workspace_config_cmd,
            validate_search_query_cmd,
            validate_archive_config_cmd,
            batch_validate_workspace_configs,
            validate_workspace_id_format,
            validate_path_security,
            // ===== 异步搜索 =====
            async_search_logs,
            cancel_async_search,
            get_active_searches_count,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            // 处理应用事件
            if let tauri::RunEvent::ExitRequested { .. } = event {
                info!("应用退出请求，开始清理资源");

                let state_guard = app_handle.state::<AppState>();

                // 1. 清理 DiskResultStore（删除磁盘缓存文件）
                state_guard.disk_result_store.read().cleanup_all();
                tracing::debug!("DiskResultStore 已清理所有磁盘缓存");

                // 2. 关闭所有 MetadataStore（WAL checkpoint）
                {
                    let stores_clone: Vec<_> = {
                        let stores = state_guard.metadata_stores.lock();
                        stores.values().cloned().collect()
                    };
                    if !stores_clone.is_empty() {
                        // H1 Fix: Reuse Tauri's global async runtime instead of creating a new one
                        tauri::async_runtime::block_on(async {
                            for store in stores_clone {
                                store.close().await;
                            }
                        });
                    }
                }

                // 3. 收集并关闭 SearchEngineManager 和 TaskManager
                let task_manager_opt: Option<TaskManager> = {
                    let guard = state_guard.task_manager.lock();
                    guard.as_ref().cloned()
                };

                let search_managers: Vec<_> = {
                    let managers = state_guard.search_engine_managers.lock();
                    managers.values().cloned().collect()
                };

                if let Some(task_manager) = task_manager_opt {
                    // H1 Fix: Reuse Tauri's global async runtime instead of creating a new one
                    tauri::async_runtime::block_on(async move {
                        // 关闭所有 SearchEngineManager（提交 IndexWriter 缓冲区）
                        for mgr in search_managers {
                            let close_result = tokio::time::timeout(
                                std::time::Duration::from_secs(3),
                                mgr.close(),
                            )
                            .await;
                            match close_result {
                                Ok(()) => info!("SearchEngineManager 已成功关闭"),
                                Err(_) => {
                                    error!("SearchEngineManager 关闭超时 (3秒)")
                                }
                            }
                        }

                        info!("正在关闭 TaskManager...");

                        // 使用 5 秒超时执行异步关闭
                        let shutdown_result = tokio::time::timeout(
                            std::time::Duration::from_secs(5),
                            task_manager.shutdown_async(),
                        )
                        .await;

                        match shutdown_result {
                            Ok(Ok(())) => info!("TaskManager 已成功关闭"),
                            Ok(Err(e)) => error!("TaskManager 关闭失败: {}", e),
                            Err(_) => error!("TaskManager 关闭超时 (5秒)"),
                        }
                    });
                } else {
                    warn!("TaskManager 未初始化，跳过清理");
                }

                info!("应用退出清理完成");
            }
        });

    Ok(())
}
