//! 指标状态管理
//!
//! 使用原子类型和轻量级锁实现高性能指标收集

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// 指标状态 - 管理性能指标和统计数据（完全无锁设计）
pub struct MetricsState {
    /// 总搜索次数
    pub total_searches: AtomicU64,
    /// 缓存命中次数
    pub cache_hits: AtomicU64,
    /// 上次搜索持续时间（微秒表示）
    pub last_search_duration_us: AtomicU64,
}

impl Default for MetricsState {
    fn default() -> Self {
        Self {
            total_searches: AtomicU64::new(0),
            cache_hits: AtomicU64::new(0),
            last_search_duration_us: AtomicU64::new(0),
        }
    }
}

impl MetricsState {
    /// 创建新的指标状态
    pub fn new() -> Self {
        Self::default()
    }

    /// 记录一次搜索
    pub fn record_search(&self) {
        self.total_searches.fetch_add(1, Ordering::Relaxed);
    }

    /// 记录缓存命中
    pub fn record_cache_hit(&self) {
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
    }

    /// 获取总搜索次数
    pub fn get_total_searches(&self) -> u64 {
        self.total_searches.load(Ordering::Relaxed)
    }

    /// 获取缓存命中次数
    pub fn get_cache_hits(&self) -> u64 {
        self.cache_hits.load(Ordering::Relaxed)
    }

    /// 获取缓存命中率
    pub fn get_cache_hit_rate(&self) -> f64 {
        let total = self.get_total_searches();
        if total == 0 {
            0.0
        } else {
            let hits = self.get_cache_hits();
            hits as f64 / total as f64
        }
    }

    /// 记录搜索持续时间
    pub fn record_search_duration(&self, duration: Duration) {
        self.last_search_duration_us
            .store(duration.as_micros() as u64, Ordering::Relaxed);
    }

    /// 获取上次搜索持续时间
    pub fn get_last_search_duration(&self) -> Duration {
        Duration::from_micros(self.last_search_duration_us.load(Ordering::Relaxed))
    }

    /// 重置所有指标
    pub fn reset(&self) {
        self.total_searches.store(0, Ordering::Relaxed);
        self.cache_hits.store(0, Ordering::Relaxed);
        self.last_search_duration_us.store(0, Ordering::Relaxed);
    }
}
