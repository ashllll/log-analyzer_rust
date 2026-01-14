//! 日志分析领域模型

pub mod entities;
pub mod value_objects;
// pub mod services; // TODO: 模块文件缺失，暂时注释
// pub mod events; // TODO: 模块文件缺失，暂时注释
// pub mod repositories; // TODO: 模块文件缺失，暂时注释

pub use entities::LogEntry;
pub use value_objects::{LogLevel, Timestamp, LogMessage};
// pub use services::LogParserService; // TODO: 模块文件缺失，暂时注释
// pub use events::LogAnalysisEvent; // TODO: 模块文件缺失，暂时注释