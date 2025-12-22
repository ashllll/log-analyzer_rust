//! 资源追踪器 - 监控和管理活跃资源
//!
//! 本模块提供资源生命周期追踪功能，包括：
//! - 活跃资源的注册和追踪
//! - 资源泄漏检测
//! - 应用关闭时的自动清理
//! - 清理队列处理和重试机制
//!
//! # 设计原则
//!
//! - 集中管理所有资源的生命周期
//! - 提供资源泄漏检测和报告
//! - 支持优雅关闭和强制清理
//! - 集成 tracing 进行资源追踪

use crossbeam::queue::SegQueue;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tracing::{error, info, warn};

use super::cleanup::{process_cleanup_queue, try_cleanup_temp_dir};

/// 资源类型
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ResourceType {
    /// 临时目录
    TempDirectory,
    /// 文件句柄
    FileHandle,
    /// 网络连接
    NetworkConnection,
    /// 后台任务
    BackgroundTask,
    /// 其他资源
    Other(String),
}

/// 资源信息
#[derive(Debug, Clone)]
pub struct ResourceInfo {
    /// 资源ID
    pub id: String,
    /// 资源类型
    pub resource_type: ResourceType,
    /// 资源路径或描述
    pub path: String,
    /// 创建时间
    pub created_at: SystemTime,
    /// 是否已清理
    pub cleaned: bool,
}

impl ResourceInfo {
    /// 创建新的资源信息
    pub fn new(id: String, resource_type: ResourceType, path: String) -> Self {
        Self {
            id,
            resource_type,
            path,
            created_at: SystemTime::now(),
            cleaned: false,
        }
    }

    /// 获取资源存活时间
    pub fn age(&self) -> Duration {
        SystemTime::now()
            .duration_since(self.created_at)
            .unwrap_or(Duration::from_secs(0))
    }
}

/// 资源追踪器
///
/// 追踪应用中所有活跃资源，提供资源泄漏检测和自动清理功能。
pub struct ResourceTracker {
    /// 活跃资源映射
    resources: Arc<Mutex<HashMap<String, ResourceInfo>>>,
    /// 清理队列
    cleanup_queue: Arc<SegQueue<PathBuf>>,
    /// 是否启用泄漏检测
    leak_detection_enabled: bool,
}

impl ResourceTracker {
    /// 创建新的资源追踪器
    ///
    /// # 参数
    ///
    /// - `cleanup_queue` - 清理队列引用
    pub fn new(cleanup_queue: Arc<SegQueue<PathBuf>>) -> Self {
        info!("ResourceTracker initialized");
        Self {
            resources: Arc::new(Mutex::new(HashMap::new())),
            cleanup_queue,
            leak_detection_enabled: true,
        }
    }

    /// 注册新资源
    ///
    /// # 参数
    ///
    /// - `id` - 资源唯一标识符
    /// - `resource_type` - 资源类型
    /// - `path` - 资源路径或描述
    pub fn register_resource(&self, id: String, resource_type: ResourceType, path: String) {
        let info = ResourceInfo::new(id.clone(), resource_type.clone(), path.clone());
        
        {
            let mut resources = self.resources.lock();
            resources.insert(id.clone(), info);
        }
        
        info!(
            "Registered resource: {} (type: {:?}, path: {})",
            id, resource_type, path
        );
    }

    /// 标记资源为已清理
    ///
    /// # 参数
    ///
    /// - `id` - 资源ID
    pub fn mark_cleaned(&self, id: &str) {
        let mut resources = self.resources.lock();
        if let Some(info) = resources.get_mut(id) {
            info.cleaned = true;
            info!("Marked resource as cleaned: {}", id);
        }
    }

    /// 移除资源记录
    ///
    /// # 参数
    ///
    /// - `id` - 资源ID
    pub fn unregister_resource(&self, id: &str) {
        let mut resources = self.resources.lock();
        if resources.remove(id).is_some() {
            info!("Unregistered resource: {}", id);
        }
    }

