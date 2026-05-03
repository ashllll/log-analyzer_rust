//! la-archive 内部辅助模块
//!
//! 包含从主 crate 复制的纯逻辑模块，避免循环依赖。

pub mod file_type_filter;
pub mod metadata_db;

use la_core::error::{AppError, Result};
use la_core::storage_types::FileMetadata;
use std::path::Path;

/// 从文件路径提取元数据（纯函数，无外部依赖）
///
/// 从 file_watcher::get_file_metadata 复制，用于 archive 模块内部。
pub fn get_file_metadata(path: &Path) -> Result<FileMetadata> {
    use std::time::SystemTime;

    let metadata = path.metadata().map_err(AppError::Io)?;

    let modified = metadata.modified().map_err(AppError::Io)?;

    let modified_time: i64 = modified
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|e| AppError::validation_error(format!("Invalid timestamp: {}", e)))?
        .as_secs()
        .try_into()
        .map_err(|_| AppError::validation_error("Timestamp overflow (Y2K38)".to_string()))?;

    Ok(FileMetadata {
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
    })
}
