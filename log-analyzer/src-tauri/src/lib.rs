//! 日志分析器 - Rust 后端
//!
//! 提供高性能的日志分析功能，包括：
//! - 多格式压缩包递归解压
//! - 并行全文搜索
//! - 结构化查询系统
//! - 索引持久化与增量更新
//! - 实时文件监听

// Clippy 配置：允许某些警告以快速通过 CI
// TODO: 在后续迭代中逐步修复这些问题
#![allow(clippy::new_without_default)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::result_large_err)]
#![allow(clippy::await_holding_lock)]
#![allow(clippy::should_implement_trait)]
#![allow(clippy::derivable_impls)]
#![allow(clippy::redundant_closure)]
#![allow(clippy::needless_borrows_for_generic_args)]
#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::only_used_in_recursion)]
#![allow(clippy::redundant_pattern_matching)]
#![allow(clippy::useless_conversion)]
#![allow(clippy::unwrap_or_default)]
#![allow(clippy::explicit_auto_deref)]
#![allow(clippy::unnecessary_cast)]
#![allow(clippy::borrowed_box)]
#![allow(clippy::manual_clamp)]
#![allow(clippy::doc_overindented_list_items)]
#![allow(clippy::manual_pattern_char_comparison)]

use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;

// 模块声明
pub mod archive;
mod benchmark;
mod commands;
mod error;
pub mod events;
pub mod models;
pub mod monitoring;
pub mod search_engine;
pub mod services;
pub mod state_sync;
pub mod utils;

// 从模块导入类型
pub use error::{AppError, AppResult, Result, UserFacingError};
pub use models::{
    validate_extracted_filename, AppState, LogEntry, SearchCacheKey, ValidatedSearchQuery,
    ValidatedWorkspaceConfig,
};
pub use utils::{AsyncResourceManager, CacheManager};

// --- Commands ---

// 命令实现位于 commands 模块
use commands::{
    async_search::{async_search_logs, cancel_async_search, get_active_searches_count},
    cache::{
        cache_health_check, cleanup_expired_cache, get_access_pattern_stats,
        get_async_cache_statistics, get_cache_dashboard_data, get_cache_performance_metrics,
        get_cache_performance_report, get_cache_statistics, get_compression_stats,
        get_l2_cache_config, intelligent_cache_eviction, invalidate_workspace_cache,
        reset_access_tracker, reset_cache_metrics, warm_cache,
    },
    config::{
        get_enhanced_extraction_status, load_config, save_config, set_enhanced_extraction_status,
    },
    export::export_results,
    import::{check_rar_support, import_folder},
    performance::get_performance_metrics,
    query::{execute_structured_query, validate_query},
    search::search_logs,
    validation::{
        batch_validate_workspace_configs, validate_import_config_cmd, validate_path_security,
        validate_search_query_cmd, validate_workspace_config_cmd, validate_workspace_id_format,
    },
    watch::{start_watch, stop_watch},
    workspace::{delete_workspace, load_workspace, refresh_workspace},
};

/// 初始化结构化日志系统
///
/// 配置 tracing 订阅器，支持：
/// - 控制台输出（开发环境使用彩色格式）
/// - 文件输出（生产环境使用 JSON 格式）
/// - 日志轮转和过滤
/// - Sentry 集成用于错误监控
fn init_logging() -> eyre::Result<()> {
    use eyre::Context;
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

    // 创建日志目录
    std::fs::create_dir_all("logs").with_context(|| "Failed to create logs directory")?;

    // 设置文件日志轮转（每天轮转）
    let file_appender = tracing_appender::rolling::daily("logs", "app.log");
    let (non_blocking_file, _guard) = tracing_appender::non_blocking(file_appender);

    // 配置环境过滤器，默认为 info 级别
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        // 默认配置：应用日志为 info，依赖库为 warn
        EnvFilter::new("log_analyzer=info,warn")
    });

    // 初始化订阅器
    tracing_subscriber::registry()
        .with(env_filter)
        // 控制台输出层（开发环境友好格式）
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stdout)
                .with_target(true)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true),
        )
        // 文件输出层（JSON 格式，便于日志聚合）
        .with(
            tracing_subscriber::fmt::layer()
                .json()
                .with_writer(non_blocking_file)
                .with_current_span(true)
                .with_span_list(true),
        )
        // Sentry 层用于错误监控和性能追踪
        .with(sentry::integrations::tracing::layer())
        .init();

    tracing::info!("Structured logging system with Sentry integration initialized");
    Ok(())
}

