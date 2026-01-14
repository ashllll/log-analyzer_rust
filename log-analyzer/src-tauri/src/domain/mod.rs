//! 领域层 - 核心业务逻辑
//!
//! 采用领域驱动设计(DDD)模式，包含：
//! - 实体(Entity)
//! - 值对象(Value Object)
//! - 领域服务(Domain Service)
//! - 领域事件(Domain Event)
//! - 仓储接口(Repository Interface)

pub mod log_analysis;
// pub mod search; // TODO: 模块文件缺失，暂时注释
// pub mod export; // TODO: 模块文件缺失，暂时注释
pub mod shared;

/// 领域事件总线
pub use shared::events::DomainEventBus;