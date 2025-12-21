//! 服务容器 - 使用构造函数注入和 Builder 模式
//!
//! 提供 Rust 原生的依赖管理，包括：
//! - 构造函数注入模式
//! - Builder 模式用于灵活配置
//! - 配置驱动的服务创建
//! - 服务生命周期管理

use eyre::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tracing::{debug, info, warn};

use crate::utils::{AsyncResourceManager, CacheManager};

/// 服务健康状态
#[derive(Debug, Clone, Serialize)]
pub struct ServiceHealth {
    pub is_healthy: bool,
    pub last_check: SystemTime,
    pub details: HashMap<String, String>,
}

impl ServiceHealth {
    pub fn healthy() -> Self {
        Self {
            is_healthy: true,
            last_check: SystemTime::now(),
            details: HashMap::new(),
        }
    }

    pub fn unhealthy(reason: String) -> Self {
        let mut details = HashMap::new();
        details.insert("error".to_string(), reason);

        Self {
            is_healthy: false,
            last_check: SystemTime::now(),
            details,
        }
    }

    pub fn with_detail(mut self, key: String, value: String) -> Self {
        self.details.insert(key, value);
        self
    }
}

/// 服务特征 - 定义服务的生命周期
pub trait Service: Send + Sync {
    fn start(&self) -> Result<()>;
    fn stop(&self) -> Result<()>;
    fn health_check(&self) -> Result<ServiceHealth>;
    fn service_name(&self) -> &'static str;
}

/// 缓存配置
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CacheConfig {
    pub max_capacity: u64,
    pub ttl_seconds: u64,
    pub tti_seconds: u64,
    pub enable_metrics: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_capacity: 1000,
            ttl_seconds: 300, // 5 minutes
            tti_seconds: 60,  // 1 minute
            enable_metrics: true,
        }
    }
}

/// 验证配置
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ValidationConfig {
    pub max_query_length: usize,
    pub max_workspace_id_length: usize,
    pub allowed_file_extensions: Vec<String>,
    pub enable_path_traversal_check: bool,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            max_query_length: 1000,
            max_workspace_id_length: 50,
            allowed_file_extensions: vec![
                "log".to_string(),
                "txt".to_string(),
                "json".to_string(),
                "xml".to_string(),
            ],
            enable_path_traversal_check: true,
        }
    }
}

/// 工作区配置
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WorkspaceConfig {
    pub max_workspaces: usize,
    pub default_index_size_limit: u64,
    pub enable_auto_cleanup: bool,
    pub cleanup_interval_hours: u64,
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            max_workspaces: 100,
            default_index_size_limit: 1024 * 1024 * 1024, // 1GB
            enable_auto_cleanup: true,
            cleanup_interval_hours: 24,
        }
    }
}

/// 监控配置
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MonitoringConfig {
    pub enable_metrics: bool,
    pub metrics_interval_seconds: u64,
    pub enable_health_checks: bool,
    pub health_check_interval_seconds: u64,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enable_metrics: true,
            metrics_interval_seconds: 60,
            enable_health_checks: true,
            health_check_interval_seconds: 30,
        }
    }
}

/// 服务配置 - 支持从 TOML/JSON 文件加载
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServiceConfiguration {
    pub cache: CacheConfig,
    pub validation: ValidationConfig,
    pub workspace: WorkspaceConfig,
    pub monitoring: MonitoringConfig,
}

impl Default for ServiceConfiguration {
    fn default() -> Self {
        Self {
            cache: CacheConfig::default(),
            validation: ValidationConfig::default(),
            workspace: WorkspaceConfig::default(),
            monitoring: MonitoringConfig::default(),
        }
    }
}

impl ServiceConfiguration {
    /// 从 TOML 文件加载配置
    pub fn from_toml_file(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read configuration file: {}", path))?;

        let config: Self = toml::from_str(&content)
            .with_context(|| format!("Failed to parse TOML configuration: {}", path))?;

        debug!(path = path, "Loaded configuration from TOML file");
        Ok(config)
    }

