//! 资源管理器 - 使用 RAII 模式进行自动资源清理
//!
//! 提供基于 scopeguard 的自动资源管理功能，包括：
//! - 临时目录自动清理
//! - 文件句柄管理
//! - 资源生命周期跟踪
//! - 应用程序关闭时的清理

use eyre::{eyre, Result};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use tracing::{debug, error, info, warn};

/// 资源类型枚举
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ResourceType {
    /// 临时目录
    TempDirectory,
    /// 文件句柄
    FileHandle,
    /// 搜索操作
    SearchOperation,
    /// 工作区资源
    WorkspaceResource,
}

/// 资源信息
#[derive(Debug, Clone)]
pub struct ResourceInfo {
    /// 资源ID
    pub id: String,
    /// 资源类型
    pub resource_type: ResourceType,
    /// 资源路径
    pub path: Option<PathBuf>,
    /// 创建时间
    pub created_at: std::time::SystemTime,
    /// 是否已清理
    pub cleaned: bool,
}

/// 资源管理器
///
/// 使用 RAII 模式自动管理资源生命周期
pub struct ResourceManager {
    /// 活跃资源注册表
    resources: Arc<Mutex<HashMap<String, ResourceInfo>>>,
    /// 临时目录存储
    temp_dirs: Arc<Mutex<HashMap<String, TempDir>>>,
    /// 清理队列
    cleanup_queue: Arc<Mutex<Vec<String>>>,
}

impl ResourceManager {
    /// 创建新的资源管理器
    pub fn new() -> Self {
        let manager = Self {
            resources: Arc::new(Mutex::new(HashMap::new())),
            temp_dirs: Arc::new(Mutex::new(HashMap::new())),
            cleanup_queue: Arc::new(Mutex::new(Vec::new())),
        };

        // 注册应用程序关闭时的清理
        let cleanup_resources = manager.resources.clone();
        let cleanup_temp_dirs = manager.temp_dirs.clone();

        std::panic::set_hook(Box::new(move |_| {
            Self::emergency_cleanup(&cleanup_resources, &cleanup_temp_dirs);
        }));

        manager
    }

    /// 创建临时目录并自动管理其生命周期
    pub fn create_temp_dir(&self, prefix: &str) -> Result<(String, PathBuf)> {
        let temp_dir = tempfile::Builder::new()
            .prefix(prefix)
            .tempdir()
            .map_err(|e| eyre!("Failed to create temporary directory: {}", e))?;

        let path = temp_dir.path().to_path_buf();
        let resource_id = uuid::Uuid::new_v4().to_string();

        // 注册资源信息
        let resource_info = ResourceInfo {
            id: resource_id.clone(),
            resource_type: ResourceType::TempDirectory,
            path: Some(path.clone()),
            created_at: std::time::SystemTime::now(),
            cleaned: false,
        };

        {
            let mut resources = self.resources.lock();
            resources.insert(resource_id.clone(), resource_info);
        }

        {
            let mut temp_dirs = self.temp_dirs.lock();
            temp_dirs.insert(resource_id.clone(), temp_dir);
        }

        info!(
            resource_id = %resource_id,
            path = %path.display(),
            "Created temporary directory"
        );

        Ok((resource_id, path))
    }

    /// 创建带有自动清理的临时目录守卫
    pub fn create_temp_dir_guard(&self, prefix: &str) -> Result<TempDirGuard> {
        let (resource_id, path) = self.create_temp_dir(prefix)?;
        let manager = Arc::new(self.clone());

        Ok(TempDirGuard {
            resource_id,
            path,
            manager,
        })
    }

