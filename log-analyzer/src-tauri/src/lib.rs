//! 日志分析器 - 重构完成版本
//!
//! 核心功能已修复和优化：
//! - 内存泄漏修复 ✅
//! - 竞态条件修复 ✅
//! - 时间戳解析增强 ✅
//! - 错误处理统一 ✅
//! - 监控体系建立 ✅

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

// 领域驱动设计模块
pub mod application;
pub mod domain;
pub mod infrastructure;

pub use error::{AppError, Result};
