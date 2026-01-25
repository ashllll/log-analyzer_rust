//! 日志分析器 - 主入口
//!
//! 应用程序入口点，负责：
//! - 初始化日志系统
//! - 配置 Tauri 应用
//! - 注册所有命令处理器
//! - 管理应用状态

// 导入 log_analyzer 库的模块
use log_analyzer::commands::{
    config::*, error_reporting::*, export::*, import::*, legacy::*, query::*, search::*,
    state_sync::*, virtual_tree::*, watch::*, workspace::*,
};
use log_analyzer::models::AppState;
use log_analyzer::task_manager::TaskManager;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt::init();

    info!("🚀 Log Analyzer v{} - 启动中...", env!("CARGO_PKG_VERSION"));

    tauri::Builder::default()
        // 初始化 dialog 插件（供前端使用）
        .plugin(tauri_plugin_dialog::init())
        // 管理应用状态
        .manage(AppState::default())
        // 初始化后设置 TaskManager
        .setup(|app| {
            use log_analyzer::models::AppState;
            use tauri::Manager;

            let app_state: tauri::State<'_, AppState> = app.state();

            // 初始化 TaskManager
            let task_manager = TaskManager::new(
                app.handle().clone(),
                log_analyzer::task_manager::TaskManagerConfig::default(),
            )?;

            // 设置到 AppState
            let mut state_guard = app_state.task_manager.lock();
            *state_guard = Some(task_manager);

            info!("✅ TaskManager 初始化成功");
            Ok(())
        })
        // 注册所有命令
        .invoke_handler(tauri::generate_handler![
            // ===== 配置管理 =====
            load_config,
            save_config,
            get_file_filter_config,
            save_file_filter_config,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    Ok(())
}
