//! 临时文件清理工具
//!
//! 提供临时目录的清理功能，支持重试和清理队列机制。

use crossbeam::queue::SegQueue;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{error, info, warn};

use super::retry::retry_file_operation;

#[cfg(target_os = "windows")]
use super::path::remove_readonly;

#[cfg(target_os = "windows")]
use walkdir::WalkDir;

/// 清理任务条目，包含重试计数
#[derive(Debug)]
pub struct CleanupItem {
    pub path: PathBuf,
    pub retry_count: u32,
}

/// 共享清理队列
pub type CleanupQueue = SegQueue<CleanupItem>;

/// 尝试清理临时目录
///
/// 尝试删除指定的临时目录,如果失败则将路径添加到清理队列供后续处理。
///
/// # 参数
///
/// - `path` - 要清理的路径
/// - `cleanup_queue` - 清理队列的Arc引用
pub fn try_cleanup_temp_dir(path: &Path, cleanup_queue: &Arc<CleanupQueue>) {
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
            // 添加到无锁清理队列，重试计数从 0 开始
            cleanup_queue.push(CleanupItem {
                path: path.to_path_buf(),
                retry_count: 0,
            });
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
pub fn process_cleanup_queue(cleanup_queue: &Arc<CleanupQueue>) {
    // 限制最大重试次数，防止无限循环
    const MAX_QUEUE_RETRIES: u32 = 5;

    // 收集所有待清理的项
    let mut items_to_clean = Vec::new();
    while let Some(item) = cleanup_queue.pop() {
        items_to_clean.push(item);
    }

    if items_to_clean.is_empty() {
        return;
    }

    let total_count = items_to_clean.len();
    info!(count = total_count, "Processing cleanup queue");

    let mut successful = 0;
    let mut dropped = 0;

    for mut item in items_to_clean {
        if !item.path.exists() {
            successful += 1;
            continue;
        }

        match fs::remove_dir_all(&item.path) {
            Ok(_) => {
                successful += 1;
                info!(path = %item.path.display(), "Cleaned up directory from queue");
            }
            Err(e) => {
                // 检查是否为可重试错误（例如文件被占用）
                let err_msg = e.to_string();
                let is_retryable = err_msg.contains("Access is denied")
                    || err_msg.contains("being used")
                    || err_msg.contains("cannot access");

                if is_retryable && item.retry_count < MAX_QUEUE_RETRIES {
                    item.retry_count += 1;
                    warn!(
                        error = %e,
                        path = %item.path.display(),
                        retry = item.retry_count,
                        "Still cannot clean up directory, re-queueing"
                    );
                    cleanup_queue.push(item);
                } else {
                    dropped += 1;
                    error!(
                        error = %e,
                        path = %item.path.display(),
                        "Abandoned cleanup of directory after {} attempts or non-retryable error",
                        item.retry_count + 1
                    );
                }
            }
        }
    }

    info!(
        successful = successful,
        dropped = dropped,
        total = total_count,
        "Cleanup queue processed"
    );
}