    /// 获取活跃资源数量
    pub fn active_count(&self) -> usize {
        let resources = self.resources.lock();
        resources.values().filter(|r| !r.cleaned).count()
    }

    /// 获取所有活跃资源
    pub fn get_active_resources(&self) -> Vec<ResourceInfo> {
        let resources = self.resources.lock();
        resources
            .values()
            .filter(|r| !r.cleaned)
            .cloned()
            .collect()
    }

    /// 检测资源泄漏
    ///
    /// 返回存活时间超过阈值的未清理资源
    ///
    /// # 参数
    ///
    /// - `threshold` - 时间阈值
    pub fn detect_leaks(&self, threshold: Duration) -> Vec<ResourceInfo> {
        if !self.leak_detection_enabled {
            return Vec::new();
        }

        let resources = self.resources.lock();
        let leaks: Vec<ResourceInfo> = resources
            .values()
            .filter(|r| !r.cleaned && r.age() > threshold)
            .cloned()
            .collect();

        if !leaks.is_empty() {
            warn!("Detected {} potential resource leaks", leaks.len());
            for leak in &leaks {
                warn!(
                    "Leaked resource: {} (type: {:?}, age: {:?})",
                    leak.id,
                    leak.resource_type,
                    leak.age()
                );
            }
        }

        leaks
    }

    /// 清理特定资源
    ///
    /// # 参数
    ///
    /// - `id` - 资源ID
    ///
    /// # 返回值
    ///
    /// - `Ok(())` - 清理成功
    /// - `Err(String)` - 清理失败
    pub fn cleanup_resource(&self, id: &str) -> Result<(), String> {
        let resource_info = {
            let resources = self.resources.lock();
            resources.get(id).cloned()
        };

        if let Some(info) = resource_info {
            match info.resource_type {
                ResourceType::TempDirectory => {
                    let path = PathBuf::from(&info.path);
                    try_cleanup_temp_dir(&path, &self.cleanup_queue);
                    self.mark_cleaned(id);
                    Ok(())
                }
                _ => {
                    warn!("Cleanup not implemented for resource type: {:?}", info.resource_type);
                    self.mark_cleaned(id);
                    Ok(())
                }
            }
        } else {
            Err(format!("Resource not found: {}", id))
        }
    }

    /// 清理所有资源
    ///
    /// 用于应用关闭时的清理
    pub fn cleanup_all(&self) {
        info!("Cleaning up all resources");

        let resource_ids: Vec<String> = {
            let resources = self.resources.lock();
            resources.keys().cloned().collect()
        };

        let mut success_count = 0;
        let mut failure_count = 0;

        for id in &resource_ids {
            match self.cleanup_resource(id) {
                Ok(_) => success_count += 1,
                Err(e) => {
                    error!("Failed to cleanup resource {}: {}", id, e);
                    failure_count += 1;
                }
            }
        }

        // 处理清理队列
        process_cleanup_queue(&self.cleanup_queue);

        info!(
            "Resource cleanup completed: {} succeeded, {} failed",
            success_count, failure_count
        );
    }

    /// 生成资源报告
    pub fn generate_report(&self) -> ResourceReport {
        let resources = self.resources.lock();
        
        let total = resources.len();
        let active = resources.values().filter(|r| !r.cleaned).count();
        let cleaned = resources.values().filter(|r| r.cleaned).count();
        
        let by_type: HashMap<String, usize> = resources
            .values()
            .filter(|r| !r.cleaned)
            .fold(HashMap::new(), |mut acc, r| {
                let type_name = format!("{:?}", r.resource_type);
                *acc.entry(type_name).or_insert(0) += 1;
                acc
            });

        ResourceReport {
            total,
            active,
            cleaned,
            by_type,
        }
    }

    /// 启用或禁用泄漏检测
    pub fn set_leak_detection(&mut self, enabled: bool) {
        self.leak_detection_enabled = enabled;
        info!("Leak detection {}", if enabled { "enabled" } else { "disabled" });
    }
}