    /// 注册资源并返回清理守卫
    pub fn register_resource<F>(
        &self,
        resource_type: ResourceType,
        path: Option<PathBuf>,
        cleanup_fn: F,
    ) -> Result<ResourceGuard<F>>
    where
        F: FnOnce() + Send + 'static,
    {
        let resource_id = uuid::Uuid::new_v4().to_string();

        let resource_info = ResourceInfo {
            id: resource_id.clone(),
            resource_type,
            path: path.clone(),
            created_at: std::time::SystemTime::now(),
            cleaned: false,
        };

        {
            let mut resources = self.resources.lock();
            resources.insert(resource_id.clone(), resource_info);
        }

        debug!(
            resource_id = %resource_id,
            path = ?path,
            "Registered resource"
        );

        Ok(ResourceGuard {
            resource_id,
            manager: Arc::new(self.clone()),
            cleanup_fn: Some(cleanup_fn),
        })
    }

    /// 手动清理资源
    pub fn cleanup_resource(&self, resource_id: &str) -> Result<()> {
        let mut cleaned = false;

        // 清理临时目录
        {
            let mut temp_dirs = self.temp_dirs.lock();
            if temp_dirs.remove(resource_id).is_some() {
                cleaned = true;
                debug!(resource_id = %resource_id, "Cleaned up temporary directory");
            }
        }

        // 更新资源状态
        {
            let mut resources = self.resources.lock();
            if let Some(resource) = resources.get_mut(resource_id) {
                resource.cleaned = true;
                cleaned = true;
            }
        }

        if cleaned {
            info!(resource_id = %resource_id, "Resource cleaned up");
            Ok(())
        } else {
            warn!(resource_id = %resource_id, "Resource not found for cleanup");
            Err(eyre!("Resource not found: {}", resource_id))
        }
    }

    /// 获取资源信息
    pub fn get_resource_info(&self, resource_id: &str) -> Option<ResourceInfo> {
        let resources = self.resources.lock();
        resources.get(resource_id).cloned()
    }

    /// 列出所有活跃资源
    pub fn list_active_resources(&self) -> Vec<ResourceInfo> {
        let resources = self.resources.lock();
        resources.values().filter(|r| !r.cleaned).cloned().collect()
    }

    /// 获取资源统计信息
    pub fn get_resource_stats(&self) -> ResourceStats {
        let resources = self.resources.lock();
        let temp_dirs = self.temp_dirs.lock();

        let total_resources = resources.len();
        let active_resources = resources.values().filter(|r| !r.cleaned).count();
        let temp_dir_count = temp_dirs.len();

        let by_type = resources
            .values()
            .fold(HashMap::new(), |mut acc, resource| {
                *acc.entry(resource.resource_type.clone()).or_insert(0) += 1;
                acc
            });

        ResourceStats {
            total_resources,
            active_resources,
            temp_dir_count,
            by_type,
        }
    }

    /// 调度延迟清理
    pub fn schedule_cleanup(&self, resource_id: String) {
        let mut queue = self.cleanup_queue.lock();
        if !queue.contains(&resource_id) {
            queue.push(resource_id.clone());
            debug!(resource_id = %resource_id, "Scheduled resource for cleanup");
        }
    }

    /// 获取清理队列大小
    pub fn cleanup_queue_size(&self) -> usize {
        let queue = self.cleanup_queue.lock();
        queue.len()
    }

    /// 清理所有资源
    pub fn cleanup_all(&self) -> Result<()> {
        info!("Starting cleanup of all resources");

        // 清理临时目录
        {
            let mut temp_dirs = self.temp_dirs.lock();
            let count = temp_dirs.len();
            temp_dirs.clear();
            info!(count = count, "Cleaned up temporary directories");
        }

        // 标记所有资源为已清理
        {
            let mut resources = self.resources.lock();
            for resource in resources.values_mut() {
                resource.cleaned = true;
            }
            info!(count = resources.len(), "Marked all resources as cleaned");
        }

        // 清空清理队列
        {
            let mut queue = self.cleanup_queue.lock();
            queue.clear();
        }

        info!("Completed cleanup of all resources");
        Ok(())
    }

