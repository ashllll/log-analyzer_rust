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
pub mod log_config;
pub mod query;
pub mod search;
pub mod state_sync;
pub mod validation;
pub mod virtual_tree;
pub mod watch;
pub mod workspace;

// ========== Tauri AppHandle 的 trait 实现 ==========
// 为主 crate 中 Tauri 类型实现 la-core 定义的 trait，
// 桥接框架层与业务层
// 由于孤儿规则限制，使用 newtype 包装器

use la_core::traits::AppConfigProvider;
use tauri::Manager;

/// Tauri AppHandle 的 AppConfigProvider 包装器
///
/// 由于 Rust 孤儿规则，无法直接为 `tauri::AppHandle` 实现外部 trait。
/// 使用 newtype 包装器绕过此限制。
pub struct TauriAppConfigProvider(pub tauri::AppHandle);

impl AppConfigProvider for TauriAppConfigProvider {
    fn config_dir(&self) -> std::result::Result<std::path::PathBuf, String> {
        self.0
            .path()
            .app_config_dir()
            .map_err(|e: tauri::Error| e.to_string())
    }
}
