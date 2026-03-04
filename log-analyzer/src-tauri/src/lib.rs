//! 日志分析器 - 重构完成版本
//!
//! 核心功能已修复和优化：
//! - 内存泄漏修复 ✅
//! - 竞态条件修复 ✅
//! - 时间戳解析增强 ✅
//! - 错误处理统一 ✅
//! - 监控体系建立 ✅

// FFI 桥接代码生成（仅在启用 ffi feature 时编译）
#[cfg(feature = "ffi")]
mod frb_generated; /* AUTO INJECTED BY flutter_rust_bridge. This line may not be accurate, and you can change it according to your needs. */

// 核心模块
pub mod commands;
pub mod error;
pub mod models;
pub mod utils;

// 存储和搜索模块
pub mod archive;
pub mod search_engine;
pub mod services;
pub mod storage;

// 任务和状态管理
pub mod state_sync;
pub mod task_manager;

// 事件和监控
pub mod events;
pub mod monitoring;

// 安全防护
pub mod security;

// 领域驱动设计模块
pub mod application;
pub mod domain;
pub mod infrastructure;

// FFI 桥接模块（仅在启用 ffi feature 时编译）
#[cfg(feature = "ffi")]
pub mod ffi;

// 测试策略模块
#[cfg(test)]
pub mod proptest_strategies;

pub use error::{AppError, Result};
