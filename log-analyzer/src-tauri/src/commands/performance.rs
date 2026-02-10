//! 性能监控命令
//!
//! 提供系统性能指标监控功能，包括：
//! - 搜索性能（延迟、吞吐量）
//! - 缓存命中率
//! - 内存使用情况
//! - 任务执行统计
//! - 索引状态
//! - 历史数据查询
//!
//! # 前后端集成规范
//!
//! 为保持与 JavaScript camelCase 惯例一致，返回数据字段使用 camelCase 命名。
//!
//! # 技术选型
//!
//! - **P95/P99 计算**: 业内成熟的排序算法计算百分位数
//! - **历史数据存储**: SQLite 时序数据（metrics_store）
//! - **定时快照**: tokio::time::interval 异步定时器

use crate::models::AppState;
use crate::storage::{MetricsSnapshot, MetricsStore, MetricsStoreStats, SearchEvent, TimeRange};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::Path;
use std::sync::Arc;
use tauri::{command, State};
use sysinfo::System;
use tokio::sync::Mutex;

/// 全局指标存储实例
static METRICS_STORE: once_cell::sync::Lazy<Arc<Mutex<Option<MetricsStore>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(None)));

/// 搜索延迟历史（用于计算 P95/P99）
/// 保留最近 1000 次搜索的延迟数据
static SEARCH_LATENCIES: once_cell::sync::Lazy<Arc<Mutex<VecDeque<u64>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(VecDeque::with_capacity(1000))));

/// 初始化指标存储
pub async fn init_metrics_store(data_dir: &Path) -> Result<(), String> {
    let store = MetricsStore::new(data_dir)
        .await
        .map_err(|e| format!("Failed to initialize metrics store: {}", e))?;

    *METRICS_STORE.lock().await = Some(store);
    Ok(())
}

/// 记录搜索延迟（用于百分位数计算）
pub fn record_search_latency(latency_ms: u64) {
    let mut latencies = SEARCH_LATENCIES.blocking_lock();
    if latencies.len() >= 1000 {
        latencies.pop_front();
    }
    latencies.push_back(latency_ms);
}

/// 计算百分位数（业内成熟方案）
///
/// 使用标准排序算法计算百分位数，适合小到中等规模数据集。
/// 对于大规模数据集，应使用 TDigest 算法库（如 `tdigest` crate）。
fn calculate_percentile(values: &mut [u64], percentile: f64) -> u64 {
    if values.is_empty() {
        return 0;
    }

    values.sort_unstable();
    let index = ((values.len() - 1) as f64 * percentile / 100.0) as usize;
    values[index.min(values.len() - 1)]
}

/// 性能指标数据结构
///
/// 与前端的 PerformanceMetrics 类型完全匹配
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceMetrics {
    /// 搜索延迟指标
    pub search_latency: SearchLatency,
    /// 搜索吞吐量指标
    pub search_throughput: SearchThroughput,
    /// 缓存性能指标
    pub cache_metrics: CacheMetrics,
    /// 内存使用指标
    pub memory_metrics: MemoryMetrics,
    /// 任务执行指标
    pub task_metrics: TaskMetrics,
    /// 索引指标
    pub index_metrics: IndexMetrics,
}

/// 搜索延迟指标
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchLatency {
    /// 当前延迟 (ms)
    pub current: u64,
    /// 平均延迟 (ms)
    pub average: u64,
    /// 95分位延迟 (ms)
    pub p95: u64,
    /// 99分位延迟 (ms)
    pub p99: u64,
}

/// 搜索吞吐量指标
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchThroughput {
    /// 当前吞吐量 (次/秒)
    pub current: u64,
    /// 平均吞吐量 (次/秒)
    pub average: u64,
    /// 峰值吞吐量 (次/秒)
    pub peak: u64,
}

/// 缓存性能指标
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheMetrics {
    /// 命中率 (0-100)
    pub hit_rate: f64,
    /// 未命中次数
    pub miss_count: u64,
    /// 命中次数
    pub hit_count: u64,
    /// 当前缓存大小
    pub size: u64,
    /// 缓存容量
    pub capacity: u64,
    /// 驱逐次数
    pub evictions: u64,
}

/// 内存使用指标
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryMetrics {
    /// 已用内存 (MB)
    pub used: u64,
    /// 总内存 (MB)
    pub total: u64,
    /// 堆内存使用 (MB)
    pub heap_used: u64,
    /// 外部内存 (MB)
    pub external: u64,
}

/// 任务执行指标
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskMetrics {
    /// 总任务数
    pub total: u64,
    /// 运行中任务数
    pub running: u64,
    /// 已完成任务数
    pub completed: u64,
    /// 失败任务数
    pub failed: u64,
    /// 平均执行时间 (ms)
    pub average_duration: u64,
}

