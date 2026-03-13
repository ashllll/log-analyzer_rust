//! 背压控制实现
//!
//! 提供多种背压控制机制：
//! - 信号量（Semaphore）
//! - 令牌桶（Token Bucket）
//! - 自适应限流（Adaptive Rate Limiting）

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, Semaphore, SemaphorePermit};
use tracing::{debug, trace, warn};

/// 信号量配置
#[derive(Debug, Clone)]
pub struct SemaphoreConfig {
    /// 最大并发数
    pub max_concurrent: usize,
    /// 获取许可的超时时间
    pub acquire_timeout: Duration,
    /// 是否启用公平模式
    pub fair: bool,
}

impl Default for SemaphoreConfig {
    fn default() -> Self {
        let cpu_count = num_cpus::get();
        Self {
            max_concurrent: cpu_count * 2,
            acquire_timeout: Duration::from_secs(30),
            fair: true,
        }
    }
}

/// 背压控制器
///
/// 使用信号量控制并发访问，防止资源耗尽
pub struct BackpressureController {
    semaphore: Arc<Semaphore>,
    config: SemaphoreConfig,
    /// 当前等待的请求数
    waiting_count: Arc<Mutex<usize>>,
    /// 拒绝的请求数
    rejected_count: Arc<Mutex<u64>>,
}

impl BackpressureController {
    /// 创建新的背压控制器
    pub fn new(config: SemaphoreConfig) -> Self {
        let semaphore = Arc::new(Semaphore::new(config.max_concurrent));

        Self {
            semaphore,
            config,
            waiting_count: Arc::new(Mutex::new(0)),
            rejected_count: Arc::new(Mutex::new(0)),
        }
    }

    /// 获取默认配置的背压控制器
    pub fn default_with_concurrency(max_concurrent: usize) -> Self {
        let config = SemaphoreConfig {
            max_concurrent,
            ..Default::default()
        };
        Self::new(config)
    }

    /// 获取访问许可（异步）
    ///
    /// 如果有可用许可，立即返回；否则等待
    pub async fn acquire(&self) -> Option<BackpressurePermit<'_>> {
        let start = Instant::now();

        // 增加等待计数
        {
            let mut waiting = self.waiting_count.lock().await;
            *waiting += 1;
        }

        trace!(
            available = self.semaphore.available_permits(),
            "Acquiring backpressure permit"
        );

        let result =
            match tokio::time::timeout(self.config.acquire_timeout, self.semaphore.acquire()).await
            {
                Ok(Ok(permit)) => {
                    debug!(
                        wait_time_ms = start.elapsed().as_millis(),
                        "Acquired backpressure permit"
                    );
                    Some(BackpressurePermit {
                        permit: Some(permit),
                        controller: self,
                    })
                }
                Ok(Err(_)) => {
                    warn!("Semaphore closed");
                    None
                }
                Err(_) => {
                    warn!(
                        timeout_secs = self.config.acquire_timeout.as_secs(),
                        "Timeout acquiring backpressure permit"
                    );
                    let mut rejected = self.rejected_count.lock().await;
                    *rejected += 1;
                    None
                }
            };

        // 减少等待计数
        {
            let mut waiting = self.waiting_count.lock().await;
            *waiting -= 1;
        }

        result
    }

    /// 尝试获取许可（非阻塞）
    pub fn try_acquire(&self) -> Option<BackpressurePermit<'_>> {
        match self.semaphore.try_acquire() {
            Ok(permit) => Some(BackpressurePermit {
                permit: Some(permit),
                controller: self,
            }),
            Err(_) => {
                trace!("Failed to acquire permit immediately");
                None
            }
        }
    }

    /// 获取当前可用许可数
    pub fn available_permits(&self) -> usize {
        self.semaphore.available_permits()
    }

    /// 获取当前等待的请求数
    pub async fn waiting_count(&self) -> usize {
        *self.waiting_count.lock().await
    }

    /// 获取被拒绝的请求数
    pub async fn rejected_count(&self) -> u64 {
        *self.rejected_count.lock().await
    }

    /// 检查系统是否处于高负载
    pub fn is_under_load(&self) -> bool {
        self.semaphore.available_permits() == 0
    }
}

/// 背压许可
///
/// 当此对象被丢弃时，许可自动释放
pub struct BackpressurePermit<'a> {
    permit: Option<SemaphorePermit<'a>>,
    controller: &'a BackpressureController,
}

impl<'a> BackpressurePermit<'a> {
    /// 获取控制器引用
    pub fn controller(&self) -> &'a BackpressureController {
        self.controller
    }

    /// 手动释放许可（提前释放）
    pub fn release(mut self) {
        self.permit.take();
    }
}

impl<'a> Drop for BackpressurePermit<'a> {
    fn drop(&mut self) {
        if self.permit.take().is_some() {
            trace!("Backpressure permit released");
        }
    }
}

/// 令牌桶限流器
///
/// 实现令牌桶算法进行限流
pub struct RateLimiter {
    /// 每秒生成的令牌数
    rate: f64,
    /// 桶容量
    capacity: f64,
    /// 当前令牌数
    tokens: Arc<Mutex<f64>>,
    /// 上次更新时间
    last_update: Arc<Mutex<Instant>>,
}

