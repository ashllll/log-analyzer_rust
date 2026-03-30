//! TaskManager 核心类型定义

use serde::{Deserialize, Serialize};
use std::time::Instant;
use thiserror::Error;

/// 任务管理器错误类型
#[derive(Error, Debug)]
pub enum TaskManagerError {
    /// Actor 通道已关闭
    #[error("TaskManager actor has stopped")]
    ActorStopped,

    /// 操作超时
    #[error("Operation timed out")]
    OperationTimeout,

    /// Actor 响应通道被丢弃
    #[error("Actor dropped response channel")]
    ActorDroppedResponse,

    /// 发送关闭消息失败
    #[error("Failed to send shutdown message: {0}")]
    ShutdownFailed(String),

    /// 通道已满，背压触发
    #[error("TaskManager channel is full")]
    ChannelFull,
}

/// 任务状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum TaskStatus {
    /// 运行中
    Running,
    /// 已完成
    Completed,
    /// 失败
    Failed,
    /// 已停止
    Stopped,
}

/// 任务管理器指标
#[derive(Debug, Clone, Serialize)]
pub struct TaskManagerMetrics {
    /// 总任务数
    pub total_tasks: usize,
    /// 运行中的任务数
    pub running_tasks: usize,
    /// 已完成的任务数
    pub completed_tasks: usize,
    /// 失败的任务数
    pub failed_tasks: usize,
    /// 已停止的任务数
    pub stopped_tasks: usize,
    /// Actor 是否健康
    pub is_healthy: bool,
}

/// 任务信息
#[derive(Debug, Clone, Serialize)]
pub struct TaskInfo {
    pub task_id: String,
    pub task_type: String,
    pub target: String,
    pub progress: u8,
    pub message: String,
    pub status: TaskStatus,
    /// 版本号 - 使用 u64 防止长时间运行时溢出
    /// 单调递增，用于前端幂等性检查
    pub version: u64,
    pub workspace_id: Option<String>,
    #[serde(skip)]
    pub created_at: Instant,
    #[serde(skip)]
    pub completed_at: Option<Instant>,
}

/// 任务管理器配置
#[derive(Debug, Clone)]
pub struct TaskManagerConfig {
    /// 完成任务的保留时间（秒）
    pub completed_task_ttl: u64,
    /// 失败任务的保留时间（秒）
    pub failed_task_ttl: u64,
    /// 清理检查间隔（秒）
    pub cleanup_interval: u64,
    /// 操作超时时间（秒）
    pub operation_timeout: u64,
    /// Actor 消息通道容量（0 表示无限制，与旧行为兼容）
    pub channel_capacity: usize,
}

impl Default for TaskManagerConfig {
    fn default() -> Self {
        Self {
            completed_task_ttl: 300, // 5 分钟
            failed_task_ttl: 1800,   // 30 分钟
            cleanup_interval: 60,    // 1 分钟
            operation_timeout: 30,   // 30 秒
            channel_capacity: 1024,  // 默认 bounded channel
        }
    }
}

impl TaskManagerConfig {
    /// 从 AppConfig 创建配置
    pub fn from_app_config(config: &crate::models::config::TaskManagerConfig) -> Self {
        Self {
            completed_task_ttl: config.completed_task_ttl,
            failed_task_ttl: config.failed_task_ttl,
            cleanup_interval: config.cleanup_interval,
            operation_timeout: config.operation_timeout,
            channel_capacity: 1024,
        }
    }
}

/// 批量更新项
#[derive(Debug, Clone)]
pub struct TaskUpdateItem {
    pub id: String,
    pub progress: u8,
    pub message: String,
    pub status: TaskStatus,
}
