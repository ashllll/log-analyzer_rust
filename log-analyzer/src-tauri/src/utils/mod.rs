//! 通用工具函数模块
//!
//! 提供路径处理、编码转换、参数验证、重试机制和清理功能等通用工具。

pub mod async_resource_manager;
pub mod cache_manager;
pub mod cancellation_manager;
pub mod cleanup;
pub mod encoding;
pub mod encoding_detector; // M4.1: chardetng 编码嗅探
pub mod legacy_detection;
pub mod log_file_detector;
pub mod path;
pub mod path_security;
pub mod resource_manager;
pub mod resource_tracker;
pub mod retry;
pub mod transcoding_pipe; // M4.1: 流式转码管道
pub mod validation; // ✅ 添加缺失的模块导出

#[cfg(test)]
mod resource_management_property_tests;

// 重新导出常用工具函数
pub use async_resource_manager::{AsyncResourceManager, OperationType}; // ✅ 添加异步资源管理
pub use cache_manager::CacheManager;
pub use cancellation_manager::{run_with_cancellation, CancellableOperation, CancellationManager};
pub use encoding_detector::{EncodingDetectionResult, EncodingDetector}; // M4.1: 编码检测
pub use legacy_detection::{
    check_workspace_legacy_format, generate_legacy_message, scan_legacy_workspaces,
    LegacyFormatType, LegacyWorkspaceInfo,
};
pub use path::{canonicalize_path, normalize_path_separator};
pub use resource_manager::{create_guarded_temp_dir, ResourceManager, TempDirGuard};
pub use resource_tracker::{ResourceInfo, ResourceReport, ResourceTracker, ResourceType};
pub use transcoding_pipe::{
    breaks_simd, create_transcoding_pipe, needs_transcoding, TranscodingError, TranscodingPipe,
    TranscodingStats,
}; // M4.1: 转码管道
pub use validation::{validate_path_param, validate_workspace_id};
