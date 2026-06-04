//! 文件监听服务
//!
//! 提供实时文件监听和增量读取功能，支持：
//! - 从指定偏移量读取文件新增内容
//! - 日志行解析（委托给 la_core::utils）
//! - 增量索引更新
//! - 实时事件推送到前端

use la_core::error::{AppError, Result};
use la_core::models::log_entry::LogEntry;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

/// 文件监听器状态
#[derive(Debug, Clone)]
pub struct WatcherState {
    pub workspace_id: String,
    pub watched_path: std::path::PathBuf,
    pub file_offsets: HashMap<String, u64>,
    /// 每个文件已处理的行数（用于精确计算新内容的起始行号）
    pub line_counts: HashMap<String, usize>,
    pub is_active: bool,
    /// 监听线程的 JoinHandle，用于确保正确退出并清理资源
    /// 使用 parking_lot::Mutex 避免 poison 问题（B-M3）
    pub thread_handle: Arc<parking_lot::Mutex<Option<std::thread::JoinHandle<()>>>>,
    /// 底层文件监听器，存放在这里确保其生命周期与状态同步
    /// 使用 parking_lot::Mutex 避免 poison 问题（B-M3）
    pub watcher: Arc<parking_lot::Mutex<Option<notify::RecommendedWatcher>>>,
}
use tracing::{debug, warn};

// TimestampParser / parse_metadata / parse_log_lines 已提取至 la_core::utils
// 保留 re-export 以保持向后兼容
pub use la_core::utils::{parse_log_lines, parse_metadata, TimestampParser};

/// 从指定偏移量读取文件新增内容（支持增量读取）
///
/// # Arguments
///
/// * `path` - 文件路径
/// * `offset` - 上次读取的偏移量（字节）
///
/// # Returns
///
/// 返回元组：(新增的行, 新的偏移量)
///
/// # Errors
///
/// - 文件打开失败
/// - 文件元数据读取失败
/// - 偏移量定位失败
///
/// # 特性
///
/// - **截断检测**：如果文件被截断（大小小于上次偏移量），自动从头读取
/// - **增量读取**：只读取新增内容，避免重复处理
/// - **错误容忍**：单行读取错误不中断整体流程
/// - **大文件优化**：使用大缓冲区提高读取效率
/// - **资源安全**：使用作用域确保文件句柄正确关闭
pub fn read_file_from_offset(path: &Path, offset: u64) -> Result<(Vec<String>, u64)> {
    use std::io::{Seek, SeekFrom};

    // 使用作用域确保文件句柄自动关闭，防止资源泄漏
    let (lines, file_size, start_offset) = {
        let mut file = File::open(path).map_err(AppError::Io)?;

        // 获取当前文件大小
        let file_size = file.metadata().map_err(AppError::Io)?.len();

        // If file was truncated (smaller than last offset), read from beginning
        let start_offset = if file_size < offset {
            warn!(
                file = %path.display(),
                "File truncated, reading from beginning"
            );
            0
        } else {
            offset
        };

        // 如果没有新内容，直接返回
        if start_offset >= file_size {
            return Ok((Vec::new(), file_size));
        }

        // 移动到偏移量位置
        file.seek(SeekFrom::Start(start_offset))
            .map_err(AppError::Io)?;

        // Read new content - use large buffer (64KB) for better performance with large files
        let reader = BufReader::with_capacity(65536, file);
        let mut lines = Vec::new();

        for line_result in reader.lines() {
            match line_result {
                Ok(line) => lines.push(line),
                Err(e) => {
                    warn!(
                        error = %e,
                        "Error reading line, continuing with next line"
                    );
                    // Continue reading instead of breaking to avoid losing subsequent valid lines
                    continue;
                }
            }
        }

        (lines, file_size, start_offset)
    }; // File handle automatically closed here, preventing resource leaks

    debug!(
        lines_read = lines.len(),
        file = %path.display(),
        offset_start = start_offset,
        offset_end = file_size,
        "Read new lines from file"
    );

    Ok((lines, file_size))
}