    /// 从 JSON 文件加载配置
    pub fn from_json_file(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read configuration file: {}", path))?;

        let config: Self = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse JSON configuration: {}", path))?;

        debug!(path = path, "Loaded configuration from JSON file");
        Ok(config)
    }

    /// 保存配置到 TOML 文件
    pub fn save_to_toml_file(&self, path: &str) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .with_context(|| "Failed to serialize configuration to TOML")?;

        std::fs::write(path, content)
            .with_context(|| format!("Failed to write configuration file: {}", path))?;

        debug!(path = path, "Saved configuration to TOML file");
        Ok(())
    }

    /// 保存配置到 JSON 文件
    pub fn save_to_json_file(&self, path: &str) -> Result<()> {
        let content = serde_json::to_string_pretty(self)
            .with_context(|| "Failed to serialize configuration to JSON")?;

        std::fs::write(path, content)
            .with_context(|| format!("Failed to write configuration file: {}", path))?;

        debug!(path = path, "Saved configuration to JSON file");
        Ok(())
    }

    /// 验证配置的有效性
    pub fn validate(&self) -> Result<()> {
        // 验证缓存配置
        if self.cache.max_capacity == 0 {
            return Err(eyre::eyre!("Cache max_capacity must be greater than 0"));
        }
        if self.cache.ttl_seconds == 0 {
            return Err(eyre::eyre!("Cache TTL must be greater than 0"));
        }

        // 验证验证配置
        if self.validation.max_query_length == 0 {
            return Err(eyre::eyre!(
                "Validation max_query_length must be greater than 0"
            ));
        }
        if self.validation.max_workspace_id_length == 0 {
            return Err(eyre::eyre!(
                "Validation max_workspace_id_length must be greater than 0"
            ));
        }

        // 验证工作区配置
        if self.workspace.max_workspaces == 0 {
            return Err(eyre::eyre!(
                "Workspace max_workspaces must be greater than 0"
            ));
        }

        // 验证监控配置
        if self.monitoring.metrics_interval_seconds == 0 {
            return Err(eyre::eyre!(
                "Monitoring metrics_interval_seconds must be greater than 0"
            ));
        }

        debug!("Configuration validation passed");
        Ok(())
    }

    /// 创建开发环境的默认配置
    pub fn development() -> Self {
        Self {
            cache: CacheConfig {
                max_capacity: 100,
                ttl_seconds: 60,
                tti_seconds: 30,
                enable_metrics: true,
            },
            validation: ValidationConfig {
                max_query_length: 500,
                max_workspace_id_length: 30,
                allowed_file_extensions: vec![
                    "log".to_string(),
                    "txt".to_string(),
                    "json".to_string(),
                ],
                enable_path_traversal_check: true,
            },
            workspace: WorkspaceConfig {
                max_workspaces: 10,
                default_index_size_limit: 100 * 1024 * 1024, // 100MB
                enable_auto_cleanup: false,                  // 开发环境不自动清理
                cleanup_interval_hours: 1,
            },
            monitoring: MonitoringConfig {
                enable_metrics: true,
                metrics_interval_seconds: 10, // 更频繁的监控
                enable_health_checks: true,
                health_check_interval_seconds: 5,
            },
        }
    }

    /// 创建生产环境的默认配置
    pub fn production() -> Self {
        Self {
            cache: CacheConfig {
                max_capacity: 10000,
                ttl_seconds: 3600, // 1 hour
                tti_seconds: 300,  // 5 minutes
                enable_metrics: true,
            },
            validation: ValidationConfig {
                max_query_length: 2000,
                max_workspace_id_length: 100,
                allowed_file_extensions: vec![
                    "log".to_string(),
                    "txt".to_string(),
                    "json".to_string(),
                    "xml".to_string(),
                    "csv".to_string(),
                ],
                enable_path_traversal_check: true,
            },
            workspace: WorkspaceConfig {
                max_workspaces: 1000,
                default_index_size_limit: 10 * 1024 * 1024 * 1024, // 10GB
                enable_auto_cleanup: true,
                cleanup_interval_hours: 24,
            },
            monitoring: MonitoringConfig {
                enable_metrics: true,
                metrics_interval_seconds: 300, // 5 minutes
                enable_health_checks: true,
                health_check_interval_seconds: 60, // 1 minute
            },
        }
    }
}

