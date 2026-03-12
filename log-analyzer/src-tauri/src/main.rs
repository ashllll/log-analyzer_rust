//! 日志分析器 - 主入口
//!
//! 此文件根据编译特性有不同的行为:
//! - `ffi` (默认): 纯 FFI 动态库模式，导出 C ABI 函数供 Flutter 调用
//! - `standalone`: 独立二进制模式，包含 Tauri 运行时（用于测试或桌面功能）
//!
//! ## 架构模式
//!
//! ### FFI 模式 (推荐用于 Flutter)
//! ```
//! Flutter App → FFI Bridge → Rust Library → Business Logic
//! ```
//!
//! ### Standalone 模式 (用于测试/桌面)
//! ```
//! Tauri App → Commands → Business Logic
//! ```

// =============================================================================
// 条件编译入口
// =============================================================================

#[cfg(feature = "standalone")]
mod standalone;

#[cfg(feature = "standalone")]
#[tokio::main]
async fn main() -> eyre::Result<()> {
    standalone::run().await
}

#[cfg(not(feature = "standalone"))]
fn main() {
    eprintln!(
        "Log Analyzer FFI Library v{}",
        env!("CARGO_PKG_VERSION")
    );
    eprintln!("此 crate 设计为作为动态库使用。");
    eprintln!("使用 --features standalone 构建独立二进制。");
    std::process::exit(1);
}

// =============================================================================
// FFI 导出函数 (C ABI) - 在 FFI 和 Standalone 模式下都可用
// =============================================================================

use std::sync::OnceLock;

/// FFI 初始化标志
static FFI_INITIALIZED: OnceLock<bool> = OnceLock::new();

/// FFI 库初始化函数
///
/// 当动态库被加载时自动调用（某些平台）
/// 或由 flutter_rust_bridge 显式调用
#[no_mangle]
pub extern "C" fn log_analyzer_ffi_init() -> i32 {
    // 初始化日志系统
    init_logging();

    // 设置初始化标志
    if FFI_INITIALIZED.set(true).is_err() {
        tracing::warn!("FFI 已经初始化");
    }

    tracing::info!("🚀 Log Analyzer FFI Library v{} 初始化成功", env!("CARGO_PKG_VERSION"));

    0 // 成功返回 0
}

/// FFI 库版本查询
#[no_mangle]
pub extern "C" fn log_analyzer_version() -> *const u8 {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr()
}

/// FFI 健康检查
#[no_mangle]
pub extern "C" fn log_analyzer_health_check() -> i32 {
    0 // 0 表示健康
}

/// 初始化日志系统
fn init_logging() {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .init();
}

/// FFI: 安全检查字符串是否有效 UTF-8
///
/// 用于验证从 Dart 传入的字符串指针
#[no_mangle]
pub unsafe extern "C" fn log_analyzer_validate_string(ptr: *const u8, len: usize) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let slice = std::slice::from_raw_parts(ptr, len);
    match std::str::from_utf8(slice) {
        Ok(_) => 0,
        Err(_) => -2,
    }
}
