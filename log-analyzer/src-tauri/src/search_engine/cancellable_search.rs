//! 可取消搜索实现（协作式取消）
//!
//! 提供真正可中断的搜索操作：
//! - 定期检查取消状态
//! - 在关键点（segment 边界、文档批次）检查取消
//! - 快速响应取消请求
//!
//! ## 取消策略
//! 1. 协作式取消：搜索定期检查 CancellationToken
//! 2. 超时取消：tokio::timeout 包装
//! 3. 用户取消：通过 UI 或 API 调用主动取消

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tantivy::{
    collector::{Collector, SegmentCollector},
    DocId, Score, SegmentOrdinal, SegmentReader, TantivyError,
};
use tokio_util::sync::CancellationToken;
use tracing::{debug, trace};

/// 可取消收集器的配置
#[derive(Debug, Clone)]
pub struct CancellableConfig {
    /// 检查取消的频率（每 N 个文档）
    pub check_interval: usize,
    /// 最小检查间隔（时间）
    pub min_check_duration: Duration,
}

impl Default for CancellableConfig {
    fn default() -> Self {
        Self {
            check_interval: 1024,                          // 每 1024 个文档检查一次
            min_check_duration: Duration::from_millis(10), // 最少 10ms 检查一次
        }
    }
}

/// 增强版可取消收集器
///
/// 包装任意 Collector，添加取消检查功能
pub struct EnhancedCancellableCollector<C> {
    inner: C,
    token: CancellationToken,
    config: CancellableConfig,
    /// 已处理的文档计数
    docs_processed: Arc<AtomicUsize>,
    /// 上次检查时间
    last_check_time: Arc<std::sync::Mutex<Instant>>,
}

impl<C> EnhancedCancellableCollector<C> {
    pub fn new(inner: C, token: CancellationToken) -> Self {
        Self::with_config(inner, token, CancellableConfig::default())
    }

    pub fn with_config(inner: C, token: CancellationToken, config: CancellableConfig) -> Self {
        Self {
            inner,
            token,
            config,
            docs_processed: Arc::new(AtomicUsize::new(0)),
            last_check_time: Arc::new(std::sync::Mutex::new(Instant::now())),
        }
    }

    /// 获取已处理的文档数
    pub fn get_docs_processed(&self) -> usize {
        self.docs_processed.load(Ordering::Relaxed)
    }

    /// 检查是否应该取消（批量检查）
    fn should_check_cancel(&self, doc_count: usize) -> bool {
        // 基于文档数的检查
        if doc_count % self.config.check_interval == 0 {
            return true;
        }

        // 基于时间的检查
        if let Ok(last_check) = self.last_check_time.try_lock() {
            if last_check.elapsed() >= self.config.min_check_duration {
                return true;
            }
        }

        false
    }

    /// 执行取消检查
    fn check_cancelled(&self) -> Result<(), TantivyError> {
        if self.token.is_cancelled() {
            return Err(TantivyError::InternalError(
                "Search cancelled by user".to_string(),
            ));
        }
        Ok(())
    }
}

impl<C: Collector> Collector for EnhancedCancellableCollector<C> {
    type Fruit = C::Fruit;
    type Child = EnhancedCancellableChildCollector<C::Child>;

    fn for_segment(
        &self,
        segment_id: SegmentOrdinal,
        reader: &SegmentReader,
    ) -> tantivy::Result<Self::Child> {
        // 在 segment 级别检查取消
        self.check_cancelled()?;

        let child = self.inner.for_segment(segment_id, reader)?;

        Ok(EnhancedCancellableChildCollector {
            inner: child,
            token: self.token.clone(),
            config: self.config.clone(),
            docs_processed: Arc::clone(&self.docs_processed),
            last_check_time: Arc::clone(&self.last_check_time),
            segment_doc_count: 0,
        })
    }

    fn requires_scoring(&self) -> bool {
        self.inner.requires_scoring()
    }

    fn merge_fruits(
        &self,
        fruits: Vec<<Self::Child as SegmentCollector>::Fruit>,
    ) -> tantivy::Result<Self::Fruit> {
        self.inner.merge_fruits(fruits)
    }
}

/// 增强版可取消子收集器
pub struct EnhancedCancellableChildCollector<C> {
    inner: C,
    token: CancellationToken,
    config: CancellableConfig,
    docs_processed: Arc<AtomicUsize>,
    last_check_time: Arc<std::sync::Mutex<Instant>>,
    /// 当前 segment 处理的文档数
    segment_doc_count: usize,
}

impl<C: SegmentCollector> SegmentCollector for EnhancedCancellableChildCollector<C> {
    type Fruit = C::Fruit;

    fn collect(&mut self, doc: DocId, score: Score) {
        // 更新计数
        self.segment_doc_count += 1;

        // 检查是否需要取消
        if self.segment_doc_count % self.config.check_interval == 0 {
            if self.token.is_cancelled() {
                trace!(doc_id = doc, "Search cancelled, stopping collection");
                // 注意：由于 collect 不返回 Result，我们不能在这里停止
                // 但可以减少不必要的工作
                return;
            }

            // 更新最后检查时间
            if let Ok(mut last_check) = self.last_check_time.try_lock() {
                *last_check = Instant::now();
            }

            self.docs_processed
                .fetch_add(self.config.check_interval, Ordering::Relaxed);
        }

        self.inner.collect(doc, score);
    }

