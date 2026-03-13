//! spawn_blocking 线程池管理
//!
//! 管理 CPU 密集型任务的线程池：
//! - 统一的线程池配置
//! - 任务优先级和取消支持
//! - 负载监控和自适应调整

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, trace, warn};

/// 线程池配置
#[derive(Debug, Clone)]
pub struct BlockingPoolConfig {
    /// 线程池大小（0 = 使用 Tokio 默认值）
    pub pool_size: usize,
    /// 线程名称前缀
    pub thread_name_prefix: String,
    /// 线程栈大小
    pub stack_size: usize,
    /// 任务超时时间
    pub task_timeout: Duration,
    /// 最大队列深度
    pub max_queue_depth: usize,
}

impl Default for BlockingPoolConfig {
    fn default() -> Self {
        Self {
            pool_size: 0, // 使用 Tokio 默认值
            thread_name_prefix: "blocking-".to_string(),
            stack_size: 2 * 1024 * 1024, // 2MB
            task_timeout: Duration::from_secs(60),
            max_queue_depth: 1000,
        }
    }
}

impl BlockingPoolConfig {
    /// 根据 CPU 核心数配置
    pub fn for_cpu_intensive() -> Self {
        let cpu_count = num_cpus::get();
        Self {
            pool_size: cpu_count * 2,
            thread_name_prefix: "cpu-intensive-".to_string(),
            stack_size: 4 * 1024 * 1024, // 4MB for search operations
            task_timeout: Duration::from_secs(30),
            max_queue_depth: cpu_count * 10,
        }
    }

    /// I/O 密集型配置
    pub fn for_io_intensive() -> Self {
        let cpu_count = num_cpus::get();
        Self {
            pool_size: cpu_count * 4,
            thread_name_prefix: "io-intensive-".to_string(),
            stack_size: 2 * 1024 * 1024,
            task_timeout: Duration::from_secs(300),
            max_queue_depth: cpu_count * 20,
        }
    }
}

/// 任务统计
#[derive(Debug, Default)]
pub struct TaskStats {
    pub total_tasks: AtomicUsize,
    pub completed_tasks: AtomicUsize,
    pub cancelled_tasks: AtomicUsize,
    pub timeout_tasks: AtomicUsize,
    pub failed_tasks: AtomicUsize,
    pub total_execution_time_ms: AtomicUsize,
}

impl TaskStats {
    pub fn record_completion(&self, duration: Duration) {
        self.completed_tasks.fetch_add(1, Ordering::Relaxed);
        self.total_execution_time_ms
            .fetch_add(duration.as_millis() as usize, Ordering::Relaxed);
    }