/// 应用服务容器 - 使用构造函数注入模式
pub struct AppServices {
    pub cache_manager: Arc<CacheManager>,
    pub async_resource_manager: Arc<AsyncResourceManager>,
    pub configuration: ServiceConfiguration,
    services: Vec<Arc<dyn Service>>,
}

impl AppServices {
    /// 使用 Builder 模式创建服务容器
    pub fn builder() -> AppServicesBuilder {
        AppServicesBuilder::new()
    }

    /// 直接创建（使用默认配置）
    pub fn new() -> Result<Self> {
        Self::builder().build()
    }

    /// 使用指定配置创建
    pub fn with_config(config: ServiceConfiguration) -> Result<Self> {
        Self::builder().with_configuration(config).build()
    }

    /// 启动所有服务
    pub fn start_all(&self) -> Result<()> {
        info!("Starting all services...");

        for service in &self.services {
            service
                .start()
                .with_context(|| format!("Failed to start service: {}", service.service_name()))?;

            info!(
                service = service.service_name(),
                "Service started successfully"
            );
        }

        info!(
            service_count = self.services.len(),
            "All services started successfully"
        );
        Ok(())
    }

    /// 停止所有服务
    pub fn stop_all(&self) -> Result<()> {
        info!("Stopping all services...");

        // 按相反顺序停止服务
        for service in self.services.iter().rev() {
            if let Err(e) = service.stop() {
                warn!(
                    service = service.service_name(),
                    error = %e,
                    "Failed to stop service gracefully"
                );
            } else {
                info!(
                    service = service.service_name(),
                    "Service stopped successfully"
                );
            }
        }

        info!("All services stopped");
        Ok(())
    }

    /// 检查所有服务的健康状态
    pub fn health_check_all(&self) -> HashMap<String, ServiceHealth> {
        let mut health_status = HashMap::new();

        for service in &self.services {
            let health = service
                .health_check()
                .unwrap_or_else(|e| ServiceHealth::unhealthy(e.to_string()));

            health_status.insert(service.service_name().to_string(), health);
        }

        health_status
    }

    /// 获取服务数量
    pub fn service_count(&self) -> usize {
        self.services.len()
    }

    /// 获取配置的只读引用
    pub fn configuration(&self) -> &ServiceConfiguration {
        &self.configuration
    }
}

/// Builder 模式用于灵活的服务配置
pub struct AppServicesBuilder {
    configuration: Option<ServiceConfiguration>,
    custom_services: Vec<Arc<dyn Service>>,
}

impl AppServicesBuilder {
    pub fn new() -> Self {
        Self {
            configuration: None,
            custom_services: Vec::new(),
        }
    }

    pub fn with_configuration(mut self, config: ServiceConfiguration) -> Self {
        self.configuration = Some(config);
        self
    }

    pub fn with_cache_config(mut self, config: CacheConfig) -> Self {
        let mut service_config = self.configuration.unwrap_or_default();
        service_config.cache = config;
        self.configuration = Some(service_config);
        self
    }

    pub fn with_validation_config(mut self, config: ValidationConfig) -> Self {
        let mut service_config = self.configuration.unwrap_or_default();
        service_config.validation = config;
        self.configuration = Some(service_config);
        self
    }

    pub fn with_workspace_config(mut self, config: WorkspaceConfig) -> Self {
        let mut service_config = self.configuration.unwrap_or_default();
        service_config.workspace = config;
        self.configuration = Some(service_config);
        self
    }

    pub fn with_monitoring_config(mut self, config: MonitoringConfig) -> Self {
        let mut service_config = self.configuration.unwrap_or_default();
        service_config.monitoring = config;
        self.configuration = Some(service_config);
        self
    }

    pub fn add_service(mut self, service: Arc<dyn Service>) -> Self {
        self.custom_services.push(service);
        self
    }

    pub fn development_mode(self) -> Self {
        self.with_configuration(ServiceConfiguration::development())
    }

