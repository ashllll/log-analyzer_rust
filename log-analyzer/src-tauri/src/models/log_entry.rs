use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::services::MatchDetail;

/// 日志条目
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LogEntry {
    pub id: usize,
    pub timestamp: Arc<str>,
    pub level: Arc<str>,
    pub file: Arc<str>,
    pub real_path: Arc<str>,
    pub line: usize,
    pub content: Arc<str>,
    pub tags: Vec<String>,
    /// 匹配详情（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_details: Option<Vec<MatchDetail>>,
    /// 匹配的关键词列表（可选，用于统计面板）
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub matched_keywords: Option<Vec<String>>,
}

/// 任务进度
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TaskProgress {
    pub task_id: String,
    pub task_type: String, // 任务类型: "Import", "Export", etc.
    pub target: String,    // 目标路径或文件名
    pub status: String,
    pub message: String,
    pub progress: u8,
    pub workspace_id: Option<String>, // 工作区 ID，用于前端匹配
}

/// 文件变化事件
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileChangeEvent {
    pub event_type: String,   // "modified", "created", "deleted"
    pub file_path: String,    // 变化的文件路径
    pub workspace_id: String, // 所属工作区
    pub timestamp: i64,       // 事件发生时间戳
}