impl RateLimiter {
    /// 创建新的限流器
    pub fn new(rate: f64, capacity: f64) -> Self {
        Self {
            rate,
            capacity,
            tokens: Arc::new(Mutex::new(capacity)),
            last_update: Arc::new(Mutex::new(Instant::now())),
        }
    }

    /// 尝试获取令牌
    pub async fn try_acquire(&self, tokens: f64) -> bool {
        self.update_tokens().await;

        let mut current = self.tokens.lock().await;
        if *current >= tokens {
            *current -= tokens;
            true
        } else {
            false
        }
    }

    /// 获取令牌（阻塞直到获得）
    pub async fn acquire(&self, tokens: f64) {
        while !self.try_acquire(tokens).await {
            let wait_time = Duration::from_secs_f64(tokens / self.rate);
            tokio::time::sleep(wait_time.min(Duration::from_millis(100))).await;
        }
    }

    /// 更新令牌数
    async fn update_tokens(&self) {
        let now = Instant::now();
        let mut last = self.last_update.lock().await;
        let elapsed = now.duration_since(*last).as_secs_f64();
        *last = now;
        drop(last);

        let mut current = self.tokens.lock().await;
        *current = (*current + elapsed * self.rate).min(self.capacity);
    }
}

/// 自适应背压控制器
///
/// 根据系统负载动态调整并发限制
pub struct AdaptiveBackpressure {
    base_controller: BackpressureController,
    current_limit: Arc<Mutex<usize>>,
    min_limit: usize,
    max_limit: usize,
    /// 响应时间阈值（超过则减少并发）
    latency_threshold: Duration,
    /// 最近响应时间
    recent_latencies: Arc<Mutex<Vec<Duration>>>,
    max_latency_samples: usize,
}

impl AdaptiveBackpressure {
    pub fn new(
        initial_limit: usize,
        min_limit: usize,
        max_limit: usize,
        latency_threshold: Duration,
    ) -> Self {
        let config = SemaphoreConfig {
            max_concurrent: initial_limit,
            ..Default::default()
        };

        Self {
            base_controller: BackpressureController::new(config),
            current_limit: Arc::new(Mutex::new(initial_limit)),
            min_limit,
            max_limit,
            latency_threshold,
            recent_latencies: Arc::new(Mutex::new(Vec::new())),
            max_latency_samples: 100,
        }
    }

    /// 记录响应时间并调整限制
    pub async fn record_latency(&self, latency: Duration) {
        let mut latencies = self.recent_latencies.lock().await;
        latencies.push(latency);

        if latencies.len() > self.max_latency_samples {
            latencies.remove(0);
        }

        // 计算平均响应时间
        if latencies.len() >= 10 {
            let avg_latency: Duration = latencies.iter().sum::<Duration>() / latencies.len() as u32;

            let mut current = self.current_limit.lock().await;

            if avg_latency > self.latency_threshold && *current > self.min_limit {
                // 响应时间过长，减少并发
                *current = (*current - 1).max(self.min_limit);
                warn!(
                    new_limit = *current,
                    avg_latency_ms = avg_latency.as_millis(),
                    "Reducing concurrency due to high latency"
                );
            } else if avg_latency < self.latency_threshold / 2 && *current < self.max_limit {
                // 响应时间短，增加并发
                *current = (*current + 1).min(self.max_limit);
                debug!(
                    new_limit = *current,
                    avg_latency_ms = avg_latency.as_millis(),
                    "Increasing concurrency"
                );
            }
        }
    }

    /// 获取当前限制
    pub async fn current_limit(&self) -> usize {
        *self.current_limit.lock().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_backpressure_acquire() {
        let controller = BackpressureController::default_with_concurrency(2);

        // 获取两个许可
        let permit1 = controller.acquire().await;
        assert!(permit1.is_some());

        let permit2 = controller.acquire().await;
        assert!(permit2.is_some());

        // 可用许可应该为 0
        assert_eq!(controller.available_permits(), 0);
        assert!(controller.is_under_load());
    }

    #[tokio::test]
    async fn test_backpressure_permit_drop() {
        let controller = BackpressureController::default_with_concurrency(1);

        {
            let permit = controller.acquire().await;
            assert!(permit.is_some());
            assert_eq!(controller.available_permits(), 0);
            // permit 在这里被 drop
        }

        // 许可应该被释放
        assert_eq!(controller.available_permits(), 1);
    }

    #[tokio::test]
    async fn test_rate_limiter() {
        let limiter = RateLimiter::new(10.0, 10.0); // 每秒 10 个令牌，容量 10

        // 应该能立即获取
        assert!(limiter.try_acquire(5.0).await);
        assert!(limiter.try_acquire(3.0).await);

        // 剩余 2 个，不能获取 5 个
        assert!(!limiter.try_acquire(5.0).await);

        // 等待后应该可以
        tokio::time::sleep(Duration::from_millis(500)).await;
        assert!(limiter.try_acquire(3.0).await);
    }
}
