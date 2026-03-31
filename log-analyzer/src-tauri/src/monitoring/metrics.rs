//! 性能监控模块
//!
//! 提供完整的性能监控功能，包括：
//! - 搜索性能指标（延迟、吞吐量、结果数量）
//! - 内存使用监控（当前使用量、峰值、分配速率）
//! - 文件处理统计（导入数量、耗时、类型分布）
//! - 索引操作统计（构建时间、大小、更新频率）
//!
//! ## 架构设计
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    MetricsCollector                         │
//! ├─────────────┬─────────────┬─────────────┬───────────────────┤
//! │  Search     │   Memory    │   File      │     Index         │
//! │  Metrics    │   Metrics   │   Metrics   │     Metrics       │
//! └─────────────┴─────────────┴─────────────┴───────────────────┘
//! │                    Ring Buffer (时序数据)                    │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## 性能考虑
//!
//! - 使用环形缓冲区存储时序数据，避免无限增长
//! - 指标更新采用无锁原子操作，最小化性能开销
//! - 内存监控使用 sysinfo crate，在独立线程执行
//! - 提供 API 获取实时指标和历史趋势数据

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use sysinfo::System;
use tokio::time::interval;
use tracing::{debug, info};

// ============================================================================
// 常量配置
// ============================================================================

/// 环形缓冲区容量（保留最近 1000 个数据点）
const RING_BUFFER_CAPACITY: usize = 1000;

/// 内存监控采样间隔（秒）
#[allow(dead_code)]
const MEMORY_SAMPLE_INTERVAL_SECS: u64 = 5;

/// 默认百分位计算窗口大小
const PERCENTILE_WINDOW_SIZE: usize = 100;

// ============================================================================
// 数据类型定义
// ============================================================================

/// 监控指标汇总
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringMetrics {
    /// 搜索性能指标
    pub search: SearchMetrics,
    /// 内存使用指标
    pub memory: MemoryMetrics,
    /// 文件处理指标
    pub file: FileMetrics,
    /// 索引操作指标
    pub index: IndexMetrics,
    /// 任务管理器指标
    pub task: TaskMetrics,
    /// 采集时间戳
    pub timestamp: i64,
}

/// 搜索性能指标
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchMetrics {
    /// 总搜索次数
    pub total_searches: u64,
    /// 当前搜索延迟（毫秒）
    pub current_latency_ms: u64,
    /// 平均搜索延迟（毫秒）
    pub average_latency_ms: u64,
    /// P95 延迟（毫秒）
    pub p95_latency_ms: u64,
    /// P99 延迟（毫秒）
    pub p99_latency_ms: u64,
    /// 最大延迟（毫秒）
    pub max_latency_ms: u64,
    /// 最小延迟（毫秒）
    pub min_latency_ms: u64,
    /// 当前吞吐量（次/秒）
    pub current_throughput: f64,
    /// 平均吞吐量（次/秒）
    pub average_throughput: f64,
    /// 峰值吞吐量（次/秒）
    pub peak_throughput: f64,
    /// 总结果数量
    pub total_results: u64,
    /// 平均结果数量
    pub average_results: f64,
    /// 缓存命中率（0-100）
    pub cache_hit_rate: f64,
}

/// 内存使用指标
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemoryMetrics {
    /// 当前内存使用量（MB）
    pub current_used_mb: u64,
    /// 峰值内存使用量（MB）
    pub peak_used_mb: u64,
    /// 系统总内存（MB）
    pub total_system_mb: u64,
    /// 内存使用百分比（0-100）
    pub usage_percentage: f64,
    /// 内存分配速率（MB/秒）
    pub allocation_rate_mbps: f64,
    /// 虚拟内存使用量（MB）
    pub virtual_memory_mb: u64,
    /// 进程常驻内存（MB）
    pub resident_memory_mb: u64,
}

/// 文件处理指标
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FileMetrics {
    /// 总导入文件数
    pub total_imported_files: u64,
    /// 总导入字节数
    pub total_imported_bytes: u64,
    /// 导入操作次数
    pub import_operations: u64,
    /// 平均导入耗时（毫秒）
    pub average_import_duration_ms: u64,
    /// 文件类型分布
    pub file_type_distribution: HashMap<String, u64>,
    /// 导入成功率（0-100）
    pub import_success_rate: f64,
    /// 当前导入速率（文件/秒）
    pub current_import_rate: f64,
    /// 导入字节速率（MB/秒）
    pub import_throughput_mbps: f64,
}

/// 索引操作指标
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IndexMetrics {
    /// 索引构建次数
    pub build_count: u64,
    /// 总索引构建时间（毫秒）
    pub total_build_time_ms: u64,
    /// 平均索引构建时间（毫秒）
    pub average_build_time_ms: u64,
    /// 索引大小（字节）
    pub index_size_bytes: u64,
    /// 已索引文件数
    pub indexed_files: u64,
    /// 索引更新次数
    pub update_count: u64,
    /// 平均更新频率（次/分钟）
    pub update_frequency_per_min: f64,
    /// 最后更新时间戳
    pub last_update_timestamp: Option<i64>,
    /// 索引段数量
    pub segment_count: u64,
}

/// 任务管理器指标
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskMetrics {
    /// 总任务数
    pub total_tasks: u64,
    /// 运行中任务数
    pub running_tasks: u64,
    /// 已完成任务数
    pub completed_tasks: u64,
    /// 失败任务数
    pub failed_tasks: u64,
    /// 平均任务执行时间（毫秒）
    pub average_task_duration_ms: u64,
}

