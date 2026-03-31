//! TTL 缓存策略实现
//!
//! 提供带生存时间（TTL）的缓存功能：
//! - 每个缓存项可配置独立的 TTL
//! - 惰性过期检查（访问时检查）
//! - 定期后台清理任务
//! - 详细的过期统计信息

use parking_lot::RwLock;
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// TTL 缓存 trait - 定义带过期时间的缓存接口
pub trait TtlCache<K, V> {
    /// 插入带 TTL 的缓存项
    fn put_with_ttl(&self, key: K, value: V, ttl: Duration);

    /// 检查键是否已过期
    fn is_expired(&self, key: &K) -> bool;

    /// 清理所有过期项
    fn cleanup_expired(&self) -> usize;

    /// 获取缓存项（如果未过期）
    fn get_with_ttl_check(&self, key: &K) -> Option<V>;

    /// 获取缓存项的剩余 TTL
    fn get_remaining_ttl(&self, key: &K) -> Option<Duration>;
}

/// 带 TTL 的缓存项
#[derive(Debug, Clone)]
pub struct CacheEntry<V> {
    /// 缓存值
    pub value: V,
    /// 创建时间
    pub created_at: Instant,
    /// TTL（生存时间），None 表示永不过期
    pub ttl: Option<Duration>,
    /// 最后访问时间（用于 TTI 计算）
    pub last_accessed: Instant,
    /// TTI（空闲时间），None 表示不限制
    pub tti: Option<Duration>,
}

impl<V> CacheEntry<V> {
    /// 创建新的缓存项（永不过期）
    pub fn new(value: V) -> Self {
        let now = Instant::now();
        Self {
            value,
            created_at: now,
            ttl: None,
            last_accessed: now,
            tti: None,
        }
    }

    /// 创建带 TTL 的缓存项
    pub fn with_ttl(value: V, ttl: Duration) -> Self {
        let now = Instant::now();
        Self {
            value,
            created_at: now,
            ttl: Some(ttl),
            last_accessed: now,
            tti: None,
        }
    }

    /// 创建带 TTL 和 TTI 的缓存项
    pub fn with_ttl_and_tti(value: V, ttl: Duration, tti: Duration) -> Self {
        let now = Instant::now();
        Self {
            value,
            created_at: now,
            ttl: Some(ttl),
            last_accessed: now,
            tti: Some(tti),
        }
    }

    /// 检查是否已过期（TTL 或 TTI）
    pub fn is_expired(&self) -> bool {
        // 检查 TTL
        if let Some(ttl) = self.ttl {
            if self.created_at.elapsed() >= ttl {
                return true;
            }
        }

        // 检查 TTI（空闲时间）
        if let Some(tti) = self.tti {
            if self.last_accessed.elapsed() >= tti {
                return true;
            }
        }

        false
    }

    /// 获取剩余 TTL
    pub fn remaining_ttl(&self) -> Option<Duration> {
        self.ttl.map(|ttl| {
            let elapsed = self.created_at.elapsed();
            if elapsed >= ttl {
                Duration::from_secs(0)
            } else {
                ttl - elapsed
            }
        })
    }

    /// 更新最后访问时间
    pub fn touch(&mut self) {
        self.last_accessed = Instant::now();
    }
}

/// TTL 缓存统计信息
#[derive(Debug, Clone, Default)]
pub struct TtlCacheStats {
    /// 总条目数
    pub total_entries: usize,
    /// 已过期条目数
    pub expired_entries: usize,
    /// 因过期被清理的条目总数（累计）
    pub total_expired_cleaned: u64,
    /// 设置了 TTL 的条目数
    pub entries_with_ttl: usize,
    /// 平均 TTL（毫秒）
    pub avg_ttl_ms: f64,
    /// 下次清理时间
    pub next_cleanup_in: Option<Duration>,
}

/// 内存中的 TTL 缓存实现
pub struct InMemoryTtlCache<K, V> {
    /// 存储的条目
    entries: RwLock<HashMap<K, CacheEntry<V>>>,
    /// 过期清理统计
    expired_cleaned_count: AtomicU64,
    /// 默认 TTL
    default_ttl: Option<Duration>,
    /// 默认 TTI
    default_tti: Option<Duration>,
}

