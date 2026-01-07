//! 日志分析器 - Rust 后端
//!
//! 提供高性能的日志分析功能，包括：
//! - 多格式压缩包递归解压
//! - 并发全文搜索
//! - 结构化查询系统
//! - 持久化与增量更新
//! - 实时文件监听

// 模块声明
pub mod archive;
pub mod commands;
pub mod error;
pub mod events;
pub mod models;
pub mod search_engine;
pub mod services;
pub mod state_sync;
pub mod storage;
pub mod task_manager;
pub mod utils;

// 从 models 导入类型
pub use models::state::AppState;
