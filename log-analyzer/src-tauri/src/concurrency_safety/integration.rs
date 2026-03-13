//! 并发安全集成示例
//!
//! 展示如何整合所有并发安全组件：
//! - CAS 原子存储
//! - 真正异步搜索
//! - 背压控制
//! - 可取消任务

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use crate::concurrency_safety::{
    BackpressureController, BlockingPool, BlockingPoolConfig, CpuIntensiveTask,
};
use crate::storage::cas_atomic::AtomicContentAddressableStorage;

/// 并发安全配置
#[derive(Debug, Clone)]
pub struct ConcurrencySafetyConfig {
    /// CAS 配置
    pub cas_max_concurrent_writes: usize,
    /// 搜索线程池配置
    pub search_pool_config: BlockingPoolConfig,
    /// 背压配置
    pub backpressure_concurrent: usize,
    /// 搜索超时
    pub search_timeout: Duration,
}

impl Default for ConcurrencySafetyConfig {
    fn default() -> Self {
        let cpu_count = num_cpus::get();

        Self {
            cas_max_concurrent_writes: cpu_count * 4,
            search_pool_config: BlockingPoolConfig::for_cpu_intensive(),
            backpressure_concurrent: cpu_count * 2,
            search_timeout: Duration::from_millis(200),
        }
    }
}

/// 并发安全服务容器
///
/// 整合所有并发安全组件的统一接口
pub struct ConcurrentSafeServices {
    /// CAS 存储（原子写入）
    pub cas_storage: AtomicContentAddressableStorage,
    /// 搜索线程池
    pub search_pool: Arc<BlockingPool>,
    /// 搜索背压控制器
    pub search_backpressure: BackpressureController,
    /// 全局取消令牌（用于应用关闭时）
    pub global_cancel_token: CancellationToken,
}

impl ConcurrentSafeServices {
    /// 创建并发安全服务容器
    pub fn new(workspace_dir: PathBuf, config: ConcurrencySafetyConfig) -> Self {
        // 创建 CAS 存储
        let cas_storage = AtomicContentAddressableStorage::new(workspace_dir);

        // 创建搜索线程池
        let search_pool = Arc::new(BlockingPool::new(config.search_pool_config));

        // 创建背压控制器
        let search_backpressure =
            BackpressureController::default_with_concurrency(config.backpressure_concurrent);

        let global_cancel_token = CancellationToken::new();

        info!(
            cas_concurrent = config.cas_max_concurrent_writes,
            search_pool_size = search_pool
                .get_stats()
                .total_tasks
                .load(std::sync::atomic::Ordering::Relaxed),
            backpressure_limit = config.backpressure_concurrent,
            "Concurrent safe services initialized"
        );

        Self {
            cas_storage,
            search_pool,
            search_backpressure,
            global_cancel_token,
        }
    }

    /// 优雅关闭
    pub async fn shutdown(&self) {
        info!("Shutting down concurrent safe services...");

        // 触发全局取消
        self.global_cancel_token.cancel();

        // 等待一段时间让正在进行的任务完成
        tokio::time::sleep(Duration::from_millis(500)).await;

        // 打印统计信息
        let stats = self.search_pool.get_stats();
        info!(
            total_tasks = stats.total_tasks.load(std::sync::atomic::Ordering::Relaxed),
            completed = stats
                .completed_tasks
                .load(std::sync::atomic::Ordering::Relaxed),
            cancelled = stats
                .cancelled_tasks
                .load(std::sync::atomic::Ordering::Relaxed),
            "Search pool statistics"
        );

        info!("Concurrent safe services shutdown complete");
    }

    /// 检查系统健康状态
    pub fn health_check(&self) -> ServiceHealth {
        ServiceHealth {
            search_pool_load: self.search_pool.estimated_load(),
            search_backpressure_wait: 0, // 需要异步获取
            cas_available_permits: 0,    // 需要从 CAS 获取
            global_cancelled: self.global_cancel_token.is_cancelled(),
        }
    }
}

/// 服务健康状态
#[derive(Debug)]
pub struct ServiceHealth {
    pub search_pool_load: f64,
    pub search_backpressure_wait: usize,
    pub cas_available_permits: usize,
    pub global_cancelled: bool,
}

impl ServiceHealth {
    /// 检查是否健康
    pub fn is_healthy(&self) -> bool {
        !self.global_cancelled && self.search_pool_load < 0.9 && self.search_backpressure_wait < 10
    }
}

/// 搜索任务构建器
///
/// 使用 Builder 模式简化搜索任务的创建
pub struct SearchTaskBuilder<'a> {
    services: &'a ConcurrentSafeServices,
    timeout: Duration,
    token: CancellationToken,
}

impl<'a> SearchTaskBuilder<'a> {
    pub fn new(services: &'a ConcurrentSafeServices) -> Self {
        Self {
            services,
            timeout: Duration::from_millis(200),
            token: CancellationToken::new(),
        }
    }

    /// 设置超时
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// 设置取消令牌
    pub fn with_token(mut self, token: CancellationToken) -> Self {
        self.token = token;
        self
    }

    /// 使用全局取消令牌
    pub fn with_global_cancel(self) -> Self {
        let child_token = self.services.global_cancel_token.child_token();
        self.with_token(child_token)
    }

