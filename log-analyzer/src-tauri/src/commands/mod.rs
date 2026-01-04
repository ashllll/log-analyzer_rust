//! Tauri 命令层
//!
//! 提供前端调用的所有命令接口，包括：
//! - 工作区管理（导入、加载、刷新）
//! - 搜索功能（全文搜索、结构化查询）
//! - 导出功能
//! - 性能监控
//! - 实时文件监听
//! - 配置管理
//!
//! 注意：当前阶段模块已创建，但命令实现仍在lib.rs中。
//! 在阶段5整合时将命令从lib.rs迁移到此处。

pub mod config;
pub mod export;
pub mod import;
pub mod legacy;
pub mod performance;
pub mod query;
pub mod search;
pub mod search_history;
pub mod state_sync;
pub mod virtual_tree;
pub mod watch;
pub mod workspace;