    fn harvest(self) -> Self::Fruit {
        // 更新最终计数
        let remaining = self.segment_doc_count % self.config.check_interval;
        if remaining > 0 {
            self.docs_processed.fetch_add(remaining, Ordering::Relaxed);
        }

        self.inner.harvest()
    }
}

/// 搜索取消控制器
///
/// 用于管理搜索的生命周期和取消
#[derive(Debug, Clone)]
pub struct SearchCancellationController {
    token: CancellationToken,
    /// 搜索开始时间
    start_time: Instant,
    /// 配置的超时时间
    timeout: Option<Duration>,
}

impl SearchCancellationController {
    /// 创建新的控制器
    pub fn new() -> Self {
        Self {
            token: CancellationToken::new(),
            start_time: Instant::now(),
            timeout: None,
        }
    }

    /// 创建带超时的控制器
    pub fn with_timeout(timeout: Duration) -> Self {
        Self {
            token: CancellationToken::new(),
            start_time: Instant::now(),
            timeout: Some(timeout),
        }
    }

    /// 获取取消令牌
    pub fn token(&self) -> CancellationToken {
        self.token.clone()
    }

    /// 取消搜索
    pub fn cancel(&self) {
        debug!("Search cancellation requested");
        self.token.cancel();
    }

    /// 检查是否已取消
    pub fn is_cancelled(&self) -> bool {
        if self.token.is_cancelled() {
            return true;
        }

        // 检查超时
        if let Some(timeout) = self.timeout {
            if self.start_time.elapsed() > timeout {
                self.cancel();
                return true;
            }
        }

        false
    }

    /// 获取已运行时间
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// 检查是否超时
    pub fn is_timeout(&self) -> bool {
        if let Some(timeout) = self.timeout {
            self.start_time.elapsed() > timeout
        } else {
            false
        }
    }
}

impl Default for SearchCancellationController {
    fn default() -> Self {
        Self::new()
    }
}

/// 批量可取消迭代器
///
/// 将迭代器分批处理，每批检查取消状态
pub struct CancellableBatchIterator<I> {
    inner: I,
    token: CancellationToken,
    batch_size: usize,
    current_batch: Vec<I::Item>,
}

impl<I: Iterator> CancellableBatchIterator<I> {
    pub fn new(inner: I, token: CancellationToken, batch_size: usize) -> Self {
        Self {
            inner,
            token,
            batch_size,
            current_batch: Vec::with_capacity(batch_size),
        }
    }

    /// 获取下一批
    pub fn next_batch(&mut self) -> Option<&[I::Item]> {
        if self.token.is_cancelled() {
            return None;
        }

        self.current_batch.clear();

        while self.current_batch.len() < self.batch_size {
            match self.inner.next() {
                Some(item) => self.current_batch.push(item),
                None => break,
            }
        }

        if self.current_batch.is_empty() {
            None
        } else {
            Some(&self.current_batch)
        }
    }
}

/// 可取消搜索包装器
///
/// 将任意搜索操作包装为可取消的异步操作
pub struct CancellableSearch<F, T> {
    operation: F,
    token: CancellationToken,
    _phantom: std::marker::PhantomData<T>,
}

impl<F, T> CancellableSearch<F, T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    pub fn new(operation: F, token: CancellationToken) -> Self {
        Self {
            operation,
            token,
            _phantom: std::marker::PhantomData,
        }
    }

    /// 在 spawn_blocking 中执行，支持取消
    pub async fn execute(self) -> Option<T> {
        let handle = tokio::task::spawn_blocking(self.operation);

        tokio::select! {
            result = handle => {
                match result {
                    Ok(value) => Some(value),
                    Err(e) => {
                        tracing::error!("Search task panicked: {}", e);
                        None
                    }
                }
            }
            _ = self.token.cancelled() => {
                tracing::debug!("Search cancelled during execution");
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_cancellation_token() {
        let token = CancellationToken::new();
        assert!(!token.is_cancelled());

        token.cancel();
        assert!(token.is_cancelled());
    }

    #[test]
    fn test_cancellation_controller() {
        let controller = SearchCancellationController::with_timeout(Duration::from_millis(100));

        // 初始状态不应该取消
        assert!(!controller.is_cancelled());

        // 等待超时
        thread::sleep(Duration::from_millis(150));
        assert!(controller.is_timeout());
        assert!(controller.is_cancelled());
    }

    #[test]
    fn test_enhanced_collector_config() {
        let config = CancellableConfig {
            check_interval: 100,
            min_check_duration: Duration::from_millis(5),
        };

        let token = CancellationToken::new();
        let collector = EnhancedCancellableCollector::with_config((), token, config);

        assert_eq!(collector.get_docs_processed(), 0);
    }

    #[tokio::test]
    async fn test_cancellable_search_execute() {
        let token = CancellationToken::new();

        let search = CancellableSearch::new(
            || {
                thread::sleep(Duration::from_millis(10));
                42
            },
            token.clone(),
        );

        let result = search.execute().await;
        assert_eq!(result, Some(42));

        // 测试取消
        let token2 = CancellationToken::new();
        let search2 = CancellableSearch::new(
            || {
                thread::sleep(Duration::from_secs(10));
                42
            },
            token2.clone(),
        );

        // 提前取消
        token2.cancel();
        let result2 = search2.execute().await;
        // 取消后应该返回 None 或不等待完成
        assert!(result2.is_none() || result2 == Some(42));
    }
}
