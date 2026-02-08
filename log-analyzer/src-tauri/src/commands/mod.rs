//! Tauri 命令层
//!
//! 提供前端调用的所有命令接口，包括：
//! - 工作区管理（导入、加载、刷新）
//! - 搜索功能（全文搜索、结构化查询）
//! - 导出功能
//! - 实时文件监听
//! - 配置管理
//!
//! 注意：部分命令模块暂时禁用，因为它们需要额外的依赖修复

pub mod async_search;
pub mod cache;
pub mod config;
pub mod error_reporting;
pub mod export;
pub mod import;
pub mod legacy;
pub mod query;
pub mod search;
pub mod state_sync;
pub mod validation;
pub mod virtual_tree;
pub mod watch;
pub mod workspace;
