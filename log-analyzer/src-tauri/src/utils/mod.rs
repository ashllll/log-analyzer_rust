//! 通用工具函数模块
//!
//! 提供路径处理、编码转换、参数验证、重试机制和清理功能等通用工具。

pub mod cancellation_manager;
pub mod cleanup;
pub mod encoding;
pub mod log_file_detector;
pub mod path;
pub mod path_security;
pub mod resource_manager;
pub mod resource_tracker;
pub mod retry;
pub mod validation;

#[cfg(test)]
mod resource_management_property_tests;

// 重新导出常用工具函数
pub use cancellation_manager::{run_with_cancellation, CancellableOperation, CancellationManager};
pub use path::{canonicalize_path, normalize_path_separator};
pub use resource_manager::{create_guarded_temp_dir, ResourceManager, TempDirGuard};
pub use resource_tracker::{ResourceInfo, ResourceReport, ResourceTracker, ResourceType};
pub use validation::{validate_path_param, validate_workspace_id};
