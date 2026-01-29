use crate::error::Result;
use std::time::Duration;
use tokio_retry::strategy::{jitter, ExponentialBackoff};
use tokio_retry::Retry;
use tracing::warn;

/// Retry a fallible operation with exponential backoff
pub async fn with_retry<F, Fut, T>(operation: F) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let retry_strategy = ExponentialBackoff::from_millis(100)
        .map(jitter) // add jitter to delays
        .take(3); // limit to 3 retries

    Retry::spawn(retry_strategy, || {
        let fut = operation();
        async move {
            let res = fut.await;
            if let Err(ref e) = res {
                warn!("Operation failed, retrying... Error: {}", e);
            }
            res
        }
    })
    .await
}

/// A simple circuit breaker implementation
pub struct CircuitBreaker {
    failure_threshold: usize,
    reset_timeout: Duration,
    failures: std::sync::atomic::AtomicUsize,
    last_failure: std::sync::RwLock<Option<std::time::Instant>>,
}

impl CircuitBreaker {
    pub fn new(threshold: usize, reset_timeout: Duration) -> Self {
        Self {
            failure_threshold: threshold,
            reset_timeout,
            failures: std::sync::atomic::AtomicUsize::new(0),
            last_failure: std::sync::RwLock::new(None),
        }
    }

    pub fn is_open(&self) -> bool {
        if self.failures.load(std::sync::atomic::Ordering::Relaxed) >= self.failure_threshold {
            // ✅ 安全处理锁中毒：恢复数据而非 panic
            let last = match self.last_failure.read() {
                Ok(guard) => guard,
                Err(poisoned) => {
                    warn!("CircuitBreaker lock poisoned, recovering");
                    poisoned.into_inner()
                }
            };
            if let Some(instant) = *last {
                if instant.elapsed() < self.reset_timeout {
                    return true;
                }
            }
        }
        false
    }

    pub fn record_success(&self) {
        self.failures.store(0, std::sync::atomic::Ordering::Relaxed);
        // ✅ 安全处理锁中毒
        let mut last = self
            .last_failure
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *last = None;
    }

    pub fn record_failure(&self) {
        self.failures
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        // ✅ 安全处理锁中毒
        let mut last = self
            .last_failure
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *last = Some(std::time::Instant::now());
    }
}
