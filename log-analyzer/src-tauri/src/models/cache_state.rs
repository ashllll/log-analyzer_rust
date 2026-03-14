//! 缓存状态管理
//!
//! 提供缓存管理器和清理队列的共享访问

use crate::utils::cache_manager::CacheManager;
use crate::utils::cleanup::CleanupQueue;
use moka::sync::Cache;
use std::sync::Arc;

/// 缓存状态 - 管理缓存相关的所有资源
pub struct CacheState {
    /// 缓存管理器 - 提供高级缓存功能
    pub cache_manager: Arc<CacheManager>,
    /// 清理队列 - 用于异步清理临时文件
    pub cleanup_queue: Arc<CleanupQueue>,
}

impl Default for CacheState {
    fn default() -> Self {
        // 创建同步缓存 (L1 cache)
        let sync_cache = Cache::builder().max_capacity(1000).build();
        let cache_manager = CacheManager::new(Arc::new(sync_cache));

        Self {
            cache_manager: Arc::new(cache_manager),
            cleanup_queue: Arc::new(CleanupQueue::new()),
        }
    }
}

impl CacheState {
    /// 创建新的缓存状态，使用自定义缓存管理器
    pub fn new(cache_manager: CacheManager) -> Self {
        Self {
            cache_manager: Arc::new(cache_manager),
            cleanup_queue: Arc::new(CleanupQueue::new()),
        }
    }

    /// 创建空的缓存状态（用于测试）
    pub fn empty() -> Self {
        Self::default()
    }
}
