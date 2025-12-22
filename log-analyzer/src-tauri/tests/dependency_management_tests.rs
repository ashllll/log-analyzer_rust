//! 依赖管理测试
//!
//! 测试服务容器和依赖注入系统的功能，包括：
//! - 服务创建和依赖注入
//! - 配置加载和验证
//! - 服务生命周期和健康检查
//! - 服务交互的集成测试

use eyre::Result;
use log_analyzer::services::{
    AppServices, AppServicesBuilder, AsyncResourceManagerService, CacheConfig, CacheManagerService,
    FileWatcherService, MonitoringConfig, QueryExecutorService, Service, ServiceConfiguration,
    ServiceHealth, SystemMonitoringService, ValidationConfig, WorkspaceConfig,
};
use log_analyzer::utils::{AsyncResourceManager, CacheManager};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tempfile::NamedTempFile;
use tokio::time::sleep;

/// 测试服务实现
struct TestService {
    name: &'static str,
    started: AtomicBool,
    should_fail_start: AtomicBool,
    should_fail_health: AtomicBool,
}

impl TestService {
    fn new(name: &'static str) -> Self {
        Self {
            name,
            started: AtomicBool::new(false),
            should_fail_start: AtomicBool::new(false),
            should_fail_health: AtomicBool::new(false),
        }
    }

    fn set_fail_start(&self, should_fail: bool) {
        self.should_fail_start.store(should_fail, Ordering::SeqCst);
    }

    fn set_fail_health(&self, should_fail: bool) {
        self.should_fail_health.store(should_fail, Ordering::SeqCst);
    }

    fn is_started(&self) -> bool {
        self.started.load(Ordering::SeqCst)
    }
}

