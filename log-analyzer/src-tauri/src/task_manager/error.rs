//! 任务管理器错误类型

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