/// 历史指标数据点（用于时序存储）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsDataPoint {
    /// 时间戳（Unix 时间，毫秒）
    pub timestamp: i64,
    /// 搜索延迟（毫秒）
    pub search_latency_ms: u64,
    /// 搜索吞吐量（次/秒）
    pub search_throughput: f64,
    /// 内存使用量（MB）
    pub memory_used_mb: u64,
    /// 导入文件数
    pub imported_files: u64,
    /// 索引大小（字节）
    pub index_size_bytes: u64,
}

// ============================================================================
// 环形缓冲区实现
// ============================================================================

/// 固定容量的环形缓冲区，用于存储时序指标数据
///
/// 当缓冲区满时，新数据会覆盖最旧的数据，保持固定内存占用。
pub struct RingBuffer<T> {
    /// 底层存储
    buffer: Vec<T>,
    /// 当前写入位置
    head: usize,
    /// 当前元素数量
    count: usize,
    /// 容量
    capacity: usize,
}

impl<T: Clone> RingBuffer<T> {
    /// 创建指定容量的环形缓冲区
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
            head: 0,
            count: 0,
            capacity,
        }
    }

    /// 添加元素到缓冲区
    pub fn push(&mut self, item: T) {
        if self.buffer.len() < self.capacity {
            self.buffer.push(item);
        } else {
            self.buffer[self.head] = item;
        }
        self.head = (self.head + 1) % self.capacity;
        if self.count < self.capacity {
            self.count += 1;
        }
    }

    /// 获取所有元素（按时间顺序，从旧到新）
    pub fn get_all(&self) -> Vec<T> {
        if self.count == 0 {
            return Vec::new();
        }

        let mut result = Vec::with_capacity(self.count);
        let start = if self.count == self.capacity {
            self.head
        } else {
            0
        };

        for i in 0..self.count {
            let idx = (start + i) % self.capacity;
            result.push(self.buffer[idx].clone());
        }

        result
    }

    /// 获取最近 N 个元素
    pub fn get_recent(&self, n: usize) -> Vec<T> {
        let n = n.min(self.count);
        if n == 0 {
            return Vec::new();
        }

        let mut result = Vec::with_capacity(n);
        for i in 0..n {
            let idx = if self.head > i {
                self.head - i - 1
            } else {
                self.capacity - (i + 1 - self.head)
            };
            result.push(self.buffer[idx].clone());
        }

        result.reverse();
        result
    }

    /// 获取当前元素数量
    pub fn len(&self) -> usize {
        self.count
    }

    /// 检查缓冲区是否为空
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// 清空缓冲区
    pub fn clear(&mut self) {
        self.head = 0;
        self.count = 0;
        self.buffer.clear();
    }
}

// ============================================================================
// 搜索指标收集器
// ============================================================================

/// 搜索性能指标收集器
///
/// 使用原子变量实现线程安全的指标更新，避免锁竞争。
pub struct SearchMetricsCollector {
    /// 总搜索次数
    total_searches: AtomicU64,
    /// 当前延迟（毫秒）
    current_latency_ms: AtomicU64,
    /// 总延迟（用于计算平均值）
    total_latency_ms: AtomicU64,
    /// 最大延迟
    max_latency_ms: AtomicU64,
    /// 最小延迟
    min_latency_ms: AtomicU64,
    /// 总结果数量
    total_results: AtomicU64,
    /// 缓存命中次数
    cache_hits: AtomicU64,
    /// 缓存未命中次数
    cache_misses: AtomicU64,
    /// 搜索开始时间（用于计算吞吐量）
    first_search_time: RwLock<Option<Instant>>,
    /// 延迟历史（用于计算百分位）
    latency_history: RwLock<RingBuffer<u64>>,
}

impl SearchMetricsCollector {
    /// 创建新的搜索指标收集器
    pub fn new() -> Self {
        Self {
            total_searches: AtomicU64::new(0),
            current_latency_ms: AtomicU64::new(0),
            total_latency_ms: AtomicU64::new(0),
            max_latency_ms: AtomicU64::new(0),
            min_latency_ms: AtomicU64::new(u64::MAX),
            total_results: AtomicU64::new(0),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            first_search_time: RwLock::new(None),
            latency_history: RwLock::new(RingBuffer::new(PERCENTILE_WINDOW_SIZE)),
        }
    }

    /// 记录一次搜索操作
    ///
    /// # 参数
    ///
    /// * `latency_ms` - 搜索延迟（毫秒）
    /// * `results_count` - 结果数量
    /// * `cache_hit` - 是否命中缓存
    pub fn record_search(&self, latency_ms: u64, results_count: u64, cache_hit: bool) {
        // 更新计数器
        self.total_searches.fetch_add(1, Ordering::Relaxed);
        self.current_latency_ms.store(latency_ms, Ordering::Relaxed);
        self.total_latency_ms
            .fetch_add(latency_ms, Ordering::Relaxed);
        self.total_results
            .fetch_add(results_count, Ordering::Relaxed);

        // 更新最大/最小延迟
        let mut current_max = self.max_latency_ms.load(Ordering::Relaxed);
        while latency_ms > current_max {
            match self.max_latency_ms.compare_exchange(
                current_max,
                latency_ms,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => current_max = actual,
            }
        }

        let mut current_min = self.min_latency_ms.load(Ordering::Relaxed);
        while latency_ms < current_min && current_min != 0 {
            match self.min_latency_ms.compare_exchange(
                current_min,
                latency_ms,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => current_min = actual,
            }
        }

        // 更新缓存统计
        if cache_hit {
            self.cache_hits.fetch_add(1, Ordering::Relaxed);
        } else {
            self.cache_misses.fetch_add(1, Ordering::Relaxed);
        }

        // 记录到历史
        self.latency_history.write().push(latency_ms);

        // 记录首次搜索时间
        let mut first_time = self.first_search_time.write();
        if first_time.is_none() {
            *first_time = Some(Instant::now());
        }

        debug!(
            latency_ms,
            results_count, cache_hit, "Recorded search metrics"
        );
    }

