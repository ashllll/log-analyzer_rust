//! 共享领域模型

pub mod events;

pub use events::{DomainEvent, DomainEventBus, EventHandler};
