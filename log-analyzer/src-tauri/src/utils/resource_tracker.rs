//! 资源追踪器 - 监控和管理活跃资源
//!
//! 本模块提供资源生命周期追踪功能，包括：
//! - 活跃资源的注册和追踪
//! - 资源泄漏检测
//! - 应用关闭时的自动清理
//! - 清理队列处理和重试机制

use crate::utils::cleanup::{process_cleanup_queue, try_cleanup_temp_dir, CleanupQueue};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tracing::{error, info, warn};

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
    pub fn new(id: String, resource_type: ResourceType, path: String) -> Self {
        Self {
            id,
            resource_type,
            path,
            created_at: SystemTime::now(),
            cleaned: false,
        }
    }

    pub fn age(&self) -> Duration {
        SystemTime::now()
            .duration_since(self.created_at)
            .unwrap_or(Duration::from_secs(0))
    }
}

/// 资源追踪器
pub struct ResourceTracker {
    resources: Arc<Mutex<HashMap<String, ResourceInfo>>>,
    cleanup_queue: Arc<CleanupQueue>,
    leak_detection_enabled: bool,
}

impl ResourceTracker {
    pub fn new(cleanup_queue: Arc<CleanupQueue>) -> Self {
        info!("ResourceTracker initialized");
        Self {
            resources: Arc::new(Mutex::new(HashMap::new())),
            cleanup_queue,
            leak_detection_enabled: true,
        }
    }

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

    pub fn mark_cleaned(&self, id: &str) {
        let mut resources = self.resources.lock();
        if let Some(info) = resources.get_mut(id) {
            info.cleaned = true;
            info!("Marked resource as cleaned: {}", id);
        }
    }

    pub fn unregister_resource(&self, id: &str) {
        let mut resources = self.resources.lock();
        if resources.remove(id).is_some() {
            info!("Unregistered resource: {}", id);
        }
    }

    pub fn active_count(&self) -> usize {
        let resources = self.resources.lock();
        resources.values().filter(|r| !r.cleaned).count()
    }

    pub fn get_active_resources(&self) -> Vec<ResourceInfo> {
        let resources = self.resources.lock();
        resources.values().filter(|r| !r.cleaned).cloned().collect()
    }

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
                    warn!(
                        "Cleanup not implemented for resource type: {:?}",
                        info.resource_type
                    );
                    self.mark_cleaned(id);
                    Ok(())
                }
            }
        } else {
            Err(format!("Resource not found: {}", id))
        }
    }

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

        process_cleanup_queue(&self.cleanup_queue);

        info!(
            "Resource cleanup completed: {} succeeded, {} failed",
            success_count, failure_count
        );
    }

    pub fn generate_report(&self) -> ResourceReport {
        let resources = self.resources.lock();

        let total = resources.len();
        let active = resources.values().filter(|r| !r.cleaned).count();
        let cleaned = resources.values().filter(|r| r.cleaned).count();

        let by_type: HashMap<String, usize> =
            resources
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

    pub fn set_leak_detection(&mut self, enabled: bool) {
        self.leak_detection_enabled = enabled;
        info!(
            "Leak detection {}",
            if enabled { "enabled" } else { "disabled" }
        );
    }
}

/// 资源报告
#[derive(Debug, Clone)]
pub struct ResourceReport {
    pub total: usize,
    pub active: usize,
    pub cleaned: usize,
    pub by_type: HashMap<String, usize>,
}

impl ResourceReport {
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
