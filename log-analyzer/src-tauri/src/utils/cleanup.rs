//! 临时文件清理工具
//!
//! 提供临时目录的清理功能，支持重试和清理队列机制。

use crossbeam::queue::SegQueue;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{info, warn};

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
/// - `cleanup_queue` - 清理队列的Arc引用（无锁队列）
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
pub fn try_cleanup_temp_dir(path: &Path, cleanup_queue: &Arc<SegQueue<PathBuf>>) {
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
        3,    // 最多重试3次
        100,  // 基础延迟 100ms
        5000, // 最大延迟 5s
        &format!("cleanup_temp_dir({})", path.display()),
    );

    match result {
        Ok(_) => {
            info!(path = %path.display(), "Successfully cleaned up temp directory");
        }
        Err(e) => {
            warn!(
                error = %e,
                path = %path.display(),
                "Failed to clean up temp directory, adding to cleanup queue"
            );
            // 添加到无锁清理队列
            cleanup_queue.push(path.to_path_buf());
        }
    }
}

/// 执行清理队列中的任务
///
/// 尝试清理队列中所有待处理的临时目录。
///
/// # 参数
///
/// - `cleanup_queue` - 清理队列的Arc引用（无锁队列）
///
/// # 功能
///
/// 1. 从队列中弹出所有路径
/// 2. 逐个尝试删除
/// 3. 统计成功和失败的数量
/// 4. 将失败的路径重新加入队列
pub fn process_cleanup_queue(cleanup_queue: &Arc<SegQueue<PathBuf>>) {
    // 收集所有待清理的路径
    let mut paths_to_clean = Vec::new();
    while let Some(path) = cleanup_queue.pop() {
        paths_to_clean.push(path);
    }

    if paths_to_clean.is_empty() {
        return;
    }

    let total_count = paths_to_clean.len();

    info!(count = total_count, "Processing cleanup queue");

    let mut successful = 0;

    for path in &paths_to_clean {
        if !path.exists() {
            successful += 1;
            continue;
        }

        match fs::remove_dir_all(path) {
            Ok(_) => {
                successful += 1;
                info!(path = %path.display(), "Cleaned up directory");
            }
            Err(e) => {
                warn!(
                    error = %e,
                    path = %path.display(),
                    "Still cannot clean up directory, re-queueing"
                );
                // 重新加入队列供下次尝试
                cleanup_queue.push(path.clone());
            }
        }
    }

    info!(
        successful = successful,
        total = total_count,
        "Cleanup queue processed"
    );
}