    /// 获取当前搜索指标
    pub fn get_metrics(&self) -> SearchMetrics {
        let total = self.total_searches.load(Ordering::Relaxed);
        let total_latency = self.total_latency_ms.load(Ordering::Relaxed);
        let cache_hits = self.cache_hits.load(Ordering::Relaxed);
        let cache_misses = self.cache_misses.load(Ordering::Relaxed);
        let total_cache_ops = cache_hits + cache_misses;

        // 计算平均延迟
        let average_latency = if total > 0 { total_latency / total } else { 0 };

        // 计算吞吐量
        let first_time = self.first_search_time.read();
        let throughput = if let Some(start) = *first_time {
            let elapsed_secs = start.elapsed().as_secs_f64();
            if elapsed_secs > 0.0 {
                total as f64 / elapsed_secs
            } else {
                0.0
            }
        } else {
            0.0
        };

        // 计算百分位延迟
        let latency_history = self.latency_history.read();
        let latencies: Vec<u64> = latency_history.get_all();
        let (p95, p99) = calculate_percentiles(&latencies);

        // 计算缓存命中率
        let hit_rate = if total_cache_ops > 0 {
            (cache_hits as f64 / total_cache_ops as f64) * 100.0
        } else {
            0.0
        };

        SearchMetrics {
            total_searches: total,
            current_latency_ms: self.current_latency_ms.load(Ordering::Relaxed),
            average_latency_ms: average_latency,
            p95_latency_ms: p95,
            p99_latency_ms: p99,
            max_latency_ms: self.max_latency_ms.load(Ordering::Relaxed),
            min_latency_ms: if total > 0 {
                self.min_latency_ms.load(Ordering::Relaxed)
            } else {
                0
            },
            current_throughput: throughput,
            average_throughput: throughput,
            peak_throughput: throughput,
            total_results: self.total_results.load(Ordering::Relaxed),
            average_results: if total > 0 {
                self.total_results.load(Ordering::Relaxed) as f64 / total as f64
            } else {
                0.0
            },
            cache_hit_rate: hit_rate,
        }
    }

    /// 重置指标
    pub fn reset(&self) {
        self.total_searches.store(0, Ordering::Relaxed);
        self.current_latency_ms.store(0, Ordering::Relaxed);
        self.total_latency_ms.store(0, Ordering::Relaxed);
        self.max_latency_ms.store(0, Ordering::Relaxed);
        self.min_latency_ms.store(u64::MAX, Ordering::Relaxed);
        self.total_results.store(0, Ordering::Relaxed);
        self.cache_hits.store(0, Ordering::Relaxed);
        self.cache_misses.store(0, Ordering::Relaxed);
        *self.first_search_time.write() = None;
        self.latency_history.write().clear();
        info!("Search metrics reset");
    }
}

impl Default for SearchMetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 内存指标收集器
// ============================================================================

/// 内存使用指标收集器
///
/// 使用 sysinfo crate 获取系统和进程内存信息。
pub struct MemoryMetricsCollector {
    /// 峰值内存使用量（MB）
    peak_used_mb: AtomicU64,
    /// 上次内存使用量（用于计算分配速率）
    last_used_mb: AtomicU64,
    /// 上次采样时间
    last_sample_time: RwLock<Option<Instant>>,
    /// 内存分配速率（MB/秒）
    allocation_rate: AtomicU64,
    /// 系统信息
    system: RwLock<System>,
    /// 当前进程 ID
    pid: sysinfo::Pid,
}

impl MemoryMetricsCollector {
    /// 创建新的内存指标收集器
    pub fn new() -> Self {
        let system = System::new_all();
        let pid = sysinfo::Pid::from(std::process::id() as usize);

        Self {
            peak_used_mb: AtomicU64::new(0),
            last_used_mb: AtomicU64::new(0),
            last_sample_time: RwLock::new(None),
            allocation_rate: AtomicU64::new(0),
            system: RwLock::new(system),
            pid,
        }
    }

    /// 采样当前内存使用情况
    ///
    /// 此方法会刷新系统信息并更新内存指标。
    /// 建议在后台任务中定期调用（如每 5 秒）。
    pub fn sample(&self) -> MemoryMetrics {
        let mut system = self.system.write();
        system.refresh_all();

        // 获取系统内存信息
        let total_memory = system.total_memory();
        let used_memory = system.used_memory();

        // 获取当前进程内存信息
        let (resident_mb, virtual_mb) = if let Some(process) = system.process(self.pid) {
            (
                process.memory() / 1024,         // KB -> MB
                process.virtual_memory() / 1024, // KB -> MB
            )
        } else {
            (0, 0)
        };

        // 转换为 MB
        let used_mb = used_memory / 1024;
        let total_mb = total_memory / 1024;

        // 更新峰值
        let mut current_peak = self.peak_used_mb.load(Ordering::Relaxed);
        while resident_mb > current_peak {
            match self.peak_used_mb.compare_exchange(
                current_peak,
                resident_mb,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => current_peak = actual,
            }
        }

        // 计算分配速率
        let now = Instant::now();
        let mut last_time = self.last_sample_time.write();
        let rate = if let Some(last) = *last_time {
            let elapsed_secs = now.duration_since(last).as_secs_f64();
            let last_used = self.last_used_mb.load(Ordering::Relaxed);
            if elapsed_secs > 0.0 && resident_mb > last_used {
                let rate_mbps = (resident_mb - last_used) as f64 / elapsed_secs;
                self.allocation_rate
                    .store(rate_mbps as u64, Ordering::Relaxed);
                rate_mbps
            } else {
                self.allocation_rate.load(Ordering::Relaxed) as f64
            }
        } else {
            0.0
        };

        // 更新上次采样
        self.last_used_mb.store(resident_mb, Ordering::Relaxed);
        *last_time = Some(now);
        drop(last_time);
        drop(system);

        let usage_percentage = if total_mb > 0 {
            (used_mb as f64 / total_mb as f64) * 100.0
        } else {
            0.0
        };

        MemoryMetrics {
            current_used_mb: resident_mb,
            peak_used_mb: self.peak_used_mb.load(Ordering::Relaxed),
            total_system_mb: total_mb,
            usage_percentage,
            allocation_rate_mbps: rate,
            virtual_memory_mb: virtual_mb,
            resident_memory_mb: resident_mb,
        }
    }