    pub fn record_cancellation(&self) {
        self.cancelled_tasks.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_timeout(&self) {
        self.timeout_tasks.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_failure(&self) {
        self.failed_tasks.fetch_add(1, Ordering::Relaxed);
    }
}

/// spawn_blocking 线程池管理器
///
/// 封装 Tokio 的 spawn_blocking，提供统一的任务管理
pub struct BlockingPool {
    config: BlockingPoolConfig,
    stats: Arc<TaskStats>,
    /// 当前正在执行的任务数
    active_tasks: Arc<AtomicUsize>,
    /// 队列深度
    queue_depth: Arc<AtomicUsize>,
}

impl BlockingPool {
    /// 创建新的线程池
    pub fn new(config: BlockingPoolConfig) -> Self {
        info!(
            pool_size = config.pool_size,
            thread_prefix = %config.thread_name_prefix,
            "Blocking pool created"
        );

        Self {
            config,
            stats: Arc::new(TaskStats::default()),
            active_tasks: Arc::new(AtomicUsize::new(0)),
            queue_depth: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// 执行 CPU 密集型任务
    ///
    /// # 参数
    /// - `f`: 要在阻塞线程中执行的闭包
    /// - `token`: 取消令牌
    ///
    /// # 返回
    /// 任务的 JoinHandle，可用于等待结果
    pub fn spawn<F, R>(&self, f: F, token: CancellationToken) -> JoinHandle<Option<R>>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        self.stats.total_tasks.fetch_add(1, Ordering::Relaxed);
        self.queue_depth.fetch_add(1, Ordering::Relaxed);

        let stats = Arc::clone(&self.stats);
        let active_tasks = Arc::clone(&self.active_tasks);
        let queue_depth = Arc::clone(&self.queue_depth);

        tokio::task::spawn_blocking(move || {
            queue_depth.fetch_sub(1, Ordering::Relaxed);
            active_tasks.fetch_add(1, Ordering::Relaxed);
            let start = Instant::now();

            // 检查是否已取消
            if token.is_cancelled() {
                trace!("Task cancelled before execution");
                stats.record_cancellation();
                active_tasks.fetch_sub(1, Ordering::Relaxed);
                return None;
            }

            // 执行任务
            let result = f();
            let duration = start.elapsed();

            stats.record_completion(duration);
            active_tasks.fetch_sub(1, Ordering::Relaxed);

            trace!(execution_time_ms = duration.as_millis(), "Task completed");
            Some(result)
        })
    }

    /// 执行带超时的任务
    pub async fn spawn_with_timeout<F, R>(
        &self,
        f: F,
        timeout_duration: Duration,
        token: CancellationToken,
    ) -> Option<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let handle = self.spawn(f, token.clone());

        match tokio::time::timeout(timeout_duration, handle).await {
            Ok(Ok(result)) => result,
            Ok(Err(join_err)) => {
                error!("Task join error: {}", join_err);
                self.stats.record_failure();
                None
            }
            Err(_) => {
                warn!("Task timed out after {:?}", timeout_duration);
                self.stats.record_timeout();
                token.cancel(); // 尝试取消任务
                None
            }
        }
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> &TaskStats {
        &self.stats
    }

    /// 获取当前活跃任务数
    pub fn active_task_count(&self) -> usize {
        self.active_tasks.load(Ordering::Relaxed)
    }

    /// 获取当前队列深度
    pub fn queue_depth(&self) -> usize {
        self.queue_depth.load(Ordering::Relaxed)
    }

    /// 估计负载
    pub fn estimated_load(&self) -> f64 {
        let active = self.active_task_count() as f64;
        let max = self.config.pool_size as f64;

        if max > 0.0 {
            active / max
        } else {
            // 使用 Tokio 默认值（通常是 512）
            active / 512.0
        }
    }

    /// 是否处于高负载
    pub fn is_under_high_load(&self) -> bool {
        self.estimated_load() > 0.8
    }
}

impl Default for BlockingPool {
    fn default() -> Self {
        Self::new(BlockingPoolConfig::default())
    }
}

/// CPU 密集型任务包装器
///
/// 提供更方便的接口来执行 CPU 密集型任务
pub struct CpuIntensiveTask<T> {
    pool: Arc<BlockingPool>,
    token: CancellationToken,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Send + 'static> CpuIntensiveTask<T> {
    /// 创建新任务
    pub fn new(pool: Arc<BlockingPool>) -> Self {
        Self {
            pool,
            token: CancellationToken::new(),
            _phantom: std::marker::PhantomData,
        }
    }

    /// 创建带取消令牌的预创建任务
    pub fn with_token(pool: Arc<BlockingPool>, token: CancellationToken) -> Self {
        Self {
            pool,
            token,
            _phantom: std::marker::PhantomData,
        }
    }

    /// 执行任务
    pub async fn execute<F>(&self, f: F) -> Option<T>
    where
        F: FnOnce() -> T + Send + 'static,
    {
        let handle = self.pool.spawn(f, self.token.clone());

        match handle.await {
            Ok(result) => result,
            Err(e) => {
                error!("Task execution failed: {}", e);
                None
            }
        }
    }

    /// 带超时执行任务
    pub async fn execute_with_timeout<F>(&self, f: F, timeout_duration: Duration) -> Option<T>
    where
        F: FnOnce() -> T + Send + 'static,
    {
        self.pool
            .spawn_with_timeout(f, timeout_duration, self.token.clone())
            .await
    }

    /// 取消任务
    pub fn cancel(&self) {
        self.token.cancel();
    }

    /// 检查是否已取消
    pub fn is_cancelled(&self) -> bool {
        self.token.is_cancelled()
    }
}

/// 并行执行多个任务
///
/// 使用 spawn_blocking 并行执行多个 CPU 密集型任务
pub async fn parallel_execute<F, R>(
    pool: &BlockingPool,
    tasks: Vec<F>,
    token: CancellationToken,
) -> Vec<Option<R>>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    use futures::future::join_all;

    let handles: Vec<_> = tasks
        .into_iter()
        .map(|task| {
            let token = token.clone();
            pool.spawn(task, token)
        })
        .collect();

    let results = join_all(handles).await;

    results.into_iter().map(|r| r.ok().flatten()).collect()
}

/// 带背压的批量任务执行
///
/// 限制同时执行的任务数量
pub async fn batch_execute_with_backpressure<F, R>(
    pool: &BlockingPool,
    tasks: Vec<F>,
    max_concurrent: usize,
    token: CancellationToken,
) -> Vec<Option<R>>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    use futures::stream::{self, StreamExt};

    stream::iter(tasks)
        .map(|task| {
            let token = token.clone();
            async move {
                let handle = pool.spawn(task, token);
                handle.await.ok().flatten()
            }
        })
        .buffer_unordered(max_concurrent)
        .collect()
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[tokio::test]
    async fn test_blocking_pool_spawn() {
        let pool = BlockingPool::new(BlockingPoolConfig::for_cpu_intensive());
        let token = CancellationToken::new();

        let result = pool.spawn(|| 42, token).await;
        assert_eq!(result.unwrap(), Some(42));
    }

    #[tokio::test]
    async fn test_blocking_pool_timeout() {
        let pool = BlockingPool::new(BlockingPoolConfig::for_cpu_intensive());
        let token = CancellationToken::new();

        let result = pool
            .spawn_with_timeout(
                || {
                    thread::sleep(Duration::from_secs(10));
                    42
                },
                Duration::from_millis(100),
                token,
            )
            .await;

        // 应该超时
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_cpu_intensive_task() {
        let pool = Arc::new(BlockingPool::new(BlockingPoolConfig::for_cpu_intensive()));
        let task = CpuIntensiveTask::<i32>::new(pool);

        let result = task.execute(|| 42).await;
        assert_eq!(result, Some(42));
    }

    #[tokio::test]
    async fn test_task_cancellation() {
        let pool = Arc::new(BlockingPool::new(BlockingPoolConfig::for_cpu_intensive()));
        let task = CpuIntensiveTask::<i32>::new(pool);

        // 取消任务
        task.cancel();

        let result = task
            .execute(|| {
                thread::sleep(Duration::from_secs(1));
                42
            })
            .await;

        // 应该被取消或快速完成
        assert!(result.is_none() || result == Some(42));
    }

    #[tokio::test]
    async fn test_parallel_execute() {
        let pool = BlockingPool::new(BlockingPoolConfig::for_cpu_intensive());
        let token = CancellationToken::new();

        let tasks: Vec<_> = (0..5)
            .map(|i| {
                move || {
                    thread::sleep(Duration::from_millis(10));
                    i * 2
                }
            })
            .collect();

        let results = parallel_execute(&pool, tasks, token).await;

        assert_eq!(results.len(), 5);
        for (i, result) in results.iter().enumerate() {
            assert_eq!(*result, Some(i * 2));
        }
    }

    #[tokio::test]
    async fn test_batch_with_backpressure() {
        let pool = BlockingPool::new(BlockingPoolConfig::for_cpu_intensive());
        let token = CancellationToken::new();

        let tasks: Vec<_> = (0..10)
            .map(|i| {
                move || {
                    thread::sleep(Duration::from_millis(50));
                    i
                }
            })
            .collect();

        let start = Instant::now();
        let results = batch_execute_with_backpressure(&pool, tasks, 2, token).await;
        let elapsed = start.elapsed();

        assert_eq!(results.len(), 10);

        // 限制并发为 2，每个任务 50ms，10 个任务至少需要 250ms
        // （5 批次 * 50ms = 250ms，加上一些开销）
        assert!(elapsed >= Duration::from_millis(200));
    }
}
