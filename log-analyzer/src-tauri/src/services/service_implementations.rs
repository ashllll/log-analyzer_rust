//! 具体服务实现
//!
//! 为现有服务提供 Service trait 实现，包括：
//! - 查询执行服务
//! - 文件监听服务
//! - 搜索统计服务
//! - 缓存管理服务

use eyre::Result;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::{debug, info, warn};

use super::query_executor::QueryExecutor;
use super::service_container::{Service, ServiceHealth};
use crate::utils::{AsyncResourceManager, CacheManager};

/// 查询执行服务实现
pub struct QueryExecutorService {
    executor: Arc<parking_lot::Mutex<QueryExecutor>>,
    is_running: AtomicBool,
    cache_size: usize,
}

impl QueryExecutorService {
    pub fn new(cache_size: usize) -> Self {
        Self {
            executor: Arc::new(parking_lot::Mutex::new(QueryExecutor::new(cache_size))),
            is_running: AtomicBool::new(false),
            cache_size,
        }
    }

    pub fn get_executor(&self) -> Arc<parking_lot::Mutex<QueryExecutor>> {
        self.executor.clone()
    }
}

impl Service for QueryExecutorService {
    fn start(&self) -> Result<()> {
        if self.is_running.load(Ordering::SeqCst) {
            return Ok(());
        }

        info!(
            service = "QueryExecutorService",
            cache_size = self.cache_size,
            "Starting query executor service"
        );

        // 初始化查询执行器
        let _executor = self.executor.lock();

        self.is_running.store(true, Ordering::SeqCst);
        info!("Query executor service started successfully");

        Ok(())
    }

    fn stop(&self) -> Result<()> {
        if !self.is_running.load(Ordering::SeqCst) {
            return Ok(());
        }

        info!("Stopping query executor service");

        // 清理资源
        self.is_running.store(false, Ordering::SeqCst);

        info!("Query executor service stopped successfully");
        Ok(())
    }

    fn health_check(&self) -> Result<ServiceHealth> {
        let is_healthy = self.is_running.load(Ordering::SeqCst);

        if is_healthy {
            Ok(ServiceHealth::healthy()
                .with_detail("cache_size".to_string(), self.cache_size.to_string())
                .with_detail("status".to_string(), "running".to_string()))
        } else {
            Ok(ServiceHealth::unhealthy("Service not running".to_string()))
        }
    }

    fn service_name(&self) -> &'static str {
        "QueryExecutorService"
    }
}

/// 缓存管理服务实现
pub struct CacheManagerService {
    cache_manager: Arc<CacheManager>,
    is_running: AtomicBool,
    cleanup_interval: std::time::Duration,
    cleanup_handle: Arc<parking_lot::Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl CacheManagerService {
    pub fn new(cache_manager: Arc<CacheManager>) -> Self {
        Self {
            cache_manager,
            is_running: AtomicBool::new(false),
            cleanup_interval: std::time::Duration::from_secs(300), // 5 minutes
            cleanup_handle: Arc::new(parking_lot::Mutex::new(None)),
        }
    }

    pub fn with_cleanup_interval(mut self, interval: std::time::Duration) -> Self {
        self.cleanup_interval = interval;
        self
    }

    pub fn get_cache_manager(&self) -> Arc<CacheManager> {
        self.cache_manager.clone()
    }
}

impl Service for CacheManagerService {
    fn start(&self) -> Result<()> {
        if self.is_running.load(Ordering::SeqCst) {
            return Ok(());
        }

        info!(
            service = "CacheManagerService",
            cleanup_interval_secs = self.cleanup_interval.as_secs(),
            "Starting cache manager service"
        );

        // 只在有 tokio runtime 的情况下启动定期清理任务
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            let cache_manager = self.cache_manager.clone();
            let cleanup_interval = self.cleanup_interval;
            let is_running = Arc::new(AtomicBool::new(true));
            let is_running_clone = is_running.clone();

            let cleanup_task = handle.spawn(async move {
                let mut interval = tokio::time::interval(cleanup_interval);

                while is_running_clone.load(Ordering::SeqCst) {
                    interval.tick().await;

                    debug!("Running cache cleanup");
                    if let Err(e) = cache_manager.cleanup_expired_entries() {
                        warn!(error = %e, "Cache cleanup failed");
                    } else {
                        debug!("Cache cleanup completed successfully");
                    }
                }

                info!("Cache cleanup task stopped");
            });

            *self.cleanup_handle.lock() = Some(cleanup_task);
        } else {
            debug!("No tokio runtime available, skipping cleanup task");
        }