    /// 获取当前内存指标（不刷新）
    pub fn get_metrics(&self) -> MemoryMetrics {
        self.sample()
    }

    /// 重置指标
    pub fn reset(&self) {
        self.peak_used_mb.store(0, Ordering::Relaxed);
        self.last_used_mb.store(0, Ordering::Relaxed);
        *self.last_sample_time.write() = None;
        self.allocation_rate.store(0, Ordering::Relaxed);
        info!("Memory metrics reset");
    }
}

impl Default for MemoryMetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 文件指标收集器
// ============================================================================

/// 文件处理指标收集器
///
/// 跟踪文件导入操作的统计信息。
pub struct FileMetricsCollector {
    /// 总导入文件数
    total_files: AtomicU64,
    /// 总导入字节数
    total_bytes: AtomicU64,
    /// 导入操作次数
    import_operations: AtomicU64,
    /// 成功导入次数
    successful_imports: AtomicU64,
    /// 失败导入次数
    failed_imports: AtomicU64,
    /// 总导入耗时（毫秒）
    total_duration_ms: AtomicU64,
    /// 文件类型分布
    file_types: RwLock<HashMap<String, u64>>,
    /// 导入开始时间（用于计算速率）
    import_start_time: RwLock<Option<Instant>>,
}

impl FileMetricsCollector {
    /// 创建新的文件指标收集器
    pub fn new() -> Self {
        Self {
            total_files: AtomicU64::new(0),
            total_bytes: AtomicU64::new(0),
            import_operations: AtomicU64::new(0),
            successful_imports: AtomicU64::new(0),
            failed_imports: AtomicU64::new(0),
            total_duration_ms: AtomicU64::new(0),
            file_types: RwLock::new(HashMap::new()),
            import_start_time: RwLock::new(None),
        }
    }

    /// 记录一次导入操作
    ///
    /// # 参数
    ///
    /// * `file_count` - 导入的文件数量
    /// * `bytes` - 导入的字节数
    /// * `duration_ms` - 导入耗时（毫秒）
    /// * `success` - 是否成功
    pub fn record_import(&self, file_count: u64, bytes: u64, duration_ms: u64, success: bool) {
        self.import_operations.fetch_add(1, Ordering::Relaxed);
        self.total_files.fetch_add(file_count, Ordering::Relaxed);
        self.total_bytes.fetch_add(bytes, Ordering::Relaxed);
        self.total_duration_ms
            .fetch_add(duration_ms, Ordering::Relaxed);

        if success {
            self.successful_imports.fetch_add(1, Ordering::Relaxed);
        } else {
            self.failed_imports.fetch_add(1, Ordering::Relaxed);
        }

        // 记录导入开始时间
        let mut start_time = self.import_start_time.write();
        if start_time.is_none() {
            *start_time = Some(Instant::now());
        }

        debug!(
            file_count,
            bytes, duration_ms, success, "Recorded file import metrics"
        );
    }

    /// 记录文件类型
    ///
    /// # 参数
    ///
    /// * `extension` - 文件扩展名（如 "log", "txt"）
    pub fn record_file_type(&self, extension: &str) {
        let mut types = self.file_types.write();
        let count = types.entry(extension.to_lowercase()).or_insert(0);
        *count += 1;
    }

    /// 获取当前文件指标
    pub fn get_metrics(&self) -> FileMetrics {
        let total_ops = self.import_operations.load(Ordering::Relaxed);
        let total_duration = self.total_duration_ms.load(Ordering::Relaxed);
        let successful = self.successful_imports.load(Ordering::Relaxed);
        let failed = self.failed_imports.load(Ordering::Relaxed);
        let total_attempts = successful + failed;

        // 计算平均导入耗时
        let average_duration = if total_ops > 0 {
            total_duration / total_ops
        } else {
            0
        };

        // 计算成功率
        let success_rate = if total_attempts > 0 {
            (successful as f64 / total_attempts as f64) * 100.0
        } else {
            100.0
        };

        // 计算导入速率
        let start_time = self.import_start_time.read();
        let (import_rate, throughput) = if let Some(start) = *start_time {
            let elapsed_secs = start.elapsed().as_secs_f64();
            if elapsed_secs > 0.0 {
                let rate = self.total_files.load(Ordering::Relaxed) as f64 / elapsed_secs;
                let mbps = (self.total_bytes.load(Ordering::Relaxed) as f64 / 1024.0 / 1024.0)
                    / elapsed_secs;
                (rate, mbps)
            } else {
                (0.0, 0.0)
            }
        } else {
            (0.0, 0.0)
        };

        FileMetrics {
            total_imported_files: self.total_files.load(Ordering::Relaxed),
            total_imported_bytes: self.total_bytes.load(Ordering::Relaxed),
            import_operations: total_ops,
            average_import_duration_ms: average_duration,
            file_type_distribution: self.file_types.read().clone(),
            import_success_rate: success_rate,
            current_import_rate: import_rate,
            import_throughput_mbps: throughput,
        }
    }