/// 索引指标
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexMetrics {
    /// 总文件数
    pub total_files: u64,
    /// 已索引文件数
    pub indexed_files: u64,
    /// 总大小 (bytes)
    pub total_size: u64,
    /// 索引大小 (bytes)
    pub index_size: u64,
}

/// 获取性能指标
///
/// 返回当前系统性能的完整监控数据。
///
/// # 返回
///
/// 返回性能指标数据
///
/// # 示例
///
/// ```typescript
/// const metrics = await invoke('get_performance_metrics');
/// console.log(metrics.searchLatency.current);
/// ```
#[command]
pub fn get_performance_metrics(state: State<'_, AppState>) -> Result<PerformanceMetrics, String> {
    // 获取缓存统计信息
    let cache_stats = state.get_cache_statistics();

    // 获取搜索统计数据
    let total_searches = *state.total_searches.lock();
    let cache_hits = *state.cache_hits.lock();
    let last_duration = *state.last_search_duration.lock();

    // 计算搜索延迟（毫秒）
    let current_latency = last_duration.as_millis() as u64;

    // 计算真实的 P95/P99 延迟（使用业内成熟的排序算法）
    // 使用 try_lock() 避免阻塞，如果失败则返回默认值
    let mut latencies: Vec<u64> = if let Ok(guard) = SEARCH_LATENCIES.try_lock() {
        guard.iter().copied().collect()
    } else {
        vec![]
    };
    let p95_latency = calculate_percentile(&mut latencies, 95.0);
    let p99_latency = calculate_percentile(&mut latencies, 99.0);

    // 计算平均延迟
    let average_latency = if !latencies.is_empty() {
        let sum: u64 = latencies.iter().sum();
        sum / latencies.len() as u64
    } else {
        current_latency
    };

    // 计算缓存命中率
    let total_requests = cache_hits + cache_stats.l1_miss_count;
    let hit_rate = if total_requests > 0 {
        (cache_hits as f64 / total_requests as f64) * 100.0
    } else {
        0.0
    };

    // 获取任务管理器指标
    let task_manager_metrics = get_task_manager_metrics(&state);

    // 获取系统内存信息
    let memory_metrics = get_system_memory_metrics();

    // 获取索引指标
    let index_metrics = get_index_metrics(&state);

    Ok(PerformanceMetrics {
        search_latency: SearchLatency {
            current: current_latency,
            average: average_latency,
            p95: p95_latency,
            p99: p99_latency,
        },
        search_throughput: SearchThroughput {
            current: if last_duration.as_secs() > 0 {
                1000 / last_duration.as_secs()
            } else {
                0
            },
            average: total_searches,
            peak: total_searches.max(1),
        },
        cache_metrics: CacheMetrics {
            hit_rate,
            miss_count: cache_stats.l1_miss_count,
            hit_count: cache_hits,
            size: cache_stats.estimated_size,
            capacity: 1000,
            evictions: cache_stats.eviction_count,
        },
        memory_metrics,
        task_metrics: task_manager_metrics,
        index_metrics,
    })
}

/// 获取任务管理器指标
fn get_task_manager_metrics(_state: &AppState) -> TaskMetrics {
    // 简化处理：返回默认值
    // TODO: 通过异步消息获取实际的任务管理器指标
    TaskMetrics {
        total: 0,
        running: 0,
        completed: 0,
        failed: 0,
        average_duration: 0,
    }
}

/// 获取系统内存指标
fn get_system_memory_metrics() -> MemoryMetrics {
    let mut sys = System::new_all();
    sys.refresh_all();

    let total_memory = sys.total_memory();
    let used_memory = sys.used_memory();

    MemoryMetrics {
        used: used_memory / 1024, // 转换为 MB
        total: total_memory / 1024, // 转换为 MB
        heap_used: used_memory / 1024, // 简化：与 used 相同
        external: 0, // 外部内存（对于 Rust 应用通常是 0）
    }
}

/// 获取索引指标
fn get_index_metrics(state: &AppState) -> IndexMetrics {
    // 从所有元数据存储中聚合索引信息
    let stores = state.metadata_stores.lock();
    let store_count = stores.len() as u64;

    // 简化处理：使用存储数量作为文件计数
    // TODO: 从 MetadataStore 获取实际的文件统计信息
    IndexMetrics {
        total_files: store_count,
        indexed_files: store_count,
        total_size: 0,
        index_size: 0,
    }
}

// ============================================================================
// 历史数据查询命令
// ============================================================================

/// 时间范围类型（与前端保持一致）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TimeRangeDto {
    LastHour,
    Last6Hours,
    Last24Hours,
    Last7Days,
    Last30Days,
}

impl From<TimeRangeDto> for TimeRange {
    fn from(value: TimeRangeDto) -> Self {
        match value {
            TimeRangeDto::LastHour => TimeRange::LastHour,
            TimeRangeDto::Last6Hours => TimeRange::Last6Hours,
            TimeRangeDto::Last24Hours => TimeRange::Last24Hours,
            TimeRangeDto::Last7Days => TimeRange::Last7Days,
            TimeRangeDto::Last30Days => TimeRange::Last30Days,
        }
    }
}

