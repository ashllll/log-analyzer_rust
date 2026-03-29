//! 虚拟搜索管理器 - 服务端虚拟化实现
//!
//! 提供搜索结果的内存管理和按需加载功能，支持大数据集的虚拟化展示。
//! 前端无需一次性加载所有结果，可以按需请求数据范围。

use crate::models::LogEntry;
use lru::LruCache;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use tracing::{debug, info, warn};

/// 搜索会话状态
#[derive(Debug, Clone)]
pub struct SearchSession {
    pub search_id: String,
    pub query: String,
    pub total_count: usize,
    pub created_at: std::time::Instant,
    pub last_accessed: std::time::Instant,
}

/// 虚拟搜索管理器
///
/// 管理搜索会话和结果缓存，支持：
/// - 分页加载搜索结果
/// - 自动过期清理
/// - 内存限制管理
pub struct VirtualSearchManager {
    /// 搜索会话缓存 (search_id -> 完整结果列表)
    /// 使用 LRU 策略自动清理旧会话
    sessions: Mutex<LruCache<String, Vec<LogEntry>>>,

    /// 会话元数据 (search_id -> 会话信息)
    session_metadata: Mutex<HashMap<String, SearchSession>>,

    /// 会话总数限制
    max_sessions: usize,

    /// 单个会话最大条目数
    max_entries_per_session: usize,

    /// 会话过期时间
    session_ttl: std::time::Duration,
}

impl VirtualSearchManager {
    /// 创建新的虚拟搜索管理器
    pub fn new(max_sessions: usize) -> Self {
        let cache_size = NonZeroUsize::new(max_sessions.max(1)).unwrap();

        Self {
            sessions: Mutex::new(LruCache::new(cache_size)),
            session_metadata: Mutex::new(HashMap::new()),
            max_sessions,
            max_entries_per_session: 100_000, // 默认 10万条
            session_ttl: std::time::Duration::from_secs(3600), // 默认 1小时
        }
    }

    /// 使用自定义配置创建
    pub fn with_config(
        max_sessions: usize,
        max_entries_per_session: usize,
        session_ttl_seconds: u64,
    ) -> Self {
        let cache_size = NonZeroUsize::new(max_sessions.max(1)).unwrap();

        Self {
            sessions: Mutex::new(LruCache::new(cache_size)),
            session_metadata: Mutex::new(HashMap::new()),
            max_sessions,
            max_entries_per_session,
            session_ttl: std::time::Duration::from_secs(session_ttl_seconds),
        }
    }

    /// 注册新的搜索会话
    ///
    /// 将搜索结果存入会话缓存，返回 search_id
    pub fn register_session(
        &self,
        search_id: String,
        query: String,
        mut entries: Vec<LogEntry>,
    ) -> String {
        let now = std::time::Instant::now();

        // 保存原始数量，用于 total_count
        let total_count = entries.len();

        // 限制条目数量，避免内存溢出
        let truncated_entries = if entries.len() > self.max_entries_per_session {
            warn!(
                search_id = %search_id,
                original_count = entries.len(),
                max_allowed = self.max_entries_per_session,
                "Search results truncated to prevent memory overflow"
            );
            entries.truncate(self.max_entries_per_session);
            entries
        } else {
            entries
        };

        let session = SearchSession {
            search_id: search_id.clone(),
            query,
            total_count,
            created_at: now,
            last_accessed: now,
        };

        // 先写 metadata，再写 sessions（LRU put 可能驱逐旧条目）
        // 通过 peek_lru 预先记录将被驱逐的 key，put 后同步清理 metadata
        let evicted_id = {
            let mut sessions = self.sessions.lock();
            // LruCache::put 返回 Option<V>（旧值），无法得知被驱逐的 key
            // 使用 peek_lru 在 put 前预先记录即将被驱逐的 key
            let lru_key = if sessions.len() >= sessions.cap().get() {
                sessions.peek_lru().map(|(k, _)| k.clone())
            } else {
                None
            };
            sessions.put(search_id.clone(), truncated_entries);
            lru_key
        };

        {
            let mut metadata = self.session_metadata.lock();
            // 若 LRU 驱逐了旧条目，同步从 metadata 中删除，保持两者一致
            if let Some(evicted_key) = evicted_id {
                metadata.remove(&evicted_key);
            }
            metadata.insert(search_id.clone(), session);
        }

        info!(
            search_id = %search_id,
            total_count = total_count,
            "Search session registered"
        );

        search_id
    }

    /// 获取指定范围的搜索结果
    ///
    /// # Arguments
    /// * `search_id` - 搜索会话 ID
    /// * `offset` - 起始偏移量
    /// * `limit` - 返回条目数限制
    ///
    /// # Returns
    /// 指定范围的日志条目列表
    pub fn get_range(&self, search_id: &str, offset: usize, limit: usize) -> Vec<LogEntry> {
        let mut sessions = self.sessions.lock();

        if let Some(entries) = sessions.get_mut(search_id) {
            // 更新最后访问时间
            {
                let mut metadata = self.session_metadata.lock();
                if let Some(session) = metadata.get_mut(search_id) {
                    session.last_accessed = std::time::Instant::now();
                }
            }

            let end = (offset + limit).min(entries.len());
            if offset < entries.len() {
                debug!(
                    search_id = %search_id,
                    offset = offset,
                    limit = limit,
                    returned = end - offset,
                    "Retrieved search results range"
                );
                entries[offset..end].to_vec()
            } else {
                Vec::new()
            }
        } else {
            warn!(
                search_id = %search_id,
                "Search session not found or expired"
            );
            Vec::new()
        }
    }