    /// 重置指标
    pub fn reset(&self) {
        self.total_files.store(0, Ordering::Relaxed);
        self.total_bytes.store(0, Ordering::Relaxed);
        self.import_operations.store(0, Ordering::Relaxed);
        self.successful_imports.store(0, Ordering::Relaxed);
        self.failed_imports.store(0, Ordering::Relaxed);
        self.total_duration_ms.store(0, Ordering::Relaxed);
        self.file_types.write().clear();
        *self.import_start_time.write() = None;
        info!("File metrics reset");
    }
}

impl Default for FileMetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 索引指标收集器
// ============================================================================

/// 索引操作指标收集器
///
/// 跟踪索引构建和更新操作的统计信息。
pub struct IndexMetricsCollector {
    /// 索引构建次数
    build_count: AtomicU64,
    /// 总构建时间（毫秒）
    total_build_time_ms: AtomicU64,
    /// 索引大小（字节）
    index_size_bytes: AtomicU64,
    /// 已索引文件数
    indexed_files: AtomicU64,
    /// 索引更新次数
    update_count: AtomicU64,
    /// 最后更新时间
    last_update_time: RwLock<Option<Instant>>,
    /// 索引段数量
    segment_count: AtomicU64,
    /// 第一次构建时间（用于计算更新频率）
    first_build_time: RwLock<Option<Instant>>,
}

impl IndexMetricsCollector {
    /// 创建新的索引指标收集器
    pub fn new() -> Self {
        Self {
            build_count: AtomicU64::new(0),
            total_build_time_ms: AtomicU64::new(0),
            index_size_bytes: AtomicU64::new(0),
            indexed_files: AtomicU64::new(0),
            update_count: AtomicU64::new(0),
            last_update_time: RwLock::new(None),
            segment_count: AtomicU64::new(0),
            first_build_time: RwLock::new(None),
        }
    }

    /// 记录一次索引构建操作
    ///
    /// # 参数
    ///
    /// * `duration_ms` - 构建耗时（毫秒）
    /// * `files_indexed` - 索引的文件数量
    /// * `index_size` - 索引大小（字节）
    pub fn record_build(&self, duration_ms: u64, files_indexed: u64, index_size: u64) {
        self.build_count.fetch_add(1, Ordering::Relaxed);
        self.total_build_time_ms
            .fetch_add(duration_ms, Ordering::Relaxed);
        self.indexed_files
            .fetch_add(files_indexed, Ordering::Relaxed);
        self.index_size_bytes.store(index_size, Ordering::Relaxed);

        // 记录首次构建时间
        let mut first_time = self.first_build_time.write();
        if first_time.is_none() {
            *first_time = Some(Instant::now());
        }

        // 记录更新时间
        *self.last_update_time.write() = Some(Instant::now());

        debug!(
            duration_ms,
            files_indexed, index_size, "Recorded index build metrics"
        );
    }

    /// 记录一次索引更新操作
    pub fn record_update(&self) {
        self.update_count.fetch_add(1, Ordering::Relaxed);
        *self.last_update_time.write() = Some(Instant::now());
        debug!("Recorded index update");
    }

    /// 更新索引大小
    pub fn update_index_size(&self, size_bytes: u64) {
        self.index_size_bytes.store(size_bytes, Ordering::Relaxed);
    }

    /// 更新索引段数量
    pub fn update_segment_count(&self, count: u64) {
        self.segment_count.store(count, Ordering::Relaxed);
    }

    /// 获取当前索引指标
    pub fn get_metrics(&self) -> IndexMetrics {
        let build_count = self.build_count.load(Ordering::Relaxed);
        let total_build_time = self.total_build_time_ms.load(Ordering::Relaxed);
        let update_count = self.update_count.load(Ordering::Relaxed);

        // 计算平均构建时间
        let average_build_time = if build_count > 0 {
            total_build_time / build_count
        } else {
            0
        };

        // 计算更新频率（次/分钟）
        let first_time = self.first_build_time.read();
        let frequency = if let Some(start) = *first_time {
            let elapsed_mins = start.elapsed().as_secs_f64() / 60.0;
            if elapsed_mins > 0.0 {
                update_count as f64 / elapsed_mins
            } else {
                0.0
            }
        } else {
            0.0
        };

        // 获取最后更新时间戳
        let last_update = self
            .last_update_time
            .read()
            .map(|t| t.elapsed().as_secs() as i64);

        IndexMetrics {
            build_count,
            total_build_time_ms: total_build_time,
            average_build_time_ms: average_build_time,
            index_size_bytes: self.index_size_bytes.load(Ordering::Relaxed),
            indexed_files: self.indexed_files.load(Ordering::Relaxed),
            update_count,
            update_frequency_per_min: frequency,
            last_update_timestamp: last_update,
            segment_count: self.segment_count.load(Ordering::Relaxed),
        }
    }

    /// 重置指标
    pub fn reset(&self) {
        self.build_count.store(0, Ordering::Relaxed);
        self.total_build_time_ms.store(0, Ordering::Relaxed);
        self.index_size_bytes.store(0, Ordering::Relaxed);
        self.indexed_files.store(0, Ordering::Relaxed);
        self.update_count.store(0, Ordering::Relaxed);
        *self.last_update_time.write() = None;
        self.segment_count.store(0, Ordering::Relaxed);
        *self.first_build_time.write() = None;
        info!("Index metrics reset");
    }
}