        self.is_running.store(true, Ordering::SeqCst);

        info!("Cache manager service started successfully");
        Ok(())
    }

    fn stop(&self) -> Result<()> {
        if !self.is_running.load(Ordering::SeqCst) {
            return Ok(());
        }

        info!("Stopping cache manager service");

        self.is_running.store(false, Ordering::SeqCst);

        // 停止清理任务
        if let Some(handle) = self.cleanup_handle.lock().take() {
            handle.abort();
            debug!("Cache cleanup task aborted");
        }

        info!("Cache manager service stopped successfully");
        Ok(())
    }

    fn health_check(&self) -> Result<ServiceHealth> {
        let is_healthy = self.is_running.load(Ordering::SeqCst);

        if is_healthy {
            // 获取缓存统计信息
            let stats = self.cache_manager.get_cache_statistics();

            Ok(ServiceHealth::healthy()
                .with_detail("status".to_string(), "running".to_string())
                .with_detail("entry_count".to_string(), stats.entry_count.to_string())
                .with_detail(
                    "l1_hit_rate".to_string(),
                    format!("{:.2}%", stats.l1_hit_rate * 100.0),
                )
                .with_detail(
                    "l2_hit_rate".to_string(),
                    format!("{:.2}%", stats.l2_hit_rate * 100.0),
                )
                .with_detail(
                    "cleanup_interval_secs".to_string(),
                    self.cleanup_interval.as_secs().to_string(),
                ))
        } else {
            Ok(ServiceHealth::unhealthy("Service not running".to_string()))
        }
    }

    fn service_name(&self) -> &'static str {
        "CacheManagerService"
    }
}

/// 异步资源管理服务实现
pub struct AsyncResourceManagerService {
    resource_manager: Arc<AsyncResourceManager>,
    is_running: AtomicBool,
}

impl AsyncResourceManagerService {
    pub fn new(resource_manager: Arc<AsyncResourceManager>) -> Self {
        Self {
            resource_manager,
            is_running: AtomicBool::new(false),
        }
    }

    pub fn get_resource_manager(&self) -> Arc<AsyncResourceManager> {
        self.resource_manager.clone()
    }
}

impl Service for AsyncResourceManagerService {
    fn start(&self) -> Result<()> {
        if self.is_running.load(Ordering::SeqCst) {
            return Ok(());
        }

        info!(
            service = "AsyncResourceManagerService",
            "Starting async resource manager service"
        );

        self.is_running.store(true, Ordering::SeqCst);

        info!("Async resource manager service started successfully");
        Ok(())
    }

    fn stop(&self) -> Result<()> {
        if !self.is_running.load(Ordering::SeqCst) {
            return Ok(());
        }

        info!("Stopping async resource manager service");

        self.is_running.store(false, Ordering::SeqCst);

        info!("Async resource manager service stopped successfully");
        Ok(())
    }

    fn health_check(&self) -> Result<ServiceHealth> {
        let is_healthy = self.is_running.load(Ordering::SeqCst);

        if is_healthy {
            Ok(ServiceHealth::healthy()
                .with_detail("status".to_string(), "running".to_string()))
        } else {
            Ok(ServiceHealth::unhealthy("Service not running".to_string()))
        }
    }

    fn service_name(&self) -> &'static str {
        "AsyncResourceManagerService"
    }
}

/// 文件监听服务实现
pub struct FileWatcherService {
    is_running: AtomicBool,
    watchers: Arc<parking_lot::Mutex<HashMap<String, notify::RecommendedWatcher>>>,
}

