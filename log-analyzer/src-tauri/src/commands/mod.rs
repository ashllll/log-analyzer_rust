//! Tauri 命令层
//!
//! 提供前端调用的所有命令接口，包括：
//! - 工作区管理（导入、加载、刷新、删除、状态）
//! - 搜索功能（search_logs、fetch_search_page、cancel_search）
//! - 导入与导出功能
//! - 日志配置管理（运行时调整日志级别与预设）
//! - 实时文件监听
//! - 虚拟文件树
//! - 状态同步
//! - 参数验证
//! - 全局配置管理

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

// FIX(CR-01): level_to_mask 已提取到 la_core::utils::log_levels
// 保留 re-export 以保持二进制兼容性
pub use la_core::utils::level_to_mask;