impl<K, V> InMemoryTtlCache<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    /// 创建新的 TTL 缓存
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            expired_cleaned_count: AtomicU64::new(0),
            default_ttl: None,
            default_tti: None,
        }
    }

    /// 创建带默认 TTL 的缓存
    pub fn with_default_ttl(default_ttl: Duration) -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            expired_cleaned_count: AtomicU64::new(0),
            default_ttl: Some(default_ttl),
            default_tti: None,
        }
    }

    /// 创建带默认 TTL 和 TTI 的缓存
    pub fn with_defaults(default_ttl: Duration, default_tti: Duration) -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            expired_cleaned_count: AtomicU64::new(0),
            default_ttl: Some(default_ttl),
            default_tti: Some(default_tti),
        }
    }

    /// 插入缓存项（使用默认 TTL）
    pub fn put(&self, key: K, value: V) {
        let entry = if let Some(ttl) = self.default_ttl {
            if let Some(tti) = self.default_tti {
                CacheEntry::with_ttl_and_tti(value, ttl, tti)
            } else {
                CacheEntry::with_ttl(value, ttl)
            }
        } else {
            CacheEntry::new(value)
        };

        self.entries.write().insert(key, entry);
    }

    /// 获取缓存项（不检查过期，不更新访问时间）
    pub fn get_raw(&self, key: &K) -> Option<V> {
        self.entries.read().get(key).map(|e| e.value.clone())
    }

    /// 获取缓存项数量
    pub fn len(&self) -> usize {
        self.entries.read().len()
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.entries.read().is_empty()
    }

    /// 使指定键失效
    pub fn invalidate(&self, key: &K) -> bool {
        self.entries.write().remove(key).is_some()
    }

    /// 使所有键失效
    pub fn clear(&self) {
        self.entries.write().clear();
    }

    /// 获取统计信息
    pub fn stats(&self) -> TtlCacheStats {
        let entries = self.entries.read();
        let total_entries = entries.len();
        let expired_entries = entries.values().filter(|e| e.is_expired()).count();
        let entries_with_ttl = entries.values().filter(|e| e.ttl.is_some()).count();

        // 计算平均 TTL
        let total_ttl_ms: u64 = entries
            .values()
            .filter_map(|e| e.ttl.map(|t| t.as_millis() as u64))
            .sum();
        let avg_ttl_ms = if entries_with_ttl > 0 {
            total_ttl_ms as f64 / entries_with_ttl as f64
        } else {
            0.0
        };

        // 计算下次清理时间（最近的过期时间）
        let next_cleanup_in = entries
            .values()
            .filter(|e| !e.is_expired())
            .filter_map(|e| e.remaining_ttl())
            .min();

        TtlCacheStats {
            total_entries,
            expired_entries,
            total_expired_cleaned: self.expired_cleaned_count.load(Ordering::Relaxed),
            entries_with_ttl,
            avg_ttl_ms,
            next_cleanup_in,
        }
    }

    /// 获取所有键（不过滤过期项）
    pub fn keys(&self) -> Vec<K> {
        self.entries.read().keys().cloned().collect()
    }

    /// 迭代所有条目（不过滤过期项）
    pub fn iter_entries<F, R>(&self, f: F) -> Vec<R>
    where
        F: Fn(&K, &CacheEntry<V>) -> R,
    {
        let entries = self.entries.read();
        entries.iter().map(|(k, v)| f(k, v)).collect()
    }
}

impl<K, V> TtlCache<K, V> for InMemoryTtlCache<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    fn put_with_ttl(&self, key: K, value: V, ttl: Duration) {
        let entry = CacheEntry::with_ttl(value, ttl);
        self.entries.write().insert(key, entry);
    }

    fn is_expired(&self, key: &K) -> bool {
        self.entries
            .read()
            .get(key)
            .map(|e| e.is_expired())
            .unwrap_or(true) // 不存在的键视为已过期
    }

    fn cleanup_expired(&self) -> usize {
        let mut entries = self.entries.write();
        let expired_keys: Vec<K> = entries
            .iter()
            .filter(|(_, entry)| entry.is_expired())
            .map(|(key, _)| key.clone())
            .collect();

        let cleaned_count = expired_keys.len();
        for key in expired_keys {
            entries.remove(&key);
        }

        // 更新统计
        self.expired_cleaned_count
            .fetch_add(cleaned_count as u64, Ordering::Relaxed);

        cleaned_count
    }

    fn get_with_ttl_check(&self, key: &K) -> Option<V> {
        let mut entries = self.entries.write();

        if let Some(entry) = entries.get_mut(key) {
            if entry.is_expired() {
                // 过期了，删除并返回 None
                entries.remove(key);
                self.expired_cleaned_count.fetch_add(1, Ordering::Relaxed);
                None
            } else {
                // 未过期，更新访问时间并返回值
                entry.touch();
                Some(entry.value.clone())
            }
        } else {
            None
        }
    }

    fn get_remaining_ttl(&self, key: &K) -> Option<Duration> {
        self.entries.read().get(key).and_then(|e| e.remaining_ttl())
    }
}