impl Service for TestService {
    fn start(&self) -> Result<()> {
        if self.should_fail_start.load(Ordering::SeqCst) {
            return Err(eyre::eyre!("Simulated start failure"));
        }

        self.started.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn stop(&self) -> Result<()> {
        self.started.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn health_check(&self) -> Result<ServiceHealth> {
        if self.should_fail_health.load(Ordering::SeqCst) {
            return Ok(ServiceHealth::unhealthy(
                "Simulated health failure".to_string(),
            ));
        }

        Ok(if self.started.load(Ordering::SeqCst) {
            ServiceHealth::healthy().with_detail("status".to_string(), "running".to_string())
        } else {
            ServiceHealth::unhealthy("Service not started".to_string())
        })
    }

    fn service_name(&self) -> &'static str {
        self.name
    }
}

#[test]
fn test_service_configuration_defaults() {
    let config = ServiceConfiguration::default();

    // 验证默认值
    assert_eq!(config.cache.max_capacity, 1000);
    assert_eq!(config.cache.ttl_seconds, 300);
    assert_eq!(config.cache.tti_seconds, 60);
    assert!(config.cache.enable_metrics);

    assert_eq!(config.validation.max_query_length, 1000);
    assert_eq!(config.validation.max_workspace_id_length, 50);
    assert!(config.validation.enable_path_traversal_check);

    assert_eq!(config.workspace.max_workspaces, 100);
    assert!(config.workspace.enable_auto_cleanup);

    assert!(config.monitoring.enable_metrics);
    assert_eq!(config.monitoring.metrics_interval_seconds, 60);
}

#[test]
fn test_service_configuration_development() {
    let config = ServiceConfiguration::development();

    // 验证开发环境配置
    assert_eq!(config.cache.max_capacity, 100);
    assert_eq!(config.workspace.max_workspaces, 10);
    assert!(!config.workspace.enable_auto_cleanup); // 开发环境不自动清理
    assert_eq!(config.monitoring.metrics_interval_seconds, 10); // 更频繁的监控
}

#[test]
fn test_service_configuration_production() {
    let config = ServiceConfiguration::production();

    // 验证生产环境配置
    assert_eq!(config.cache.max_capacity, 10000);
    assert_eq!(config.workspace.max_workspaces, 1000);
    assert!(config.workspace.enable_auto_cleanup);
    assert_eq!(config.monitoring.metrics_interval_seconds, 300); // 较少的监控频率
}

#[test]
fn test_service_configuration_validation() {
    let mut config = ServiceConfiguration::default();

    // 有效配置应该通过验证
    assert!(config.validate().is_ok());

    // 测试无效的缓存配置
    config.cache.max_capacity = 0;
    assert!(config.validate().is_err());

    config.cache.max_capacity = 1000;
    config.cache.ttl_seconds = 0;
    assert!(config.validate().is_err());

    // 测试无效的验证配置
    config.cache.ttl_seconds = 300;
    config.validation.max_query_length = 0;
    assert!(config.validate().is_err());

    // 测试无效的工作区配置
    config.validation.max_query_length = 1000;
    config.workspace.max_workspaces = 0;
    assert!(config.validate().is_err());

    // 测试无效的监控配置
    config.workspace.max_workspaces = 100;
    config.monitoring.metrics_interval_seconds = 0;
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
    assert_eq!(
        config.workspace.max_workspaces,
        loaded_config.workspace.max_workspaces
    );
    assert_eq!(
        config.monitoring.enable_metrics,
        loaded_config.monitoring.enable_metrics
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
    assert_eq!(
        config.monitoring.metrics_interval_seconds,
        loaded_config.monitoring.metrics_interval_seconds
    );

    Ok(())
}

#[test]
fn test_app_services_builder_default() -> Result<()> {
    let services = AppServices::builder().build()?;

    // 验证默认配置
    assert_eq!(services.configuration().cache.max_capacity, 1000);
    assert!(services.service_count() > 0); // 应该有核心服务

    Ok(())
}

#[test]
fn test_app_services_builder_development_mode() -> Result<()> {
    let services = AppServices::builder().development_mode().build()?;

    // 验证开发模式配置
    assert_eq!(services.configuration().cache.max_capacity, 100);
    assert_eq!(services.configuration().workspace.max_workspaces, 10);
    assert!(!services.configuration().workspace.enable_auto_cleanup);

    Ok(())
}

#[test]
fn test_app_services_builder_production_mode() -> Result<()> {
    let services = AppServices::builder().production_mode().build()?;

    // 验证生产模式配置
    assert_eq!(services.configuration().cache.max_capacity, 10000);
    assert_eq!(services.configuration().workspace.max_workspaces, 1000);
    assert!(services.configuration().workspace.enable_auto_cleanup);

    Ok(())
}

#[test]
fn test_app_services_builder_custom_config() -> Result<()> {
    let custom_cache_config = CacheConfig {
        max_capacity: 500,
        ttl_seconds: 120,
        tti_seconds: 30,
        enable_metrics: false,
    };

    let services = AppServices::builder()
        .with_cache_config(custom_cache_config.clone())
        .build()?;

    // 验证自定义配置
    assert_eq!(services.configuration().cache.max_capacity, 500);
    assert_eq!(services.configuration().cache.ttl_seconds, 120);
    assert_eq!(services.configuration().cache.tti_seconds, 30);
    assert!(!services.configuration().cache.enable_metrics);

    Ok(())
}

#[test]
fn test_app_services_with_custom_service() -> Result<()> {
    let test_service = Arc::new(TestService::new("custom_test_service"));

    let services = AppServices::builder()
        .add_service(test_service.clone())
        .build()?;

    // 验证自定义服务被添加
    let initial_count = services.service_count();
    assert!(initial_count > 0);

    Ok(())
}

#[test]
fn test_service_lifecycle_management() -> Result<()> {
    let test_service1 = Arc::new(TestService::new("test_service_1"));
    let test_service2 = Arc::new(TestService::new("test_service_2"));

    let services = AppServices::builder()
        .add_service(test_service1.clone())
        .add_service(test_service2.clone())
        .build()?;

    // 初始状态 - 服务未启动
    assert!(!test_service1.is_started());
    assert!(!test_service2.is_started());

    // 启动所有服务
    services.start_all()?;
    assert!(test_service1.is_started());
    assert!(test_service2.is_started());

    // 停止所有服务
    services.stop_all()?;
    assert!(!test_service1.is_started());
    assert!(!test_service2.is_started());

    Ok(())
}

#[test]
fn test_service_lifecycle_failure_handling() -> Result<()> {
    let test_service1 = Arc::new(TestService::new("test_service_1"));
    let test_service2 = Arc::new(TestService::new("test_service_2"));

    // 设置第二个服务启动失败
    test_service2.set_fail_start(true);

    let services = AppServices::builder()
        .add_service(test_service1.clone())
        .add_service(test_service2.clone())
        .build()?;

    // 启动应该失败
    let result = services.start_all();
    assert!(result.is_err());

    // 第一个服务可能已经启动，但第二个服务应该失败
    assert!(!test_service2.is_started());

    Ok(())
}

#[test]
fn test_service_health_checks() -> Result<()> {
    let test_service1 = Arc::new(TestService::new("healthy_service"));
    let test_service2 = Arc::new(TestService::new("unhealthy_service"));

    // 设置第二个服务健康检查失败
    test_service2.set_fail_health(true);

    let services = AppServices::builder()
        .add_service(test_service1.clone())
        .add_service(test_service2.clone())
        .build()?;

    // 启动服务
    services.start_all()?;

    // 检查健康状态
    let health_status = services.health_check_all();

    // 验证健康状态
    assert!(health_status.contains_key("healthy_service"));
    assert!(health_status.contains_key("unhealthy_service"));

    let healthy_status = &health_status["healthy_service"];
    let unhealthy_status = &health_status["unhealthy_service"];

    assert!(healthy_status.is_healthy);
    assert!(!unhealthy_status.is_healthy);

    Ok(())
}

#[test]
fn test_service_configuration_builder_chain() -> Result<()> {
    let cache_config = CacheConfig {
        max_capacity: 200,
        ttl_seconds: 180,
        tti_seconds: 45,
        enable_metrics: true,
    };

    let validation_config = ValidationConfig {
        max_query_length: 2000,
        max_workspace_id_length: 100,
        allowed_file_extensions: vec!["log".to_string(), "txt".to_string()],
        enable_path_traversal_check: true,
    };

    let workspace_config = WorkspaceConfig {
        max_workspaces: 50,
        default_index_size_limit: 500 * 1024 * 1024, // 500MB
        enable_auto_cleanup: false,
        cleanup_interval_hours: 12,
    };

    let monitoring_config = MonitoringConfig {
        enable_metrics: true,
        metrics_interval_seconds: 120,
        enable_health_checks: true,
        health_check_interval_seconds: 30,
    };

    let services = AppServices::builder()
        .with_cache_config(cache_config.clone())
        .with_validation_config(validation_config.clone())
        .with_workspace_config(workspace_config.clone())
        .with_monitoring_config(monitoring_config.clone())
        .build()?;

    // 验证所有配置都被正确设置
    let config = services.configuration();
    assert_eq!(config.cache.max_capacity, 200);
    assert_eq!(config.validation.max_query_length, 2000);
    assert_eq!(config.workspace.max_workspaces, 50);
    assert_eq!(config.monitoring.metrics_interval_seconds, 120);

    Ok(())
}

#[tokio::test]
async fn test_service_integration_with_real_services() -> Result<()> {
    // 创建真实的服务实例
    let search_cache = Arc::new(
        moka::sync::Cache::builder()
            .max_capacity(100)
            .time_to_live(Duration::from_secs(60))
            .build(),
    );

    let cache_manager = Arc::new(CacheManager::new(search_cache));
    let async_resource_manager = Arc::new(AsyncResourceManager::new());

    // 创建服务实例
    let query_service = Arc::new(QueryExecutorService::new(100));
    let cache_service = Arc::new(CacheManagerService::new(cache_manager.clone()));
    let async_service = Arc::new(AsyncResourceManagerService::new(
        async_resource_manager.clone(),
    ));
    let file_service = Arc::new(FileWatcherService::new());
    let monitor_service = Arc::new(SystemMonitoringService::new());

    let services = AppServices::builder()
        .add_service(query_service.clone())
        .add_service(cache_service.clone())
        .add_service(async_service.clone())
        .add_service(file_service.clone())
        .add_service(monitor_service.clone())
        .build()?;

    // 测试服务生命周期
    services.start_all()?;

    // 等待一小段时间让异步服务初始化
    sleep(Duration::from_millis(100)).await;

    // 检查所有服务的健康状态
    let health_status = services.health_check_all();

    // 验证所有服务都健康
    for (service_name, health) in &health_status {
        println!(
            "Service: {}, Healthy: {}, Details: {:?}",
            service_name, health.is_healthy, health.details
        );
        assert!(
            health.is_healthy,
            "Service {} should be healthy",
            service_name
        );
    }

    // 停止所有服务
    services.stop_all()?;

    // 等待一小段时间让服务停止
    sleep(Duration::from_millis(100)).await;

    Ok(())
}

#[test]
fn test_service_configuration_validation_edge_cases() {
    let mut config = ServiceConfiguration::default();

    // 测试边界值
    config.cache.max_capacity = 1;
    config.cache.ttl_seconds = 1;
    config.cache.tti_seconds = 1;
    config.validation.max_query_length = 1;
    config.validation.max_workspace_id_length = 1;
    config.workspace.max_workspaces = 1;
    config.monitoring.metrics_interval_seconds = 1;
    config.monitoring.health_check_interval_seconds = 1;

    // 边界值应该通过验证
    assert!(config.validate().is_ok());

    // 测试空的文件扩展名列表
    config.validation.allowed_file_extensions = vec![];
    assert!(config.validate().is_ok()); // 空列表应该是允许的
}

#[test]
fn test_service_health_detail_management() {
    let health = ServiceHealth::healthy()
        .with_detail("version".to_string(), "1.0.0".to_string())
        .with_detail("uptime".to_string(), "3600".to_string())
        .with_detail("memory_usage".to_string(), "50MB".to_string());

    assert!(health.is_healthy);
    assert_eq!(health.details.len(), 3);
    assert_eq!(health.details.get("version"), Some(&"1.0.0".to_string()));
    assert_eq!(health.details.get("uptime"), Some(&"3600".to_string()));
    assert_eq!(
        health.details.get("memory_usage"),
        Some(&"50MB".to_string())
    );

    let unhealthy = ServiceHealth::unhealthy("Database connection failed".to_string())
        .with_detail(
            "last_attempt".to_string(),
            "2023-01-01T00:00:00Z".to_string(),
        )
        .with_detail("retry_count".to_string(), "3".to_string());

    assert!(!unhealthy.is_healthy);
    assert_eq!(
        unhealthy.details.get("error"),
        Some(&"Database connection failed".to_string())
    );
    assert_eq!(
        unhealthy.details.get("last_attempt"),
        Some(&"2023-01-01T00:00:00Z".to_string())
    );
    assert_eq!(unhealthy.details.get("retry_count"), Some(&"3".to_string()));
}

#[test]
fn test_service_count_and_configuration_access() -> Result<()> {
    let test_service = Arc::new(TestService::new("test_service"));

    let services = AppServices::builder()
        .development_mode()
        .add_service(test_service)
        .build()?;

    // 测试服务数量（核心服务 + 自定义服务）
    let service_count = services.service_count();
    assert!(service_count > 1); // 至少有核心服务和我们添加的测试服务

    // 测试配置访问
    let config = services.configuration();
    assert_eq!(config.cache.max_capacity, 100); // 开发模式配置

    Ok(())
}

#[test]
fn test_app_services_direct_creation_methods() -> Result<()> {
    // 测试直接创建（默认配置）
    let services1 = AppServices::new()?;
    assert_eq!(services1.configuration().cache.max_capacity, 1000);

    // 测试使用指定配置创建
    let config = ServiceConfiguration::production();
    let services2 = AppServices::with_config(config)?;
    assert_eq!(services2.configuration().cache.max_capacity, 10000);

    Ok(())
}

#[tokio::test]
async fn test_service_graceful_shutdown_scenario() -> Result<()> {
    let test_service1 = Arc::new(TestService::new("service_1"));
    let test_service2 = Arc::new(TestService::new("service_2"));

    let services = AppServices::builder()
        .add_service(test_service1.clone())
        .add_service(test_service2.clone())
        .build()?;

    // 启动服务
    services.start_all()?;
    assert!(test_service1.is_started());
    assert!(test_service2.is_started());

    // 模拟应用程序关闭
    services.stop_all()?;

    // 验证所有服务都已停止
    assert!(!test_service1.is_started());
    assert!(!test_service2.is_started());

    Ok(())
}