    /// 紧急清理（用于 panic 处理）
    fn emergency_cleanup(
        resources: &Arc<Mutex<HashMap<String, ResourceInfo>>>,
        temp_dirs: &Arc<Mutex<HashMap<String, TempDir>>>,
    ) {
        eprintln!("Emergency cleanup triggered");

        // 清理临时目录
        if let Some(mut temp_dirs) = temp_dirs.try_lock() {
            let count = temp_dirs.len();
            temp_dirs.clear();
            eprintln!("Emergency cleanup: removed {} temporary directories", count);
        }

        // 标记资源为已清理
        if let Some(mut resources) = resources.try_lock() {
            for resource in resources.values_mut() {
                resource.cleaned = true;
            }
            eprintln!(
                "Emergency cleanup: marked {} resources as cleaned",
                resources.len()
            );
        }
    }
}

impl Clone for ResourceManager {
    fn clone(&self) -> Self {
        Self {
            resources: Arc::clone(&self.resources),
            temp_dirs: Arc::clone(&self.temp_dirs),
            cleanup_queue: Arc::clone(&self.cleanup_queue),
        }
    }
}

impl Default for ResourceManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 资源统计信息
#[derive(Debug, Clone)]
pub struct ResourceStats {
    pub total_resources: usize,
    pub active_resources: usize,
    pub temp_dir_count: usize,
    pub by_type: HashMap<ResourceType, usize>,
}

/// 临时目录守卫
///
/// 当守卫被丢弃时自动清理临时目录
pub struct TempDirGuard {
    resource_id: String,
    path: PathBuf,
    manager: Arc<ResourceManager>,
}

impl TempDirGuard {
    /// 获取临时目录路径
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 获取资源ID
    pub fn resource_id(&self) -> &str {
        &self.resource_id
    }

    /// 手动释放守卫并清理资源
    pub fn cleanup(self) -> Result<()> {
        drop(self); // 触发 Drop trait，实际清理在 Drop 中进行
        Ok(())
    }
}

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        if let Err(e) = self.manager.cleanup_resource(&self.resource_id) {
            error!(
                resource_id = %self.resource_id,
                error = %e,
                "Failed to cleanup temporary directory in guard drop"
            );
        }
    }
}

/// 通用资源守卫
///
/// 当守卫被丢弃时执行自定义清理函数
pub struct ResourceGuard<F>
where
    F: FnOnce() + Send + 'static,
{
    resource_id: String,
    manager: Arc<ResourceManager>,
    cleanup_fn: Option<F>,
}

impl<F> ResourceGuard<F>
where
    F: FnOnce() + Send + 'static,
{
    /// 获取资源ID
    pub fn resource_id(&self) -> &str {
        &self.resource_id
    }

    /// 手动释放守卫并执行清理
    pub fn cleanup(mut self) -> Result<()> {
        if let Some(cleanup_fn) = self.cleanup_fn.take() {
            cleanup_fn();
        }
        self.manager.cleanup_resource(&self.resource_id)?;
        Ok(())
    }
}

impl<F> Drop for ResourceGuard<F>
where
    F: FnOnce() + Send + 'static,
{
    fn drop(&mut self) {
        if let Some(cleanup_fn) = self.cleanup_fn.take() {
            cleanup_fn();
        }
        if let Err(e) = self.manager.cleanup_resource(&self.resource_id) {
            error!(
                resource_id = %self.resource_id,
                error = %e,
                "Failed to cleanup resource in guard drop"
            );
        }
    }
}

/// 便利宏：创建带有延迟清理的作用域
#[macro_export]
macro_rules! defer_cleanup {
    ($cleanup:expr) => {
        scopeguard::defer! { $cleanup }
    };
}

/// 便利宏：创建带有条件清理的作用域
#[macro_export]
macro_rules! defer_cleanup_on {
    ($condition:expr, $cleanup:expr) => {
        scopeguard::defer_on_unwind! {
            if $condition {
                $cleanup
            }
        }
    };
}

