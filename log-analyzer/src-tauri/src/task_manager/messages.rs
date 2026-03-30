//! Actor 消息类型

use crate::task_manager::types::{TaskManagerMetrics, TaskStatus, TaskInfo, TaskUpdateItem};

#[derive(Debug)]
pub(crate) enum ActorMessage {
    /// 创建新任务
    CreateTask {
        id: String,
        task_type: String,
        target: String,
        workspace_id: Option<String>,
        respond_to: tokio::sync::oneshot::Sender<TaskInfo>,
    },
    /// 更新任务
    UpdateTask {
        id: String,
        progress: u8,
        message: String,
        status: TaskStatus,
        respond_to: tokio::sync::oneshot::Sender<Option<TaskInfo>>,
    },
    /// 批量更新任务
    UpdateTasksBatch {
        updates: Vec<TaskUpdateItem>,
        respond_to: tokio::sync::oneshot::Sender<usize>,
    },
    /// 获取任务
    GetTask {
        id: String,
        respond_to: tokio::sync::oneshot::Sender<Option<TaskInfo>>,
    },
    /// 获取所有任务
    GetAllTasks {
        respond_to: tokio::sync::oneshot::Sender<Vec<TaskInfo>>,
    },
    /// 删除任务
    RemoveTask {
        id: String,
        respond_to: tokio::sync::oneshot::Sender<Option<TaskInfo>>,
    },
    /// 获取指标
    GetMetrics {
        respond_to: tokio::sync::oneshot::Sender<TaskManagerMetrics>,
    },
    /// 清理过期任务
    CleanupExpired,
    /// 停止 Actor
    Shutdown,
}
