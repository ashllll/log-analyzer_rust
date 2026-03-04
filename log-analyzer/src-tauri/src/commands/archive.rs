use serde::{Deserialize, Serialize};
use crate::archive::{ArchiveEntry, find_handler};

/**
 * 列出压缩包内容的结果
 */
#[derive(Debug, Serialize, Deserialize)]
pub struct ArchiveContentResult {
    /// 条目列表
    pub entries: Vec<ArchiveEntry>,
    /// 总数量
    pub total_count: usize,
}

/**
 * 读取压缩包内文件的结果
 */
#[derive(Debug, Serialize, Deserialize)]
pub struct ArchiveFileResult {
    /// 文件内容
    pub content: String,
    /// 文件大小（字节）
    pub size: usize,
    /// 是否被截断
    pub truncated: bool,
}

/**
 * 列出压缩包内容
 *
 * # 参数
 * * `archive_path` - 压缩包文件路径
 *
 * # 返回
 * * `Ok(ArchiveContentResult)` - 压缩包内容
 * * `Err(String)` - 错误信息
 */
#[tauri::command]
pub async fn list_archive_contents(archive_path: String) -> Result<ArchiveContentResult, String> {
    let path = std::path::Path::new(&archive_path);

    // 检查文件是否存在
    if !path.exists() {
        return Err(format!("File not found: {}", archive_path));
    }

    let handler = find_handler(path).ok_or_else(|| {
        format!("Unsupported archive format: {:?}", path.extension())
    })?;

    let entries = handler.list_contents(path).await.map_err(|e| e.to_string())?;

    Ok(ArchiveContentResult {
        total_count: entries.len(),
        entries,
    })
}

/**
 * 读取压缩包内单个文件
 *
 * # 参数
 * * `archive_path` - 压缩包文件路径
 * * `file_name` - 要读取的文件名（完整路径）
 *
 * # 返回
 * * `Ok(ArchiveFileResult)` - 文件内容
 * * `Err(String)` - 错误信息
 */
#[tauri::command]
pub async fn read_archive_file(archive_path: String, file_name: String) -> Result<ArchiveFileResult, String> {
    let path = std::path::Path::new(&archive_path);

    // 检查文件是否存在
    if !path.exists() {
        return Err(format!("File not found: {}", archive_path));
    }

    let handler = find_handler(path).ok_or_else(|| {
        format!("Unsupported archive format: {:?}", path.extension())
    })?;

    let content = handler.read_file(path, &file_name).await.map_err(|e| e.to_string())?;
    let truncated = content.contains("已截断显示");
    let size = content.len();

    Ok(ArchiveFileResult {
        content,
        size,
        truncated,
    })
}
