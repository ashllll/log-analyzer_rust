//! 共享领域模型

pub mod events;
// pub mod value_objects; // TODO: value_objects 在 log_analysis 中，暂时注释
// pub mod specifications; // TODO: 模块文件缺失，暂时注释

pub use events::{DomainEvent, DomainEventBus, EventHandler};
