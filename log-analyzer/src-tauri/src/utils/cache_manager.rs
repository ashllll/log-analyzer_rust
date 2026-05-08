//! 缓存管理器
//!
//! 提供搜索结果的缓存功能：
//! - 工作区特定的缓存失效
//! - 同步获取/插入/移除
//!
//! 基于 moka::sync::Cache 实现，利用其内置的 TTL/TTI 和自动驱逐能力。

use la_core::models::{LogEntry, SearchCacheKey};
use moka::sync::Cache;
use std::sync::Arc;

/// 缓存管理器
///
/// 管理搜索缓存的生命周期
pub struct CacheManager {
    /// 搜索结果缓存（moka sync Cache，内置 TTL/TTI/自动驱逐）
    search_cache: Arc<Cache<SearchCacheKey, Vec<LogEntry>>>,
}

impl Clone for CacheManager {
    fn clone(&self) -> Self {
        Self {
            search_cache: self.search_cache.clone(),
        }
    }
}

impl CacheManager {
    /// 创建新的缓存管理器
    pub fn new(search_cache: Arc<Cache<SearchCacheKey, Vec<LogEntry>>>) -> Self {
        Self { search_cache }
    }

    /// 同步获取缓存条目
    pub fn get_sync(&self, key: &SearchCacheKey) -> Option<Vec<LogEntry>> {
        self.search_cache.get(key)
    }

    /// 同步插入缓存条目
    pub fn insert_sync(&self, key: SearchCacheKey, value: Vec<LogEntry>) {
        self.search_cache.insert(key, value);
    }

    /// 同步移除缓存条目
    pub fn remove_sync(&self, key: &SearchCacheKey) {
        self.search_cache.invalidate(key);
    }

    /// 使工作区相关的缓存失效
    pub fn invalidate_workspace_cache(&self, workspace_id: &str) -> usize {
        let keys_to_invalidate: Vec<SearchCacheKey> = self
            .search_cache
            .iter()
            .filter_map(|(key, _)| {
                if key.1 == workspace_id {
                    Some((*key).clone())
                } else {
                    None
                }
            })
            .collect();

        let count = keys_to_invalidate.len();
        for key in &keys_to_invalidate {
            self.search_cache.invalidate(key);
        }

        tracing::info!(
            workspace_id = %workspace_id,
            invalidated_count = count,
            "Invalidated workspace-specific cache entries"
        );

        count
    }

    /// 获取缓存条目数量
    pub fn entry_count(&self) -> u64 {
        self.search_cache.entry_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn create_test_cache() -> Arc<Cache<SearchCacheKey, Vec<LogEntry>>> {
        Arc::new(
            Cache::builder()
                .max_capacity(100)
                .time_to_live(Duration::from_secs(300))
                .time_to_idle(Duration::from_secs(60))
                .build(),
        )
    }

    #[test]
    fn test_cache_manager_creation() {
        let cache = create_test_cache();
        let manager = CacheManager::new(cache);
        assert_eq!(manager.entry_count(), 0);
    }

    #[test]
    fn test_sync_get_insert_remove() {
        let cache = create_test_cache();
        let manager = CacheManager::new(cache);

        let key: SearchCacheKey = (
            "test_query".to_string(),
            "workspace".to_string(),
            None,
            None,
            vec![],
            None,
            false,
            100,
            String::new(),
        );
        let value = vec![LogEntry {
            id: 1,
            timestamp: Arc::from("2024-01-01T00:00:00Z"),
            level: Arc::from("INFO"),
            file: Arc::from("test.log"),
            real_path: Arc::from("test.log"),
            line: 1,
            content: Arc::from("test"),
            tags: vec![],
            match_details: None,
            matched_keywords: None,
        }];

        // 插入
        manager.insert_sync(key.clone(), value.clone());
        // moka 缓存的 entry_count 可能有延迟，需要触发同步
        manager.search_cache.run_pending_tasks();
        assert_eq!(manager.entry_count(), 1);

        // 获取
        let result = manager.get_sync(&key);
        assert_eq!(result.unwrap().len(), value.len());

        // 移除
        manager.remove_sync(&key);
        assert!(manager.get_sync(&key).is_none());
    }

    #[test]
    fn test_workspace_cache_invalidation() {
        let cache = create_test_cache();
        let manager = CacheManager::new(cache);

        let key1: SearchCacheKey = (
            "query1".to_string(),
            "workspace1".to_string(),
            None,
            None,
            vec![],
            None,
            false,
            100,
            String::new(),
        );
        let key2: SearchCacheKey = (
            "query2".to_string(),
            "workspace2".to_string(),
            None,
            None,
            vec![],
            None,
            false,
            100,
            String::new(),
        );

        manager.insert_sync(key1.clone(), vec![]);
        manager.insert_sync(key2.clone(), vec![]);

        let invalidated = manager.invalidate_workspace_cache("workspace1");
        assert_eq!(invalidated, 1);

        assert!(manager.get_sync(&key1).is_none());
        assert!(manager.get_sync(&key2).is_some());
    }
}
