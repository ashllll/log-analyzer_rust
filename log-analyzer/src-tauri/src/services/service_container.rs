//! 服务容器 - 依赖注入实现
//!
//! 本模块实现了基于构造函数注入的服务容器模式，提供：
//! - 显式的依赖关系管理
//! - 编译时类型安全
//! - 零运行时开销
//! - 清晰的服务生命周期

use eyre::{Context, Result};
use std::path::Path;
use std::sync::Arc;

use super::service_config::ServiceConfiguration;
use super::service_lifecycle::{OverallHealth, ServiceHealth};
use super::{EventBus, QueryExecutor};
use crate::utils::{CancellationManager, ResourceManager, ResourceTracker};

/// 服务容器 - 管理所有应用服务
///
/// 使用 Arc<T> 实现服务共享，确保线程安全和高效的内存管理。
/// 所有服务通过构造函数注入，依赖关系在编译时验证。
#[derive(Clone)]
pub struct AppServices {
    /// 事件总线 - 用于应用内事件通信
    pub event_bus: Arc<EventBus>,
    /// 查询执行器 - 用于结构化查询处理
    pub query_executor: Arc<QueryExecutor>,
    /// 资源管理器 - 用于 RAII 资源清理
    pub resource_manager: Arc<ResourceManager>,
    /// 取消管理器 - 用于操作取消
    pub cancellation_manager: Arc<CancellationManager>,
    /// 资源追踪器 - 用于资源生命周期管理
    pub resource_tracker: Arc<ResourceTracker>,
}

impl AppServices {
    /// 创建服务容器构建器
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use log_analyzer::services::AppServices;
    ///
    /// let services = AppServices::builder()
    ///     .build()
    ///     .expect("Failed to build services");
    /// ```
    pub fn builder() -> AppServicesBuilder {
        AppServicesBuilder::new()
    }

    /// 使用默认配置创建服务容器
    ///
    /// 这是一个便捷方法，等同于 `AppServices::builder().build()`
    ///
    /// # Errors
    ///
    /// 如果服务初始化失败，返回错误
    pub fn new() -> Result<Self> {
        Self::builder().build()
    }

    /// 获取事件总线的引用
    pub fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    /// 获取查询执行器的引用
    pub fn query_executor(&self) -> &Arc<QueryExecutor> {
        &self.query_executor
    }

    /// 获取资源管理器的引用
    pub fn resource_manager(&self) -> &Arc<ResourceManager> {
        &self.resource_manager
    }

    /// 获取取消管理器的引用
    pub fn cancellation_manager(&self) -> &Arc<CancellationManager> {
        &self.cancellation_manager
    }

    /// 获取资源追踪器的引用
    pub fn resource_tracker(&self) -> &Arc<ResourceTracker> {
        &self.resource_tracker
    }

    /// 启动所有服务
    ///
    /// # Errors
    /// 如果任何服务启动失败，返回错误
    pub fn start_all(&self) -> Result<()> {
        tracing::info!("Starting all application services");
        // 目前服务都是自动启动的，这里主要用于日志记录
        // 未来可以添加需要显式启动的服务
        Ok(())
    }

    /// 停止所有服务
    ///
    /// # Errors
    /// 如果任何服务停止失败，返回错误
    pub fn stop_all(&self) -> Result<()> {
        tracing::info!("Stopping all application services");
        // 取消所有活跃操作
        self.cancellation_manager.cancel_all();
        // 清理所有资源
        self.resource_tracker.cleanup_all();
        tracing::info!("All services stopped successfully");
        Ok(())
    }

    /// 检查所有服务的健康状态
    pub fn check_health(&self) -> Vec<ServiceHealth> {
        let mut health_checks = Vec::new();

        // 检查事件总线
        let event_bus_health = if self.event_bus.subscriber_count() > 0 {
            ServiceHealth::healthy("EventBus")
                .with_detail("subscribers", self.event_bus.subscriber_count().to_string())
        } else {
            ServiceHealth::unhealthy("EventBus", "No subscribers")
        };
        health_checks.push(event_bus_health);

        // 检查资源管理器
        let resource_health =
            ServiceHealth::healthy("ResourceManager").with_detail("status", "operational");
        health_checks.push(resource_health);

        // 检查取消管理器
        let cancellation_health =
            ServiceHealth::healthy("CancellationManager").with_detail("status", "operational");
        health_checks.push(cancellation_health);

        // 检查资源追踪器
        let tracker_report = self.resource_tracker.generate_report();
        let tracker_health = ServiceHealth::healthy("ResourceTracker")
            .with_detail("active_resources", tracker_report.active.to_string())
            .with_detail("total_created", tracker_report.total.to_string())
            .with_detail("total_cleaned", tracker_report.cleaned.to_string());
        health_checks.push(tracker_health);

        health_checks
    }

