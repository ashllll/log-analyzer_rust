//! 任务生命周期管理器
//!
//! 基于 Actor Model 和消息传递的任务管理系统
//!
//! ## 设计模式
//!
//! 1. **Actor Model**: 每个任务管理器是一个独立的 Actor
//! 2. **Message Passing**: 通过消息通道进行通信
//! 3. **Supervision**: 自动监控和清理任务
//! 4. **Event Sourcing**: 所有状态变更通过事件记录
//!
//! ## 参考实现
//!
//! - Actix (Rust Actor Framework)
//! - Tokio Actors Pattern
//! - Erlang/OTP Supervision Trees

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;
use tokio::time::interval;

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

/// 任务信息
#[derive(Debug, Clone, Serialize)]
pub struct TaskInfo {
    pub id: String,
    #[serde(rename = "type")]
    pub task_type: String,
    pub target: String,
    pub progress: u8,
    pub message: String,
    pub status: TaskStatus,
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
}

impl Default for TaskManagerConfig {
    fn default() -> Self {
        Self {
            completed_task_ttl: 3, // 完成任务保留 3 秒
            failed_task_ttl: 10,   // 失败任务保留 10 秒
            cleanup_interval: 1,   // 每秒检查一次
        }
    }
}

/// Actor 消息类型
#[derive(Debug)]
enum ActorMessage {
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
    /// 清理过期任务
    CleanupExpired,
    /// 停止 Actor
    Shutdown,
}

/// 任务管理器 Actor
struct TaskManagerActor {
    tasks: HashMap<String, TaskInfo>,
    config: TaskManagerConfig,
    app: AppHandle,
}

impl TaskManagerActor {
    fn new(app: AppHandle, config: TaskManagerConfig) -> Self {
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
                let task = TaskInfo {
                    id: id.clone(),
                    task_type,
                    target,
                    progress: 0,
                    message: "Starting...".to_string(),
                    status: TaskStatus::Running,
                    workspace_id,
                    created_at: Instant::now(),
                    completed_at: None,
                };
                self.tasks.insert(id, task.clone());
                let _ = respond_to.send(task);
            }
            ActorMessage::UpdateTask {
                id,
                progress,
                message,
                status,
                respond_to,
            } => {
                let result = if let Some(task) = self.tasks.get_mut(&id) {
                    task.progress = progress;
                    task.message = message;
                    task.status = status;

                    // 如果任务完成或失败，记录完成时间
                    if matches!(
                        status,
                        TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Stopped
                    ) {
                        task.completed_at = Some(Instant::now());
                    }

                    Some(task.clone())
                } else {
                    None
                };
                let _ = respond_to.send(result);
            }
            ActorMessage::GetTask { id, respond_to } => {
                let result = self.tasks.get(&id).cloned();
                let _ = respond_to.send(result);
            }
            ActorMessage::GetAllTasks { respond_to } => {
                let tasks: Vec<TaskInfo> = self.tasks.values().cloned().collect();
                let _ = respond_to.send(tasks);
            }
            ActorMessage::RemoveTask { id, respond_to } => {
                let result = self.tasks.remove(&id);
                let _ = respond_to.send(result);
            }
            ActorMessage::CleanupExpired => {
                self.cleanup_expired_tasks();
            }
            ActorMessage::Shutdown => {
                // Actor 将在消息循环结束后停止
            }
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
                eprintln!(
                    "[TaskManager] Auto-removed expired task: {} (status: {:?})",
                    id, task.status
                );

                // 发送任务移除事件
                let _ = self.app.emit(
                    "task-removed",
                    serde_json::json!({
                        "task_id": id,
                    }),
                );
            }
        }
    }

    async fn run(mut self, mut receiver: mpsc::UnboundedReceiver<ActorMessage>) {
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

        eprintln!("[TaskManager] Actor stopped");
    }
}

/// 任务管理器句柄（客户端）
pub struct TaskManager {
    sender: mpsc::UnboundedSender<ActorMessage>,
}

impl TaskManager {
    /// 创建新的任务管理器
    pub fn new(app: AppHandle, config: TaskManagerConfig) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();

        let actor = TaskManagerActor::new(app, config);

        // 启动 Actor
        tokio::spawn(async move {
            actor.run(receiver).await;
        });

        Self { sender }
    }

    /// 创建新任务
    pub fn create_task(
        &self,
        id: String,
        task_type: String,
        target: String,
        workspace_id: Option<String>,
    ) -> TaskInfo {
        let (tx, rx) = tokio::sync::oneshot::channel();

        let msg = ActorMessage::CreateTask {
            id,
            task_type,
            target,
            workspace_id,
            respond_to: tx,
        };

        if self.sender.send(msg).is_err() {
            panic!("TaskManager actor has stopped");
        }

        // 阻塞等待响应（在同步上下文中）
        tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(rx))
            .expect("Actor dropped response channel")
    }

    /// 更新任务进度
    pub fn update_task(
        &self,
        id: &str,
        progress: u8,
        message: String,
        status: TaskStatus,
    ) -> Option<TaskInfo> {
        let (tx, rx) = tokio::sync::oneshot::channel();

        let msg = ActorMessage::UpdateTask {
            id: id.to_string(),
            progress,
            message,
            status,
            respond_to: tx,
        };

        if self.sender.send(msg).is_err() {
            return None;
        }

        tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(rx))
            .ok()
            .flatten()
    }

    /// 获取任务信息
    pub fn get_task(&self, id: &str) -> Option<TaskInfo> {
        let (tx, rx) = tokio::sync::oneshot::channel();

        let msg = ActorMessage::GetTask {
            id: id.to_string(),
            respond_to: tx,
        };

        if self.sender.send(msg).is_err() {
            return None;
        }

        tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(rx))
            .ok()
            .flatten()
    }

    /// 获取所有任务
    pub fn get_all_tasks(&self) -> Vec<TaskInfo> {
        let (tx, rx) = tokio::sync::oneshot::channel();

        let msg = ActorMessage::GetAllTasks { respond_to: tx };

        if self.sender.send(msg).is_err() {
            return Vec::new();
        }

        tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(rx))
            .unwrap_or_default()
    }

    /// 删除任务
    pub fn remove_task(&self, id: &str) -> Option<TaskInfo> {
        let (tx, rx) = tokio::sync::oneshot::channel();

        let msg = ActorMessage::RemoveTask {
            id: id.to_string(),
            respond_to: tx,
        };

        if self.sender.send(msg).is_err() {
            return None;
        }

        tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(rx))
            .ok()
            .flatten()
    }

    /// 停止任务管理器
    pub fn shutdown(&self) {
        let _ = self.sender.send(ActorMessage::Shutdown);
    }
}

impl Drop for TaskManager {
    fn drop(&mut self) {
        self.shutdown();
    }
}
