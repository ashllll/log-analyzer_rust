//! 日志分析器 - Rust 后端
//!
//! 提供高性能的日志分析功能，包括：
//! - 多格式压缩包递归解压
//! - 并行全文搜索
//! - 结构化查询系统
//! - 索引持久化与增量更新
//! - 实时文件监听

use crossbeam::queue::SegQueue;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;

// 模块声明
mod archive;
mod benchmark;
mod commands;
mod error;
mod models;
mod monitoring;
pub mod services; // 公开 services 模块用于基准测试
pub mod utils; // 公开 utils 模块用于基准测试

// 从模块导入类型
pub use error::{AppError, Result};
use models::AppState;

// --- Commands ---

// 命令实现位于 commands 模块
use commands::{
    config::{load_config, save_config},
    export::export_results,
    import::{check_rar_support, import_folder},
    performance::get_performance_metrics,
    query::{execute_structured_query, validate_query},
    search::{cancel_search, search_logs},
    watch::{start_watch, stop_watch},
    workspace::{delete_workspace, load_workspace, refresh_workspace},
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 初始化 color-eyre 用于增强的错误报告
    color_eyre::install().expect("Failed to install color-eyre");

    // 初始化 tracing 结构化日志系统
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    tracing_subscriber::registry()
        .with(fmt::layer().with_target(true).with_thread_ids(true))
        .with(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .init();

    tracing::info!("Starting log analyzer application");

    // 设置全局 panic hook
    std::panic::set_hook(Box::new(|panic_info| {
        tracing::error!("Application panic: {:?}", panic_info);
        eprintln!("[PANIC] Application panic: {:?}", panic_info);
    }));

    // 配置 Rayon 线程池（优化多核性能）
    let num_cpus = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4); // 默认 4 线程

    rayon::ThreadPoolBuilder::new()
        .num_threads(num_cpus)
        .thread_name(|idx| format!("rayon-worker-{}", idx))
        .build_global()
        .expect("Failed to build Rayon thread pool");

    tracing::info!("Rayon thread pool initialized with {} threads", num_cpus);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage({
            let cleanup_queue = Arc::new(SegQueue::new());
            let resource_manager = Arc::new(utils::ResourceManager::new(cleanup_queue.clone()));
            let cancellation_manager = Arc::new(utils::CancellationManager::new());
            let resource_tracker = Arc::new(utils::ResourceTracker::new(cleanup_queue.clone()));

            AppState {
                temp_dir: Mutex::new(None),
                path_map: Arc::new(Mutex::new(HashMap::new())),
                file_metadata: Arc::new(Mutex::new(HashMap::new())),
                workspace_indices: Mutex::new(HashMap::new()),
                search_cache: Arc::new(
                    moka::sync::Cache::builder()
                        .max_capacity(100)
                        .time_to_live(std::time::Duration::from_secs(300))
                        .time_to_idle(std::time::Duration::from_secs(60))
                        .build(),
                ),
                last_search_duration: Arc::new(Mutex::new(0)),
                total_searches: Arc::new(Mutex::new(0)),
                cache_hits: Arc::new(Mutex::new(0)),
                watchers: Arc::new(Mutex::new(HashMap::new())),
                cleanup_queue,
                search_cancellation_tokens: Arc::new(Mutex::new(HashMap::new())),
                resource_manager,
                cancellation_manager,
                resource_tracker,
            }
        })
        .invoke_handler(tauri::generate_handler![
            save_config,
            load_config,
            search_logs,
            cancel_search,
            import_folder,
            load_workspace,
            refresh_workspace,
            export_results,
            get_performance_metrics,
            check_rar_support,
            start_watch,
            stop_watch,
            execute_structured_query,
            validate_query,
            delete_workspace,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// ============================================================================
// 单元测试（私有函数）
// ============================================================================

#[cfg(test)]
mod tests {
    use crate::utils::validation::{validate_path_param, validate_workspace_id};
    use std::path::PathBuf;

    #[test]
    fn test_path_utils() {
        // 测试路径工具函数 - 使用当前目录而不是不存在的路径
        let path = ".";
        let validated = validate_path_param(path, "test_path").unwrap();
        assert!(validated.is_absolute());
    }

    #[test]
    fn test_workspace_id_validation() {
        assert!(validate_workspace_id("valid-id-123").is_ok());
        assert!(validate_workspace_id("").is_err());
        assert!(validate_workspace_id("invalid@id!").is_err());
    }
}