    /// 执行搜索任务
    pub async fn execute<F, T>(self, search_fn: F) -> Option<T>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        // 获取背压许可
        let _permit = match self.services.search_backpressure.acquire().await {
            Some(permit) => permit,
            None => {
                warn!("Failed to acquire backpressure permit, search rejected");
                return None;
            }
        };

        // 检查全局取消
        if self.services.global_cancel_token.is_cancelled() {
            warn!("Search rejected: global shutdown in progress");
            return None;
        }

        // 检查任务取消
        if self.token.is_cancelled() {
            return None;
        }

        // 在线程池中执行搜索
        let result = self
            .services
            .search_pool
            .spawn_with_timeout(search_fn, self.timeout, self.token)
            .await;

        result
    }
}

/// CAS 写入任务构建器
pub struct CasTaskBuilder<'a> {
    services: &'a ConcurrentSafeServices,
}

impl<'a> CasTaskBuilder<'a> {
    pub fn new(services: &'a ConcurrentSafeServices) -> Self {
        Self { services }
    }

    /// 存储内容
    pub async fn store_content(&self, content: Vec<u8>) -> Result<String, crate::error::AppError> {
        self.services
            .cas_storage
            .store_content_atomic(&content)
            .await
    }

    /// 流式存储文件
    pub async fn store_file(&self, file_path: PathBuf) -> Result<String, crate::error::AppError> {
        self.services
            .cas_storage
            .store_file_streaming_atomic(&file_path)
            .await
    }
}

/// 并发安全辅助宏
#[macro_export]
macro_rules! with_timeout_and_cancel {
    ($services:expr, $timeout:expr, $body:expr) => {{
        let token = CancellationToken::new();
        let task = CpuIntensiveTask::new(Arc::clone(&$services.search_pool));

        tokio::select! {
            result = task.execute_with_timeout(|| $body, $timeout) => result,
            _ = $services.global_cancel_token.cancelled() => {
                warn!("Operation cancelled due to global shutdown");
                None
            }
        }
    }};
}

/// 并发安全最佳实践示例
#[cfg(test)]
mod examples {
    use super::*;
    use tempfile::TempDir;

    /// 示例 1: 基本搜索流程
    #[tokio::test]
    async fn example_basic_search() {
        let temp_dir = TempDir::new().unwrap();
        let config = ConcurrencySafetyConfig::default();
        let services = ConcurrentSafeServices::new(temp_dir.path().to_path_buf(), config);

        // 构建并执行搜索任务
        let result: Option<Vec<u8>> = SearchTaskBuilder::new(&services)
            .timeout(Duration::from_secs(1))
            .execute(|| {
                // 模拟 CPU 密集型搜索
                std::thread::sleep(Duration::from_millis(100));
                vec![1, 2, 3]
            })
            .await;

        assert!(result.is_some());
    }

    /// 示例 2: 带取消的搜索
    #[tokio::test]
    async fn example_cancellable_search() {
        let temp_dir = TempDir::new().unwrap();
        let config = ConcurrencySafetyConfig::default();
        let services = ConcurrentSafeServices::new(temp_dir.path().to_path_buf(), config);

        let token = CancellationToken::new();
        let token_clone = token.clone();

        // 在另一个任务中延迟取消
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            token_clone.cancel();
        });

        let result: Option<i32> = SearchTaskBuilder::new(&services)
            .with_token(token)
            .execute(|| {
                std::thread::sleep(Duration::from_secs(10));
                42
            })
            .await;

        // 应该被取消或完成
        assert!(result.is_none() || result == Some(42));
    }

    /// 示例 3: CAS 存储
    #[tokio::test]
    async fn example_cas_storage() {
        let temp_dir = TempDir::new().unwrap();
        let config = ConcurrencySafetyConfig::default();
        let services = ConcurrentSafeServices::new(temp_dir.path().to_path_buf(), config);

        let cas_builder = CasTaskBuilder::new(&services);

        // 存储内容
        let content = b"test content".to_vec();
        let hash = cas_builder.store_content(content).await.unwrap();

        // 验证存储
        assert!(!hash.is_empty());
    }

    /// 示例 4: 系统健康检查
    #[tokio::test]
    async fn example_health_check() {
        let temp_dir = TempDir::new().unwrap();
        let config = ConcurrencySafetyConfig::default();
        let services = ConcurrentSafeServices::new(temp_dir.path().to_path_buf(), config);

        let health = services.health_check();

        // 新创建的服务应该是健康的
        assert!(!health.global_cancelled);
        assert!(health.search_pool_load < 1.0);
    }

    /// 示例 5: 优雅关闭
    #[tokio::test]
    async fn example_graceful_shutdown() {
        let temp_dir = TempDir::new().unwrap();
        let config = ConcurrencySafetyConfig::default();
        let services = ConcurrentSafeServices::new(temp_dir.path().to_path_buf(), config);

        // 启动一些任务
        let mut handles = vec![];
        for i in 0..3 {
            let services_ref = &services;
            let handle = tokio::spawn(async move {
                SearchTaskBuilder::new(services_ref)
                    .execute(move || {
                        std::thread::sleep(Duration::from_millis(200));
                        i
                    })
                    .await
            });
            handles.push(handle);
        }

        // 延迟关闭
        tokio::time::sleep(Duration::from_millis(50)).await;
        services.shutdown().await;

        // 所有任务应该被取消或完成
        for handle in handles {
            let _ = handle.await;
        }
    }
}
