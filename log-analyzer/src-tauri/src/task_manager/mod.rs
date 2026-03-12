//! 任务生命周期管理器（FFI 版本）
#![allow(dead_code)]
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

use eyre::{Result, WrapErr};
use scopeguard::defer;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::time::{interval, timeout};
use tracing::{debug, error, info, warn};

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
}

impl Default for TaskManagerConfig {
    fn default() -> Self {
        Self {
            completed_task_ttl: 300, // 5 分钟
            failed_task_ttl: 1800,   // 30 分钟
            cleanup_interval: 60,    // 1 分钟
            operation_timeout: 30,   // 30 秒
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
struct TaskManagerActor {
    tasks: HashMap<String, TaskInfo>,
    config: TaskManagerConfig,
}

impl TaskManagerActor {
    fn new(config: TaskManagerConfig) -> Self {
        Self {
            tasks: HashMap::new(),
            config,
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

                // FFI 模式下通过事件总线发送事件
                let _ = crate::events::emit_event(crate::events::AppEvent::TaskUpdate {
                    progress: crate::models::TaskProgress {
                        task_id: id.clone(),
                        task_type: task.task_type.clone(),
                        target: task.target.clone(),
                        status: "Running".to_string(),
                        message: "Starting...".to_string(),
                        progress: 0,
                        workspace_id: task.workspace_id.clone(),
                    },
                });

                let _ = respond_to.send(task);
            }
            ActorMessage::UpdateTask {
                id,
                progress,
                message,
                status,
                respond_to,
            } => {
                debug!(
                    task_id = %id,
                    progress = progress,
                    status = ?status,
                    "Updating task"
                );

                let result = if let Some(task) = self.tasks.get_mut(&id) {
                    task.progress = progress;
                    task.message = message.clone();
                    task.status = status;
                    task.version = task.version.saturating_add(1);

                    // 如果任务完成或失败，记录完成时间
                    if matches!(
                        status,
                        TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Stopped
                    ) {
                        task.completed_at = Some(Instant::now());
                        info!(
                            task_id = %id,
                            status = ?status,
                            "Task finished"
                        );
                    }

                    // FFI 模式下通过事件总线发送事件
                    let _ = crate::events::emit_event(crate::events::AppEvent::TaskUpdate {
                        progress: crate::models::TaskProgress {
                            task_id: id.clone(),
                            task_type: task.task_type.clone(),
                            target: task.target.clone(),
                            status: match status {
                                TaskStatus::Running => "Running".to_string(),
                                TaskStatus::Completed => "Completed".to_string(),
                                TaskStatus::Failed => "Failed".to_string(),
                                TaskStatus::Stopped => "Stopped".to_string(),
                            },
                            message: message.clone(),
                            progress,
                            workspace_id: task.workspace_id.clone(),
                        },
                    });

                    Some(task.clone())
                } else {
                    warn!(task_id = %id, "Task not found for update");
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
                debug!(task_id = %id, "Removing task");
                let result = self.tasks.remove(&id);
                let _ = respond_to.send(result);
            }
            ActorMessage::GetMetrics { respond_to } => {
                let metrics = self.collect_metrics();
                debug!(
                    total = metrics.total_tasks,
                    running = metrics.running_tasks,
                    completed = metrics.completed_tasks,
                    failed = metrics.failed_tasks,
                    "Collected TaskManager metrics"
                );
                let _ = respond_to.send(metrics);
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

                debug!(
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
            }
        }
    }

    async fn run(mut self, mut receiver: mpsc::UnboundedReceiver<ActorMessage>) {
        info!("TaskManager actor started");

        // 使用 scopeguard 确保清理日志被记录
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

        // 优雅关闭：处理所有待处理的消息
        info!("Processing remaining messages before shutdown");
        let shutdown_timeout = Duration::from_secs(5);
        let shutdown_deadline = tokio::time::Instant::now() + shutdown_timeout;

        while let Ok(Some(msg)) = timeout(
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

        // 记录未完成的任务
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

/// 任务管理器句柄（客户端）
#[derive(Clone)]
pub struct TaskManager {
    sender: mpsc::UnboundedSender<ActorMessage>,
    config: TaskManagerConfig,
}

impl TaskManager {
    /// 创建新的任务管理器（FFI 模式）
    ///
    /// 用于 flutter_rust_bridge FFI 调用场景：
    /// - 不需要 AppHandle（无 Tauri 事件系统）
    /// - 使用单线程 tokio runtime（FRB 要求同步调用）
    /// - Flutter 端通过轮询获取任务状态
    ///
    /// # 注意
    ///
    /// Flutter 端需要通过 `get_metrics_async()` 或 `get_task()` 轮询状态。
    pub fn new_single_threaded(config: TaskManagerConfig) -> Self {
        info!("Initializing TaskManager (FFI single-threaded mode)");

        let (sender, receiver) = mpsc::unbounded_channel();
        let actor = TaskManagerActor::new(config.clone());

        // 创建单线程 tokio runtime 并启动 Actor
        // 注意：这里使用 std::thread 启动独立的 runtime，避免与 FRB 冲突
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create tokio runtime for TaskManager");

            rt.block_on(async {
                actor.run(receiver).await;
            });
        });

        info!("TaskManager initialized successfully (FFI mode)");
        Self { sender, config }
    }

    /// 创建新任务（异步版本）
    pub async fn create_task_async(
        &self,
        id: String,
        task_type: String,
        target: String,
        workspace_id: Option<String>,
    ) -> Result<TaskInfo> {
        let (tx, rx) = tokio::sync::oneshot::channel();

        let msg = ActorMessage::CreateTask {
            id: id.clone(),
            task_type,
            target,
            workspace_id,
            respond_to: tx,
        };

        self.sender
            .send(msg)
            .wrap_err("TaskManager actor has stopped")?;

        // 直接使用 await，不需要 block_on
        let timeout_duration = Duration::from_secs(self.config.operation_timeout);
        timeout(timeout_duration, rx)
            .await
            .wrap_err("Operation timed out")?
            .wrap_err("Actor dropped response channel")
    }

    /// 更新任务进度（异步版本）
    pub async fn update_task_async(
        &self,
        id: &str,
        progress: u8,
        message: String,
        status: TaskStatus,
    ) -> Result<Option<TaskInfo>> {
        let (tx, rx) = tokio::sync::oneshot::channel();

        let msg = ActorMessage::UpdateTask {
            id: id.to_string(),
            progress,
            message,
            status,
            respond_to: tx,
        };

        self.sender
            .send(msg)
            .wrap_err("TaskManager actor has stopped")?;

        let timeout_duration = Duration::from_secs(self.config.operation_timeout);
        timeout(timeout_duration, rx)
            .await
            .wrap_err("Operation timed out")?
            .wrap_err("Actor dropped response channel")
    }

    /// 获取任务管理器指标（异步版本）
    pub async fn get_metrics_async(&self) -> Result<TaskManagerMetrics> {
        let (tx, rx) = tokio::sync::oneshot::channel();

        let msg = ActorMessage::GetMetrics { respond_to: tx };

        self.sender
            .send(msg)
            .wrap_err("TaskManager actor has stopped")?;

        let timeout_duration = Duration::from_secs(self.config.operation_timeout);
        timeout(timeout_duration, rx)
            .await
            .wrap_err("Operation timed out")?
            .wrap_err("Actor dropped response channel")
    }

    /// 健康检查
    pub fn health_check(&self) -> bool {
        !self.sender.is_closed()
    }

    /// 停止任务管理器
    pub fn shutdown(&self) -> Result<()> {
        info!("Shutting down TaskManager");

        // 尝试优雅关闭，带重试机制
        let max_retries = 50; // 5秒 (50次 * 100ms)
        let mut retries = 0;

        loop {
            match self.sender.send(ActorMessage::Shutdown) {
                Ok(()) => {
                    info!("Shutdown message sent successfully");
                    break;
                }
                Err(e) => {
                    retries += 1;
                    if retries >= max_retries {
                        error!(
                            error = %e,
                            retries = retries,
                            "Failed to send shutdown message after {} retries, forcing shutdown",
                            max_retries
                        );
                        return Err(e).wrap_err("Failed to send shutdown message after timeout");
                    }

                    if retries % 10 == 0 {
                        // 每秒记录一次警告
                        warn!(
                            error = %e,
                            retries = retries,
                            "Actor channel closed, retrying shutdown..."
                        );
                    }

                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }
        }

        Ok(())
    }
}