impl Default for IndexMetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 任务指标收集器
// ============================================================================

/// 任务管理器指标收集器
///
/// 跟踪任务执行情况的统计信息。
pub struct TaskMetricsCollector {
    /// 总任务数
    total_tasks: AtomicU64,
    /// 运行中任务数
    running_tasks: AtomicU64,
    /// 已完成任务数
    completed_tasks: AtomicU64,
    /// 失败任务数
    failed_tasks: AtomicU64,
    /// 总任务执行时间（毫秒）
    total_task_duration_ms: AtomicU64,
    /// 任务执行历史
    task_durations: RwLock<RingBuffer<u64>>,
}

impl TaskMetricsCollector {
    /// 创建新的任务指标收集器
    pub fn new() -> Self {
        Self {
            total_tasks: AtomicU64::new(0),
            running_tasks: AtomicU64::new(0),
            completed_tasks: AtomicU64::new(0),
            failed_tasks: AtomicU64::new(0),
            total_task_duration_ms: AtomicU64::new(0),
            task_durations: RwLock::new(RingBuffer::new(PERCENTILE_WINDOW_SIZE)),
        }
    }

    /// 记录任务创建
    pub fn record_task_created(&self) {
        self.total_tasks.fetch_add(1, Ordering::Relaxed);
        self.running_tasks.fetch_add(1, Ordering::Relaxed);
    }

    /// 记录任务完成
    ///
    /// # 参数
    ///
    /// * `duration_ms` - 任务执行时间（毫秒）
    /// * `success` - 是否成功完成
    pub fn record_task_completed(&self, duration_ms: u64, success: bool) {
        self.running_tasks.fetch_sub(1, Ordering::Relaxed);

        if success {
            self.completed_tasks.fetch_add(1, Ordering::Relaxed);
        } else {
            self.failed_tasks.fetch_add(1, Ordering::Relaxed);
        }

        self.total_task_duration_ms
            .fetch_add(duration_ms, Ordering::Relaxed);
        self.task_durations.write().push(duration_ms);

        debug!(duration_ms, success, "Recorded task completion");
    }

    /// 获取当前任务指标
    pub fn get_metrics(&self) -> TaskMetrics {
        let completed = self.completed_tasks.load(Ordering::Relaxed);
        let failed = self.failed_tasks.load(Ordering::Relaxed);
        let total_duration = self.total_task_duration_ms.load(Ordering::Relaxed);
        let total_completed = completed + failed;

        let average_duration = if total_completed > 0 {
            total_duration / total_completed
        } else {
            0
        };

        TaskMetrics {
            total_tasks: self.total_tasks.load(Ordering::Relaxed),
            running_tasks: self.running_tasks.load(Ordering::Relaxed),
            completed_tasks: completed,
            failed_tasks: failed,
            average_task_duration_ms: average_duration,
        }
    }

    /// 重置指标
    pub fn reset(&self) {
        self.total_tasks.store(0, Ordering::Relaxed);
        self.running_tasks.store(0, Ordering::Relaxed);
        self.completed_tasks.store(0, Ordering::Relaxed);
        self.failed_tasks.store(0, Ordering::Relaxed);
        self.total_task_duration_ms.store(0, Ordering::Relaxed);
        self.task_durations.write().clear();
        info!("Task metrics reset");
    }
}

impl Default for TaskMetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 主收集器
// ============================================================================

/// 监控指标主收集器
///
/// 统一管理所有子收集器，提供完整的监控功能。
///
/// # 使用示例
///
/// ```rust
/// use log_analyzer::monitoring::MetricsCollector;
///
/// // 创建收集器
/// let collector = MetricsCollector::new();
///
/// // 记录搜索操作
/// collector.record_search(100, 50, false);
///
/// // 获取当前指标
/// let metrics = collector.get_metrics();
/// println!("平均搜索延迟: {} ms", metrics.search.average_latency_ms);
/// ```
pub struct MetricsCollector {
    /// 搜索指标收集器
    search: Arc<SearchMetricsCollector>,
    /// 内存指标收集器
    memory: Arc<MemoryMetricsCollector>,
    /// 文件指标收集器
    file: Arc<FileMetricsCollector>,
    /// 索引指标收集器
    index: Arc<IndexMetricsCollector>,
    /// 任务指标收集器
    task: Arc<TaskMetricsCollector>,
    /// 历史数据环形缓冲区
    history: RwLock<RingBuffer<MetricsDataPoint>>,
    /// 启动时间
    start_time: Instant,
}

impl MetricsCollector {
    /// 创建新的监控指标收集器
    pub fn new() -> Self {
        Self {
            search: Arc::new(SearchMetricsCollector::new()),
            memory: Arc::new(MemoryMetricsCollector::new()),
            file: Arc::new(FileMetricsCollector::new()),
            index: Arc::new(IndexMetricsCollector::new()),
            task: Arc::new(TaskMetricsCollector::new()),
            history: RwLock::new(RingBuffer::new(RING_BUFFER_CAPACITY)),
            start_time: Instant::now(),
        }
    }

    // ========================================================================
    // 搜索指标 API
    // ========================================================================

    /// 记录一次搜索操作
    ///
    /// # 参数
    ///
    /// * `latency_ms` - 搜索延迟（毫秒）
    /// * `results_count` - 结果数量
    /// * `cache_hit` - 是否命中缓存
    pub fn record_search(&self, latency_ms: u64, results_count: u64, cache_hit: bool) {
        self.search
            .record_search(latency_ms, results_count, cache_hit);
    }

