//! Flutter Rust FFI 桥接模块（重构版）
//!
//! 提供与 Flutter 前端的高性能、安全的 FFI 通信接口。
//!
//! ## 架构说明
//!
//! 本模块使用 flutter_rust_bridge 2.x 实现 Rust 与 Dart 的类型安全 FFI 通信。
//!
//! ### 特性
//! - **类型安全**：通过代码生成确保 Rust 和 Dart 类型一致
//! - **零拷贝**：避免不必要的序列化开销
//! - **异步支持**：使用 `#[frb]`（异步）替代同步调用，避免阻塞 UI
//! - **安全错误处理**：使用 `FfiError` 替代 panic，提供结构化错误信息
//! - **全局单例 Runtime**：使用 `OnceLock` 管理 Tokio Runtime
//!
//! ## 模块结构
//!
//! ```
//! ffi/
//! ├── error.rs          # FFI 错误类型定义（FfiError, FfiResult）
//! ├── runtime.rs        # 全局 Tokio Runtime 管理
//! ├── global_state.rs   # 全局状态管理（修复版）
//! ├── bridge.rs         # flutter_rust_bridge 接口
//! ├── commands_bridge.rs # 命令桥接（需添加异步变体）
//! └── types.rs          # FFI 类型定义
//! ```
//!
//! ## 使用方式
//!
//! ### Flutter 端
//!
//! ```dart
//! import 'package:log_analyzer_flutter/bridge_generated.dart';
//!
//! final api = LogAnalyzerApi();
//!
//! // 异步调用（推荐）
//! final workspaces = await api.getWorkspaces();
//!
//! // 同步调用（仅用于轻量级操作）
//! final isHealthy = api.healthCheck();
//! ```
//!
//! ### Rust 端
//!
//! ```rust
//! use crate::ffi::bridge::create_workspace;
//! use crate::ffi::error::FfiResult;
//!
//! async fn example() -> FfiResult<String> {
//!     create_workspace("my_workspace".to_string(), "/path/to/logs".to_string()).await
//! }
//! ```

// 仅在启用 ffi feature 时编译此模块
#[cfg(feature = "ffi")]
pub mod error;

#[cfg(feature = "ffi")]
pub mod runtime;

#[cfg(feature = "ffi")]
pub mod global_state;

#[cfg(feature = "ffi")]
pub mod bridge;

#[cfg(feature = "ffi")]
pub mod types;

// 条件编译：commands_bridge 需要添加异步变体
// 保留原有模块用于向后兼容，建议迁移到异步版本
#[cfg(feature = "ffi")]
pub mod commands_bridge;

#[cfg(feature = "ffi")]
pub mod commands_bridge_async;

// ==================== 公开导出 ====================

#[cfg(feature = "ffi")]
pub use error::{
    catch_panic_as_ffi_error, catch_panic_with_default, map_error, setup_ffi_panic_hook,
    FfiError, FfiErrorCode, FfiResult, FfiResultWrapper,
};

#[cfg(feature = "ffi")]
pub use runtime::{
    block_on, get_cancellation_token, get_runtime, get_runtime_stats,
    init_runtime, is_runtime_initialized, runtime_health_check,
    shutdown_runtime, spawn, spawn_blocking, RuntimeConfig, RuntimeHandle, RuntimeStats,
};

#[cfg(feature = "ffi")]
pub use global_state::{
    clear_all_sessions, clear_global_state, cleanup_expired_sessions, create_session,
    get_all_session_ids, get_app_data_dir, get_app_state, get_global_state,
    get_session, get_session_count, get_session_entries, get_session_info, get_session_stats,
    index_session, init_global_state, is_initialized, map_session, remove_session,
    update_global_state, FfiContext, SessionConfig, SessionHolder,
};

#[cfg(feature = "ffi")]
pub use bridge::{
    init_bridge, BridgeContext, // 添加其他需要导出的桥接函数
};

#[cfg(feature = "ffi")]
pub use types::*;

// ==================== 便捷宏 ====================

/// FFI 模块初始化
///
/// 在应用启动时调用，初始化所有 FFI 组件
#[cfg(feature = "ffi")]
pub fn init_ffi() -> error::FfiResult<()> {
    // 设置 panic 钩子
    setup_ffi_panic_hook();

    // 初始化全局 Runtime
    runtime::init_runtime(None)?;

    tracing::info!("FFI 模块初始化完成");

    Ok(())
}

/// FFI 模块关闭
///
/// 在应用退出时调用，清理资源
#[cfg(feature = "ffi")]
pub fn shutdown_ffi() {
    use std::time::Duration;

    // 关闭 Runtime
    let _ = runtime::shutdown_runtime(Duration::from_secs(5));

    // 清理全局状态
    global_state::clear_global_state();

    tracing::info!("FFI 模块已关闭");
}

// ==================== 测试 ====================

#[cfg(all(test, feature = "ffi"))]
mod tests {
    use super::*;

    #[test]
    fn test_error_types() {
        let err = error::FfiError::new(error::FfiErrorCode::NotFound, "test");
        assert!(matches!(err.code(), error::FfiErrorCode::NotFound));
    }

    #[test]
    fn test_runtime_config() {
        let config = runtime::RuntimeConfig::for_ffi();
        assert!(config.worker_threads > 0);
    }

    #[test]
    fn test_ffi_error_variants() {
        use error::FfiError;

        // 测试各种错误变体
        let err = FfiError::NotInitialized;
        assert!(matches!(err, FfiError::NotInitialized));

        let err = FfiError::Io {
            message: "test".to_string(),
            path: Some("/path".to_string()),
        };
        assert!(matches!(err, FfiError::Io { .. }));

        let err = FfiError::Search {
            message: "search failed".to_string(),
        };
        assert!(matches!(err, FfiError::Search { .. }));

        let err = FfiError::NotFound {
            resource: "file".to_string(),
            id: "123".to_string(),
        };
        assert!(matches!(err, FfiError::NotFound { .. }));
    }

    #[test]
    fn test_map_error() {
        let ok: Result<i32, String> = Ok(42);
        let result = error::map_error(ok, "context");
        assert_eq!(result.unwrap(), 42);

        let err: Result<i32, String> = Err("error".to_string());
        let result = error::map_error(err, "context");
        assert!(result.is_err());
    }
}