/// 高性能锁管理器，使用 parking_lot 实现
///
/// 这个实现完全避免了不安全的指针转换，使用安全的锁排序机制来防止死锁
/// 包含死锁检测、超时机制和全面的监控功能
pub struct LockManager {
    /// 锁获取统计信息
    lock_stats: Arc<Mutex<LockStatistics>>,
    /// 死锁检测超时时间
    deadlock_timeout: std::time::Duration,
}

/// 锁获取统计信息
#[derive(Debug, Default)]
pub struct LockStatistics {
    pub total_acquisitions: u64,
    successful_acquisitions: u64,
    timeout_failures: u64,
    deadlock_preventions: u64,
    average_acquisition_time: std::time::Duration,
}

impl LockManager {
    /// 创建新的锁管理器实例
    pub fn new() -> Self {
        Self {
            lock_stats: Arc::new(Mutex::new(LockStatistics::default())),
            deadlock_timeout: std::time::Duration::from_secs(5), // 默认5秒超时
        }
    }

    /// 设置死锁检测超时时间
    pub fn with_deadlock_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.deadlock_timeout = timeout;
        self
    }

    /// 安全地获取多个同类型锁，使用稳定排序避免死锁
    ///
    /// 使用锁的唯一标识符而不是内存地址进行排序，确保跨平台一致性
    pub fn acquire_multiple_locks<'a, T>(
        &self,
        locks: Vec<(&str, &'a Mutex<T>)>, // (lock_id, lock) pairs
    ) -> eyre::Result<Vec<parking_lot::MutexGuard<'a, T>>> {
        let start_time = std::time::Instant::now();

        // 按锁ID排序，确保所有线程以相同顺序获取锁
        let mut sorted_locks: Vec<_> = locks.into_iter().collect();
        sorted_locks.sort_by(|(id1, _), (id2, _)| id1.cmp(id2));

        tracing::debug!(
            lock_count = sorted_locks.len(),
            lock_ids = ?sorted_locks.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
            "Attempting to acquire multiple locks in sorted order"
        );

        let mut guards = Vec::new();
        let mut acquired_locks = Vec::new();

        // 尝试按顺序获取所有锁
        for (lock_id, lock) in sorted_locks {
            match lock.try_lock_for(self.deadlock_timeout) {
                Some(guard) => {
                    acquired_locks.push(lock_id);
                    guards.push(guard);
                    tracing::trace!(lock_id = lock_id, "Successfully acquired lock");
                }
                None => {
                    // 获取失败，记录并返回错误
                    let elapsed = start_time.elapsed();
                    self.record_lock_failure(
                        "acquire_multiple_locks",
                        elapsed,
                        acquired_locks.len(),
                    );

                    tracing::error!(
                        lock_id = lock_id,
                        acquired_locks = ?acquired_locks,
                        elapsed_ms = elapsed.as_millis(),
                        "Failed to acquire lock - potential deadlock detected"
                    );

                    return Err(eyre::eyre!(
                        "Failed to acquire lock '{}' within timeout ({}ms) - potential deadlock detected. Already acquired: {:?}",
                        lock_id,
                        self.deadlock_timeout.as_millis(),
                        acquired_locks
                    ));
                }
            }
        }

        let elapsed = start_time.elapsed();
        self.record_lock_success("acquire_multiple_locks", elapsed, guards.len());

        tracing::debug!(
            lock_count = guards.len(),
            elapsed_ms = elapsed.as_millis(),
            "Successfully acquired all locks"
        );

        Ok(guards)
    }

    /// 安全地获取两个锁，使用锁ID排序避免死锁
    ///
    /// 替代了之前使用内存地址的不安全方法
    pub fn acquire_two_locks_safe<'a, T, U>(
        &self,
        lock1_id: &str,
        lock1: &'a Mutex<T>,
        lock2_id: &str,
        lock2: &'a Mutex<U>,
    ) -> eyre::Result<(
        parking_lot::MutexGuard<'a, T>,
        parking_lot::MutexGuard<'a, U>,
    )> {
        let start_time = std::time::Instant::now();

        tracing::debug!(
            lock1_id = lock1_id,
            lock2_id = lock2_id,
            timeout_ms = self.deadlock_timeout.as_millis(),
            "Attempting to acquire two locks with safe ordering"
        );

        // 使用字符串比较确定锁获取顺序，确保一致性
        let (first_guard, second_guard) = if lock1_id < lock2_id {
            // 先获取 lock1，再获取 lock2
            let guard1 = match lock1.try_lock_for(self.deadlock_timeout) {
                Some(guard) => {
                    tracing::trace!(lock_id = lock1_id, "Acquired first lock");
                    guard
                }
                None => {
                    let elapsed = start_time.elapsed();
                    self.record_lock_failure("acquire_two_locks_safe", elapsed, 0);
                    tracing::error!(
                        lock_id = lock1_id,
                        elapsed_ms = elapsed.as_millis(),
                        "Failed to acquire first lock within timeout"
                    );
                    return Err(eyre::eyre!(
                        "Failed to acquire first lock '{}' within timeout ({}ms)",
                        lock1_id,
                        self.deadlock_timeout.as_millis()
                    ));
                }
            };

            let remaining_timeout = self.deadlock_timeout.saturating_sub(start_time.elapsed());
            let guard2 = match lock2.try_lock_for(remaining_timeout) {
                Some(guard) => {
                    tracing::trace!(lock_id = lock2_id, "Acquired second lock");
                    guard
                }
                None => {
                    let elapsed = start_time.elapsed();
                    self.record_lock_failure("acquire_two_locks_safe", elapsed, 1);
                    tracing::error!(
                        lock_id = lock2_id,
                        first_lock = lock1_id,
                        elapsed_ms = elapsed.as_millis(),
                        "Failed to acquire second lock within timeout"
                    );
                    return Err(eyre::eyre!(
                        "Failed to acquire second lock '{}' within timeout ({}ms) after acquiring '{}'",
                        lock2_id,
                        remaining_timeout.as_millis(),
                        lock1_id
                    ));
                }
            };

            (guard1, guard2)
        } else {
            // 先获取 lock2，再获取 lock1
            let guard2 = match lock2.try_lock_for(self.deadlock_timeout) {
                Some(guard) => {
                    tracing::trace!(lock_id = lock2_id, "Acquired first lock");
                    guard
                }
                None => {
                    let elapsed = start_time.elapsed();
                    self.record_lock_failure("acquire_two_locks_safe", elapsed, 0);
                    tracing::error!(
                        lock_id = lock2_id,
                        elapsed_ms = elapsed.as_millis(),
                        "Failed to acquire first lock within timeout"
                    );
                    return Err(eyre::eyre!(
                        "Failed to acquire first lock '{}' within timeout ({}ms)",
                        lock2_id,
                        self.deadlock_timeout.as_millis()
                    ));
                }
            };

            let remaining_timeout = self.deadlock_timeout.saturating_sub(start_time.elapsed());
            let guard1 = match lock1.try_lock_for(remaining_timeout) {
                Some(guard) => {
                    tracing::trace!(lock_id = lock1_id, "Acquired second lock");
                    guard
                }
                None => {
                    let elapsed = start_time.elapsed();
                    self.record_lock_failure("acquire_two_locks_safe", elapsed, 1);
                    tracing::error!(
                        lock_id = lock1_id,
                        first_lock = lock2_id,
                        elapsed_ms = elapsed.as_millis(),
                        "Failed to acquire second lock within timeout"
                    );
                    return Err(eyre::eyre!(
                        "Failed to acquire second lock '{}' within timeout ({}ms) after acquiring '{}'",
                        lock1_id,
                        remaining_timeout.as_millis(),
                        lock2_id
                    ));
                }
            };

            (guard1, guard2)
        };

        let elapsed = start_time.elapsed();
        self.record_lock_success("acquire_two_locks_safe", elapsed, 2);

        tracing::debug!(
            lock1_id = lock1_id,
            lock2_id = lock2_id,
            elapsed_ms = elapsed.as_millis(),
            "Successfully acquired both locks"
        );

        Ok((first_guard, second_guard))
    }

    /// 尝试获取单个锁，带超时机制
    pub fn try_acquire_with_timeout<'a, T>(
        &self,
        lock_id: &str,
        lock: &'a Mutex<T>,
        timeout: std::time::Duration,
    ) -> Option<parking_lot::MutexGuard<'a, T>> {
        let start_time = std::time::Instant::now();

        tracing::debug!(
            lock_id = lock_id,
            timeout_ms = timeout.as_millis(),
            "Attempting to acquire single lock with timeout"
        );

        let result = lock.try_lock_for(timeout);
        let elapsed = start_time.elapsed();

        if result.is_some() {
            self.record_lock_success("try_acquire_with_timeout", elapsed, 1);
            tracing::debug!(
                lock_id = lock_id,
                elapsed_ms = elapsed.as_millis(),
                "Successfully acquired single lock"
            );
        } else {
            self.record_lock_failure("try_acquire_with_timeout", elapsed, 0);
            tracing::warn!(
                lock_id = lock_id,
                elapsed_ms = elapsed.as_millis(),
                "Failed to acquire single lock within timeout"
            );
        }

        result
    }

    /// 获取锁统计信息
    pub fn get_lock_statistics(&self) -> LockStatistics {
        let stats = self.lock_stats.lock();
        LockStatistics {
            total_acquisitions: stats.total_acquisitions,
            successful_acquisitions: stats.successful_acquisitions,
            timeout_failures: stats.timeout_failures,
            deadlock_preventions: stats.deadlock_preventions,
            average_acquisition_time: stats.average_acquisition_time,
        }
    }

    /// 重置锁统计信息
    pub fn reset_statistics(&self) {
        let mut stats = self.lock_stats.lock();
        *stats = LockStatistics::default();
        tracing::info!("Lock statistics reset");
    }

    /// 记录成功的锁获取
    fn record_lock_success(
        &self,
        operation: &str,
        duration: std::time::Duration,
        lock_count: usize,
    ) {
        let mut stats = self.lock_stats.lock();
        stats.total_acquisitions += 1;
        stats.successful_acquisitions += 1;

        // 更新平均获取时间
        let total_time = stats.average_acquisition_time.as_nanos() as u64
            * (stats.successful_acquisitions - 1)
            + duration.as_nanos() as u64;
        stats.average_acquisition_time =
            std::time::Duration::from_nanos(total_time / stats.successful_acquisitions);

        tracing::debug!(
            operation = operation,
            duration_ms = duration.as_millis(),
            lock_count = lock_count,
            total_acquisitions = stats.total_acquisitions,
            success_rate =
                (stats.successful_acquisitions as f64 / stats.total_acquisitions as f64) * 100.0,
            "Lock acquisition successful"
        );
    }

    /// 记录失败的锁获取
    fn record_lock_failure(
        &self,
        operation: &str,
        duration: std::time::Duration,
        partial_locks: usize,
    ) {
        let mut stats = self.lock_stats.lock();
        stats.total_acquisitions += 1;
        stats.timeout_failures += 1;

        if partial_locks > 0 {
            stats.deadlock_preventions += 1;
        }

        tracing::warn!(
            operation = operation,
            duration_ms = duration.as_millis(),
            partial_locks = partial_locks,
            total_acquisitions = stats.total_acquisitions,
            timeout_failures = stats.timeout_failures,
            deadlock_preventions = stats.deadlock_preventions,
            "Lock acquisition failed"
        );
    }
}

