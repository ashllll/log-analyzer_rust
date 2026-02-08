//! 缓存管理命令
//!
//! 提供缓存管理和监控功能

use crate::models::AppState;
use crate::utils::cache_manager::{
    AccessPatternStats, CacheDashboardData, CacheHealthCheck, CacheMetricsSnapshot,
    CachePerformanceReport, CacheStatistics, CompressionStats, L2CacheConfig,
};
use tauri::{command, State};

/// 获取缓存统计信息
#[command]
pub async fn get_cache_statistics(state: State<'_, AppState>) -> Result<CacheStatistics, String> {
    Ok(state.get_cache_statistics())
}

/// 获取异步缓存统计信息
#[command]
pub async fn get_async_cache_statistics(
    state: State<'_, AppState>,
) -> Result<CacheStatistics, String> {
    Ok(state.get_async_cache_statistics().await)
}

/// 清理工作区缓存
#[command]
pub async fn invalidate_workspace_cache(
    #[allow(non_snake_case)] workspaceId: String,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    state.invalidate_workspace_cache(&workspaceId)
}

/// 清理过期缓存条目
#[command]
pub async fn cleanup_expired_cache(state: State<'_, AppState>) -> Result<(), String> {
    // 清理同步缓存
    state.cleanup_expired_entries()?;

    // 清理异步缓存
    state.cleanup_expired_entries_async().await
}

/// 获取缓存性能指标
#[command]
pub async fn get_cache_performance_metrics(
    state: State<'_, AppState>,
) -> Result<CacheMetricsSnapshot, String> {
    Ok(state.get_cache_performance_metrics())
}

/// 获取缓存性能报告
#[command]
pub async fn get_cache_performance_report(
    state: State<'_, AppState>,
) -> Result<CachePerformanceReport, String> {
    Ok(state.get_cache_performance_report())
}

/// 执行缓存健康检查
#[command]
pub async fn cache_health_check(state: State<'_, AppState>) -> Result<CacheHealthCheck, String> {
    Ok(state.cache_health_check().await)
}

/// 获取访问模式统计
#[command]
pub async fn get_access_pattern_stats(
    state: State<'_, AppState>,
) -> Result<AccessPatternStats, String> {
    Ok(state.get_access_pattern_stats())
}

/// 获取压缩统计信息
#[command]
pub async fn get_compression_stats(state: State<'_, AppState>) -> Result<CompressionStats, String> {
    Ok(state.get_compression_stats())
}

/// 获取 L2 缓存配置
#[command]
pub async fn get_l2_cache_config(state: State<'_, AppState>) -> Result<L2CacheConfig, String> {
    Ok(state.get_l2_cache_config())
}

/// 智能缓存驱逐
#[command]
pub async fn intelligent_cache_eviction(
    target_reduction_percent: f64,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    state
        .intelligent_cache_eviction(target_reduction_percent)
        .await
}

/// 重置缓存性能指标
#[command]
pub async fn reset_cache_metrics(state: State<'_, AppState>) -> Result<(), String> {
    state.reset_cache_metrics();
    Ok(())
}

/// 重置访问模式追踪器
#[command]
pub async fn reset_access_tracker(state: State<'_, AppState>) -> Result<(), String> {
    state.reset_access_tracker();
    Ok(())
}

/// 获取缓存仪表板数据
#[command]
pub async fn get_cache_dashboard_data(
    state: State<'_, AppState>,
) -> Result<CacheDashboardData, String> {
    Ok(state.get_cache_dashboard_data())
}
