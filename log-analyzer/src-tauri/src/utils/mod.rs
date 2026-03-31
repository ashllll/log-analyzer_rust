//! 通用工具函数模块
//!
//! 提供路径处理、编码转换、参数验证、重试机制和清理功能等通用工具。

pub mod async_resource_manager;
pub mod cache;
pub mod cache_manager;
pub mod cancellation_manager;
pub mod cleanup;
pub mod encoding;
pub mod legacy_detection;
#[cfg(test)]
pub mod log_file_detector;
pub mod log_config;
pub mod path;
pub mod path_security;
pub mod resource_manager;
pub mod resource_tracker;
pub mod retry;
pub mod validation;

#[cfg(test)]
mod resource_management_property_tests;

// 重新导出常用工具函数
pub use async_resource_manager::{AsyncResourceManager, OperationType}; // ✅ 添加异步资源管理
pub use cache_manager::CacheManager;
pub use cancellation_manager::{run_with_cancellation, CancellableOperation, CancellationManager};
pub use legacy_detection::{
    check_workspace_legacy_format, generate_legacy_message, scan_legacy_workspaces,
    LegacyFormatType, LegacyWorkspaceInfo,
};
pub use log_config::{
    get_debug_log_config, get_log_config, get_production_log_config, load_log_config_from_file,
    reset_log_config, save_log_config_to_file, set_global_log_level, set_module_log_level,
    LogConfig, LogLevel, ModuleLogConfig,
};
pub use path::{canonicalize_path, normalize_path_separator};
pub use resource_manager::{create_guarded_temp_dir, ResourceManager, TempDirGuard};
pub use resource_tracker::{ResourceInfo, ResourceReport, ResourceTracker, ResourceType};
pub use validation::{validate_path_param, validate_workspace_id};
