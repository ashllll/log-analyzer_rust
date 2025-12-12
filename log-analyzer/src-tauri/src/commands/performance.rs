//! 性能监控命令实现

use std::fs;
use std::path::Path;

use sysinfo::System;

use tauri::{command, AppHandle, Manager, State};

use crate::models::{AppState, PerformanceMetrics};

#[command]
pub async fn get_performance_metrics(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<PerformanceMetrics, String> {
    let memory_used_mb = get_process_memory_mb();

    let path_map_size = state
        .path_map
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?
        .len();

    let cache_size = state
        .search_cache
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?
        .len();

    let last_search_duration_ms = *state
        .last_search_duration
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?;

    let total_searches = *state
        .total_searches
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?;

    let cache_hits = *state
        .cache_hits
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?;

    let cache_hit_rate = if total_searches > 0 {
        (cache_hits as f64 / total_searches as f64) * 100.0
    } else {
        0.0
    };

    let index_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?
        .join("indices");

    let index_file_size_mb = if index_dir.exists() {
        calculate_dir_size(&index_dir)
            .map(|bytes| bytes as f64 / 1024.0 / 1024.0)
            .unwrap_or(0.0)
    } else {
        0.0
    };

    Ok(PerformanceMetrics {
        memory_used_mb,
        path_map_size,
        cache_size,
        last_search_duration_ms,
        cache_hit_rate,
        indexed_files_count: path_map_size,
        index_file_size_mb,
    })
}

fn get_process_memory_mb() -> f64 {
    if let Ok(pid) = sysinfo::get_current_pid() {
        // 使用正确的刷新方法，先获取系统信息
        let system = System::new_all();

        if let Some(process) = system.process(pid) {
            // sysinfo 返回 KiB，这里统一转换为 MB
            return process.memory() as f64 / 1024.0;
        }
    }

    0.0
}

fn calculate_dir_size(dir: &Path) -> Result<u64, std::io::Error> {
    let mut total_size = 0u64;

    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                total_size += calculate_dir_size(&path)?;
            } else if path.is_file() {
                total_size += entry.metadata()?.len();
            }
        }
    }

    Ok(total_size)
}