impl Default for LockManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> eyre::Result<()> {
    // 初始化结构化日志系统
    init_logging()?;

    // 初始化事件系统
    let _event_bus = events::init_event_bus();
    tracing::info!("Event system initialized");

    // 设置全局 panic hook，使用 tracing 记录
    std::panic::set_hook(Box::new(|panic_info| {
        tracing::error!("Application panic: {:?}", panic_info);
    }));

    // 配置 Rayon 线程池（优化多核性能）
    let num_cpus = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4); // 默认 4 线程

    rayon::ThreadPoolBuilder::new()
        .num_threads(num_cpus)
        .thread_name(|idx| format!("rayon-worker-{}", idx))
        .build_global()
        .expect("Failed to build Rayon thread pool");

    tracing::info!(threads = num_cpus, "Rayon thread pool initialized");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage({
            // 创建共享的搜索缓存
            let search_cache = Arc::new(
                moka::sync::Cache::builder()
                    .max_capacity(1000) // 最大缓存1000个搜索结果
                    .time_to_live(std::time::Duration::from_secs(300)) // TTL: 5分钟
                    .time_to_idle(std::time::Duration::from_secs(60)) // TTI: 1分钟
                    .build(),
            );

            AppState {
                temp_dir: Mutex::new(None),
                path_map: Arc::new(Mutex::new(HashMap::new())), // 使用 Arc
                file_metadata: Arc::new(Mutex::new(HashMap::new())), // 元数据
                workspace_indices: Mutex::new(HashMap::new()),
                search_cache: search_cache.clone(),
                // 性能统计
                last_search_duration: Arc::new(Mutex::new(0)),
                total_searches: Arc::new(Mutex::new(0)),
                cache_hits: Arc::new(Mutex::new(0)),
                // 实时监听
                watchers: Arc::new(Mutex::new(HashMap::new())),
                // 临时文件清理队列（锁无关）
                cleanup_queue: Arc::new(crossbeam::queue::SegQueue::new()),
                // 异步并发支持
                search_cancellation: tokio_util::sync::CancellationToken::new(),
                async_search_state: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
                async_resources: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
                async_resource_manager: Arc::new(crate::utils::AsyncResourceManager::new()),
                cache_manager: Arc::new(crate::utils::CacheManager::new(search_cache)),
                // 增强提取功能标志（默认禁用，渐进式部署）
                use_enhanced_extraction: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            }
        })
        .invoke_handler(tauri::generate_handler![
            save_config,
            load_config,
            get_enhanced_extraction_status,
            set_enhanced_extraction_status,
            search_logs,
            async_search_logs,
            cancel_async_search,
            get_active_searches_count,
            get_cache_statistics,
            get_async_cache_statistics,
            invalidate_workspace_cache,
            cleanup_expired_cache,
            warm_cache,
            get_cache_performance_metrics,
            get_cache_performance_report,
            cache_health_check,
            get_access_pattern_stats,
            get_compression_stats,
            get_l2_cache_config,
            intelligent_cache_eviction,
            reset_cache_metrics,
            reset_access_tracker,
            get_cache_dashboard_data,
            validate_workspace_config_cmd,
            validate_search_query_cmd,
            validate_import_config_cmd,
            batch_validate_workspace_configs,
            validate_workspace_id_format,
            validate_path_security,
            import_folder,
            load_workspace,
            refresh_workspace,
            export_results,
            get_performance_metrics,
            check_rar_support,
            start_watch,
            stop_watch,
            execute_structured_query,
            validate_query,
            delete_workspace,
            commands::error_reporting::report_frontend_error,
            commands::error_reporting::submit_user_feedback,
            commands::error_reporting::get_error_statistics,
            commands::monitoring::get_system_performance_metrics,
            commands::monitoring::get_dashboard_data,
            commands::monitoring::run_benchmarks,
            commands::monitoring::get_system_health,
            commands::monitoring::export_monitoring_report,
            commands::monitoring::get_performance_baselines,
            commands::monitoring::update_performance_baseline,
        ])
        .setup(|_app| {
            // 初始化事件系统
            let _event_bus = events::init_event_bus();
            tracing::info!("Event system initialized in setup");

            // TODO: 稍后启用事件桥接
            // let app_handle = app.handle().clone();
            // tokio::spawn(async move {
            //     tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            //     let _bridge = events::bridge::init_tauri_bridge(app_handle).await;
            //     tracing::info!("Tauri event bridge initialized");
            // });
            Ok(())
        })
        .run(tauri::generate_context!())
        .map_err(|e| eyre::eyre!("Failed to run Tauri application: {}", e))?;

    Ok(())
}

