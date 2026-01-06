//! 日志分析器 - Rust 后端
//!
//! 提供高性能的日志分析功能，包括：
//! - 多格式压缩包递归解压
//! - 并发全文搜索
//! - 结构化查询系统
//! - 持久化与增量更新
//! - 实时文件监听

use crossbeam::queue::SegQueue;
use parking_lot::Mutex;
use std::sync::Arc;

// 从 archive 模块导入 rar 验证函数
use crate::archive::rar_handler::{get_unrar_path, validate_unrar_binary};

// 从 models 导入类型
pub use models::state::AppState;

// 从 utils 导入 ResourceTracker
pub use crate::utils::resource_tracker::ResourceTracker;

/// AppState 全局状态管理器
pub struct AppState {
    /// 清理队列
    pub cleanup_queue: Arc<SegQueue<()>>,
    
    /// 资源管理器
    pub resource_manager: Arc<ResourceManager>,
    
    /// 取消管理器
    pub cancellation_manager: Arc<CancellationManager>,
    
    /// 嵄源追踪器
    // pub resource_tracker: Arc<ResourceTracker>,
    
    /// 搜索缓存（Moka L1 缓存）
    pub search_cache: Arc<moka::sync::Cache<crate::models::SearchCacheKey, Vec<crate::models::LogEntry>>>,
    
    /// 最后搜索耗时
    pub last_search_duration: Arc<Mutex<u128>>,
    
    /// 总搜索次数
    pub total_searches: Arc<Mutex<u128>>,
    
    /// 缓存命中次数
    pub cache_hits: Arc<Mutex<u128>>,
    
    /// 文件监听器
    pub watchers: Arc<Mutex<HashMap<String, crate::utils::FileWatcher>>,
    
    /// 搜索取消令牌
    pub search_cancellation_tokens: Arc<Mutex<HashMap<String, crate::utils::CancellationToken>>>,
    
    /// 资源管理器
    pub resource_manager: Arc<ResourceManager>,
    
    /// 取消管理器
    pub cancellation_manager: Arc<CancellationManager>,
    
    /// 资源追踪器
    pub resource_tracker: Arc<ResourceTracker>,
    
    /// 缓存管理器（L1 Moka 内存缓存）
    pub cache_manager: Arc<crate::utils::CacheManager>,
    
    /// 搜索引擎（延迟初始化，首次搜索时创建）
    pub search_engine: Arc<Mutex<Option<crate::search_engine::SearchEngine>>>,
    
    /// 状态同步管理器（延迟初始化，在 setup hook 中创建）
    pub state_sync: Arc<Mutex<Option<crate::state_sync::StateSyncManager>>>,
    
    /// 过滤引擎
    pub filter_engine: Arc<Mutex<crate::search_engine::advanced_features::FilterEngine>>,
    
    /// 正则表达式引擎
    pub regex_engine: Arc<Mutex<crate::search_engine::advanced_features::RegexSearchEngine>>,
    
    /// 时间分区索引
    pub time_partitioned_index: Arc<Mutex<HashMap<String, crate::search_engine::TimePartitionedIndex>>>,
    
    /// 自动补全引擎
    pub autocomplete_engine: Arc<crate::search_engine::advanced_features::AutocompleteEngine>,
    
    /// 任务生命周期管理
    pub task_manager: Arc<parking_lot::Mutex<Option<crate::task_manager::TaskManager>>,
    
    /// 过滤引擎
    pub filter_engine: Arc<Mutex<crate::search_engine::advanced_features::FilterEngine>>,
    
    /// 时间分区索引
    pub time_partitioned_index: Arc<Mutex<HashMap<String, crate::search_engine::TimePartitionedIndex>>,

    /// 自动补全引擎
    pub autocomplete_engine: Arc<crate::search_engine::advanced_features::AutocompleteEngine>,
}