    /// 获取会话总条目数
    pub fn get_total_count(&self, search_id: &str) -> usize {
        let metadata = self.session_metadata.lock();
        metadata.get(search_id).map(|s| s.total_count).unwrap_or(0)
    }

    /// 获取会话信息
    pub fn get_session_info(&self, search_id: &str) -> Option<SearchSession> {
        let metadata = self.session_metadata.lock();
        metadata.get(search_id).cloned()
    }

    /// 检查会话是否存在
    pub fn has_session(&self, search_id: &str) -> bool {
        let sessions = self.sessions.lock();
        // 检查 LRU 缓存中是否存在（实际存储的会话数据）
        sessions.contains(search_id)
    }

    /// 移除搜索会话
    pub fn remove_session(&self, search_id: &str) -> bool {
        let mut sessions = self.sessions.lock();
        let mut metadata = self.session_metadata.lock();

        let removed = sessions.pop(search_id).is_some();
        metadata.remove(search_id);

        if removed {
            info!(search_id = %search_id, "Search session removed");
        }

        removed
    }

    /// 清理过期会话
    pub fn cleanup_expired_sessions(&self) -> usize {
        let now = std::time::Instant::now();
        let mut sessions = self.sessions.lock();
        let mut metadata = self.session_metadata.lock();

        let expired_ids: Vec<String> = metadata
            .iter()
            .filter(|(_, session)| now.duration_since(session.last_accessed) > self.session_ttl)
            .map(|(id, _)| id.clone())
            .collect();

        for id in &expired_ids {
            sessions.pop(id);
            metadata.remove(id);
        }

        if !expired_ids.is_empty() {
            info!(
                count = expired_ids.len(),
                "Expired search sessions cleaned up"
            );
        }

        expired_ids.len()
    }

    /// 获取活跃会话数量
    pub fn active_session_count(&self) -> usize {
        let metadata = self.session_metadata.lock();
        metadata.len()
    }

    /// 获取会话统计信息
    pub fn get_statistics(&self) -> VirtualSearchStats {
        let sessions = self.sessions.lock();
        let metadata = self.session_metadata.lock();

        let total_entries: usize = sessions.iter().map(|(_, entries)| entries.len()).sum();

        VirtualSearchStats {
            active_sessions: metadata.len(),
            total_cached_entries: total_entries,
            max_sessions: self.max_sessions,
            max_entries_per_session: self.max_entries_per_session,
            session_ttl_seconds: self.session_ttl.as_secs(),
        }
    }

    /// 清除所有会话
    pub fn clear_all_sessions(&self) {
        let mut sessions = self.sessions.lock();
        let mut metadata = self.session_metadata.lock();

        sessions.clear();
        metadata.clear();

        info!("All search sessions cleared");
    }
}

/// 虚拟搜索统计信息
#[derive(Debug, Clone)]
pub struct VirtualSearchStats {
    pub active_sessions: usize,
    pub total_cached_entries: usize,
    pub max_sessions: usize,
    pub max_entries_per_session: usize,
    pub session_ttl_seconds: u64,
}

impl Default for VirtualSearchManager {
    fn default() -> Self {
        Self::new(100) // 默认最多 100 个会话
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_entry(id: usize) -> LogEntry {
        LogEntry {
            id,
            timestamp: "2024-01-01T00:00:00".into(),
            level: "INFO".into(),
            file: "/test/file.log".into(),
            real_path: "/real/file.log".into(),
            line: id,
            content: format!("Test log entry {}", id).into(),
            tags: vec![],
            match_details: None,
            matched_keywords: None,
        }
    }

    #[test]
    fn test_register_and_get_range() {
        let manager = VirtualSearchManager::new(10);
        let entries: Vec<LogEntry> = (0..100).map(create_test_entry).collect();

        let search_id =
            manager.register_session("test-123".to_string(), "test query".to_string(), entries);

        assert_eq!(manager.get_total_count(&search_id), 100);

        // 获取第一页
        let page1 = manager.get_range(&search_id, 0, 10);
        assert_eq!(page1.len(), 10);
        assert_eq!(page1[0].id, 0);

        // 获取第二页
        let page2 = manager.get_range(&search_id, 10, 10);
        assert_eq!(page2.len(), 10);
        assert_eq!(page2[0].id, 10);

        // 获取超出范围的页面
        let empty = manager.get_range(&search_id, 1000, 10);
        assert!(empty.is_empty());
    }

    #[test]
    fn test_session_expiration() {
        let manager = VirtualSearchManager::with_config(10, 1000, 1); // 1秒过期
        let entries: Vec<LogEntry> = (0..10).map(create_test_entry).collect();

        let search_id =
            manager.register_session("test-expire".to_string(), "test".to_string(), entries);

        assert!(manager.has_session(&search_id));

        // 模拟等待过期
        std::thread::sleep(std::time::Duration::from_secs(2));

        let cleaned = manager.cleanup_expired_sessions();
        assert_eq!(cleaned, 1);
        assert!(!manager.has_session(&search_id));
    }

    #[test]
    fn test_lru_eviction() {
        let manager = VirtualSearchManager::new(2); // 最多2个会话

        let entries1 = vec![create_test_entry(1)];
        let entries2 = vec![create_test_entry(2)];
        let entries3 = vec![create_test_entry(3)];

        let id1 = manager.register_session("s1".to_string(), "q1".to_string(), entries1);
        let id2 = manager.register_session("s2".to_string(), "q2".to_string(), entries2);
        let id3 = manager.register_session("s3".to_string(), "q3".to_string(), entries3);

        // 由于 LRU 限制，第一个会话应该被移除
        assert!(!manager.has_session(&id1));
        assert!(manager.has_session(&id2));
        assert!(manager.has_session(&id3));
    }
}
