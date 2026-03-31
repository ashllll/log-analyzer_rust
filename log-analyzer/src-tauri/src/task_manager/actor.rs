//! 任务管理器 Actor 实现

use scopeguard::defer;
use serde::Serialize;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;
use tokio::time::interval;
use tracing::{debug, error, info, trace, warn};

use crate::task_manager::types::{TaskInfo, TaskManagerConfig, TaskManagerMetrics, TaskStatus};

/// Actor 消息类型
#[derive(Debug)]
pub enum ActorMessage {
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

/// 任务管理器 Actor
pub struct TaskManagerActor {
    tasks: HashMap<String, TaskInfo>,
    config: TaskManagerConfig,
    app: AppHandle,
}

impl TaskManagerActor {
    pub fn new(app: AppHandle, config: TaskManagerConfig) -> Self {
        Self {
            tasks: HashMap::new(),
            config,
            app,
        }
    }

    fn handle_message(&mut self, msg: ActorMessage) {
        match msg {
            ActorMessage::CreateTask {
                id,
                task_type,
                target,
                workspace_id,
                respond_to,
            } => {
                info!(
                    task_id = %id,
                    task_type = %task_type,
                    target = %target,
                    workspace_id = ?workspace_id,
                    "Creating new task"
                );

                let task = TaskInfo {
                    task_id: id.clone(),
                    task_type,
                    target,
                    progress: 0,
                    message: "Starting...".to_string(),
                    status: TaskStatus::Running,
                    version: 1u64,
                    workspace_id,
                    created_at: Instant::now(),
                    completed_at: None,
                };
                self.tasks.insert(id.clone(), task.clone());

                self.emit_task_event("task-update", &id, &task);

                if respond_to.send(task).is_err() {
                    tracing::debug!("任务管理器：create_task 响应接收方已取消");
                }
            }
            ActorMessage::UpdateTask {
                id,
                progress,
                message,
                status,
                respond_to,
            } => {
                // 高频操作使用 trace 级别，避免 DEBUG 日志性能开销
                trace!(
                    task_id = %id,
                    progress = progress,
                    status = ?status,
                    "Updating task"
                );

                /// 版本号接近 u64::MAX 时重置为阈值+1，防止饱和后幂等性检查失效
                const VERSION_RESET_THRESHOLD: u64 = u64::MAX - 10_000;

                let result = if let Some(task) = self.tasks.get_mut(&id) {
                    task.progress = progress;
                    task.message = message.clone();
                    task.status = status;
                    task.version = if task.version >= VERSION_RESET_THRESHOLD {
                        warn!(
                            task_id = %id,
                            current_version = task.version,
                            reset_to = VERSION_RESET_THRESHOLD + 1,
                            "任务版本号接近饱和，从阈值+1继续递增（不跳回1，保持前端幂等性）"
                        );
                        VERSION_RESET_THRESHOLD + 1
                    } else {
                        task.version.saturating_add(1)
                    };

                    if matches!(
                        status,
                        TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Stopped
                    ) {
                        task.completed_at = Some(Instant::now());
                        info!(task_id = %id, status = ?status, "Task finished");
                    }

                    self.emit_task_event_with_retry("task-update", &id, task);

                    Some(task.clone())
                } else {
                    warn!(task_id = %id, "Task not found for update");
                    None
                };

                if respond_to.send(result).is_err() {
                    tracing::debug!("任务管理器：update_task 响应接收方已取消");
                }
            }
            ActorMessage::GetTask { id, respond_to } => {
                let result = self.tasks.get(&id).cloned();
                if respond_to.send(result).is_err() {
                    tracing::debug!("任务管理器：get_task 响应接收方已取消");
                }
            }
            ActorMessage::GetAllTasks { respond_to } => {
                let tasks: Vec<TaskInfo> = self.tasks.values().cloned().collect();
                if respond_to.send(tasks).is_err() {
                    tracing::debug!("任务管理器：get_all_tasks 响应接收方已取消");
                }
            }
            ActorMessage::RemoveTask { id, respond_to } => {
                trace!(task_id = %id, "Removing task");
                let result = self.tasks.remove(&id);
                if respond_to.send(result).is_err() {
                    tracing::debug!("任务管理器：remove_task 响应接收方已取消");
                }
            }
            ActorMessage::GetMetrics { respond_to } => {
                let metrics = self.collect_metrics();
                // 高频轮询操作使用 trace 级别
                trace!(
                    total = metrics.total_tasks,
                    running = metrics.running_tasks,
                    completed = metrics.completed_tasks,
                    failed = metrics.failed_tasks,
                    "Collected TaskManager metrics"
                );
                if respond_to.send(metrics).is_err() {
                    tracing::debug!("任务管理器：get_metrics 响应接收方已取消");
                }
            }
            ActorMessage::CleanupExpired => {
                self.cleanup_expired_tasks();
            }
            ActorMessage::Shutdown => {
                info!("TaskManager actor shutting down");
            }
        }
    }

    fn collect_metrics(&self) -> TaskManagerMetrics {
        let mut running = 0;
        let mut completed = 0;
        let mut failed = 0;
        let mut stopped = 0;

        for task in self.tasks.values() {
            match task.status {
                TaskStatus::Running => running += 1,
                TaskStatus::Completed => completed += 1,
                TaskStatus::Failed => failed += 1,
                TaskStatus::Stopped => stopped += 1,
            }
        }

        TaskManagerMetrics {
            total_tasks: self.tasks.len(),
            running_tasks: running,
            completed_tasks: completed,
            failed_tasks: failed,
            stopped_tasks: stopped,
            is_healthy: true,
        }
    }