/// 资源追踪器
///
/// 用于监控活跃资源和检测资源泄漏
pub struct ResourceTracker {
    manager: Arc<ResourceManager>,
    check_interval: Duration,
    leak_threshold: Duration,
}

impl ResourceTracker {
    /// 创建新的资源追踪器
    pub fn new(manager: Arc<ResourceManager>) -> Self {
        Self {
            manager,
            check_interval: Duration::from_secs(60),
            leak_threshold: Duration::from_secs(3600), // 1 hour
        }
    }

    /// 设置检查间隔
    pub fn with_check_interval(mut self, interval: Duration) -> Self {
        self.check_interval = interval;
        self
    }

    /// 设置泄漏阈值
    pub fn with_leak_threshold(mut self, threshold: Duration) -> Self {
        self.leak_threshold = threshold;
        self
    }

    /// 检查资源泄漏
    pub fn check_for_leaks(&self) -> Vec<ResourceInfo> {
        let active_resources = self.manager.list_active_resources();
        let now = std::time::SystemTime::now();

        active_resources
            .into_iter()
            .filter(|resource| {
                if let Ok(duration) = now.duration_since(resource.created_at) {
                    duration > self.leak_threshold
                } else {
                    false
                }
            })
            .collect()
    }

    /// 生成资源报告
    pub fn generate_report(&self) -> ResourceReport {
        let stats = self.manager.get_resource_stats();
        let active_resources = self.manager.list_active_resources();
        let leaked_resources = self.check_for_leaks();

        let oldest_resource = active_resources
            .iter()
            .min_by_key(|r| r.created_at)
            .cloned();

        ResourceReport {
            stats,
            active_resources,
            leaked_resources,
            oldest_resource,
            generated_at: std::time::SystemTime::now(),
        }
    }

    /// 启动后台监控任务
    pub async fn start_monitoring(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(self.check_interval);

            loop {
                interval.tick().await;

                let leaked = self.check_for_leaks();
                if !leaked.is_empty() {
                    warn!(
                        leaked_count = leaked.len(),
                        "Detected potential resource leaks"
                    );

                    for resource in leaked {
                        warn!(
                            resource_id = %resource.id,
                            resource_type = ?resource.resource_type,
                            age_seconds = resource.created_at.elapsed().unwrap_or_default().as_secs(),
                            "Leaked resource detected"
                        );
                    }
                }

                let stats = self.manager.get_resource_stats();
                debug!(
                    total_resources = stats.total_resources,
                    active_resources = stats.active_resources,
                    temp_dirs = stats.temp_dir_count,
                    "Resource monitoring check"
                );
            }
        })
    }
}

/// 资源报告
#[derive(Debug, Clone)]
pub struct ResourceReport {
    pub stats: ResourceStats,
    pub active_resources: Vec<ResourceInfo>,
    pub leaked_resources: Vec<ResourceInfo>,
    pub oldest_resource: Option<ResourceInfo>,
    pub generated_at: std::time::SystemTime,
}

/// 清理队列处理器
///
/// 处理延迟清理和重试机制
pub struct CleanupQueueProcessor {
    manager: Arc<ResourceManager>,
    max_retries: usize,
    retry_delay: Duration,
}

impl CleanupQueueProcessor {
    /// 创建新的清理队列处理器
    pub fn new(manager: Arc<ResourceManager>) -> Self {
        Self {
            manager,
            max_retries: 3,
            retry_delay: Duration::from_secs(1),
        }
    }

    /// 设置最大重试次数
    pub fn with_max_retries(mut self, retries: usize) -> Self {
        self.max_retries = retries;
        self
    }

    /// 设置重试延迟
    pub fn with_retry_delay(mut self, delay: Duration) -> Self {
        self.retry_delay = delay;
        self
    }