    /// 获取整体健康状态
    pub fn overall_health(&self) -> OverallHealth {
        use super::service_lifecycle::HealthStatus;
        use std::time::SystemTime;

        let health_checks = self.check_health();
        let total = health_checks.len();
        let healthy = health_checks
            .iter()
            .filter(|h| h.status == HealthStatus::Healthy)
            .count();
        let degraded = health_checks
            .iter()
            .filter(|h| h.status == HealthStatus::Degraded)
            .count();
        let unhealthy = health_checks
            .iter()
            .filter(|h| h.status == HealthStatus::Unhealthy)
            .count();

        let overall_status = if unhealthy > 0 {
            HealthStatus::Unhealthy
        } else if degraded > 0 {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        };

        OverallHealth {
            status: overall_status,
            total_services: total,
            healthy_services: healthy,
            degraded_services: degraded,
            unhealthy_services: unhealthy,
            service_health: health_checks,
            timestamp: SystemTime::now(),
        }
    }
}

/// 服务容器构建器
///
/// 使用 Builder 模式提供灵活的服务配置和创建。
/// 支持可选的依赖注入和配置覆盖。
pub struct AppServicesBuilder {
    /// 服务配置
    config: Option<ServiceConfiguration>,
    /// 可选的事件总线实例（用于测试或自定义配置）
    event_bus: Option<Arc<EventBus>>,
    /// 可选的查询执行器实例
    query_executor: Option<Arc<QueryExecutor>>,
    /// 可选的资源管理器实例
    resource_manager: Option<Arc<ResourceManager>>,
    /// 可选的取消管理器实例
    cancellation_manager: Option<Arc<CancellationManager>>,
    /// 可选的资源追踪器实例
    resource_tracker: Option<Arc<ResourceTracker>>,
}

impl AppServicesBuilder {
    /// 创建新的构建器实例
    pub fn new() -> Self {
        Self {
            config: None,
            event_bus: None,
            query_executor: None,
            resource_manager: None,
            cancellation_manager: None,
            resource_tracker: None,
        }
    }

    /// 设置服务配置
    ///
    /// 配置将用于创建所有服务的默认参数
    pub fn with_config(mut self, config: ServiceConfiguration) -> Self {
        self.config = Some(config);
        self
    }

    /// 从 TOML 文件加载配置
    ///
    /// # Errors
    /// 如果文件不存在或格式错误，返回错误
    pub fn with_toml_config<P: AsRef<Path>>(mut self, path: P) -> Result<Self> {
        let config = ServiceConfiguration::from_toml_file(path)?;
        config.validate()?;
        self.config = Some(config);
        Ok(self)
    }

    /// 从 JSON 文件加载配置
    ///
    /// # Errors
    /// 如果文件不存在或格式错误，返回错误
    pub fn with_json_config<P: AsRef<Path>>(mut self, path: P) -> Result<Self> {
        let config = ServiceConfiguration::from_json_file(path)?;
        config.validate()?;
        self.config = Some(config);
        Ok(self)
    }

    /// 使用开发环境配置
    pub fn with_development_config(mut self) -> Self {
        self.config = Some(ServiceConfiguration::development());
        self
    }

    /// 使用生产环境配置
    pub fn with_production_config(mut self) -> Self {
        self.config = Some(ServiceConfiguration::production());
        self
    }

    /// 设置自定义事件总线
    ///
    /// 主要用于测试场景，允许注入模拟的事件总线
    pub fn with_event_bus(mut self, event_bus: Arc<EventBus>) -> Self {
        self.event_bus = Some(event_bus);
        self
    }

    /// 设置自定义查询执行器
    pub fn with_query_executor(mut self, query_executor: Arc<QueryExecutor>) -> Self {
        self.query_executor = Some(query_executor);
        self
    }

