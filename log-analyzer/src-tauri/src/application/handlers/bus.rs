//! CQRS 命令总线
//!
//! 负责将命令分发给对应的处理器执行

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;

use super::commands::Command;
use super::handlers::{CommandHandler, CommandResult};

/// 命令处理器工厂类型
type HandlerFactory = Arc<dyn Fn() -> Box<dyn Any + Send + Sync> + Send + Sync>;

/// 命令总线
///
/// 负责命令的分发和执行
pub struct CommandBus {
    handlers: RwLock<HashMap<TypeId, HandlerFactory>>,
}

impl CommandBus {
    /// 创建新的命令总线
    pub fn new() -> Self {
        Self {
            handlers: RwLock::new(HashMap::new()),
        }
    }

    /// 注册命令处理器
    pub fn register<C, H>(&self, handler: H)
    where
        C: Command + 'static,
        H: CommandHandler<C> + 'static,
    {
        let type_id = TypeId::of::<C>();
        let handler = Arc::new(handler);

        let factory: HandlerFactory = Arc::new(move || {
            let h: Arc<dyn CommandHandler<C>> = handler.clone();
            Box::new(h) as Box<dyn Any + Send + Sync>
        });

        let mut handlers = self.handlers.write().unwrap();
        handlers.insert(type_id, factory);
    }

    /// 执行命令
    pub async fn dispatch<C>(&self, command: C) -> CommandResult<Box<dyn Any + Send>>
    where
        C: Command + 'static,
    {
        let type_id = TypeId::of::<C>();

        // 获取处理器工厂（在 await 之前释放锁）
        let factory = {
            let handlers = self.handlers.read().unwrap();
            handlers.get(&type_id).cloned().ok_or_else(|| {
                super::handlers::CommandError::ExecutionFailed(format!(
                    "未找到命令类型的处理器: {}",
                    command.command_type()
                ))
            })?
        };

        let handler_box = factory();
        let handler = handler_box
            .downcast_ref::<Arc<dyn CommandHandler<C>>>()
            .ok_or_else(|| {
                super::handlers::CommandError::ExecutionFailed("处理器类型转换失败".to_string())
            })?;

        // 执行命令
        handler.handle(command).await
    }

    /// 检查是否注册了处理器
    pub fn has_handler<C: Command + 'static>(&self) -> bool {
        let type_id = TypeId::of::<C>();
        let handlers = self.handlers.read().unwrap();
        handlers.contains_key(&type_id)
    }

    /// 获取已注册的处理器数量
    pub fn handler_count(&self) -> usize {
        self.handlers.read().unwrap().len()
    }
}

impl Default for CommandBus {
    fn default() -> Self {
        Self::new()
    }
}

/// 命令执行器 trait
///
/// 提供更简洁的命令执行接口
#[async_trait]
pub trait CommandExecutor: Send + Sync {
    /// 执行命令
    async fn execute<C>(&self, command: C) -> CommandResult<Box<dyn Any + Send>>
    where
        C: Command + 'static;
}

#[async_trait]
impl CommandExecutor for CommandBus {
    async fn execute<C>(&self, command: C) -> CommandResult<Box<dyn Any + Send>>
    where
        C: Command + 'static,
    {
        self.dispatch(command).await
    }
}

// ==================== 单元测试 ====================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::handlers::commands::CreateWorkspaceCommand;

    #[test]
    fn test_command_bus_creation() {
        let bus = CommandBus::new();
        assert_eq!(bus.handler_count(), 0);
    }

    #[test]
    fn test_command_bus_has_handler() {
        let bus = CommandBus::new();
        assert!(!bus.has_handler::<CreateWorkspaceCommand>());
    }

    #[test]
    fn test_command_bus_default() {
        let bus = CommandBus::default();
        assert_eq!(bus.handler_count(), 0);
    }

    #[tokio::test]
    async fn test_command_bus_dispatch_no_handler() {
        let bus = CommandBus::new();
        let cmd =
            CreateWorkspaceCommand::new("Test".to_string(), std::path::PathBuf::from("/path"));

        let result = bus.dispatch(cmd).await;
        assert!(result.is_err());
    }
}