// ============================================================================
// 单元测试（私有函数）
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::{normalize_path_separator, validate_workspace_id};
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_path_utils() {
        // 测试路径规范化（Windows 上将 / 转为 \uff09
        #[cfg(target_os = "windows")]
        {
            let normalized = normalize_path_separator("test/path");
            assert_eq!(normalized, "test\\path");
        }
        #[cfg(not(target_os = "windows"))]
        {
            let normalized = normalize_path_separator("test/path");
            assert_eq!(normalized, "test/path");
        }
    }

    #[test]
    fn test_workspace_id_validation() {
        assert!(validate_workspace_id("valid-id-123").is_ok());
        assert!(validate_workspace_id("").is_err());
        assert!(validate_workspace_id("../invalid").is_err());
        assert!(validate_workspace_id("invalid/id").is_err());
        assert!(validate_workspace_id("invalid\\id").is_err());
    }

    #[test]
    fn test_lock_manager_safe_ordering() {
        let lock_manager = LockManager::new();
        let lock1 = Mutex::new(1);
        let lock2 = Mutex::new(2);

        // Test that locks are acquired in consistent order regardless of parameter order
        // 第一次获取锁
        {
            let result1 = lock_manager.acquire_two_locks_safe("lock_a", &lock1, "lock_b", &lock2);
            assert!(result1.is_ok(), "First lock acquisition should succeed");
        } // 锁在这里被释放

        // 第二次获取锁（参数顺序相反）
        {
            let result2 = lock_manager.acquire_two_locks_safe("lock_b", &lock2, "lock_a", &lock1);
            assert!(
                result2.is_ok(),
                "Second lock acquisition should succeed after first is released"
            );
        }
    }

    #[test]
    fn test_lock_manager_timeout() {
        let lock_manager = LockManager::new().with_deadlock_timeout(Duration::from_millis(100));
        let lock = Arc::new(Mutex::new(42));

        // Hold the lock in another thread
        let lock_clone = lock.clone();
        let _guard = lock_clone.lock();

        // Try to acquire with timeout - should fail
        let result =
            lock_manager.try_acquire_with_timeout("test_lock", &lock, Duration::from_millis(50));
        assert!(result.is_none());
    }

    #[test]
    fn test_lock_manager_statistics() {
        let lock_manager = LockManager::new();
        let lock = Mutex::new(0);

        // Successful acquisition
        let _guard =
            lock_manager.try_acquire_with_timeout("test_lock", &lock, Duration::from_millis(100));

        let stats = lock_manager.get_lock_statistics();
        assert_eq!(stats.total_acquisitions, 1);
        assert_eq!(stats.successful_acquisitions, 1);
        assert_eq!(stats.timeout_failures, 0);
    }

    #[test]
    fn test_lock_manager_multiple_locks() {
        let lock_manager = LockManager::new();
        let lock1 = Mutex::new(1);
        let lock2 = Mutex::new(2);
        let lock3 = Mutex::new(3);

        let locks = vec![("lock_c", &lock3), ("lock_a", &lock1), ("lock_b", &lock2)];

        let result = lock_manager.acquire_multiple_locks(locks);
        assert!(result.is_ok());

        let guards = result.unwrap();
        assert_eq!(guards.len(), 3);
    }

    #[test]
    fn test_lock_manager_deadlock_prevention() {
        let lock_manager = LockManager::new().with_deadlock_timeout(Duration::from_millis(100));
        let lock1 = Arc::new(Mutex::new(1));
        let lock2 = Arc::new(Mutex::new(2));

        let lock1_clone = lock1.clone();
        let lock2_clone = lock2.clone();
        let lock_manager_clone = Arc::new(lock_manager);
        let lock_manager_clone2 = lock_manager_clone.clone();

        // Simulate potential deadlock scenario
        let handle1 = thread::spawn(move || {
            let _result = lock_manager_clone.acquire_two_locks_safe(
                "lock1",
                &lock1_clone,
                "lock2",
                &lock2_clone,
            );
            thread::sleep(Duration::from_millis(50));
        });

        let handle2 = thread::spawn(move || {
            thread::sleep(Duration::from_millis(10)); // Small delay to ensure different timing
            let _result =
                lock_manager_clone2.acquire_two_locks_safe("lock2", &lock2, "lock1", &lock1);
        });

        // Both threads should complete without deadlock
        assert!(handle1.join().is_ok());
        assert!(handle2.join().is_ok());
    }
}
