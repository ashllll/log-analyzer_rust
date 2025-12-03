//! 配置相关数据结构
//!
//! 本模块定义了应用配置、索引数据和文件元数据等核心配置结构。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 应用配置
///
/// 存储关键词组和工作区等全局配置信息。
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppConfig {
    /// 关键词分组配置
    pub keyword_groups: serde_json::Value,
    /// 工作区配置
    pub workspaces: serde_json::Value,
}

/// 索引持久化数据结构
///
/// 支持索引的保存和加载，包含增量更新所需的元数据。
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IndexData {
    /// 路径映射：real_path -> virtual_path
    pub path_map: HashMap<String, String>,
    /// 文件元数据映射（用于增量更新）
    pub file_metadata: HashMap<String, FileMetadata>,
    /// 所属工作区 ID
    pub workspace_id: String,
    /// 创建时间戳（Unix 时间戳）
    pub created_at: i64,
}

/// 文件元数据
///
/// 用于增量更新判断，记录文件的修改时间和大小。
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileMetadata {
    /// 修改时间戳（Unix 时间戳）
    pub modified_time: i64,
    /// 文件大小（字节）
    pub size: u64,
}