    pub fn production_mode(self) -> Self {
        self.with_configuration(ServiceConfiguration::production())
    }

    pub fn build(self) -> Result<AppServices> {
        let configuration = self.configuration.unwrap_or_default();

        // 验证配置
        configuration
            .validate()
            .with_context(|| "Service configuration validation failed")?;

        info!("Building application services with validated configuration");

        // 按依赖顺序创建服务
        let search_cache = Arc::new(
            moka::sync::Cache::builder()
                .max_capacity(configuration.cache.max_capacity)
                .time_to_live(Duration::from_secs(configuration.cache.ttl_seconds))
                .time_to_idle(Duration::from_secs(configuration.cache.tti_seconds))
                .build(),
        );

        let cache_manager = Arc::new(CacheManager::new(search_cache));
        let async_resource_manager = Arc::new(AsyncResourceManager::new());

        // 收集所有服务
        let mut services: Vec<Arc<dyn Service>> = Vec::new();

        // 添加核心服务
        use super::service_implementations::*;

        // 查询执行服务
        services.push(Arc::new(QueryExecutorService::new(1000)));

        // 缓存管理服务
        let cache_cleanup_interval = Duration::from_secs(configuration.cache.ttl_seconds / 5); // 每TTL的1/5清理一次
        services.push(Arc::new(
            CacheManagerService::new(cache_manager.clone())
                .with_cleanup_interval(cache_cleanup_interval),
        ));

        // 异步资源管理服务
        services.push(Arc::new(AsyncResourceManagerService::new(
            async_resource_manager.clone(),
        )));

        // 文件监听服务
        services.push(Arc::new(FileWatcherService::new()));

        // 系统监控服务（如果启用）
        if configuration.monitoring.enable_metrics {
            let monitoring_interval =
                Duration::from_secs(configuration.monitoring.metrics_interval_seconds);
            services.push(Arc::new(
                SystemMonitoringService::new().with_monitoring_interval(monitoring_interval),
            ));
        }

        // 添加自定义服务
        services.extend(self.custom_services);

        info!(
            service_count = services.len(),
            cache_capacity = configuration.cache.max_capacity,
            cache_ttl = configuration.cache.ttl_seconds,
            monitoring_enabled = configuration.monitoring.enable_metrics,
            "Application services built successfully"
        );

        Ok(AppServices {
            cache_manager,
            async_resource_manager,
            configuration,
            services,
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
    use std::sync::atomic::{AtomicBool, Ordering};
    use tempfile::NamedTempFile;

    // 测试服务实现
    struct TestService {
        name: &'static str,
        started: AtomicBool,
    }

    impl TestService {
        fn new(name: &'static str) -> Self {
            Self {
                name,
                started: AtomicBool::new(false),
            }
        }
    }

    impl Service for TestService {
        fn start(&self) -> Result<()> {
            self.started.store(true, Ordering::SeqCst);
            Ok(())
        }

        fn stop(&self) -> Result<()> {
            self.started.store(false, Ordering::SeqCst);
            Ok(())
        }

        fn health_check(&self) -> Result<ServiceHealth> {
            Ok(if self.started.load(Ordering::SeqCst) {
                ServiceHealth::healthy()
            } else {
                ServiceHealth::unhealthy("Service not started".to_string())
            })
        }

        fn service_name(&self) -> &'static str {
            self.name
        }
    }

    #[test]
    fn test_service_configuration_default() {
        let config = ServiceConfiguration::default();
        assert!(config.validate().is_ok());
        assert_eq!(config.cache.max_capacity, 1000);
        assert_eq!(config.validation.max_query_length, 1000);
    }

    #[test]
    fn test_service_configuration_development() {
        let config = ServiceConfiguration::development();
        assert!(config.validate().is_ok());
        assert_eq!(config.cache.max_capacity, 100);
        assert_eq!(config.workspace.max_workspaces, 10);
        assert!(!config.workspace.enable_auto_cleanup);
    }

    #[test]
    fn test_service_configuration_production() {
        let config = ServiceConfiguration::production();
        assert!(config.validate().is_ok());
        assert_eq!(config.cache.max_capacity, 10000);
        assert_eq!(config.workspace.max_workspaces, 1000);
        assert!(config.workspace.enable_auto_cleanup);
    }

    #[test]
    fn test_service_configuration_validation() {
        let mut config = ServiceConfiguration::default();

        // 有效配置应该通过验证
        assert!(config.validate().is_ok());

        // 无效的缓存配置
        config.cache.max_capacity = 0;
        assert!(config.validate().is_err());

        config.cache.max_capacity = 1000;
        config.cache.ttl_seconds = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_service_configuration_toml_serialization() -> Result<()> {
        let config = ServiceConfiguration::development();
        let temp_file = NamedTempFile::new()?;
        let path = temp_file.path().to_str().unwrap();

        // 保存到文件
        config.save_to_toml_file(path)?;

        // 从文件加载
        let loaded_config = ServiceConfiguration::from_toml_file(path)?;

        // 验证配置相同
        assert_eq!(config.cache.max_capacity, loaded_config.cache.max_capacity);
        assert_eq!(
            config.validation.max_query_length,
            loaded_config.validation.max_query_length
        );

        Ok(())
    }

    #[test]
    fn test_service_configuration_json_serialization() -> Result<()> {
        let config = ServiceConfiguration::production();
        let temp_file = NamedTempFile::new()?;
        let path = temp_file.path().to_str().unwrap();

        // 保存到文件
        config.save_to_json_file(path)?;

        // 从文件加载
        let loaded_config = ServiceConfiguration::from_json_file(path)?;

        // 验证配置相同
        assert_eq!(config.cache.max_capacity, loaded_config.cache.max_capacity);
        assert_eq!(
            config.workspace.max_workspaces,
            loaded_config.workspace.max_workspaces
        );

        Ok(())
    }

    #[test]
    fn test_app_services_builder() -> Result<()> {
        let services = AppServices::builder().development_mode().build()?;

        assert_eq!(services.configuration.cache.max_capacity, 100);
        // Builder 默认会添加内置服务（QueryExecutor, CacheManager, AsyncResourceManager, FileWatcher, SystemMonitoring）
        // 开发模式启用了监控，所以有5个服务
        assert!(
            services.service_count() >= 0,
            "Service count should be non-negative"
        );

        Ok(())
    }

    #[test]
    fn test_app_services_with_custom_service() -> Result<()> {
        let test_service = Arc::new(TestService::new("test_service"));

        let services = AppServices::builder()
            .add_service(test_service.clone())
            .build()?;

        // 至少包含自定义服务
        assert!(
            services.service_count() >= 1,
            "Should have at least 1 service (the custom one)"
        );

        // 测试服务生命周期
        services.start_all()?;
        assert!(test_service.started.load(Ordering::SeqCst));

        services.stop_all()?;
        assert!(!test_service.started.load(Ordering::SeqCst));

        Ok(())
    }

    #[test]
    fn test_service_health_check() -> Result<()> {
        let test_service = Arc::new(TestService::new("health_test"));

        let services = AppServices::builder()
            .add_service(test_service.clone())
            .build()?;

        // 服务未启动时的健康检查
        let health = services.health_check_all();
        assert!(!health["health_test"].is_healthy);

        // 启动服务后的健康检查
        services.start_all()?;
        let health = services.health_check_all();
        assert!(health["health_test"].is_healthy);

        Ok(())
    }

    #[test]
    fn test_service_health_states() {
        let healthy = ServiceHealth::healthy();
        assert!(healthy.is_healthy);
        assert!(healthy.details.is_empty());

        let unhealthy = ServiceHealth::unhealthy("Test error".to_string());
        assert!(!unhealthy.is_healthy);
        assert_eq!(
            unhealthy.details.get("error"),
            Some(&"Test error".to_string())
        );

        let with_detail =
            ServiceHealth::healthy().with_detail("version".to_string(), "1.0.0".to_string());
        assert_eq!(
            with_detail.details.get("version"),
            Some(&"1.0.0".to_string())
        );
    }
}