    /// 获取搜索指标收集器引用
    pub fn search_collector(&self) -> Arc<SearchMetricsCollector> {
        Arc::clone(&self.search)
    }

    // ========================================================================
    // 内存指标 API
    // ========================================================================

    /// 采样当前内存使用情况
    pub fn sample_memory(&self) -> MemoryMetrics {
        self.memory.sample()
    }

    /// 获取内存指标收集器引用
    pub fn memory_collector(&self) -> Arc<MemoryMetricsCollector> {
        Arc::clone(&self.memory)
    }

    // ========================================================================
    // 文件指标 API
    // ========================================================================

    /// 记录一次文件导入操作
    ///
    /// # 参数
    ///
    /// * `file_count` - 导入的文件数量
    /// * `bytes` - 导入的字节数
    /// * `duration_ms` - 导入耗时（毫秒）
    /// * `success` - 是否成功
    pub fn record_import(&self, file_count: u64, bytes: u64, duration_ms: u64, success: bool) {
        self.file
            .record_import(file_count, bytes, duration_ms, success);
    }

    /// 记录文件类型
    pub fn record_file_type(&self, extension: &str) {
        self.file.record_file_type(extension);
    }

    /// 获取文件指标收集器引用
    pub fn file_collector(&self) -> Arc<FileMetricsCollector> {
        Arc::clone(&self.file)
    }

    // ========================================================================
    // 索引指标 API
    // ========================================================================

    /// 记录一次索引构建操作
    ///
    /// # 参数
    ///
    /// * `duration_ms` - 构建耗时（毫秒）
    /// * `files_indexed` - 索引的文件数量
    /// * `index_size` - 索引大小（字节）
    pub fn record_index_build(&self, duration_ms: u64, files_indexed: u64, index_size: u64) {
        self.index
            .record_build(duration_ms, files_indexed, index_size);
    }

    /// 记录一次索引更新操作
    pub fn record_index_update(&self) {
        self.index.record_update();
    }

    /// 更新索引大小
    pub fn update_index_size(&self, size_bytes: u64) {
        self.index.update_index_size(size_bytes);
    }

    /// 获取索引指标收集器引用
    pub fn index_collector(&self) -> Arc<IndexMetricsCollector> {
        Arc::clone(&self.index)
    }

    // ========================================================================
    // 任务指标 API
    // ========================================================================

    /// 记录任务创建
    pub fn record_task_created(&self) {
        self.task.record_task_created();
    }

    /// 记录任务完成
    ///
    /// # 参数
    ///
    /// * `duration_ms` - 任务执行时间（毫秒）
    /// * `success` - 是否成功完成
    pub fn record_task_completed(&self, duration_ms: u64, success: bool) {
        self.task.record_task_completed(duration_ms, success);
    }

    /// 获取任务指标收集器引用
    pub fn task_collector(&self) -> Arc<TaskMetricsCollector> {
        Arc::clone(&self.task)
    }

    // ========================================================================
    // 综合指标 API
    // ========================================================================

    /// 获取完整的监控指标
    ///
    /// 返回包含所有子系统指标的综合数据结构。
    pub fn get_metrics(&self) -> MonitoringMetrics {
        // 采样内存（确保数据最新）
        let memory_metrics = self.memory.sample();

        MonitoringMetrics {
            search: self.search.get_metrics(),
            memory: memory_metrics,
            file: self.file.get_metrics(),
            index: self.index.get_metrics(),
            task: self.task.get_metrics(),
            timestamp: chrono::Utc::now().timestamp_millis(),
        }
    }

    /// 记录当前指标到历史数据
    ///
    /// 将当前指标保存到环形缓冲区，用于趋势分析。
    pub fn record_snapshot(&self) {
        let metrics = self.get_metrics();
        let data_point = MetricsDataPoint {
            timestamp: metrics.timestamp,
            search_latency_ms: metrics.search.average_latency_ms,
            search_throughput: metrics.search.current_throughput,
            memory_used_mb: metrics.memory.current_used_mb,
            imported_files: metrics.file.total_imported_files,
            index_size_bytes: metrics.index.index_size_bytes,
        };

        self.history.write().push(data_point);
        debug!("Recorded metrics snapshot to history");
    }

    /// 获取历史指标数据
    ///
    /// 返回环形缓冲区中的所有历史数据点。
    pub fn get_history(&self) -> Vec<MetricsDataPoint> {
        self.history.read().get_all()
    }

    /// 获取最近 N 个历史数据点
    pub fn get_recent_history(&self, n: usize) -> Vec<MetricsDataPoint> {
        self.history.read().get_recent(n)
    }