/// 历史指标数据响应
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoricalMetricsData {
    pub snapshots: Vec<MetricsSnapshot>,
    pub stats: MetricsStoreStats,
}

/// 获取历史指标数据
///
/// 返回指定时间范围内的性能指标快照。
///
/// # Arguments
///
/// * `range` - 时间范围
///
/// # 返回
///
/// 返回历史指标数据和统计信息
///
/// # 示例
///
/// ```typescript
/// const data = await invoke('get_historical_metrics', { range: 'Last24Hours' });
/// console.log(data.snapshots.length);
/// ```
#[command]
pub async fn get_historical_metrics(
    range: TimeRangeDto,
) -> Result<HistoricalMetricsData, String> {
    let store_guard = METRICS_STORE.lock().await;
    let store = store_guard
        .as_ref()
        .ok_or("Metrics store not initialized")?;

    let time_range = TimeRange::from(range);
    let snapshots = store
        .get_snapshots(time_range)
        .await
        .map_err(|e| e.to_string())?;

    let stats = store
        .get_stats()
        .await
        .map_err(|e| e.to_string())?;

    Ok(HistoricalMetricsData {
        snapshots,
        stats,
    })
}

/// 获取聚合指标数据
///
/// 返回按指定时间间隔聚合的性能指标，用于绘制趋势图。
///
/// # Arguments
///
/// * `range` - 时间范围
/// * `interval_seconds` - 聚合间隔（秒）
///
/// # 返回
///
/// 返回聚合后的指标数据
///
/// # 示例
///
/// ```typescript
/// // 获取过去24小时的数据，按5分钟间隔聚合
/// const data = await invoke('get_aggregated_metrics', {
///   range: 'Last24Hours',
///   intervalSeconds: 300
/// });
/// ```
#[command]
pub async fn get_aggregated_metrics(
    range: TimeRangeDto,
    interval_seconds: i64,
) -> Result<Vec<MetricsSnapshot>, String> {
    let store_guard = METRICS_STORE.lock().await;
    let store = store_guard
        .as_ref()
        .ok_or("Metrics store not initialized")?;

    let time_range = TimeRange::from(range);
    let snapshots = store
        .get_aggregated_metrics(time_range, interval_seconds)
        .await
        .map_err(|e| e.to_string())?;

    Ok(snapshots)
}

/// 获取搜索事件
///
/// 返回指定时间范围内的搜索事件记录。
///
/// # Arguments
///
/// * `range` - 时间范围
/// * `workspace_id` - 可选的工作区 ID 过滤
///
/// # 返回
///
/// 返回搜索事件列表
///
/// # 示例
///
/// ```typescript
/// const events = await invoke('get_search_events', {
///   range: 'Last24Hours',
///   workspaceId: 'workspace-123'
/// });
/// ```
#[command]
pub async fn get_search_events(
    range: TimeRangeDto,
    #[allow(non_snake_case)] workspaceId: Option<String>,
) -> Result<Vec<SearchEvent>, String> {
    let store_guard = METRICS_STORE.lock().await;
    let store = store_guard
        .as_ref()
        .ok_or("Metrics store not initialized")?;

    let time_range = TimeRange::from(range);
    let events = store
        .get_search_events(time_range, workspaceId.as_deref())
        .await
        .map_err(|e| e.to_string())?;

    Ok(events)
}

/// 获取指标存储统计信息
///
/// 返回指标存储的统计信息，包括快照数量、事件数量等。
///
/// # 返回
///
/// 返回统计信息
///
/// # 示例
///
/// ```typescript
/// const stats = await invoke('get_metrics_stats');
/// console.log(stats.snapshotCount);
/// ```
#[command]
pub async fn get_metrics_stats() -> Result<MetricsStoreStats, String> {
    let store_guard = METRICS_STORE.lock().await;
    let store = store_guard
        .as_ref()
        .ok_or("Metrics store not initialized")?;

    let stats = store
        .get_stats()
        .await
        .map_err(|e| e.to_string())?;

    Ok(stats)
}

/// 手动触发数据清理
///
/// 删除超过保留期的旧数据（7天）。
///
/// # 返回
///
/// 返回删除的快照和事件数量
///
/// # 示例
///
/// ```typescript
/// const result = await invoke('cleanup_metrics_data');
/// console.log(`Deleted ${result.deletedSnapshots} snapshots`);
/// ```
#[command]
pub async fn cleanup_metrics_data() -> Result<MetricsStoreStats, String> {
    let store_guard = METRICS_STORE.lock().await;
    let store = store_guard
        .as_ref()
        .ok_or("Metrics store not initialized")?;

    store
        .cleanup()
        .await
        .map_err(|e| e.to_string())?;

    let stats = store
        .get_stats()
        .await
        .map_err(|e| e.to_string())?;

    Ok(stats)
}
