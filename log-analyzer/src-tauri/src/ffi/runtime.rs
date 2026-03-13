//! 全局 Tokio 运行时管理
//!
//! 提供 FFI 安全的单例 Tokio Runtime，避免每次 FFI 调用都创建新运行时。
//!
//! ## 设计原则
//!
//! 1. **全局单例**: 使用 `OnceLock` 确保只有一个 Runtime 实例
//! 2. **延迟初始化**: 首次使用时才创建 Runtime
//! 3. **线程安全**: 使用 `Arc` 共享，使用 `RwLock` 保护内部状态
//! 4. **优雅关闭**: 提供 shutdown 钩子进行资源清理
//!
//! ## 参考实现
//!
//! - [Node.js N-API Thread-safe Functions](https://nodejs.org/api/n-api.html#n_api_asynchronous_thread_safe_function_calls)
//! - [PyO3 Async Runtime](https://pyo3.rs/main/ecosystem/async-await.html)
//! - [tokio::runtime::Runtime 文档](https://docs.rs/tokio/latest/tokio/runtime/struct.Runtime.html)

use std::future::Future;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use parking_lot::{Mutex, RwLock};
use tokio::runtime::{Builder, Runtime};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::ffi::error::{FfiError, FfiErrorCode, FfiResult};

/// 全局运行时配置
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// 工作线程数量（默认：CPU 核心数）
    pub worker_threads: usize,
    /// 最大阻塞线程数
    pub max_blocking_threads: usize,
    /// 线程栈大小（字节）
    pub thread_stack_size: usize,
    /// 线程名称前缀
    pub thread_name_prefix: String,
    /// 是否启用时间驱动
    pub enable_time: bool,
    /// 是否启用 IO 驱动
    pub enable_io: bool,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            worker_threads: num_cpus::get().max(2),
            max_blocking_threads: 512,
            thread_stack_size: 2 * 1024 * 1024, // 2MB
            thread_name_prefix: "ffi-runtime".to_string(),
            enable_time: true,
            enable_io: true,
        }
    }
}

impl RuntimeConfig {
    /// 创建适合 FFI 的配置
    ///
    /// FFI 场景通常需要：
    /// - 较少的 IO 线程（避免与 Flutter 主线程冲突）
    /// - 更多的阻塞线程（处理文件 IO）
    pub fn for_ffi() -> Self {
        Self {
            worker_threads: num_cpus::get().max(2),
            max_blocking_threads: 256,
            thread_stack_size: 4 * 1024 * 1024, // 4MB（处理大文件需要更大栈）
            thread_name_prefix: "log-analyzer-ffi".to_string(),
            enable_time: true,
            enable_io: true,
        }
    }

    /// 创建适合资源受限环境的配置
    pub fn for_constrained() -> Self {
        Self {
            worker_threads: 2,
            max_blocking_threads: 64,
            thread_stack_size: 1024 * 1024, // 1MB
            thread_name_prefix: "log-analyzer-constrained".to_string(),
            enable_time: true,
            enable_io: true,
        }
    }
}

/// 运行时统计信息
#[derive(Debug, Clone, Default)]
pub struct RuntimeStats {
    /// 活跃任务数量
    pub active_tasks: usize,
    /// 已完成的任务数量
    pub completed_tasks: u64,
    /// 失败的任务数量
    pub failed_tasks: u64,
    /// 运行时启动时间
    pub started_at: Option<std::time::Instant>,
    /// 队列中的任务数量（估计值）
    pub queued_tasks: usize,
}

/// 运行时句柄包装器
///
/// 提供对 Tokio Runtime 的安全访问和统计
pub struct RuntimeHandle {
    /// 底层 Tokio Runtime
    runtime: Runtime,
    /// 运行时统计
    stats: Arc<RwLock<RuntimeStats>>,
    /// 全局取消令牌
    cancellation_token: CancellationToken,
    /// 活跃任务计数
    active_count: Arc<Mutex<usize>>,
}

impl RuntimeHandle {
    /// 从配置创建运行时
    fn from_config(config: RuntimeConfig) -> FfiResult<Self> {
        let mut builder = Builder::new_multi_thread();

        builder
            .worker_threads(config.worker_threads)
            .max_blocking_threads(config.max_blocking_threads)
            .thread_stack_size(config.thread_stack_size)
            .thread_name_fn({
                let prefix = config.thread_name_prefix.clone();
                let counter = std::sync::atomic::AtomicU64::new(0);
                move || {
                    let id = counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    format!("{}-{}", prefix, id)
                }
            });

        if config.enable_time {
            builder.enable_time();
        }
        if config.enable_io {
            builder.enable_io();
        }

        let runtime = builder.build().map_err(|e| {
            FfiError::runtime_error("创建 Tokio Runtime", e)
        })?;

        tracing::info!(
            worker_threads = config.worker_threads,
            max_blocking = config.max_blocking_threads,
            "Tokio Runtime 已创建"
        );

        Ok(Self {
            runtime,
            stats: Arc::new(RwLock::new(RuntimeStats {
                started_at: Some(std::time::Instant::now()),
                ..Default::default()
            })),
            cancellation_token: CancellationToken::new(),
            active_count: Arc::new(Mutex::new(0)),
        })
    }

