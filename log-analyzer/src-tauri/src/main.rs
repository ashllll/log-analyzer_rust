//! 日志分析器 - 主入口
//!
//! 应用程序入口点，负责：
//! - 初始化日志系统
//! - 配置 Tauri 应用
//! - 注册所有命令处理器
//! - 管理应用状态

// 导入 log_analyzer 库的模块（所有命令均在 commands/ 层定义）
use log_analyzer::commands::{
    config::*, export::*, import::*, log_config::*, search::*, state_sync::*,
    validation::*, virtual_tree::*, watch::*, workspace::*,
};
use log_analyzer::models::AppState;
use log_analyzer::task_manager::TaskManager;
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
                let mut f = EnvFilter::new("debug");
                if let Ok(d) = "log_analyzer::task_manager=info".parse() {
                    f = f.add_directive(d);
                }
                if let Ok(d) = "la_search=info".parse() {
                    f = f.add_directive(d);
                }
                f
            }
            #[cfg(not(debug_assertions))]
            {
                // Release 模式：启用 INFO 级别日志，高频模块使用 WARN
                let mut f = EnvFilter::new("info");
                if let Ok(d) = "log_analyzer::task_manager=warn".parse() {
                    f = f.add_directive(d);
                }
                if let Ok(d) = "la_search=warn".parse() {
                    f = f.add_directive(d);
                }
                if let Ok(d) = "log_analyzer::commands=info".parse() {
                    f = f.add_directive(d);
                }
                f
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
        // 初始化后设置 TaskManager
        .setup(|app| {
            use log_analyzer::models::AppState;

            let app_state: tauri::State<'_, AppState> = app.state();
            let app_config = load_app_config(app.app_handle());

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
            app_state.init_task_manager(task_manager);

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
            get_search_config,
            save_search_config,
            get_task_manager_config,
            save_task_manager_config,
            // ===== 工作区管理 =====
            create_workspace,
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
            // ===== 状态同步 =====
            init_state_sync,
            // ===== 日志配置 =====
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
            // ===== 验证 =====
            validate_workspace_id_format,
            validate_path_security,
            validate_workspace_config_cmd,
            validate_search_query_cmd,
            validate_archive_config_cmd,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::ExitRequested { .. } = event {
                info!("应用退出请求，执行清理");
                let state = app_handle.state::<AppState>();

                // 1. 清理 DiskResultStore（先执行，释放文件句柄）
                state.cleanup_disk_result_store();

                // 2. 清理异步组件（MetadataStore / SearchEngineManager / TaskManager）
                let services = state.all_workspace_services();
                let task_manager = state.task_manager_arc();

                // block_in_place 允许在 async 上下文中执行阻塞操作，
                // 使用已有的 tokio runtime 而非创建新的
                tokio::task::block_in_place(|| {
                    let rt = tokio::runtime::Handle::current();
                    rt.block_on(async {
                        for svc in services {
                            svc.metadata_store().close().await;
                            let _ = tokio::time::timeout(
                                std::time::Duration::from_secs(3),
                                svc.search_engine().close(),
                            )
                            .await;
                        }
                        let tm_opt = task_manager.lock().take();
                        if let Some(tm) = tm_opt {
                            let _ = tokio::time::timeout(
                                std::time::Duration::from_secs(5),
                                tm.shutdown(),
                            )
                            .await;
                        }
                    });
                });

                info!("应用退出清理完成");
            }
        });

    Ok(())
}
