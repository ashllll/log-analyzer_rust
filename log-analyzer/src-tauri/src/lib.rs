//! 日志分析器 - 重构完成版本
//!
//! 核心功能已修复和优化：
//! - 内存泄漏修复 ✅
//! - 竞态条件修复 ✅
//! - 时间戳解析增强 ✅
//! - 错误处理统一 ✅
//! - 监控体系建立 ✅

// Clean Architecture layers — interfaces/ was collapsed into commands/ (2026-05)
pub mod application;
pub mod infrastructure;

// 核心模块
pub mod adapters;
pub mod commands;
pub mod models;
pub mod utils;

// 业务服务与引擎（直接使用 la_search / la_storage crate）
pub mod services;

// 任务和状态管理
pub mod state_sync;
pub mod task_manager;

// 测试策略模块
#[cfg(test)]
pub mod proptest_strategies;

pub use la_core::error::{AppError, Result};
