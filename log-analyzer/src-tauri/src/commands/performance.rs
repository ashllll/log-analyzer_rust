//! 性能监控命令
//!
//! 提供系统性能指标监控功能，包括：
//! - 搜索性能（延迟、吞吐量）
//! - 缓存命中率
//! - 内存使用情况
//! - 任务执行统计
//! - 索引状态
//!
//! # 前后端集成规范
//!
//! 为保持与 JavaScript camelCase 惯例一致，返回数据字段使用 camelCase 命名。

use crate::models::AppState;
use serde::{Deserialize, Serialize};
use tauri::{command, State};
use sysinfo::System;

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
    let average_latency = if total_searches > 0 {
        // 简化计算：使用当前延迟作为平均值（实际应使用累积平均值）
        current_latency
    } else {
        0
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
            p95: current_latency, // 简化：使用当前值作为 P95（实际应计算百分位）
            p99: current_latency, // 简化：使用当前值作为 P99（实际应计算百分位）
        },
        search_throughput: SearchThroughput {
            current: if last_duration.as_secs() > 0 {
                1000 / last_duration.as_secs()
            } else {
                0
            },
            average: total_searches, // 简化：使用总搜索次数
            peak: total_searches.max(1), // 简化：使用最大值
        },
        cache_metrics: CacheMetrics {
            hit_rate,
            miss_count: cache_stats.l1_miss_count,
            hit_count: cache_hits,
            size: cache_stats.estimated_size,
            capacity: 1000, // 默认缓存容量
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
