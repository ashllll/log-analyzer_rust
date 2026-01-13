//! 日志分析器 - 重构完成版本
//!
//! 核心功能已修复和优化：
//! - 内存泄漏修复 ✅
//! - 竞态条件修复 ✅
//! - 时间戳解析增强 ✅
//! - 错误处理统一 ✅
//! - 监控体系建立 ✅

pub mod error;
pub mod models;
pub mod utils;
pub mod commands;

pub use error::{AppError, Result};