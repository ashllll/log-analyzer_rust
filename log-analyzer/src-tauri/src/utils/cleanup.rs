//! 临时文件清理工具
//!
//! 提供临时目录的清理功能，支持重试和清理队列机制。

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use super::retry::retry_file_operation;

#[cfg(target_os = "windows")]
use super::path::remove_readonly;

#[cfg(target_os = "windows")]
use walkdir::WalkDir;

/// 尝试清理临时目录
///
/// 尝试删除指定的临时目录,如果失败则将路径添加到清理队列供后续处理。
///
/// # 参数
///
/// - `path` - 要清理的路径
/// - `cleanup_queue` - 清理队列的Arc引用
///
/// # 功能
///
/// 1. 检查路径是否存在
/// 2. 在Windows上递归移除只读属性
/// 3. 重试删除目录(最多3次)
/// 4. 失败时添加到清理队列
///
/// # 使用场景
///
/// 用于删除工作区时清理解压目录,支持重试机制和延迟清理。
pub fn try_cleanup_temp_dir(path: &Path, cleanup_queue: &Arc<Mutex<Vec<PathBuf>>>) {
    if !path.exists() {
        return;
    }

    // 尝试重试3次删除目录
    let result = retry_file_operation(
        || {
            #[cfg(target_os = "windows")]
            {
                // Windows：递归移除只读属性
                for entry in WalkDir::new(path).into_iter().flatten() {
                    let _ = remove_readonly(entry.path());
                }
            }

            fs::remove_dir_all(path).map_err(|e| format!("Failed to remove directory: {}", e))
        },
        3,
        &[100, 500, 1000],
        &format!("cleanup_temp_dir({})", path.display()),
    );

    match result {
        Ok(_) => {
            eprintln!(
                "[INFO] Successfully cleaned up temp directory: {}",
                path.display()
            );
        }
        Err(e) => {
            eprintln!(
                "[WARN] Failed to clean up temp directory: {}. Adding to cleanup queue.",
                e
            );
            // 添加到清理队列
            if let Ok(mut queue) = cleanup_queue.lock() {
                queue.push(path.to_path_buf());
            }
        }
    }
}

/// 执行清理队列中的任务
///
/// 尝试清理队列中所有待处理的临时目录。
///
/// # 参数
///
/// - `cleanup_queue` - 清理队列的Arc引用
///
/// # 功能
///
/// 1. 获取队列中的所有路径
/// 2. 逐个尝试删除
/// 3. 统计成功和失败的数量
/// 4. 更新清理队列（保留失败的路径）
pub fn process_cleanup_queue(cleanup_queue: &Arc<Mutex<Vec<PathBuf>>>) {
    let paths_to_clean: Vec<PathBuf> = {
        if let Ok(queue) = cleanup_queue.lock() {
            queue.clone()
        } else {
            return;
        }
    };

    if paths_to_clean.is_empty() {
        return;
    }

    let total_count = paths_to_clean.len();

    eprintln!("[INFO] Processing cleanup queue with {} items", total_count);

    let mut successful = 0;
    let mut failed_paths = Vec::new();

    for path in &paths_to_clean {
        if !path.exists() {
            successful += 1;
            continue;
        }

        match fs::remove_dir_all(path) {
            Ok(_) => {
                successful += 1;
                eprintln!("[INFO] Cleaned up: {}", path.display());
            }
            Err(e) => {
                eprintln!("[WARN] Still cannot clean up {}: {}", path.display(), e);
                failed_paths.push(path.clone());
            }
        }
    }

    // 更新清理队列（仅保留失败的）
    if let Ok(mut queue) = cleanup_queue.lock() {
        *queue = failed_paths;
    }

    eprintln!(
        "[INFO] Cleanup queue processed: {}/{} successful",
        successful, total_count
    );
}