    fn cleanup_expired_tasks(&mut self) {
        let now = Instant::now();
        let mut to_remove = Vec::new();

        for (id, task) in &self.tasks {
            if let Some(completed_at) = task.completed_at {
                let elapsed = now.duration_since(completed_at).as_secs();

                let should_remove = match task.status {
                    TaskStatus::Completed => elapsed >= self.config.completed_task_ttl,
                    TaskStatus::Failed | TaskStatus::Stopped => {
                        elapsed >= self.config.failed_task_ttl
                    }
                    TaskStatus::Running => false,
                };

                if should_remove {
                    to_remove.push(id.clone());
                }
            }
        }

        for id in to_remove {
            if let Some(task) = self.tasks.remove(&id) {
                let elapsed = if let Some(completed_at) = task.completed_at {
                    now.duration_since(completed_at).as_secs()
                } else {
                    0
                };

                // 定期清理任务使用 trace 级别
                trace!(
                    task_id = %id,
                    task_type = %task.task_type,
                    status = ?task.status,
                    elapsed_seconds = elapsed,
                    ttl_seconds = match task.status {
                        TaskStatus::Completed => self.config.completed_task_ttl,
                        TaskStatus::Failed | TaskStatus::Stopped => self.config.failed_task_ttl,
                        TaskStatus::Running => 0,
                    },
                    "Auto-removed expired task"
                );

                if let Err(e) = self.app.emit(
                    "task-removed",
                    serde_json::json!({"task_id": id}),
                ) {
                    error!(
                        task_id = %id,
                        error = %e,
                        "Failed to emit task-removed event"
                    );
                }
            }
        }
    }

    /// 发送任务事件（无重试）
    fn emit_task_event(&self, event: &str, task_id: &str, task: &TaskInfo) {
        let payload = serde_json::json!({
            "task_id": task_id,
            "task_type": task.task_type,
            "target": task.target,
            "progress": task.progress,
            "message": task.message,
            "status": task.status,
            "version": task.version,
            "workspace_id": task.workspace_id,
        });

        match self.app.emit(event, payload) {
            Ok(()) => info!(task_id = %task_id, "Emitted {} event", event),
            Err(e) => error!(
                task_id = %task_id,
                error = %e,
                "Failed to emit {} event",
                event
            ),
        }
    }

    /// 发送任务事件，失败时重试一次（用于终态更新）
    fn emit_task_event_with_retry(&self, event: &str, task_id: &str, task: &TaskInfo) {
        let payload = serde_json::json!({
            "task_id": task_id,
            "task_type": task.task_type,
            "target": task.target,
            "progress": task.progress,
            "message": task.message,
            "status": task.status,
            "version": task.version,
            "workspace_id": task.workspace_id,
        });

        if let Err(e) = self.app.emit(event, payload.clone()) {
            warn!(
                task_id = %task_id,
                error = %e,
                "Failed to emit {} event, retrying once",
                event
            );
            if let Err(e2) = self.app.emit(event, payload) {
                error!(
                    task_id = %task_id,
                    error = %e2,
                    "Failed to emit {} event after retry",
                    event
                );
            } else {
                info!(
                    task_id = %task_id,
                    progress = task.progress,
                    status = ?task.status,
                    "Emitted {} event (retry succeeded)",
                    event
                );
            }
        } else {
            info!(
                task_id = %task_id,
                progress = task.progress,
                status = ?task.status,
                "Emitted {} event",
                event
            );
        }
    }

    pub async fn run(mut self, mut receiver: mpsc::UnboundedReceiver<ActorMessage>) {
        info!("TaskManager actor started");

        defer! {
            info!("TaskManager actor cleanup completed");
        }

        let mut cleanup_interval = interval(Duration::from_secs(self.config.cleanup_interval));

        loop {
            tokio::select! {
                Some(msg) = receiver.recv() => {
                    if matches!(msg, ActorMessage::Shutdown) {
                        self.handle_message(msg);
                        break;
                    }
                    self.handle_message(msg);
                }
                _ = cleanup_interval.tick() => {
                    self.cleanup_expired_tasks();
                }
            }
        }

        info!("Processing remaining messages before shutdown");
        let shutdown_timeout = Duration::from_secs(5);
        let shutdown_deadline = tokio::time::Instant::now() + shutdown_timeout;

        while let Ok(Some(msg)) = tokio::time::timeout(
            shutdown_deadline.saturating_duration_since(tokio::time::Instant::now()),
            receiver.recv(),
        )
        .await
        {
            if matches!(msg, ActorMessage::Shutdown) {
                break;
            }
            self.handle_message(msg);
        }

        let running_tasks: Vec<_> = self
            .tasks
            .values()
            .filter(|t| t.status == TaskStatus::Running)
            .map(|t| t.task_id.clone())
            .collect();

        if !running_tasks.is_empty() {
            warn!(
                running_tasks = ?running_tasks,
                "TaskManager shutting down with running tasks"
            );
        }

        info!("TaskManager actor stopped gracefully");
    }
}
