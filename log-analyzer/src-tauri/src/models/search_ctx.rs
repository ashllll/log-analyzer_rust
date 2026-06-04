//! 搜索上下文 — 管理搜索相关的共享资源

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::{Mutex, RwLock};
use tokio_util::sync::CancellationToken;

use la_search::DiskResultStore;

/// 搜索上下文，持有跨搜索操作的共享资源。
///
/// - `cancellation_tokens`: 活跃搜索的取消令牌，一个 search_id 一个。
/// - `disk_result_store`: 搜索结果磁盘缓存。Default 时为 None，setup() 中初始化。
/// - `thread_pool`: Rayon 线程池，所有 spawn_blocking 搜索共享。
///
/// 锁策略：disk_result_store 使用 RwLock（多读少写），其余用 Mutex。
pub struct SearchContext {
    cancellation_tokens: Arc<Mutex<HashMap<String, CancellationToken>>>,
    /// CR-06: Option 避免 Default 时因 I/O 失败导致 panic
    disk_result_store: RwLock<Option<Arc<DiskResultStore>>>,
    /// HI-01: 缓存线程池，避免每次搜索重复创建
    thread_pool: Arc<rayon::ThreadPool>,
}

impl Default for SearchContext {
    fn default() -> Self {
        Self {
            cancellation_tokens: Arc::new(Mutex::new(HashMap::new())),
            disk_result_store: RwLock::new(None),
            thread_pool: Arc::new(
                rayon::ThreadPoolBuilder::new()
                    .num_threads(num_cpus::get().max(2))
                    .build()
                    .expect("Failed to create search thread pool"),
            ),
        }
    }
}

impl SearchContext {
    // ── DiskResultStore ──

    /// 初始化 DiskResultStore 到指定的持久化目录。
    ///
    /// 由 setup() 调用，将默认临时目录替换为应用数据目录。
    /// 如果创建失败，disk_result_store 保持 None（fallback）。
    pub fn init_disk_result_store_at(&self, base_path: PathBuf) {
        let cache_dir = base_path.join("search-cache");
        match DiskResultStore::new(cache_dir.clone(), 50) {
            Ok(store) => {
                *self.disk_result_store.write() = Some(Arc::new(store));
                tracing::info!(
                    path = %cache_dir.display(),
                    "DiskResultStore initialized at app data directory"
                );
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    path = %cache_dir.display(),
                    "Failed to create DiskResultStore at app data dir; keeping fallback"
                );
            }
        }
    }

    /// 获取 DiskResultStore（如果已初始化）。
    pub fn get_disk_result_store(&self) -> Option<Arc<DiskResultStore>> {
        self.disk_result_store.read().clone()
    }

    // ── ThreadPool ──

    /// 获取共享的 Rayon 搜索线程池引用。
    pub fn get_thread_pool(&self) -> Arc<rayon::ThreadPool> {
        Arc::clone(&self.thread_pool)
    }

    // ── CancellationToken ──

    /// 获取指定搜索的取消令牌。
    pub fn get_cancellation_token(&self, search_id: &str) -> Option<CancellationToken> {
        self.cancellation_tokens.lock().get(search_id).cloned()
    }

    /// 注册搜索的取消令牌。
    pub fn insert_cancellation_token(&self, search_id: String, token: CancellationToken) {
        self.cancellation_tokens.lock().insert(search_id, token);
    }

    /// 移除搜索的取消令牌（搜索完成或取消后清理）。
    pub fn remove_cancellation_token(&self, search_id: &str) -> Option<CancellationToken> {
        self.cancellation_tokens.lock().remove(search_id)
    }

    /// 获取 cancellation_tokens Map 内部 Arc 引用。
    ///
    /// 供需要将 Map Arc 传入闭包的场景使用（如 search_logs 命令）。
    pub fn cancellation_tokens_arc(
        &self,
    ) -> Arc<Mutex<HashMap<String, CancellationToken>>> {
        Arc::clone(&self.cancellation_tokens)
    }

    /// 退出时清理所有搜索结果的磁盘缓存。
    pub fn cleanup_disk_result_store(&self) {
        if let Some(disk_store) = self.disk_result_store.read().as_ref() {
            disk_store.cleanup_all();
        }
    }
}
