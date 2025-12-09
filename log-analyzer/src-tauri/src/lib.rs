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
mod commands;
mod models;
mod services;
mod utils;

// 从模块导入类型
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
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    #[test]
    fn test_canonicalize_path() {
        let current_dir = std::env::current_dir().unwrap();
        let canonical = canonicalize_path(&current_dir);
        assert!(canonical.is_ok());

        let non_existent = Path::new("/path/that/does/not/exist/123456789");
        let result = canonicalize_path(non_existent);
        assert!(result.is_err());
    }

    #[test]
    fn test_normalize_path_separator() {
        let path = "folder/subfolder/file.txt";
        let normalized = normalize_path_separator(path);

        #[cfg(target_os = "windows")]
        assert_eq!(normalized, "folder\\subfolder\\file.txt");

        #[cfg(not(target_os = "windows"))]
        assert_eq!(normalized, "folder/subfolder/file.txt");
    }

    #[test]
    fn test_remove_readonly() -> Result<(), String> {
        let temp_dir = TempDir::new().map_err(|e| e.to_string())?;
        let test_file = temp_dir.path().join("readonly_test.txt");

        fs::write(&test_file, "test").map_err(|e| e.to_string())?;

        let metadata = test_file.metadata().map_err(|e| e.to_string())?;
        let mut perms = metadata.permissions();
        perms.set_readonly(true);
        fs::set_permissions(&test_file, perms).map_err(|e| e.to_string())?;

        let result = remove_readonly(&test_file);
        assert!(result.is_ok());

        #[cfg(target_os = "windows")]
        {
            let metadata = test_file.metadata().map_err(|e| e.to_string())?;
            assert!(!metadata.permissions().readonly());
        }

        Ok(())
    }

    #[test]
    fn test_get_file_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("metadata_test.txt");

        fs::write(&test_file, "test content").unwrap();

        let metadata = get_file_metadata(&test_file);
        assert!(metadata.is_ok());

        let metadata = metadata.unwrap();
        assert_eq!(metadata.size, 12);
        assert!(metadata.modified_time > 0);
    }

    #[test]
    fn test_parse_metadata() {
        let (ts, lvl) = parse_metadata("2024-01-01 12:00:00 ERROR Something went wrong");
        assert_eq!(lvl, "ERROR");
        assert_eq!(ts, "2024-01-01 12:00:00");

        let (_ts, lvl) = parse_metadata("2024-01-01 12:00:00 WARN Warning message");
        assert_eq!(lvl, "WARN");

        let (_ts, lvl) = parse_metadata("2024-01-01 12:00:00 INFO Info message");
        assert_eq!(lvl, "INFO");

        let (_ts, lvl) = parse_metadata("2024-01-01 12:00:00 Other message");
        assert_eq!(lvl, "DEBUG");

        let (ts, _) = parse_metadata("short");
        assert_eq!(ts, "");
    }

    #[test]
    fn test_safe_path_join() {
        let base = Path::new("/base");

        // 正常路径
        let result = safe_path_join(base, "normal/path.txt");
        assert!(result.to_string_lossy().contains("normal"));
        assert!(result.to_string_lossy().contains("path.txt"));

        // 路径穿越被清理
        let result = safe_path_join(base, "../../../etc/passwd");
        assert!(!result.to_string_lossy().contains(".."));

        // Windows 驱动器符号被清理
        let result = safe_path_join(base, "C:evil:path");
        assert!(!result.to_string_lossy().contains(":"));
    }

    #[test]
    fn test_decode_filename() {
        let utf8_bytes = "test.txt".as_bytes();
        let result = decode_filename(utf8_bytes);
        assert_eq!(result, "test.txt");

        let invalid_bytes = vec![0xFF, 0xFE, 0xFD];
        let result = decode_filename(&invalid_bytes);
        assert!(result.contains("�") || !result.is_empty());
    }

    #[test]
    fn test_validate_path_param() {
        // 测试空路径
        let result = validate_path_param("", "test_path");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cannot be empty"));

        // 测试不存在的路径
        let result = validate_path_param("/nonexistent/path/12345", "test_path");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));

        // 测试存在的路径
        let current_dir = std::env::current_dir().unwrap();
        let result = validate_path_param(&current_dir.to_string_lossy(), "test_path");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_workspace_id() {
        // 测试空 ID
        let result = validate_workspace_id("");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cannot be empty"));

        // 测试包含路径穿越
        let result = validate_workspace_id("../evil");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid characters"));

        // 测试包含路径分隔符
        let result = validate_workspace_id("folder/subfolder");
        assert!(result.is_err());

        // 测试合法 ID
        let result = validate_workspace_id("workspace_123");
        assert!(result.is_ok());
    }

    // 注意：get_bundled_unrar_path 需要 AppHandle，无法在单元测试中测试
    // 该功能通过集成测试（实际运行应用并导入 RAR 文件）进行验证
}
