//! 监控模块
//!
//! 提供完整的性能监控功能，包括：
//! - 搜索性能指标（延迟、吞吐量、结果数量）
//! - 内存使用监控（当前使用量、峰值、分配速率）
//! - 文件处理统计（导入数量、耗时、类型分布）
//! - 索引操作统计（构建时间、大小、更新频率）
//!
//! ## 使用示例
//!
//! ```rust
//! use log_analyzer::monitoring::MetricsCollector;
//!
//! // 创建收集器
//! let collector = MetricsCollector::new();
//!
//! // 记录搜索操作
//! collector.record_search(100, 50, false);
//!
//! // 获取当前指标
//! let metrics = collector.get_metrics();
//! println!("平均搜索延迟: {} ms", metrics.search.average_latency_ms);
//! ```

// 子模块
pub mod metrics;

// 公共 API 导出
pub use metrics::{
    // 数据结构
    FileMetrics,
    // 子收集器
    FileMetricsCollector,
    IndexMetrics,
    IndexMetricsCollector,
    MemoryMetrics,
    MemoryMetricsCollector,
    // 主收集器
    MetricsCollector,

    MetricsDataPoint,
    MonitoringMetrics,
    // 工具
    RingBuffer,
    SearchMetrics,
    SearchMetricsCollector,
    TaskMetrics,

    TaskMetricsCollector,
};

use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::sync::Arc;
use tracing::{debug, info};

/// 全局监控指标收集器实例
///
/// 使用 Lazy 初始化确保全局只有一个实例。
/// 使用 parking_lot::Mutex 保护，支持跨线程安全访问。
static GLOBAL_COLLECTOR: Lazy<Mutex<Option<Arc<MetricsCollector>>>> =
    Lazy::new(|| Mutex::new(None));

/// 初始化全局监控收集器
///
/// # 参数
///
/// * `auto_monitoring` - 是否启动后台自动监控任务
/// * `interval_secs` - 后台监控采样间隔（秒），默认 60 秒
///
/// # 示例
///
/// ```rust
/// use log_analyzer::monitoring;
///
/// // 初始化并启动后台监控（每 60 秒采样一次）
/// monitoring::init_global_collector(true, 60);
/// ```
pub fn init_global_collector(auto_monitoring: bool, interval_secs: u64) {
    let mut guard = GLOBAL_COLLECTOR.lock();

    if guard.is_none() {
        let collector = Arc::new(MetricsCollector::new());

        if auto_monitoring {
            collector.start_monitoring(interval_secs);
        }

        *guard = Some(collector);
        info!(
            auto_monitoring,
            interval_secs, "Global metrics collector initialized"
        );
    } else {
        debug!("Global metrics collector already initialized");
    }
}

/// 获取全局监控收集器
///
/// 返回全局收集器的引用。如果尚未初始化，返回 None。
///
/// # 示例
///
/// ```rust
/// use log_analyzer::monitoring;
///
/// if let Some(collector) = monitoring::get_global_collector() {
///     let metrics = collector.get_metrics();
///     println!("内存使用: {} MB", metrics.memory.current_used_mb);
/// }
/// ```
pub fn get_global_collector() -> Option<Arc<MetricsCollector>> {
    GLOBAL_COLLECTOR.lock().clone()
}

/// 记录搜索操作（使用全局收集器）
///
/// # 参数
///
/// * `latency_ms` - 搜索延迟（毫秒）
/// * `results_count` - 结果数量
/// * `cache_hit` - 是否命中缓存
///
/// # 注意
///
/// 如果全局收集器未初始化，此操作会被静默忽略。
pub fn record_search(latency_ms: u64, results_count: u64, cache_hit: bool) {
    if let Some(collector) = get_global_collector() {
        collector.record_search(latency_ms, results_count, cache_hit);
    }
}

/// 记录文件导入操作（使用全局收集器）
///
/// # 参数
///
/// * `file_count` - 导入的文件数量
/// * `bytes` - 导入的字节数
/// * `duration_ms` - 导入耗时（毫秒）
/// * `success` - 是否成功
///
/// # 注意
///
/// 如果全局收集器未初始化，此操作会被静默忽略。
pub fn record_import(file_count: u64, bytes: u64, duration_ms: u64, success: bool) {
    if let Some(collector) = get_global_collector() {
        collector.record_import(file_count, bytes, duration_ms, success);
    }
}

/// 记录索引构建操作（使用全局收集器）
///
/// # 参数
///
/// * `duration_ms` - 构建耗时（毫秒）
/// * `files_indexed` - 索引的文件数量
/// * `index_size` - 索引大小（字节）
///
/// # 注意
///
/// 如果全局收集器未初始化，此操作会被静默忽略。
pub fn record_index_build(duration_ms: u64, files_indexed: u64, index_size: u64) {
    if let Some(collector) = get_global_collector() {
        collector.record_index_build(duration_ms, files_indexed, index_size);
    }
}

/// 记录任务创建（使用全局收集器）
///
/// # 注意
///
/// 如果全局收集器未初始化，此操作会被静默忽略。
pub fn record_task_created() {
    if let Some(collector) = get_global_collector() {
        collector.record_task_created();
    }
}

/// 记录任务完成（使用全局收集器）
///
/// # 参数
///
/// * `duration_ms` - 任务执行时间（毫秒）
/// * `success` - 是否成功完成
///
/// # 注意
///
/// 如果全局收集器未初始化，此操作会被静默忽略。
pub fn record_task_completed(duration_ms: u64, success: bool) {
    if let Some(collector) = get_global_collector() {
        collector.record_task_completed(duration_ms, success);
    }
}

/// 获取当前监控指标（使用全局收集器）
///
/// # 返回
///
/// 如果全局收集器已初始化，返回当前指标；否则返回 None。
pub fn get_current_metrics() -> Option<MonitoringMetrics> {
    get_global_collector().map(|c| c.get_metrics())
}

/// 重置所有监控指标（使用全局收集器）
///
/// # 注意
///
/// 如果全局收集器未初始化，此操作会被静默忽略。
pub fn reset_all_metrics() {
    if let Some(collector) = get_global_collector() {
        collector.reset_all();
    }
}

// ============================================================================
// 模块测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_collector() {
        // 初始化全局收集器
        init_global_collector(false, 60);

        // 获取收集器
        let collector = get_global_collector();
        assert!(collector.is_some());

        // 记录一些操作
        record_search(100, 50, false);
        record_import(10, 1024 * 1024, 1000, true);
        record_index_build(5000, 1000, 1024 * 1024 * 10);
        record_task_created();
        record_task_completed(1000, true);

        // 获取指标
        let metrics = get_current_metrics();
        assert!(metrics.is_some());

        let m = metrics.unwrap();
        assert_eq!(m.search.total_searches, 1);
        assert_eq!(m.file.total_imported_files, 10);
        assert_eq!(m.index.build_count, 1);
        assert_eq!(m.task.total_tasks, 1);
    }
}