    /// 处理清理队列
    pub async fn process_cleanup_queue(&self) -> Result<CleanupResult> {
        let mut cleanup_queue = {
            let queue = self.manager.cleanup_queue.lock();
            queue.clone()
        };

        let mut successful = 0;
        let mut failed = Vec::new();
        let mut retried = 0;

        for resource_id in cleanup_queue.drain(..) {
            let mut attempts = 0;
            let mut success = false;

            while attempts < self.max_retries && !success {
                attempts += 1;

                match self.manager.cleanup_resource(&resource_id) {
                    Ok(_) => {
                        successful += 1;
                        success = true;

                        if attempts > 1 {
                            retried += 1;
                            info!(
                                resource_id = %resource_id,
                                attempts = attempts,
                                "Resource cleanup succeeded after retry"
                            );
                        }
                    }
                    Err(e) => {
                        if attempts < self.max_retries {
                            warn!(
                                resource_id = %resource_id,
                                attempt = attempts,
                                error = %e,
                                "Resource cleanup failed, will retry"
                            );
                            tokio::time::sleep(self.retry_delay).await;
                        } else {
                            error!(
                                resource_id = %resource_id,
                                attempts = attempts,
                                error = %e,
                                "Resource cleanup failed after all retries"
                            );
                            failed.push((resource_id.clone(), e.to_string()));
                        }
                    }
                }
            }
        }

        // 清空队列
        {
            let mut queue = self.manager.cleanup_queue.lock();
            queue.clear();
        }

        Ok(CleanupResult {
            successful,
            failed,
            retried,
        })
    }

    /// 启动后台清理处理
    pub async fn start_processing(
        self: Arc<Self>,
        interval: Duration,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);

            loop {
                interval_timer.tick().await;

                match self.process_cleanup_queue().await {
                    Ok(result) => {
                        if result.successful > 0 || !result.failed.is_empty() {
                            info!(
                                successful = result.successful,
                                failed = result.failed.len(),
                                retried = result.retried,
                                "Cleanup queue processed"
                            );
                        }
                    }
                    Err(e) => {
                        error!(error = %e, "Failed to process cleanup queue");
                    }
                }
            }
        })
    }
}

/// 清理结果
#[derive(Debug, Clone)]
pub struct CleanupResult {
    pub successful: usize,
    pub failed: Vec<(String, String)>,
    pub retried: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_temp_dir_creation_and_cleanup() {
        let manager = ResourceManager::new();

        let (resource_id, path) = manager.create_temp_dir("test").unwrap();

        // 验证目录存在
        assert!(path.exists());

        // 验证资源已注册
        let resource_info = manager.get_resource_info(&resource_id).unwrap();
        assert_eq!(resource_info.resource_type, ResourceType::TempDirectory);
        assert_eq!(resource_info.path, Some(path.clone()));
        assert!(!resource_info.cleaned);

        // 清理资源
        manager.cleanup_resource(&resource_id).unwrap();

        // 验证资源已标记为清理
        let resource_info = manager.get_resource_info(&resource_id).unwrap();
        assert!(resource_info.cleaned);
    }

    #[test]
    fn test_temp_dir_guard() {
        let manager = ResourceManager::new();
        let temp_path;

        {
            let guard = manager.create_temp_dir_guard("test_guard").unwrap();
            temp_path = guard.path().to_path_buf();

            // 验证目录存在
            assert!(temp_path.exists());

            // 创建一个测试文件
            let test_file = temp_path.join("test.txt");
            fs::write(&test_file, "test content").unwrap();
            assert!(test_file.exists());
        } // guard 在这里被丢弃，应该触发清理

        // 验证目录已被清理（注意：TempDir 的清理是自动的）
        // 我们主要验证资源管理器中的状态
        let stats = manager.get_resource_stats();
        assert_eq!(stats.active_resources, 0);
    }