impl FileWatcherService {
    pub fn new() -> Self {
        Self {
            is_running: AtomicBool::new(false),
            watchers: Arc::new(parking_lot::Mutex::new(HashMap::new())),
        }
    }

    pub fn get_watchers(
        &self,
    ) -> Arc<parking_lot::Mutex<HashMap<String, notify::RecommendedWatcher>>> {
        self.watchers.clone()
    }
}

impl Service for FileWatcherService {
    fn start(&self) -> Result<()> {
        if self.is_running.load(Ordering::SeqCst) {
            return Ok(());
        }

        info!(
            service = "FileWatcherService",
            "Starting file watcher service"
        );

        // 初始化文件监听器存储
        let _watchers = self.watchers.lock();

        self.is_running.store(true, Ordering::SeqCst);
        info!("File watcher service started successfully");

        Ok(())
    }

    fn stop(&self) -> Result<()> {
        if !self.is_running.load(Ordering::SeqCst) {
            return Ok(());
        }

        info!("Stopping file watcher service");

        // 停止所有活跃的监听器
        let mut watchers = self.watchers.lock();
        let watcher_count = watchers.len();

        watchers.clear();

        self.is_running.store(false, Ordering::SeqCst);

        info!(
            stopped_watchers = watcher_count,
            "File watcher service stopped successfully"
        );

        Ok(())
    }

    fn health_check(&self) -> Result<ServiceHealth> {
        let is_healthy = self.is_running.load(Ordering::SeqCst);

        if is_healthy {
            let watchers = self.watchers.lock();
            let active_watchers = watchers.len();

            Ok(ServiceHealth::healthy()
                .with_detail("status".to_string(), "running".to_string())
                .with_detail("active_watchers".to_string(), active_watchers.to_string()))
        } else {
            Ok(ServiceHealth::unhealthy("Service not running".to_string()))
        }
    }

    fn service_name(&self) -> &'static str {
        "FileWatcherService"
    }
}

impl Default for FileWatcherService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_query_executor_service_lifecycle() -> Result<()> {
        let service = QueryExecutorService::new(100);

        // 初始状态
        assert!(!service.is_running.load(Ordering::SeqCst));

        // 启动服务
        service.start()?;
        assert!(service.is_running.load(Ordering::SeqCst));

        // 健康检查
        let health = service.health_check()?;
        assert!(health.is_healthy);
        assert_eq!(health.details.get("cache_size"), Some(&"100".to_string()));

        // 停止服务
        service.stop()?;
        assert!(!service.is_running.load(Ordering::SeqCst));

        Ok(())
    }

    #[test]
    fn test_file_watcher_service_lifecycle() -> Result<()> {
        let service = FileWatcherService::new();

        // 初始状态
        assert!(!service.is_running.load(Ordering::SeqCst));

        // 启动服务
        service.start()?;
        assert!(service.is_running.load(Ordering::SeqCst));

        // 健康检查
        let health = service.health_check()?;
        assert!(health.is_healthy);
        assert_eq!(
            health.details.get("active_watchers"),
            Some(&"0".to_string())
        );

        // 停止服务
        service.stop()?;
        assert!(!service.is_running.load(Ordering::SeqCst));

        Ok(())
    }

    #[tokio::test]
    async fn test_cache_manager_service_lifecycle() -> Result<()> {
        let search_cache = Arc::new(
            moka::sync::Cache::builder()
                .max_capacity(100)
                .time_to_live(Duration::from_secs(60))
                .build(),
        );

        let cache_manager = Arc::new(CacheManager::new(search_cache));
        let service = CacheManagerService::new(cache_manager)
            .with_cleanup_interval(Duration::from_millis(100));

        // 启动服务
        service.start()?;
        assert!(service.is_running.load(Ordering::SeqCst));

        // 等待一小段时间让清理任务运行
        tokio::time::sleep(Duration::from_millis(150)).await;

        // 健康检查
        let health = service.health_check()?;
        assert!(health.is_healthy);

        // 停止服务
        service.stop()?;
        assert!(!service.is_running.load(Ordering::SeqCst));

        Ok(())
    }
}
