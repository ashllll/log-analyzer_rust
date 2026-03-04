//! Flutter Rust FFI 桥接模块
//!
//! 提供与 Flutter 前端的高性能 FFI 通信接口
//!
//! ## 架构说明
//!
//! 本模块使用 flutter_rust_bridge 实现 Rust 与 Dart 的类型安全 FFI 通信。
//!
//! ### 特性
//! - 类型安全：通过代码生成确保 Rust 和 Dart 类型一致
//! - 零拷贝：避免不必要的序列化开销
//! - 异步支持：支持 async 函数和 Stream
//! - 错误处理：自动转换 Rust 错误为 Dart 异常
//!
//! ## 使用方式
//!
//! ```dart
//! // Flutter 端
//! import 'package:log_analyzer_flutter/bridge_generated.dart';
//!
//! final api = LogAnalyzerApi();
//! final workspaces = await api.getWorkspaces();
//! ```

// 仅在启用 ffi feature 时编译此模块
#[cfg(feature = "ffi")]
pub mod bridge;

#[cfg(feature = "ffi")]
pub mod bridge_minimal;

#[cfg(feature = "ffi")]
pub mod types;

#[cfg(feature = "ffi")]
pub mod commands_bridge;

#[cfg(feature = "ffi")]
pub mod global_state;

// FFI 专用类型定义
#[cfg(feature = "ffi")]
pub use types::*;

// 桥接上下文（用于全局状态管理）
#[cfg(feature = "ffi")]
pub use bridge::BridgeContext;

// 全局状态管理
#[cfg(feature = "ffi")]
pub use global_state::{
    clear_global_state,
    // Session 管理函数
    create_session,
    get_all_session_ids,
    get_app_data_dir,
    get_app_state,
    get_global_state,
    get_session_count,
    get_session_entries,
    get_session_info,
    index_session,
    init_global_state,
    is_initialized,
    map_session,
    remove_session,
    FfiContext,
    SessionHolder,
};

// Typestate Session FFI 函数
#[cfg(feature = "ffi")]
pub use commands_bridge::{
    ffi_close_session,
    // PageManager 相关
    ffi_create_page_manager,
    ffi_destroy_page_manager,
    ffi_get_all_sessions,
    ffi_get_index_entries,
    ffi_get_line,
    ffi_get_page_manager_info,
    ffi_get_session_count,
    ffi_get_session_info,
    ffi_get_viewport,
    ffi_index_session,
    ffi_map_session,
    // Session 相关
    ffi_open_session,
};