    #[test]
    fn test_resource_registration_and_cleanup() {
        let manager = ResourceManager::new();
        let cleanup_called = Arc::new(Mutex::new(false));
        let cleanup_called_clone = Arc::clone(&cleanup_called);

        {
            let _guard = manager
                .register_resource(
                    ResourceType::FileHandle,
                    Some(PathBuf::from("/test/path")),
                    move || {
                        *cleanup_called_clone.lock() = true;
                    },
                )
                .unwrap();

            // 验证资源已注册
            let stats = manager.get_resource_stats();
            assert_eq!(stats.active_resources, 1);
            assert_eq!(stats.by_type.get(&ResourceType::FileHandle), Some(&1));
        } // guard 在这里被丢弃

        // 验证清理函数被调用
        assert!(*cleanup_called.lock());

        // 验证资源已清理
        let stats = manager.get_resource_stats();
        assert_eq!(stats.active_resources, 0);
    }

    #[test]
    fn test_cleanup_all() {
        let manager = ResourceManager::new();

        // 创建多个资源
        let (_id1, _path1) = manager.create_temp_dir("test1").unwrap();
        let (_id2, _path2) = manager.create_temp_dir("test2").unwrap();

        let stats = manager.get_resource_stats();
        assert_eq!(stats.active_resources, 2);

        // 清理所有资源
        manager.cleanup_all().unwrap();

        let stats = manager.get_resource_stats();
        assert_eq!(stats.active_resources, 0);
    }

    #[test]
    fn test_defer_cleanup_functionality() {
        let cleanup_called = Arc::new(Mutex::new(false));
        let cleanup_called_clone = Arc::clone(&cleanup_called);

        {
            // 使用 scopeguard 直接创建守卫
            let _guard = scopeguard::guard((), |_| {
                *cleanup_called_clone.lock() = true;
            });

            // 清理函数还未被调用
            assert!(!*cleanup_called.lock());
        } // guard 在这里被丢弃，触发清理

        // 验证清理函数被调用
        assert!(*cleanup_called.lock());
    }

    #[test]
    fn test_resource_tracker() {
        let manager = Arc::new(ResourceManager::new());
        let tracker = ResourceTracker::new(Arc::clone(&manager));

        // 创建一些资源
        let (_id1, _path1) = manager.create_temp_dir("test1").unwrap();
        let (_id2, _path2) = manager.create_temp_dir("test2").unwrap();

        // 生成报告
        let report = tracker.generate_report();
        assert_eq!(report.stats.active_resources, 2);
        assert_eq!(report.active_resources.len(), 2);
        assert!(report.oldest_resource.is_some());

        // 检查泄漏（应该没有，因为资源刚创建）
        let leaked = tracker.check_for_leaks();
        assert!(leaked.is_empty());
    }

    #[test]
    fn test_cleanup_queue() {
        let manager = Arc::new(ResourceManager::new());

        // 调度一些清理任务
        manager.schedule_cleanup("resource1".to_string());
        manager.schedule_cleanup("resource2".to_string());
        manager.schedule_cleanup("resource1".to_string()); // 重复，应该被忽略

        assert_eq!(manager.cleanup_queue_size(), 2);

        // 清理所有资源应该清空队列
        manager.cleanup_all().unwrap();
        assert_eq!(manager.cleanup_queue_size(), 0);
    }

    #[tokio::test]
    async fn test_cleanup_queue_processor() {
        let manager = Arc::new(ResourceManager::new());
        let processor = Arc::new(CleanupQueueProcessor::new(Arc::clone(&manager)));

        // 创建一些资源并调度清理
        let (id1, _path1) = manager.create_temp_dir("test1").unwrap();
        let (id2, _path2) = manager.create_temp_dir("test2").unwrap();

        manager.schedule_cleanup(id1);
        manager.schedule_cleanup(id2);
        manager.schedule_cleanup("nonexistent".to_string());

        // 处理清理队列
        let result = processor.process_cleanup_queue().await.unwrap();

        // 应该有2个成功，1个失败
        assert_eq!(result.successful, 2);
        assert_eq!(result.failed.len(), 1);
        assert_eq!(result.retried, 0);

        // 队列应该被清空
        assert_eq!(manager.cleanup_queue_size(), 0);
    }
}
