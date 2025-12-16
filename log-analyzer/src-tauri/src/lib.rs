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
    search::search_logs,
    watch::{start_watch, stop_watch},
    workspace::{delete_workspace, load_workspace, refresh_workspace},
};

/// 锁管理器，用于避免死锁
pub struct LockManager;

impl LockManager {
    /// 安全地获取多个锁，按锁的地址排序以避免死锁
    pub fn acquire_multiple_locks<T>(
        locks: Vec<&Mutex<T>>,
    ) -> Vec<std::sync::MutexGuard<'_, T>> {
        // 按锁的地址排序，确保所有线程以相同顺序获取锁
        let mut sorted_locks: Vec<_> = locks.into_iter().enumerate().collect();
        sorted_locks.sort_by_key(|(_, lock)| lock as *const _ as usize);
        
        sorted_locks.into_iter().map(|(_, lock)| lock.lock().unwrap()).collect()
    }
    
    /// 安全地获取两个锁
    pub fn acquire_two_locks<'a, T, U>(
        lock1: &'a Mutex<T>,
        lock2: &'a Mutex<U>,
    ) -> (std::sync::MutexGuard<'a, T>, std::sync::MutexGuard<'a, U>) {
        let locks = vec![lock1 as &Mutex<_>, lock2 as &Mutex<_>];
        let guards = Self::acquire_multiple_locks(locks);
        
        // 安全地转换回具体类型
        // 注意：这里使用了 unsafe，因为我们知道 guards 的顺序
        unsafe {
            let guard1 = std::mem::transmute_copy(&guards[0]);
            let guard2 = std::mem::transmute_copy(&guards[1]);
            std::mem::forget(guards); // 防止 guards 被 drop
            (guard1, guard2)
        }
    }
}

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
    use crate::utils::{normalize_path_separator, validate_workspace_id};

    #[test]
    fn test_path_utils() {
        // 测试路径规范化（Windows 上将 / 转为 \uff09
        #[cfg(target_os = "windows")]
        {
            let normalized = normalize_path_separator("test/path");
            assert_eq!(normalized, "test\\path");
        }
        #[cfg(not(target_os = "windows"))]
        {
            let normalized = normalize_path_separator("test/path");
            assert_eq!(normalized, "test/path");
        }
    }

    #[test]
    fn test_workspace_id_validation() {
        assert!(validate_workspace_id("valid-id-123").is_ok());
        assert!(validate_workspace_id("").is_err());
        assert!(validate_workspace_id("../invalid").is_err());
        assert!(validate_workspace_id("invalid/id").is_err());
        assert!(validate_workspace_id("invalid\\id").is_err());
    }
}
