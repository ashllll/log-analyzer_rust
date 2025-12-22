//! 缓存管理命令
//!
//! 提供缓存管理和监控功能

use crate::models::AppState;
use crate::utils::cache_manager::{
    CacheDashboardData, CacheHealthCheck, CacheMetricsSnapshot, CachePerformanceReport,
};
use crate::utils::{AccessPatternStats, CacheStatistics, CompressionStats, L2CacheConfig};
use tauri::{command, State};

/// 获取缓存统计信息
#[command]
pub async fn get_cache_statistics(state: State<'_, AppState>) -> Result<CacheStatistics, String> {
    Ok(state.cache_manager.get_cache_statistics())
}

/// 获取异步缓存统计信息
#[command]
pub async fn get_async_cache_statistics(
    state: State<'_, AppState>,
) -> Result<CacheStatistics, String> {
    Ok(state.cache_manager.get_async_cache_statistics().await)
}

/// 清理工作区缓存
#[command]
pub async fn invalidate_workspace_cache(
    #[allow(non_snake_case)] workspaceId: String,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    state
        .cache_manager
        .invalidate_workspace_cache(&workspaceId)
        .map_err(|e| e.to_string())
}

/// 清理过期缓存条目
#[command]
pub async fn cleanup_expired_cache(state: State<'_, AppState>) -> Result<(), String> {
    // 清理同步缓存
    state
        .cache_manager
        .cleanup_expired_entries()
        .map_err(|e| e.to_string())?;

    // 清理异步缓存
    state
        .cache_manager
        .cleanup_expired_entries_async()
        .await
        .map_err(|e| e.to_string())
}

/// 缓存预热
#[command]
pub async fn warm_cache(
    common_queries: Vec<(String, String)>,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    // 提供一个简单的搜索闭包用于预热
    state
        .cache_manager
        .warm_cache(common_queries, |_query, _workspace_id| {
            async move {
                // 这里调用实际的搜索逻辑，目前先返回空结果作为占位
                // 在实际集成中，这里应该调用 search_engine
                Ok(vec![])
            }
        })
        .await
        .map_err(|e| e.to_string())
}

/// 获取缓存性能指标
#[command]
pub async fn get_cache_performance_metrics(
    state: State<'_, AppState>,
) -> Result<CacheMetricsSnapshot, String> {
    Ok(state.cache_manager.get_performance_metrics())
}

/// 获取缓存性能报告
#[command]
pub async fn get_cache_performance_report(
    state: State<'_, AppState>,
) -> Result<CachePerformanceReport, String> {
    Ok(state.cache_manager.generate_performance_report())
}

/// 执行缓存健康检查
#[command]
pub async fn cache_health_check(state: State<'_, AppState>) -> Result<CacheHealthCheck, String> {
    Ok(state.cache_manager.health_check().await)
}

/// 获取访问模式统计
#[command]
pub async fn get_access_pattern_stats(
    state: State<'_, AppState>,
) -> Result<AccessPatternStats, String> {
    Ok(state.cache_manager.get_access_pattern_stats())
}

/// 获取压缩统计信息
#[command]
pub async fn get_compression_stats(state: State<'_, AppState>) -> Result<CompressionStats, String> {
    Ok(state.cache_manager.get_compression_stats())
}

/// 获取 L2 缓存配置
#[command]
pub async fn get_l2_cache_config(state: State<'_, AppState>) -> Result<L2CacheConfig, String> {
    Ok(state.cache_manager.get_l2_config())
}

/// 智能缓存驱逐
#[command]
pub async fn intelligent_cache_eviction(
    target_reduction_percent: f64,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    state
        .cache_manager
        .intelligent_eviction(target_reduction_percent)
        .await
        .map_err(|e| e.to_string())
}

/// 重置缓存性能指标
#[command]
pub async fn reset_cache_metrics(state: State<'_, AppState>) -> Result<(), String> {
    state.cache_manager.reset_metrics();
    Ok(())
}

/// 重置访问模式追踪器
#[command]
pub async fn reset_access_tracker(state: State<'_, AppState>) -> Result<(), String> {
    state.cache_manager.reset_access_tracker();
    Ok(())
}

/// 获取缓存仪表板数据
#[command]
pub async fn get_cache_dashboard_data(
    state: State<'_, AppState>,
) -> Result<CacheDashboardData, String> {
    Ok(state.cache_manager.get_dashboard_data())
}
