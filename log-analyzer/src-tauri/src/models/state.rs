//! 应用状态管理
//!
//! 本模块定义了应用的全局状态结构和相关类型别名。

use crossbeam::queue::SegQueue;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use tokio_util::sync::CancellationToken;

use super::{FileMetadata, LogEntry};
use crate::monitoring::alerting::AlertingSystem;
use crate::monitoring::metrics_collector::MetricsCollector;
use crate::monitoring::recommendation_engine::RecommendationEngine;
use crate::search_engine::SearchEngineManager;
use crate::state_sync::StateSync;
use crate::utils::cache_manager::CacheManager;
use crate::utils::cancellation_manager::CancellationManager;
use crate::utils::resource_manager::ResourceManager;
use crate::utils::resource_tracker::ResourceTracker;

// --- 类型别名定义 ---

/// 搜索缓存键
///
/// 包含查询字符串、工作区ID和过滤条件的组合键。
/// 增加了更多参数以避免缓存污染。
pub type SearchCacheKey = (
    String,         // query
    String,         // workspace_id
    Option<String>, // time_start
    Option<String>, // time_end
    Vec<String>,    // levels
    Option<String>, // file_pattern
    bool,           // case_sensitive
    usize,          // max_results
    String,         // query_version (查询版本，用于失效策略)
);

/// 搜索缓存类型
///
/// 使用 moka 企业级缓存存储搜索结果,支持 TTL/TTI 过期策略。
pub type SearchCache = Arc<moka::sync::Cache<SearchCacheKey, Vec<LogEntry>>>;

/// 路径映射类型
///
/// real_path -> virtual_path
pub type PathMapType = HashMap<String, String>;

/// 元数据映射类型
///
/// file_path -> FileMetadata
pub type MetadataMapType = HashMap<String, FileMetadata>;

/// 索引操作结果类型
#[allow(dead_code)] // Reserved for future use
pub type IndexResult = Result<(PathMapType, MetadataMapType), String>;

// --- 状态结构体 ---

/// 文件监听器状态
///
/// 跟踪单个工作区的文件监听状态。
pub struct WatcherState {
    /// 工作区 ID（用于日志记录和调试）
    pub workspace_id: String,
    /// 监听的路径（用于计算相对路径）
    pub watched_path: PathBuf,
    /// 文件读取偏移量（用于增量读取）
    pub file_offsets: HashMap<String, u64>,
    /// 监听器是否活跃
    pub is_active: bool,
}

/// 应用全局状态
///
/// 管理应用运行时的所有共享状态。
pub struct AppState {
    /// 临时目录（用于解压缩文件）
    pub temp_dir: Mutex<Option<TempDir>>,
    /// 路径映射（real_path -> virtual_path）
    pub path_map: Arc<Mutex<PathMapType>>,
    /// 文件元数据映射
    pub file_metadata: Arc<Mutex<MetadataMapType>>,
    /// 工作区索引文件路径映射
    pub workspace_indices: Mutex<HashMap<String, PathBuf>>,
    /// 搜索缓存
    pub search_cache: SearchCache,
    /// 最近搜索耗时（毫秒）
    pub last_search_duration: Arc<Mutex<u64>>,
    /// 总搜索次数
    pub total_searches: Arc<Mutex<u64>>,
    /// 缓存命中次数
    pub cache_hits: Arc<Mutex<u64>>,
    /// 工作区监听器映射（workspace_id -> WatcherState）
    pub watchers: Arc<Mutex<HashMap<String, WatcherState>>>,
    /// 临时文件清理队列（无锁队列）
    pub cleanup_queue: Arc<SegQueue<PathBuf>>,
    /// 活跃搜索的取消令牌映射（search_id -> CancellationToken）
    /// 注意：此字段保留用于向后兼容，新代码应使用 cancellation_manager
    pub search_cancellation_tokens: Arc<Mutex<HashMap<String, CancellationToken>>>,
    /// 资源管理器（RAII 模式）
    pub resource_manager: Arc<ResourceManager>,
    /// 取消管理器（统一的取消令牌管理）
    pub cancellation_manager: Arc<CancellationManager>,
    /// 资源追踪器（资源生命周期管理）
    pub resource_tracker: Arc<ResourceTracker>,
    /// Tantivy 搜索引擎管理器（高性能全文搜索）
    pub search_engine: Arc<Mutex<Option<SearchEngineManager>>>,
    /// 状态同步管理器（Tauri Events 实时同步）
    pub state_sync: Arc<Mutex<Option<StateSync>>>,
    /// 统一缓存管理器（L1 Moka + 可选 L2 Redis）
    pub cache_manager: Arc<CacheManager>,
    /// 性能指标收集器（搜索操作计时、系统资源监控）
    pub metrics_collector: Arc<MetricsCollector>,
    /// 告警系统（性能阈值违规、资源约束告警）
    pub alerting_system: Arc<AlertingSystem>,
    /// 智能优化建议引擎（基于规则引擎的性能分析）
    pub recommendation_engine: Arc<RecommendationEngine>,
}

impl Drop for AppState {
    fn drop(&mut self) {
        eprintln!("[INFO] AppState dropping, performing final cleanup...");

        // 生成资源报告
        let report = self.resource_tracker.generate_report();
        report.print();

        // 检测资源泄漏
        let leaks = self
            .resource_tracker
            .detect_leaks(std::time::Duration::from_secs(300));
        if !leaks.is_empty() {
            eprintln!("[WARNING] Detected {} resource leaks", leaks.len());
        }

        // 取消所有活跃操作
        self.cancellation_manager.cancel_all();
        eprintln!("[INFO] Cancelled all active operations");

        // 清理所有资源
        self.resource_tracker.cleanup_all();
        eprintln!("[INFO] Cleaned up all tracked resources");

        // 执行最后的清理队列处理
        crate::utils::cleanup::process_cleanup_queue(&self.cleanup_queue);

        // 打印性能统计摘要
        {
            let searches = self.total_searches.lock();
            let hits = self.cache_hits.lock();
            let hit_rate = if *searches > 0 {
                (*hits as f64 / *searches as f64) * 100.0
            } else {
                0.0
            };
            eprintln!(
                "[INFO] Session stats: {} searches, {} cache hits ({:.1}% hit rate)",
                searches, hits, hit_rate
            );
        }

        eprintln!("[INFO] AppState cleanup completed");
    }
}
