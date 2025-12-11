//! 索引存储服务
//!
//! 提供工作区索引的持久化功能，支持：
//! - Bincode 序列化
//! - Gzip 压缩存储
//! - 增量更新
//! - 跨平台兼容（Windows UNC 路径支持）

use crate::error::{AppError, Result};
use crate::models::config::{FileMetadata, IndexData};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

/// 索引加载结果类型
///
/// 返回元组：(路径映射表, 文件元数据映射表)
pub type IndexResult = Result<(HashMap<String, String>, HashMap<String, FileMetadata>)>;

/// 保存索引到磁盘（带压缩，Windows 兼容，支持增量更新）
///
/// # Arguments
///
/// * `app` - Tauri 应用句柄
/// * `workspace_id` - 工作区唯一标识符
/// * `path_map` - 虚拟路径到真实路径的映射表
/// * `file_metadata` - 文件元数据映射表
///
/// # Returns
///
/// 返回索引文件的完整路径
///
/// # Errors
///
/// - 应用数据目录获取失败
/// - 索引目录创建失败
/// - 序列化失败
/// - 文件写入失败
/// - 压缩失败
pub fn save_index(
    app: &AppHandle,
    workspace_id: &str,
    path_map: &HashMap<String, String>,
    file_metadata: &HashMap<String, FileMetadata>,
) -> Result<PathBuf> {
    let index_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::validation_error(format!("Failed to get app data dir: {}", e)))?
        .join("indices");
    fs::create_dir_all(&index_dir).map_err(AppError::Io)?;

    let index_path = index_dir.join(format!("{}.idx.gz", workspace_id)); // 压缩格式
    let index_data = IndexData {
        path_map: path_map.clone(),
        file_metadata: file_metadata.clone(),
        workspace_id: workspace_id.to_string(),
        created_at: chrono::Utc::now().timestamp(),
    };

    let encoded = bincode::serialize(&index_data)
        .map_err(|e| AppError::validation_error(format!("Serialization error: {}", e)))?;
    let file = File::create(&index_path).map_err(AppError::Io)?;

    // Gzip 压缩
    let mut encoder = GzEncoder::new(file, Compression::default());
    encoder.write_all(&encoded).map_err(AppError::Io)?;
    encoder.finish().map_err(AppError::Io)?;

    eprintln!(
        "[DEBUG] Index saved (compressed): {} ({} entries)",
        index_path.display(),
        path_map.len()
    );
    Ok(index_path)
}

/// 从磁盘加载索引（带解压，Windows 兼容，返回元数据）
///
/// # Arguments
///
/// * `index_path` - 索引文件路径
///
/// # Returns
///
/// 返回索引结果：(路径映射表, 文件元数据映射表)
///
/// # Errors
///
/// - 索引文件不存在
/// - 文件打开失败
/// - 解压失败
/// - 反序列化失败
pub fn load_index(index_path: &Path) -> IndexResult {
    if !index_path.exists() {
        return Err(AppError::not_found("Index file not found"));
    }

    let file = File::open(index_path).map_err(AppError::Io)?;

    // 检查是否为压缩格式
    let mut data = Vec::new();
    if index_path.extension().and_then(|s| s.to_str()) == Some("gz") {
        // 解压
        let mut decoder = GzDecoder::new(file);
        decoder
            .read_to_end(&mut data)
            .map_err(AppError::Io)?;
    } else {
        // 未压缩（兼容旧版本）
        let mut reader = BufReader::new(file);
        reader.read_to_end(&mut data).map_err(AppError::Io)?;
    }

    let index_data: IndexData = bincode::deserialize(&data)
        .map_err(|e| AppError::validation_error(format!("Deserialization error: {}", e)))?;

    eprintln!(
        "[DEBUG] Index loaded: {} ({} entries)",
        index_path.display(),
        index_data.path_map.len()
    );
    Ok((index_data.path_map, index_data.file_metadata))
}