impl<K, V> Default for InMemoryTtlCache<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    fn default() -> Self {
        Self::new()
    }
}

/// TTL 缓存清理调度器
pub struct TtlCleanupScheduler {
    /// 清理间隔
    interval: Duration,
    /// 上次清理时间
    last_cleanup: RwLock<Instant>,
    /// 清理次数统计
    cleanup_count: AtomicU64,
}

impl TtlCleanupScheduler {
    /// 创建新的调度器
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            last_cleanup: RwLock::new(Instant::now()),
            cleanup_count: AtomicU64::new(0),
        }
    }

    /// 检查是否需要清理
    pub fn should_cleanup(&self) -> bool {
        self.last_cleanup.read().elapsed() >= self.interval
    }

    /// 记录清理完成
    pub fn record_cleanup(&self) {
        *self.last_cleanup.write() = Instant::now();
        self.cleanup_count.fetch_add(1, Ordering::Relaxed);
    }

    /// 获取清理统计
    pub fn cleanup_count(&self) -> u64 {
        self.cleanup_count.load(Ordering::Relaxed)
    }

    /// 获取距离下次清理的时间
    pub fn time_until_next_cleanup(&self) -> Duration {
        let elapsed = self.last_cleanup.read().elapsed();
        if elapsed >= self.interval {
            Duration::from_secs(0)
        } else {
            self.interval - elapsed
        }
    }

    /// 获取上次清理到现在的时间
    pub fn time_since_last_cleanup(&self) -> Duration {
        self.last_cleanup.read().elapsed()
    }

    /// 重置调度器
    pub fn reset(&self) {
        *self.last_cleanup.write() = Instant::now();
    }
}

/// 带后台清理任务的 TTL 缓存
pub struct ManagedTtlCache<K, V>
where
    K: Eq + Hash + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    /// 底层缓存
    cache: Arc<InMemoryTtlCache<K, V>>,
    /// 清理调度器
    scheduler: TtlCleanupScheduler,
    /// 清理任务句柄（用于取消）
    cleanup_handle: RwLock<Option<tokio::task::JoinHandle<()>>>,
}

impl<K, V> ManagedTtlCache<K, V>
where
    K: Eq + Hash + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    /// 创建新的托管缓存
    pub fn new(cleanup_interval: Duration) -> Self {
        Self {
            cache: Arc::new(InMemoryTtlCache::new()),
            scheduler: TtlCleanupScheduler::new(cleanup_interval),
            cleanup_handle: RwLock::new(None),
        }
    }

    /// 创建带默认 TTL 的托管缓存
    pub fn with_default_ttl(default_ttl: Duration, cleanup_interval: Duration) -> Self {
        Self {
            cache: Arc::new(InMemoryTtlCache::with_default_ttl(default_ttl)),
            scheduler: TtlCleanupScheduler::new(cleanup_interval),
            cleanup_handle: RwLock::new(None),
        }
    }

    /// 启动后台清理任务
    pub fn start_cleanup_task(&self) {
        let cache = Arc::clone(&self.cache);
        let interval = self.scheduler.interval;

        let handle = tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);

            loop {
                interval_timer.tick().await;
                let cleaned = cache.cleanup_expired();
                if cleaned > 0 {
                    tracing::debug!(cleaned_count = cleaned, "TTL cache cleanup completed");
                }
            }
        });

        *self.cleanup_handle.write() = Some(handle);
    }

    /// 停止后台清理任务
    pub fn stop_cleanup_task(&self) {
        if let Some(handle) = self.cleanup_handle.write().take() {
            handle.abort();
        }
    }

    /// 执行一次清理
    pub fn cleanup_now(&self) -> usize {
        let cleaned = self.cache.cleanup_expired();
        self.scheduler.record_cleanup();
        cleaned
    }

    /// 获取底层缓存的引用
    pub fn cache(&self) -> &InMemoryTtlCache<K, V> {
        &self.cache
    }

    /// 获取调度器信息
    pub fn scheduler(&self) -> &TtlCleanupScheduler {
        &self.scheduler
    }
}

