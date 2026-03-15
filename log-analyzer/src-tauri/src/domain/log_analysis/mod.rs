//! 日志分析领域模型

pub mod entities;
pub mod value_objects;

pub use entities::LogEntry;
pub use value_objects::{LogLevel, LogMessage, Timestamp};