/// 资源报告
#[derive(Debug, Clone)]
pub struct ResourceReport {
    /// 总资源数
    pub total: usize,
    /// 活跃资源数
    pub active: usize,
    /// 已清理资源数
    pub cleaned: usize,
    /// 按类型分组的活跃资源数
    pub by_type: HashMap<String, usize>,
}

impl ResourceReport {
    /// 打印报告
    pub fn print(&self) {
        info!("=== Resource Report ===");
        info!("Total resources: {}", self.total);
        info!("Active resources: {}", self.active);
        info!("Cleaned resources: {}", self.cleaned);
        info!("By type:");
        for (type_name, count) in &self.by_type {
            info!("  {}: {}", type_name, count);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_register_and_unregister() {
        let cleanup_queue = Arc::new(SegQueue::new());
        let tracker = ResourceTracker::new(cleanup_queue);
        
        tracker.register_resource(
            "test-1".to_string(),
            ResourceType::TempDirectory,
            "/tmp/test".to_string(),
        );
        
        assert_eq!(tracker.active_count(), 1);
        
        tracker.unregister_resource("test-1");
        assert_eq!(tracker.active_count(), 0);
    }

    #[test]
    fn test_mark_cleaned() {
        let cleanup_queue = Arc::new(SegQueue::new());
        let tracker = ResourceTracker::new(cleanup_queue);
        
        tracker.register_resource(
            "test-1".to_string(),
            ResourceType::TempDirectory,
            "/tmp/test".to_string(),
        );
        
        assert_eq!(tracker.active_count(), 1);
        
        tracker.mark_cleaned("test-1");
        assert_eq!(tracker.active_count(), 0);
    }

    #[test]
    fn test_leak_detection() {
        let cleanup_queue = Arc::new(SegQueue::new());
        let tracker = ResourceTracker::new(cleanup_queue);
        
        tracker.register_resource(
            "test-1".to_string(),
            ResourceType::TempDirectory,
            "/tmp/test".to_string(),
        );
        
        // 等待一小段时间
        sleep(Duration::from_millis(100));
        
        // 检测泄漏（阈值设为 50ms）
        let leaks = tracker.detect_leaks(Duration::from_millis(50));
        assert_eq!(leaks.len(), 1);
        
        // 标记为已清理后不应再检测到泄漏
        tracker.mark_cleaned("test-1");
        let leaks = tracker.detect_leaks(Duration::from_millis(50));
        assert_eq!(leaks.len(), 0);
    }

    #[test]
    fn test_resource_report() {
        let cleanup_queue = Arc::new(SegQueue::new());
        let tracker = ResourceTracker::new(cleanup_queue);
        
        tracker.register_resource(
            "test-1".to_string(),
            ResourceType::TempDirectory,
            "/tmp/test1".to_string(),
        );
        
        tracker.register_resource(
            "test-2".to_string(),
            ResourceType::FileHandle,
            "/tmp/test2".to_string(),
        );
        
        tracker.mark_cleaned("test-1");
        
        let report = tracker.generate_report();
        assert_eq!(report.total, 2);
        assert_eq!(report.active, 1);
        assert_eq!(report.cleaned, 1);
    }

    #[test]
    fn test_cleanup_all() {
        let cleanup_queue = Arc::new(SegQueue::new());
        let tracker = ResourceTracker::new(cleanup_queue);
        
        tracker.register_resource(
            "test-1".to_string(),
            ResourceType::TempDirectory,
            "/tmp/test1".to_string(),
        );
        
        tracker.register_resource(
            "test-2".to_string(),
            ResourceType::FileHandle,
            "/tmp/test2".to_string(),
        );
        
        assert_eq!(tracker.active_count(), 2);
        
        tracker.cleanup_all();
        
        // 所有资源应该被标记为已清理
        assert_eq!(tracker.active_count(), 0);
    }
}
