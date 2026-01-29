//! 性能指标收集模块

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// 计数器指标
#[derive(Debug)]
pub struct Counter {
    name: String,
    value: AtomicU64,
}

impl Counter {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            value: AtomicU64::new(0),
        }
    }

    pub fn increment(&self, value: u64) {
        self.value.fetch_add(value, Ordering::Relaxed);
    }

    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }

    pub fn reset(&self) {
        self.value.store(0, Ordering::Relaxed);
    }
}

/// 直方图指标
#[derive(Debug)]
pub struct Histogram {
    name: String,
    count: AtomicU64,
    sum: AtomicU64,
    min: AtomicU64,
    max: AtomicU64,
}

impl Histogram {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            count: AtomicU64::new(0),
            sum: AtomicU64::new(0),
            min: AtomicU64::new(u64::MAX),
            max: AtomicU64::new(0),
        }
    }

    pub fn record(&self, value: f64) {
        let value = (value * 1000.0) as u64; // 转换为毫秒精度

        self.count.fetch_add(1, Ordering::Relaxed);
        self.sum.fetch_add(value, Ordering::Relaxed);

        // 更新最小值
        let mut current_min = self.min.load(Ordering::Relaxed);
        while value < current_min {
            match self.min.compare_exchange_weak(
                current_min,
                value,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(v) => current_min = v,
            }
        }

        // 更新最大值
        let mut current_max = self.max.load(Ordering::Relaxed);
        while value > current_max {
            match self.max.compare_exchange_weak(
                current_max,
                value,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(v) => current_max = v,
            }
        }
    }

    pub fn count(&self) -> u64 {
        self.count.load(Ordering::Relaxed)
    }

    pub fn sum(&self) -> u64 {
        self.sum.load(Ordering::Relaxed)
    }

    pub fn min(&self) -> u64 {
        let min = self.min.load(Ordering::Relaxed);
        if min == u64::MAX {
            0
        } else {
            min
        }
    }

    pub fn max(&self) -> u64 {
        self.max.load(Ordering::Relaxed)
    }

    pub fn avg(&self) -> f64 {
        let count = self.count();
        if count == 0 {
            0.0
        } else {
            self.sum() as f64 / count as f64
        }
    }
}

/// 全局指标注册表
#[derive(Debug, Default)]
pub struct MetricsRegistry {
    counters: Vec<Arc<Counter>>,
    histograms: Vec<Arc<Histogram>>,
}

impl MetricsRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_counter(&mut self, counter: Arc<Counter>) {
        self.counters.push(counter);
    }

    pub fn register_histogram(&mut self, histogram: Arc<Histogram>) {
        self.histograms.push(histogram);
    }

    pub fn get_counter(&self, name: &str) -> Option<&Arc<Counter>> {
        self.counters.iter().find(|c| c.name == name)
    }

    pub fn get_histogram(&self, name: &str) -> Option<&Arc<Histogram>> {
        self.histograms.iter().find(|h| h.name == name)
    }

    pub fn export(&self) -> String {
        let mut output = String::new();

        for counter in &self.counters {
            output.push_str(&format!("{}: {}\n", counter.name, counter.get()));
        }

        for histogram in &self.histograms {
            output.push_str(&format!(
                "{}: count={}, sum={}, min={}, max={}, avg={:.2}\n",
                histogram.name,
                histogram.count(),
                histogram.sum(),
                histogram.min(),
                histogram.max(),
                histogram.avg()
            ));
        }

        output
    }
}

/// 初始化指标系统
pub fn init_metrics() -> Result<(), Box<dyn std::error::Error>> {
    // 创建全局指标注册表
    let registry = Arc::new(std::sync::Mutex::new(MetricsRegistry::new()));

    // 注册常用指标
    let search_counter = Arc::new(Counter::new("searches_total"));
    let error_counter = Arc::new(Counter::new("errors_total"));
    let search_duration = Arc::new(Histogram::new("search_duration_seconds"));

    // ✅ 修复：单次锁定，批量注册，避免死锁风险
    {
        let mut reg = registry
            .lock()
            .map_err(|e| format!("Failed to lock metrics registry: {}", e))?;

        reg.register_counter(search_counter);
        reg.register_counter(error_counter);
        reg.register_histogram(search_duration);
    } // 锁在此自动释放

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter() {
        let counter = Counter::new("test_counter");
        assert_eq!(counter.get(), 0);

        counter.increment(5);
        assert_eq!(counter.get(), 5);

        counter.increment(3);
        assert_eq!(counter.get(), 8);

        counter.reset();
        assert_eq!(counter.get(), 0);
    }

    #[test]
    fn test_histogram() {
        let histogram = Histogram::new("test_histogram");
        assert_eq!(histogram.count(), 0);
        assert_eq!(histogram.avg(), 0.0);

        histogram.record(1.0);
        histogram.record(2.0);
        histogram.record(3.0);

        assert_eq!(histogram.count(), 3);
        assert_eq!(histogram.min(), 1000); // 转换为毫秒
        assert_eq!(histogram.max(), 3000);
        assert_eq!(histogram.avg(), 2000.0);
    }
}
