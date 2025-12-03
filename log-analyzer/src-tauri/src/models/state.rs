//! 应用状态管理
//!
//! 本模块定义了应用的全局状态结构和相关类型别名。

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

use super::{FileMetadata, LogEntry};

// --- 类型别名定义 ---

/// 搜索缓存键
///
/// 包含查询字符串、工作区ID和过滤条件的组合键。
pub type SearchCacheKey = (
    String,         // query
    String,         // workspace_id
    Option<String>, // time_start
    Option<String>, // time_end
    Vec<String>,    // levels
    Option<String>, // file_pattern
);

/// 搜索缓存类型
///
/// 使用LRU缓存存储搜索结果。
pub type SearchCache = Arc<Mutex<lru::LruCache<SearchCacheKey, Vec<LogEntry>>>>;

/// 路径映射类型
///
/// real_path -> virtual_path
pub type PathMapType = HashMap<String, String>;

/// 元数据映射类型
///
/// file_path -> FileMetadata
pub type MetadataMapType = HashMap<String, FileMetadata>;

/// 索引操作结果类型
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
    /// 临时文件清理队列
    pub cleanup_queue: Arc<Mutex<Vec<PathBuf>>>,
}

impl Drop for AppState {
    fn drop(&mut self) {
        eprintln!("[INFO] AppState dropping, performing final cleanup...");

        // 执行最后的清理队列处理
        crate::utils::cleanup::process_cleanup_queue(&self.cleanup_queue);

        // 打印性能统计摘要
        if let Ok(searches) = self.total_searches.lock() {
            if let Ok(hits) = self.cache_hits.lock() {
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
        }

        eprintln!("[INFO] AppState cleanup completed");
    }
}