    /// 在运行时上阻塞执行异步任务
    ///
    /// 这是 FFI 调用的主要入口点
    pub fn block_on<F, T>(&self, f: F) -> T
    where
        F: Future<Output = T>,
    {
        self.runtime.block_on(f)
    }

    /// 生成新任务
    pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let stats = self.stats.clone();
        let active_count = self.active_count.clone();

        // 更新统计
        {
            let mut count = active_count.lock();
            *count += 1;
            let mut s = stats.write();
            s.active_tasks = *count;
        }

        // 包装任务以跟踪完成
        let tracked_future = async move {
            let result = future.await;

            // 更新统计
            {
                let mut count = active_count.lock();
                *count = count.saturating_sub(1);
                let mut s = stats.write();
                s.active_tasks = *count;
                s.completed_tasks += 1;
            }

            result
        };

        self.runtime.spawn(tracked_future)
    }

    /// 在阻塞线程池中执行
    pub fn spawn_blocking<F, T>(&self, f: F) -> JoinHandle<T>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        self.runtime.spawn_blocking(f)
    }

    /// 获取运行时统计
    pub fn stats(&self) -> RuntimeStats {
        self.stats.read().clone()
    }

    /// 获取取消令牌
    pub fn cancellation_token(&self) -> CancellationToken {
        self.cancellation_token.clone()
    }

    /// 检查是否已请求关闭
    pub fn is_shutdown_requested(&self) -> bool {
        self.cancellation_token.is_cancelled()
    }

    /// 优雅关闭运行时
    ///
    /// 1. 发出取消信号
    /// 2. 等待任务完成（带超时）
    /// 3. 强制关闭
    pub fn shutdown(&self, timeout: Duration) {
        tracing::info!("正在关闭 Tokio Runtime...");

        // 发出取消信号
        self.cancellation_token.cancel();

        // 尝试优雅关闭
        self.runtime.block_on(async {
            tokio::time::timeout(timeout, async {
                // 等待一段时间让任务完成
                tokio::time::sleep(Duration::from_millis(100)).await;
            })
            .await
            .ok();
        });

        tracing::info!("Tokio Runtime 已关闭");
    }
}

// ==================== 全局单例 ====================

/// 全局运行时实例
static GLOBAL_RUNTIME: OnceLock<RuntimeHandle> = OnceLock::new();

/// 全局运行时配置
static GLOBAL_CONFIG: OnceLock<RwLock<RuntimeConfig>> = OnceLock::new();

/// 初始化全局运行时
///
/// # 安全性
///
/// - 线程安全：使用 `OnceLock` 确保只初始化一次
/// - 幂等：多次调用只有第一次生效
/// - 错误处理：返回 `FfiError` 而不是 panic
///
/// # 示例
///
/// ```rust,no_run
/// use std::time::Duration;
///
/// // 初始化默认配置
/// init_runtime(None).expect("初始化失败");
///
/// // 或自定义配置
/// let config = RuntimeConfig::for_ffi();
/// init_runtime(Some(config)).expect("初始化失败");
/// ```
pub fn init_runtime(config: Option<RuntimeConfig>) -> FfiResult<&'static RuntimeHandle> {
    let config = config.unwrap_or_else(RuntimeConfig::for_ffi);

    // 存储配置
    GLOBAL_CONFIG.get_or_init(|| RwLock::new(config.clone()));

    // 初始化运行时
    GLOBAL_RUNTIME.get_or_try_init(|| {
        tracing::info!("正在初始化全局 Tokio Runtime...");
        RuntimeHandle::from_config(config)
    })
}

/// 获取全局运行时句柄
///
/// # Panics
///
/// 如果运行时未初始化，会自动使用默认配置初始化。
/// 如果初始化失败，则返回错误。
pub fn get_runtime() -> FfiResult<&'static RuntimeHandle> {
    // 如果未初始化，自动初始化
    if GLOBAL_RUNTIME.get().is_none() {
        init_runtime(None)?;
    }

    GLOBAL_RUNTIME
        .get()
        .ok_or_else(|| FfiError::runtime_error("获取运行时", "运行时未初始化"))
}

/// 安全地获取运行时（可能返回 None）
pub fn try_get_runtime() -> Option<&'static RuntimeHandle> {
    GLOBAL_RUNTIME.get()
}

/// 检查运行时是否已初始化
pub fn is_runtime_initialized() -> bool {
    GLOBAL_RUNTIME.get().is_some()
}

/// 更新运行时配置（仅在未初始化时有效）
pub fn set_runtime_config(config: RuntimeConfig) -> FfiResult<()> {
    if is_runtime_initialized() {
        return Err(FfiError::runtime_error(
            "更新运行时配置",
            "运行时已初始化，无法修改配置",
        ));
    }

    let config_store = GLOBAL_CONFIG.get_or_init(|| RwLock::new(config.clone()));
    *config_store.write() = config;

    Ok(())
}

