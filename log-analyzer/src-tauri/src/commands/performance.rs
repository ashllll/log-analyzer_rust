//! 性能监控命令
//!
//! 提供性能指标查询、告警获取和优化建议的 Tauri 命令

use tauri::{command, State};

use crate::models::AppState;
use crate::monitoring::alerting::Alert;
use crate::monitoring::metrics_collector::{
    CacheMetricsSnapshot, QueryTimingStats, SystemResourceMetrics,
};

/// 性能指标摘要
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PerformanceMetricsSummary {
    /// 查询时间统计
    pub query_stats: QueryTimingStats,
    /// 缓存性能指标
    pub cache_metrics: CacheMetricsSnapshot,
    /// 系统资源指标
    pub system_metrics: Option<SystemResourceMetrics>,
    /// 状态同步统计
    pub state_sync_stats: StateSyncStats,
}

/// 状态同步统计
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct StateSyncStats {
    pub total_operations: u64,
    pub success_count: u64,
    pub failure_count: u64,
    pub success_rate: f64,
    pub avg_latency_ms: f64,
}

/// 获取当前性能指标
#[command]
pub async fn get_performance_metrics(
    state: State<'_, AppState>,
) -> Result<PerformanceMetricsSummary, String> {
    // 获取查询时间统计
    let query_stats = state.metrics_collector.get_query_timing_stats();

    // 获取缓存指标
    let cache_metrics = state.cache_manager.get_performance_metrics();

    // 获取系统资源指标
    let system_metrics = state.metrics_collector.get_current_system_metrics();

    // 获取状态同步统计
    let state_sync_stats = get_state_sync_stats(&state);

    Ok(PerformanceMetricsSummary {
        query_stats,
        cache_metrics,
        system_metrics,
        state_sync_stats,
    })
}

/// 获取性能告警列表
#[command]
pub async fn get_performance_alerts(
    state: State<'_, AppState>,
    limit: Option<usize>,
) -> Result<Vec<Alert>, String> {
    let limit = limit.unwrap_or(50);
    let mut alerts = state.alerting_system.get_active_alerts();

    // 按时间戳降序排序（最新的在前）
    alerts.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    // 限制返回数量
    alerts.truncate(limit);

    Ok(alerts)
}

/// 获取优化建议
#[command]
pub async fn get_performance_recommendations(
    state: State<'_, AppState>,
    limit: Option<usize>,
) -> Result<Vec<String>, String> {
    let limit = limit.unwrap_or(10);

    // 收集当前性能数据
    let query_stats = state.metrics_collector.get_query_timing_stats();
    let cache_metrics = state.cache_manager.get_performance_metrics();
    let system_metrics = state.metrics_collector.get_current_system_metrics();

    // 生成简单的优化建议
    let mut recommendations = Vec::new();

    // 基于查询性能的建议
    if query_stats.avg_total_ms > 200.0 {
        recommendations.push(format!(
            "搜索平均响应时间 {:.2}ms 超过阈值 200ms，建议优化查询或增加索引",
            query_stats.avg_total_ms
        ));
    }

    // 基于缓存性能的建议
    if cache_metrics.l1_hit_rate < 0.7 {
        recommendations.push(format!(
            "缓存命中率 {:.2}% 低于 70%，建议增加缓存大小或优化缓存策略",
            cache_metrics.l1_hit_rate * 100.0
        ));
    }

    // 基于系统资源的建议
    if let Some(sys_metrics) = system_metrics {
        if sys_metrics.memory_usage_percent > 80.0 {
            recommendations.push(format!(
                "内存使用率 {:.2}% 超过 80%，建议释放内存或增加系统内存",
                sys_metrics.memory_usage_percent
            ));
        }
        if sys_metrics.cpu_usage_percent > 80.0 {
            recommendations.push(format!(
                "CPU 使用率 {:.2}% 超过 80%，建议优化计算密集型操作",
                sys_metrics.cpu_usage_percent
            ));
        }
    }

    if recommendations.is_empty() {
        recommendations.push("系统性能良好，无需优化".to_string());
    }

    Ok(recommendations.into_iter().take(limit).collect())
}

/// 重置性能指标
#[command]
pub async fn reset_performance_metrics(state: State<'_, AppState>) -> Result<(), String> {
    // 重置指标收集器
    state.metrics_collector.reset_metrics();

    // 重置缓存指标
    state.cache_manager.reset_metrics();

    Ok(())
}

/// 获取状态同步统计
fn get_state_sync_stats(state: &AppState) -> StateSyncStats {
    let metrics = state.metrics_collector.get_current_metrics();

    // 从指标中提取状态同步统计
    let total_operations = metrics
        .get("counter_state_sync_operations_total:{:?}")
        .and_then(|v| v.get("value"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0) as u64;

    let success_count = metrics
        .get("counter_state_sync_success_total:{:?}")
        .and_then(|v| v.get("value"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0) as u64;

    let failure_count = metrics
        .get("counter_state_sync_failure_total:{:?}")
        .and_then(|v| v.get("value"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0) as u64;

    let success_rate = if total_operations > 0 {
        (success_count as f64 / total_operations as f64) * 100.0
    } else {
        0.0
    };

    // 计算平均延迟（简化版本，实际应该从直方图中计算）
    let avg_latency_ms = 5.0; // 占位值，实际应该从 histogram 中计算

    StateSyncStats {
        total_operations,
        success_count,
        failure_count,
        success_rate,
        avg_latency_ms,
    }
}
