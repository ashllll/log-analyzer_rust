//! Tauri 命令层
//!
//! 提供前端调用的所有命令接口，包括：
//! - 工作区管理（导入、加载、刷新）
//! - 搜索功能（全文搜索、结构化查询）
//! - 导出功能
//! - 实时文件监听
//! - 配置管理

pub mod config;
pub mod export;
pub mod import;
pub mod log_config;
pub mod search;
pub mod state_sync;
pub mod validation;
pub mod virtual_tree;
pub mod watch;
pub mod workspace;

// TauriAppConfigProvider 已移至 adapters::tauri_config 模块
// 保留 re-export 以保持向后兼容
pub use crate::adapters::tauri_config::TauriAppConfigProvider;