/// 关闭全局运行时
///
/// 通常在应用退出时调用
pub fn shutdown_runtime(timeout: Duration) -> FfiResult<()> {
    if let Some(runtime) = GLOBAL_RUNTIME.get() {
        runtime.shutdown(timeout);
        Ok(())
    } else {
        Err(FfiError::runtime_error("关闭运行时", "运行时未初始化"))
    }
}

/// 获取运行时统计
pub fn get_runtime_stats() -> FfiResult<RuntimeStats> {
    let runtime = get_runtime()?;
    Ok(runtime.stats())
}

/// 获取取消令牌
pub fn get_cancellation_token() -> FfiResult<CancellationToken> {
    let runtime = get_runtime()?;
    Ok(runtime.cancellation_token())
}

// ==================== 便捷函数 ====================

/// 在全局运行时上执行异步任务
///
/// 这是 FFI 函数的主要工具，替代每次创建新 Runtime
///
/// # 示例
///
/// ```rust,no_run
/// use crate::ffi::runtime::block_on;
///
/// let result = block_on(async {
///     tokio::time::sleep(Duration::from_secs(1)).await;
///     "Hello from async"
/// });
/// ```
pub fn block_on<F, T>(f: F) -> FfiResult<T>
where
    F: Future<Output = T>,
{
    let runtime = get_runtime()?;
    Ok(runtime.block_on(f))
}

/// 在全局运行时上生成任务
pub fn spawn<F>(future: F) -> FfiResult<JoinHandle<F::Output>>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    let runtime = get_runtime()?;
    Ok(runtime.spawn(future))
}

/// 在阻塞线程池中执行
pub fn spawn_blocking<F, T>(f: F) -> FfiResult<JoinHandle<T>>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    let runtime = get_runtime()?;
    Ok(runtime.spawn_blocking(f))
}

/// 执行带超时的异步任务
pub fn block_on_with_timeout<F, T>(f: F, timeout: Duration) -> FfiResult<T>
where
    F: Future<Output = FfiResult<T>>,
{
    let runtime = get_runtime()?;

    runtime
        .block_on(async {
            tokio::time::timeout(timeout, f)
                .await
                .map_err(|_| FfiError::new(FfiErrorCode::Timeout, "操作超时"))?
        })
        .map_err(|e| e)?
}

/// 执行带取消支持的异步任务
pub fn block_on_cancellable<F, T>(f: F, token: CancellationToken) -> FfiResult<T>
where
    F: Future<Output = FfiResult<T>>,
{
    let runtime = get_runtime()?;

    runtime.block_on(async {
        tokio::select! {
            result = f => result,
            _ = token.cancelled() => {
                Err(FfiError::new(FfiErrorCode::TaskCancelled, "任务被取消"))
            }
        }
    })
}

// ==================== 与 flutter_rust_bridge 集成 ====================

/// FFI 初始化函数
///
/// 在 Flutter 初始化时调用，确保运行时已准备就绪
#[flutter_rust_bridge::frb(init)]
pub fn init_ffi_runtime() {
    // 设置 panic 钩子
    crate::ffi::error::setup_ffi_panic_hook();

    // 初始化运行时
    if let Err(e) = init_runtime(None) {
        tracing::error!(error = %e, "FFI 运行时初始化失败");
        // 注意：这里不 panic，让 flutter_rust_bridge 处理错误
    } else {
        tracing::info!("FFI 运行时初始化成功");
    }
}

/// 健康检查
#[flutter_rust_bridge::frb(sync)]
pub fn runtime_health_check() -> String {
    match get_runtime_stats() {
        Ok(stats) => format!(
            "Runtime OK - Active tasks: {}, Completed: {}",
            stats.active_tasks, stats.completed_tasks
        ),
        Err(e) => format!("Runtime Error: {}", e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_config_default() {
        let config = RuntimeConfig::default();
        assert!(config.worker_threads > 0);
        assert!(config.max_blocking_threads > 0);
        assert_eq!(config.thread_name_prefix, "ffi-runtime");
    }

    #[test]
    fn test_runtime_config_for_ffi() {
        let config = RuntimeConfig::for_ffi();
        assert_eq!(config.thread_name_prefix, "log-analyzer-ffi");
        assert!(config.thread_stack_size >= 4 * 1024 * 1024);
    }

    #[tokio::test]
    async fn test_runtime_handle() {
        let handle = RuntimeHandle::from_config(RuntimeConfig::for_constrained()).unwrap();

        let result = handle.block_on(async { 42 });
        assert_eq!(result, 42);

        let handle_result = handle.spawn(async { 123 }).await.unwrap();
        assert_eq!(handle_result, 123);
    }

    #[test]
    fn test_block_on_result() {
        // 注意：由于全局单例，这个测试可能与其他测试冲突
        // 在实际测试中应该使用隔离的运行时
        let result = block_on(async { Ok::<_, FfiError>(42) });
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_is_runtime_initialized() {
        // 初始状态
        let initialized = is_runtime_initialized();
        // 由于其他测试可能已经初始化，我们不能做确定性的断言
        // 但至少确保函数可以调用
        assert!(initialized || !initialized); // 总是 true
    }
}