impl<K, V> Drop for ManagedTtlCache<K, V>
where
    K: Eq + Hash + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    fn drop(&mut self) {
        self.stop_cleanup_task();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_cache_entry_expiration() {
        let entry = CacheEntry::with_ttl("value", Duration::from_millis(100));
        assert!(!entry.is_expired());

        thread::sleep(Duration::from_millis(150));
        assert!(entry.is_expired());
    }

    #[test]
    fn test_cache_entry_remaining_ttl() {
        let entry = CacheEntry::with_ttl("value", Duration::from_secs(10));
        let remaining = entry.remaining_ttl().unwrap();
        assert!(remaining.as_secs() <= 10 && remaining.as_secs() >= 9);
    }

    #[test]
    fn test_in_memory_ttl_cache_basic() {
        let cache = InMemoryTtlCache::new();

        // 插入并获取
        cache.put("key1", "value1");
        assert_eq!(cache.get_raw(&"key1"), Some("value1"));

        // 不存在的键
        assert_eq!(cache.get_raw(&"key2"), None);
    }

    #[test]
    fn test_ttl_cache_trait() {
        let cache = InMemoryTtlCache::new();

        // 插入带 TTL 的项
        cache.put_with_ttl("key1", "value1", Duration::from_millis(100));
        assert!(!cache.is_expired(&"key1"));

        // 等待过期
        thread::sleep(Duration::from_millis(150));
        assert!(cache.is_expired(&"key1"));
    }

    #[test]
    fn test_cleanup_expired() {
        let cache = InMemoryTtlCache::new();

        cache.put_with_ttl("key1", "value1", Duration::from_millis(50));
        cache.put_with_ttl("key2", "value2", Duration::from_secs(60));
        cache.put("key3", "value3"); // 永不过期

        assert_eq!(cache.len(), 3);

        // 等待 key1 过期
        thread::sleep(Duration::from_millis(100));

        let cleaned = cache.cleanup_expired();
        assert_eq!(cleaned, 1);
        assert_eq!(cache.len(), 2);
        assert!(cache.get_raw(&"key1").is_none());
        assert!(cache.get_raw(&"key2").is_some());
        assert!(cache.get_raw(&"key3").is_some());
    }

    #[test]
    fn test_get_with_ttl_check() {
        let cache = InMemoryTtlCache::new();

        cache.put_with_ttl("key1", "value1", Duration::from_millis(50));
        assert_eq!(cache.get_with_ttl_check(&"key1"), Some("value1"));

        // 等待过期
        thread::sleep(Duration::from_millis(100));
        assert_eq!(cache.get_with_ttl_check(&"key1"), None);
        assert_eq!(cache.len(), 0); // 过期项被自动清理
    }

    #[test]
    fn test_cache_stats() {
        let cache = InMemoryTtlCache::new();

        cache.put_with_ttl("key1", "value1", Duration::from_millis(50));
        cache.put_with_ttl("key2", "value2", Duration::from_secs(60));
        cache.put("key3", "value3");

        let stats = cache.stats();
        assert_eq!(stats.total_entries, 3);
        assert_eq!(stats.entries_with_ttl, 2);

        // 等待 key1 过期
        thread::sleep(Duration::from_millis(100));

        let stats = cache.stats();
        assert_eq!(stats.expired_entries, 1);
    }

    #[test]
    fn test_default_ttl() {
        let cache = InMemoryTtlCache::with_default_ttl(Duration::from_millis(50));

        cache.put("key1", "value1");
        assert!(!cache.is_expired(&"key1"));

        thread::sleep(Duration::from_millis(100));
        assert!(cache.is_expired(&"key1"));
    }

    #[test]
    fn test_cleanup_scheduler() {
        let scheduler = TtlCleanupScheduler::new(Duration::from_millis(100));

        assert!(!scheduler.should_cleanup());
        assert_eq!(scheduler.cleanup_count(), 0);

        thread::sleep(Duration::from_millis(150));
        assert!(scheduler.should_cleanup());

        scheduler.record_cleanup();
        assert_eq!(scheduler.cleanup_count(), 1);
        assert!(!scheduler.should_cleanup());
    }

    #[test]
    fn test_tti_expiration() {
        let entry = CacheEntry::with_ttl_and_tti("value", Duration::from_secs(60), Duration::from_millis(50));
        assert!(!entry.is_expired());

        // 等待 TTI 过期
        thread::sleep(Duration::from_millis(100));
        assert!(entry.is_expired());
    }

    #[test]
    fn test_touch_updates_access_time() {
        let mut entry = CacheEntry::with_ttl_and_tti("value", Duration::from_secs(60), Duration::from_millis(50));

        thread::sleep(Duration::from_millis(30));
        entry.touch();

        // TTI 应该被重置
        thread::sleep(Duration::from_millis(30));
        assert!(!entry.is_expired()); // 因为 touch 重置了 TTI

        thread::sleep(Duration::from_millis(60));
        assert!(entry.is_expired());
    }
}
