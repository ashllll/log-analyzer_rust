//! 日志分析器 - Rust 后端
//!
//! 提供高性能的日志分析功能，包括：
//! - 多格式压缩包递归解压
//! - 并行全文搜索
//! - 结构化查询系统
//! - 索引持久化与增量更新
//! - 实时文件监听

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// 模块声明
mod archive;
mod benchmark;
mod commands;
mod error;
mod models;
mod services;
mod utils;

// 从模块导入类型
use models::AppState;
pub use error::{AppError, Result};

// --- Commands ---

// 命令实现位于 commands 模块
use commands::{
    config::{load_config, save_config},
    export::export_results,
    import::{check_rar_support, import_folder},
    performance::get_performance_metrics,
    query::{execute_structured_query, validate_query},
    search::search_logs,
    watch::{start_watch, stop_watch},
    workspace::{delete_workspace, load_workspace, refresh_workspace},
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 设置全局 panic hook
    std::panic::set_hook(Box::new(|panic_info| {
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

    eprintln!(
        "[INFO] Rayon thread pool initialized with {} threads",
        num_cpus
    );

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            temp_dir: Mutex::new(None),
            path_map: Arc::new(Mutex::new(HashMap::new())), // 使用 Arc
            file_metadata: Arc::new(Mutex::new(HashMap::new())), // 元数据
            workspace_indices: Mutex::new(HashMap::new()),
            search_cache: Arc::new(Mutex::new(lru::LruCache::new(
                std::num::NonZeroUsize::new(100).unwrap(), // 缓存 100 个搜索结果
            ))),
            // 性能统计
            last_search_duration: Arc::new(Mutex::new(0)),
            total_searches: Arc::new(Mutex::new(0)),
            cache_hits: Arc::new(Mutex::new(0)),
            // 实时监听
            watchers: Arc::new(Mutex::new(HashMap::new())),
            // 临时文件清理队列
            cleanup_queue: Arc::new(Mutex::new(Vec::new())),
        })
        .invoke_handler(tauri::generate_handler![
            save_config,
            load_config,
            search_logs,
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
    use crate::services::{get_file_metadata, parse_metadata};
    use crate::utils::encoding::decode_filename;
    use crate::utils::path::{remove_readonly, safe_path_join};
    use crate::utils::{
        canonicalize_path, normalize_path_separator, validate_path_param, validate_workspace_id,
    };
    use std::path::PathBuf;

    #[test]
    fn test_path_utils() {
        // 测试路径工具函数
        let path = PathBuf::from("test/path");
        let normalized = normalize_path_separator(&path);
        assert!(normalized.contains('/'));

        let validated = validate_path_param(&path).unwrap();
        assert!(validated.is_absolute());
    }

    #[test]
    fn test_workspace_id_validation() {
        assert!(validate_workspace_id("valid-id-123").is_ok());
        assert!(validate_workspace_id("").is_err());
        assert!(validate_workspace_id("invalid@id!").is_err());
    }
}