/// 将新日志条目添加到工作区索引（增量更新）
///
/// # Arguments
///
/// * `workspace_id` - 工作区 ID
/// * `new_entries` - 新的日志条目列表
/// * `app` - Tauri 应用句柄
/// * `state` - 应用状态，用于访问 SearchEngineManager 进行索引持久化
///
/// # Returns
///
/// - `Ok(())`: 成功
/// - `Err(String)`: 错误信息
///
/// # 行为
///
/// - 通过 Tauri 事件系统发送新日志到前端（事件名：`new-logs`）
/// - 持久化新日志条目到 Tantivy 索引（通过 SearchEngineManager）
/// - 提交索引变更（commit）
/// - P1-2: 同时写入 CAS 和更新 MetadataStore，修复数据一致性鸿沟
pub fn append_to_workspace_index(
    workspace_id: &str,
    new_entries: &[LogEntry],
    app: &AppHandle,
    state: &crate::models::state::AppState,
    // FIX(HI-04): 传入 Tokio runtime handle，避免在同步线程中 block_on
    runtime_handle: &tokio::runtime::Handle,
) -> Result<()> {
    if new_entries.is_empty() {
        return Ok(());
    }

    debug!(
        entries_count = new_entries.len(),
        workspace_id = %workspace_id,
        "Appending new entries to workspace"
    );

    // Send new logs to frontend (real-time update)
    if let Err(e) = app.emit("new-logs", new_entries) {
        warn!(
            error = %e,
            "Failed to emit new-logs event to frontend"
        );
    }

    // 从 WorkspaceService 获取依赖（P3 迁移：替代直接操作 AppState HashMap）
    let service = match state.get_workspace_service(workspace_id) {
        Some(svc) => svc,
        None => {
            tracing::warn!(
                workspace_id = %workspace_id,
                "Workspace service not found, skipping index append"
            );
            return Ok(());
        }
    };

    // 持久化到 Tantivy 索引
    let search_manager = service.search_engine();
    if let Err(e) = search_manager.add_documents(new_entries) {
        tracing::warn!(
            error = %e,
            count = new_entries.len(),
            workspace_id = %workspace_id,
            "Failed to add documents to index in batch"
        );
    }

    if let Err(e) = search_manager.commit() {
        tracing::warn!(
            error = %e,
            workspace_id = %workspace_id,
            "Failed to commit index changes"
        );
    } else {
        debug!(
            workspace_id = %workspace_id,
            count = new_entries.len(),
            "Successfully persisted new entries to Tantivy index"
        );
    }

    // P1-2: 将监听捕获的增量内容同步写入 CAS 和 MetadataStore
    {
        let cas = service.cas();
        let metadata_store = service.metadata_store();
        // 收集唯一源文件路径，避免重复写入
        let mut seen_sources = HashSet::new();
        for entry in new_entries {
            if seen_sources.insert(entry.real_path.as_ref()) {
                let source_path = std::path::Path::new(entry.real_path.as_ref());
                if source_path.is_file() {
                    // 读取完整文件内容
                    match std::fs::read(source_path) {
                        Ok(content) => {
                            // FIX(HI-04): 避免在同步线程中 block_on，
                            // 改用 spawn 将 CAS 写入和 metadata 更新发送到 Tokio 异步任务处理
                            let cas = Arc::clone(cas);
                            let metadata_store = Arc::clone(metadata_store);
                            let source_path = source_path.to_path_buf();
                            let real_path = entry.real_path.to_string();
                            let virtual_path = entry.file.to_string();
                            runtime_handle.spawn(async move {
                                match cas.store_content(&content).await {
                                    Ok(hash) => {
                                        let file_size = content.len() as i64;
                                        let file_name = source_path
                                            .file_name()
                                            .map(|n| n.to_string_lossy().to_string())
                                            .unwrap_or_else(|| real_path.clone());
                                        let metadata = la_storage::FileMetadata {
                                            id: 0,
                                            sha256_hash: hash,
                                            virtual_path,
                                            original_name: file_name,
                                            size: file_size,
                                            modified_time: 0,
                                            mime_type: None,
                                            parent_archive_id: None,
                                            depth_level: 0,
                                            min_timestamp: None,
                                            max_timestamp: None,
                                            level_mask: None,
                                            analysis_status:
                                                la_core::storage_types::AnalysisStatus::Pending,
                                        };
                                        if let Err(e) =
                                            metadata_store.insert_file(&metadata).await
                                        {
                                            tracing::warn!(
                                                virtual_path = %metadata.virtual_path,
                                                error = %e,
                                                "Failed to insert watcher file metadata"
                                            );
                                        }
                                    }
                                    Err(e) => {
                                        tracing::warn!(
                                            source = %real_path,
                                            error = %e,
                                            "Failed to store watcher file content in CAS"
                                        );
                                    }
                                }
                            });
                        }
                        Err(e) => {
                            tracing::warn!(
                                source = %entry.real_path,
                                error = %e,
                                "Failed to read watcher file for CAS storage"
                            );
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// 获取文件元数据（用于增量判断）
///
/// # 功能
///
/// 提取文件的修改时间和大小，用于：
/// - 增量索引更新（判断文件是否变化）
/// - 索引持久化（保存元数据到磁盘）
///
/// # Returns
///
/// - `Ok(FileMetadata)`: 包含 `modified_time` (Unix 时间戳) 和 `size` (字节)
/// - `Err(String)`: 读取失败的错误信息
///
/// # 例子
///
/// ```ignore
/// // 此例子仅用于说明，不会执行
/// use std::path::Path;
/// let metadata = get_file_metadata(Path::new("file.txt"))?;
/// println!("Modified: {}, Size: {}", metadata.modified_time, metadata.size);
/// ```
///
/// # 使用场景
///
/// - ✅ 已集成: `process_path_recursive_inner_with_metadata` 中收集普通文件元数据
pub fn get_file_metadata(path: &Path) -> Result<la_storage::FileMetadata> {
    use std::time::SystemTime;

    let metadata = path.metadata().map_err(AppError::Io)?;

    let modified = metadata.modified().map_err(AppError::Io)?;

    let modified_time: i64 = modified
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|e| AppError::validation_error(format!("Invalid timestamp: {}", e)))?
        .as_secs()
        .try_into()
        .map_err(|_| AppError::validation_error("Timestamp overflow (Y2K38)".to_string()))?;

    Ok(la_storage::FileMetadata {
        id: 0,                       // Will be auto-generated
        sha256_hash: String::new(),  // Will be filled by caller
        virtual_path: String::new(), // Will be filled by caller
        original_name: path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string(),
        size: metadata.len() as i64,
        modified_time,
        mime_type: None,
        parent_archive_id: None,
        depth_level: 0,
        min_timestamp: None,
        max_timestamp: None,
        level_mask: None,
        analysis_status: la_core::storage_types::AnalysisStatus::Pending,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp_parser_iso8601() {
        let line = "2024-01-15T10:30:45.123 [INFO] Application started";
        let timestamp = TimestampParser::parse_timestamp(line);
        assert!(timestamp.is_some());
        assert_eq!(timestamp.unwrap(), "2024-01-15T10:30:45.123");
    }

    #[test]
    fn test_timestamp_parser_common() {
        let line = "2024-01-15 10:30:45 [ERROR] Database connection failed";
        let timestamp = TimestampParser::parse_timestamp(line);
        assert!(timestamp.is_some());
        assert_eq!(timestamp.unwrap(), "2024-01-15 10:30:45");
    }

    #[test]
    fn test_timestamp_parser_us() {
        let line = "01/15/2024 10:30:45.456 [WARN] Low memory warning";
        let timestamp = TimestampParser::parse_timestamp(line);
        assert!(timestamp.is_some());
        assert_eq!(timestamp.unwrap(), "01/15/2024 10:30:45.456");
    }

    #[test]
    fn test_timestamp_parser_no_match() {
        let line = "This is a log line without timestamp";
        let timestamp = TimestampParser::parse_timestamp(line);
        assert!(timestamp.is_none());
    }

    #[test]
    fn test_parse_naive_datetime_supports_datetime_local() {
        let timestamp = TimestampParser::parse_naive_datetime("2024-01-15T10:30");
        assert!(timestamp.is_some());
        assert_eq!(
            timestamp.unwrap().format("%Y-%m-%d %H:%M:%S").to_string(),
            "2024-01-15 10:30:00"
        );
    }
}
