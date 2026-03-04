//! CQRS 命令处理器模块
//!
//! 实现命令查询职责分离(CQRS)模式的命令部分：
//! - 命令接口 (Command trait)
//! - 命令处理器 (CommandHandler trait)
//! - 命令总线 (CommandBus)
//! - 具体命令实现

#[allow(dead_code)]
mod bus;
#[allow(dead_code)]
mod commands;
#[allow(clippy::module_inception)]
#[allow(dead_code)]
mod handlers;

pub use bus::CommandBus;
pub use commands::{
    CancelTaskCommand, CreateWorkspaceCommand, DeleteWorkspaceCommand, ImportFilesCommand,
    SaveKeywordsCommand,
};
pub use handlers::{CommandHandler, CommandResult};