    /// 获取运行时间（秒）
    pub fn get_uptime_secs(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    /// 重置所有指标
    pub fn reset_all(&self) {
        self.search.reset();
        self.memory.reset();
        self.file.reset();
        self.index.reset();
        self.task.reset();
        self.history.write().clear();
        info!("All metrics reset");
    }

    /// 启动后台监控任务
    ///
    /// 启动一个异步任务，定期采样内存和记录指标快照。
    ///
    /// # 参数
    ///
    /// * `interval_secs` - 采样间隔（秒）
    pub fn start_monitoring(self: &Arc<Self>, interval_secs: u64) {
        let collector = Arc::clone(self);
        let interval_duration = Duration::from_secs(interval_secs);

        tokio::spawn(async move {
            let mut ticker = interval(interval_duration);

            loop {
                ticker.tick().await;

                // 采样内存
                collector.sample_memory();

                // 记录快照
                collector.record_snapshot();

                debug!("Background monitoring tick completed");
            }
        });

        info!(interval_secs, "Started background monitoring task");
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 工具函数
// ============================================================================

/// 计算百分位数
///
/// 使用标准排序算法计算 P95 和 P99 百分位数。
/// 采用线性插值法计算百分位位置，确保结果准确。
///
/// # 参数
///
/// * `values` - 数值数组
///
/// # 返回
///
/// 返回 (P95, P99) 元组
fn calculate_percentiles(values: &[u64]) -> (u64, u64) {
    if values.is_empty() {
        return (0, 0);
    }

    let mut sorted = values.to_vec();
    sorted.sort_unstable();

    // 使用线性插值法计算百分位位置
    let p95_idx = ((sorted.len() - 1) as f64 * 0.95).round() as usize;
    let p99_idx = ((sorted.len() - 1) as f64 * 0.99).round() as usize;

    (
        sorted[p95_idx.min(sorted.len() - 1)],
        sorted[p99_idx.min(sorted.len() - 1)],
    )
}

// ============================================================================
// 模块测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ring_buffer() {
        let mut buffer = RingBuffer::new(3);

        // 添加元素
        buffer.push(1);
        buffer.push(2);
        buffer.push(3);

        assert_eq!(buffer.len(), 3);

        // 覆盖旧元素
        buffer.push(4);

        assert_eq!(buffer.len(), 3);

        let all = buffer.get_all();
        assert_eq!(all, vec![2, 3, 4]);

        // 获取最近元素
        let recent = buffer.get_recent(2);
        assert_eq!(recent, vec![3, 4]);
    }

    #[test]
    fn test_search_metrics_collector() {
        let collector = SearchMetricsCollector::new();

        // 记录搜索
        collector.record_search(100, 50, false);
        collector.record_search(200, 30, true);
        collector.record_search(150, 40, false);

        let metrics = collector.get_metrics();

        assert_eq!(metrics.total_searches, 3);
        assert_eq!(metrics.current_latency_ms, 150);
        assert_eq!(metrics.average_latency_ms, 150);
        assert_eq!(metrics.max_latency_ms, 200);
        assert_eq!(metrics.min_latency_ms, 100);
        assert_eq!(metrics.total_results, 120);
        assert!(metrics.cache_hit_rate > 0.0);
    }

    #[test]
    fn test_file_metrics_collector() {
        let collector = FileMetricsCollector::new();

        // 记录导入
        collector.record_import(10, 1024 * 1024, 1000, true);
        collector.record_import(5, 512 * 1024, 500, true);
        collector.record_file_type("log");
        collector.record_file_type("txt");
        collector.record_file_type("log");

        let metrics = collector.get_metrics();

        assert_eq!(metrics.total_imported_files, 15);
        assert_eq!(metrics.import_operations, 2);
        assert_eq!(metrics.average_import_duration_ms, 750);
        assert_eq!(metrics.file_type_distribution.get("log"), Some(&2));
        assert_eq!(metrics.file_type_distribution.get("txt"), Some(&1));
    }

    #[test]
    fn test_index_metrics_collector() {
        let collector = IndexMetricsCollector::new();

        // 记录构建
        collector.record_build(5000, 1000, 1024 * 1024 * 10);
        collector.record_build(3000, 500, 1024 * 1024 * 5);
        collector.record_update();
        collector.record_update();

        let metrics = collector.get_metrics();

        assert_eq!(metrics.build_count, 2);
        assert_eq!(metrics.total_build_time_ms, 8000);
        assert_eq!(metrics.average_build_time_ms, 4000);
        assert_eq!(metrics.indexed_files, 1500);
        assert_eq!(metrics.update_count, 2);
    }

    #[test]
    fn test_task_metrics_collector() {
        let collector = TaskMetricsCollector::new();

        // 记录任务
        collector.record_task_created();
        collector.record_task_created();
        collector.record_task_completed(1000, true);
        collector.record_task_completed(2000, false);

        let metrics = collector.get_metrics();

        assert_eq!(metrics.total_tasks, 2);
        assert_eq!(metrics.running_tasks, 0);
        assert_eq!(metrics.completed_tasks, 1);
        assert_eq!(metrics.failed_tasks, 1);
        assert_eq!(metrics.average_task_duration_ms, 1500);
    }

    #[test]
    fn test_metrics_collector() {
        let collector = MetricsCollector::new();

        // 记录各种操作
        collector.record_search(100, 50, false);
        collector.record_import(10, 1024 * 1024, 1000, true);
        collector.record_file_type("log");
        collector.record_index_build(5000, 1000, 1024 * 1024 * 10);
        collector.record_task_created();
        collector.record_task_completed(1000, true);

        // 获取指标
        let metrics = collector.get_metrics();

        assert_eq!(metrics.search.total_searches, 1);
        assert_eq!(metrics.file.total_imported_files, 10);
        assert_eq!(metrics.index.build_count, 1);
        assert_eq!(metrics.task.total_tasks, 1);

        // 测试历史记录
        collector.record_snapshot();
        let history = collector.get_history();
        assert_eq!(history.len(), 1);
    }

    #[test]
    fn test_calculate_percentiles() {
        let values = vec![
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
        ];

        let (p95, p99) = calculate_percentiles(&values);

        // P95 应该是第 19 个元素（索引 18）
        assert_eq!(p95, 19);
        // P99 应该是第 20 个元素（索引 19）
        assert_eq!(p99, 20);
    }

    #[test]
    fn test_empty_percentiles() {
        let values: Vec<u64> = vec![];
        let (p95, p99) = calculate_percentiles(&values);
        assert_eq!(p95, 0);
        assert_eq!(p99, 0);
    }
}
