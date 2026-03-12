//! Standalone 二进制入口 - 包含 Tauri 运行时
//!
//! 此模式用于:
//! - 测试 Tauri 插件功能
//! - 桌面应用模式
//! - 调试和开发

use log_analyzer::commands::{
    archive::*, async_search::*, cache::*, config::*, error_reporting::*, export::*,
    import::*, legacy::*, performance::*, query::*, search::*, search_history::*, state_sync::*,
    validation::*, virtual_tree::*, watch::*, workspace::*,
};
use log_analyzer::models::AppState;
use log_analyzer::task_manager::TaskManager;
use tracing::info;

pub async fn run() -> eyre::Result<()> {
    // 初始化错误报告
    color_eyre::install()?;

    // 初始化日志
    tracing_subscriber::fmt::init();

    info!(
        "🚀 Log Analyzer Standalone v{} - 启动中...",
        env!("CARGO_PKG_VERSION")
    );

    tauri::Builder::default()
        // 初始化 dialog 插件（供前端使用）
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        // 管理应用状态
        .manage(AppState::default())
        // 初始化后设置 TaskManager
        .setup(|app| {
            use log_analyzer::models::config::AppConfigLoader;
            use log_analyzer::models::AppState;
            use tauri::Manager;

            let app_state: tauri::State<'_, AppState> = app.state();

            // 从配置文件加载 TaskManager 配置
            let config_path = app
                .path()
                .app_config_dir()
                .ok()
                .map(|p| p.join("config.json"));
            let task_manager_config = if let Some(ref path) = config_path {
                if path.exists() {
                    AppConfigLoader::load(Some(path.clone()))
                        .ok()
                        .map(|loader| {
                            let config = loader.get_config();
                            log_analyzer::task_manager::TaskManagerConfig::from_app_config(
                                &config.task_manager,
                            )
                        })
                        .unwrap_or_default()
                } else {
                    log_analyzer::task_manager::TaskManagerConfig::default()
                }
            } else {
                log_analyzer::task_manager::TaskManagerConfig::default()
            };

            // 初始化 TaskManager
            let task_manager = TaskManager::new(app.handle().clone(), task_manager_config)?;

            // 设置到 AppState
            let mut state_guard = app_state.task_manager.lock();
            *state_guard = Some(task_manager);

            info!("✅ TaskManager 初始化成功");

            // 初始化 FFI 全局状态（用于 Flutter FFI 调用兼容）
            #[cfg(feature = "ffi")]
            {
                use log_analyzer::ffi::init_global_state;
                let app_state_clone = app_state.inner().clone();
                let app_data_dir = app
                    .path()
                    .app_data_dir()
                    .map_err(|e| eyre::eyre!("Failed to get app data dir: {}", e))?;
                init_global_state(app_state_clone, app_data_dir);
                info!("✅ FFI 全局状态初始化成功");
            }

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
            create_workspace,
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
            // ===== 导入 =====
            import_folder,
            check_rar_support,
            // ===== 压缩包浏览 =====
            list_archive_contents,
            read_archive_file,
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
            // ===== 性能监控 =====
            get_performance_metrics,
            get_historical_metrics,
            get_aggregated_metrics,
            get_search_events,
            get_metrics_stats,
            cleanup_metrics_data,
            // ===== 搜索历史 =====
            add_search_history,
            get_search_history,
            clear_search_history,
        ])
        .run(tauri::generate_context!())
        .map_err(|e| eyre::eyre!("Tauri application error: {}", e))?;

    Ok(())
}
