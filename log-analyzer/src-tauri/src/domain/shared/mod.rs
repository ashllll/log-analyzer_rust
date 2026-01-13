//! 共享领域模型

pub mod events;
pub mod value_objects;
pub mod specifications;

pub use events::{DomainEvent, DomainEventBus, EventHandler};