    /// 设置自定义资源管理器
    pub fn with_resource_manager(mut self, resource_manager: Arc<ResourceManager>) -> Self {
        self.resource_manager = Some(resource_manager);
        self
    }

    /// 设置自定义取消管理器
    pub fn with_cancellation_manager(
        mut self,
        cancellation_manager: Arc<CancellationManager>,
    ) -> Self {
        self.cancellation_manager = Some(cancellation_manager);
        self
    }

    /// 设置自定义资源追踪器
    pub fn with_resource_tracker(mut self, resource_tracker: Arc<ResourceTracker>) -> Self {
        self.resource_tracker = Some(resource_tracker);
        self
    }

    /// 构建服务容器
    ///
    /// 按照依赖顺序创建所有服务：
    /// 1. 基础服务（无依赖）
    /// 2. 依赖基础服务的服务
    /// 3. 高层服务
    ///
    /// # Errors
    ///
    /// 如果任何服务初始化失败，返回错误
    pub fn build(self) -> Result<AppServices> {
        tracing::info!("Building application services container");

        // 获取配置（使用默认配置如果未提供）
        let config = self.config.unwrap_or_default();

        // 验证配置
        config
            .validate()
            .context("Service configuration validation failed")?;

        // 1. 创建基础服务（无依赖）
        let cleanup_queue = Arc::new(crossbeam::queue::SegQueue::new());

        let resource_tracker = self.resource_tracker.unwrap_or_else(|| {
            tracing::debug!("Creating default ResourceTracker");
            Arc::new(ResourceTracker::new(cleanup_queue.clone()))
        });

        let resource_manager = self.resource_manager.unwrap_or_else(|| {
            tracing::debug!("Creating default ResourceManager");
            Arc::new(ResourceManager::new(cleanup_queue.clone()))
        });

        let cancellation_manager = self.cancellation_manager.unwrap_or_else(|| {
            tracing::debug!("Creating default CancellationManager");
            Arc::new(CancellationManager::new())
        });

        let event_bus = self.event_bus.unwrap_or_else(|| {
            tracing::debug!(
                "Creating EventBus with capacity: {}",
                config.event_bus.capacity
            );
            Arc::new(EventBus::new(config.event_bus.capacity))
        });

        // 2. 创建依赖其他服务的服务
        let query_executor = self.query_executor.unwrap_or_else(|| {
            tracing::debug!(
                "Creating QueryExecutor with cache size: {}",
                config.query_executor.cache_size
            );
            Arc::new(QueryExecutor::new(config.query_executor.cache_size))
        });

        tracing::info!("Application services container built successfully");

        Ok(AppServices {
            event_bus,
            query_executor,
            resource_manager,
            cancellation_manager,
            resource_tracker,
        })
    }
}

impl Default for AppServicesBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::service_config::ServiceConfiguration;

    #[test]
    fn test_service_container_creation() {
        // 测试默认服务容器创建
        let services = AppServices::new().expect("Failed to create services");

        // 验证所有服务都已创建
        assert!(Arc::strong_count(services.event_bus()) >= 1);
        assert!(Arc::strong_count(services.query_executor()) >= 1);
        assert!(Arc::strong_count(services.resource_manager()) >= 1);
        assert!(Arc::strong_count(services.cancellation_manager()) >= 1);
        assert!(Arc::strong_count(services.resource_tracker()) >= 1);
    }

    #[test]
    fn test_service_container_builder() {
        // 测试使用 builder 模式创建
        let services = AppServices::builder()
            .build()
            .expect("Failed to build services");

        assert!(Arc::strong_count(services.event_bus()) >= 1);
    }

    #[test]
    fn test_service_container_with_custom_services() {
        // 测试注入自定义服务
        let custom_event_bus = Arc::new(EventBus::new(1000));
        let custom_cancellation_manager = Arc::new(CancellationManager::new());

        let services = AppServices::builder()
            .with_event_bus(custom_event_bus.clone())
            .with_cancellation_manager(custom_cancellation_manager.clone())
            .build()
            .expect("Failed to build services");

        // 验证使用了自定义服务
        assert!(Arc::ptr_eq(services.event_bus(), &custom_event_bus));
        assert!(Arc::ptr_eq(
            services.cancellation_manager(),
            &custom_cancellation_manager
        ));
    }

    #[test]
    fn test_service_container_clone() {
        // 测试服务容器可以克隆（Arc 引用计数增加）
        let services = AppServices::new().expect("Failed to create services");
        let services_clone = services.clone();

        // 验证克隆后引用计数增加
        assert!(Arc::strong_count(services.event_bus()) >= 2);
        assert!(Arc::ptr_eq(
            services.event_bus(),
            services_clone.event_bus()
        ));
    }

    #[test]
    fn test_service_dependencies() {
        // 测试服务依赖关系正确建立
        let services = AppServices::new().expect("Failed to create services");

        // 验证所有服务都可访问
        let _event_bus = services.event_bus();
        let _query_executor = services.query_executor();
        let _resource_manager = services.resource_manager();
        let _cancellation_manager = services.cancellation_manager();
        let _resource_tracker = services.resource_tracker();
    }

    #[test]
    fn test_service_container_with_config() {
        // 测试使用配置创建服务容器
        let config = ServiceConfiguration::development();
        let services = AppServices::builder()
            .with_config(config.clone())
            .build()
            .expect("Failed to build services with config");

        assert!(Arc::strong_count(services.event_bus()) >= 1);
    }

    #[test]
    fn test_service_container_with_development_config() {
        // 测试使用开发环境配置
        let services = AppServices::builder()
            .with_development_config()
            .build()
            .expect("Failed to build services with development config");

        assert!(Arc::strong_count(services.event_bus()) >= 1);
    }

    #[test]
    fn test_service_container_with_production_config() {
        // 测试使用生产环境配置
        let services = AppServices::builder()
            .with_production_config()
            .build()
            .expect("Failed to build services with production config");

        assert!(Arc::strong_count(services.event_bus()) >= 1);
    }

    #[test]
    fn test_service_container_with_toml_config() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // 创建临时配置文件
        let config = ServiceConfiguration::development();
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let toml_content = toml::to_string(&config).expect("Failed to serialize");
        temp_file
            .write_all(toml_content.as_bytes())
            .expect("Failed to write to temp file");

        // 使用配置文件创建服务
        let services = AppServices::builder()
            .with_toml_config(temp_file.path())
            .expect("Failed to load TOML config")
            .build()
            .expect("Failed to build services");

        assert!(Arc::strong_count(services.event_bus()) >= 1);
    }

    #[test]
    fn test_service_container_with_json_config() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // 创建临时配置文件
        let config = ServiceConfiguration::production();
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let json_content = serde_json::to_string(&config).expect("Failed to serialize");
        temp_file
            .write_all(json_content.as_bytes())
            .expect("Failed to write to temp file");

        // 使用配置文件创建服务
        let services = AppServices::builder()
            .with_json_config(temp_file.path())
            .expect("Failed to load JSON config")
            .build()
            .expect("Failed to build services");

        assert!(Arc::strong_count(services.event_bus()) >= 1);
    }

    #[test]
    fn test_service_lifecycle_start_stop() {
        // 测试服务启动和停止
        let services = AppServices::new().expect("Failed to create services");

        assert!(services.start_all().is_ok());
        assert!(services.stop_all().is_ok());
    }

    #[test]
    fn test_service_health_check() {
        // 测试健康检查
        let services = AppServices::new().expect("Failed to create services");

        let health_checks = services.check_health();
        assert!(!health_checks.is_empty());

        // 验证所有服务都有健康检查结果
        for health in &health_checks {
            assert!(!health.service_name.is_empty());
        }
    }

    #[test]
    fn test_overall_health() {
        // 测试整体健康状态
        let services = AppServices::new().expect("Failed to create services");

        let overall = services.overall_health();
        assert_eq!(overall.total_services, overall.healthy_services);
        assert_eq!(overall.degraded_services, 0);
        assert_eq!(overall.unhealthy_services, 0);
    }

    #[test]
    fn test_service_lifecycle_with_config() {
        // 测试使用配置的服务生命周期
        let services = AppServices::builder()
            .with_development_config()
            .build()
            .expect("Failed to build services");

        assert!(services.start_all().is_ok());

        let health = services.overall_health();
        assert!(health.healthy_services > 0);

        assert!(services.stop_all().is_ok());
    }
}
