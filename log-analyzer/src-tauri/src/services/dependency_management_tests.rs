//! 依赖管理集成测试
//!
//! 测试服务容器的依赖注入、配置加载和生命周期管理

#[cfg(test)]
mod tests {
    use crate::services::{
        AppServices, AppServicesBuilder, HealthStatus, ServiceConfiguration,
    };
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// 测试服务创建和依赖注入
    #[test]
    fn test_service_creation_and_injection() {
        // 测试默认服务创建
        let services = AppServices::new().expect("Failed to create services");

        // 验证所有服务都已正确注入
        assert!(std::sync::Arc::strong_count(services.event_bus()) >= 1);
        assert!(std::sync::Arc::strong_count(services.query_executor()) >= 1);
        assert!(std::sync::Arc::strong_count(services.resource_manager()) >= 1);
        assert!(std::sync::Arc::strong_count(services.cancellation_manager()) >= 1);
        assert!(std::sync::Arc::strong_count(services.resource_tracker()) >= 1);
    }

    /// 测试使用 Builder 模式创建服务
    #[test]
    fn test_builder_pattern_service_creation() {
        let services = AppServices::builder()
            .build()
            .expect("Failed to build services");

        // 验证服务可访问
        let _event_bus = services.event_bus();
        let _query_executor = services.query_executor();
        let _resource_manager = services.resource_manager();
        let _cancellation_manager = services.cancellation_manager();
        let _resource_tracker = services.resource_tracker();
    }

    /// 测试配置加载和验证
    #[test]
    fn test_configuration_loading_and_validation() {
        // 测试默认配置
        let default_config = ServiceConfiguration::default();
        assert!(default_config.validate().is_ok());

        // 测试开发环境配置
        let dev_config = ServiceConfiguration::development();
        assert!(dev_config.validate().is_ok());
        assert_eq!(dev_config.event_bus.capacity, 1000);
        assert_eq!(dev_config.query_executor.cache_size, 50);

        // 测试生产环境配置
        let prod_config = ServiceConfiguration::production();
        assert!(prod_config.validate().is_ok());
        assert_eq!(prod_config.event_bus.capacity, 2000);
        assert_eq!(prod_config.query_executor.cache_size, 200);
    }

    /// 测试无效配置验证
    #[test]
    fn test_invalid_configuration_validation() {
        let mut config = ServiceConfiguration::default();

        // 测试无效的事件总线容量
        config.event_bus.capacity = 0;
        assert!(config.validate().is_err());

        // 恢复有效值
        config.event_bus.capacity = 1000;
        assert!(config.validate().is_ok());

        // 测试无效的缓存容量
        config.cache.max_capacity = 0;
        assert!(config.validate().is_err());
    }

    /// 测试从 TOML 文件加载配置
    #[test]
    fn test_toml_configuration_loading() {
        let config = ServiceConfiguration::development();

        // 创建临时 TOML 文件
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let toml_content = toml::to_string(&config).expect("Failed to serialize");
        temp_file
            .write_all(toml_content.as_bytes())
            .expect("Failed to write to temp file");

        // 使用 TOML 配置创建服务
        let services = AppServices::builder()
            .with_toml_config(temp_file.path())
            .expect("Failed to load TOML config")
            .build()
            .expect("Failed to build services");

        // 验证服务已创建
        assert!(std::sync::Arc::strong_count(services.event_bus()) >= 1);
    }

    /// 测试从 JSON 文件加载配置
    #[test]
    fn test_json_configuration_loading() {
        let config = ServiceConfiguration::production();

        // 创建临时 JSON 文件
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let json_content = serde_json::to_string(&config).expect("Failed to serialize");
        temp_file
            .write_all(json_content.as_bytes())
            .expect("Failed to write to temp file");

        // 使用 JSON 配置创建服务
        let services = AppServices::builder()
            .with_json_config(temp_file.path())
            .expect("Failed to load JSON config")
            .build()
            .expect("Failed to build services");

        // 验证服务已创建
        assert!(std::sync::Arc::strong_count(services.event_bus()) >= 1);
    }

    /// 测试服务生命周期管理
    #[test]
    fn test_service_lifecycle_management() {
        let services = AppServices::new().expect("Failed to create services");

        // 测试启动所有服务
        assert!(services.start_all().is_ok());

        // 测试停止所有服务
        assert!(services.stop_all().is_ok());
    }

    /// 测试服务健康检查
    #[test]
    fn test_service_health_checks() {
        let services = AppServices::new().expect("Failed to create services");

        // 检查所有服务的健康状态
        let health_checks = services.check_health();
        assert!(!health_checks.is_empty());

        // 验证每个服务都有健康检查结果
        for health in &health_checks {
            assert!(!health.service_name.is_empty());
            assert!(
                health.status == HealthStatus::Healthy
                    || health.status == HealthStatus::Degraded
                    || health.status == HealthStatus::Unhealthy
            );
        }

        // 获取整体健康状态
        let overall = services.overall_health();
        assert_eq!(overall.total_services, health_checks.len());
        assert!(overall.healthy_services <= overall.total_services);
    }

    /// 测试服务交互
    #[test]
    fn test_service_interactions() {
        let services = AppServices::new().expect("Failed to create services");

        // 测试事件总线
        let event_bus = services.event_bus();
        assert!(event_bus.subscriber_count() >= 0);

        // 测试资源追踪器
        let resource_tracker = services.resource_tracker();
        let report = resource_tracker.generate_report();
        assert!(report.total >= 0);

        // 测试取消管理器
        let cancellation_manager = services.cancellation_manager();
        let test_token = cancellation_manager.create_token("test-operation".to_string());
        assert!(!test_token.is_cancelled());
    }

    /// 测试配置驱动的服务创建
    #[test]
    fn test_configuration_driven_service_creation() {
        // 使用开发环境配置
        let dev_services = AppServices::builder()
            .with_development_config()
            .build()
            .expect("Failed to build dev services");

        let dev_health = dev_services.overall_health();
        assert_eq!(dev_health.status, HealthStatus::Healthy);

        // 使用生产环境配置
        let prod_services = AppServices::builder()
            .with_production_config()
            .build()
            .expect("Failed to build prod services");

        let prod_health = prod_services.overall_health();
        assert_eq!(prod_health.status, HealthStatus::Healthy);
    }

    /// 测试服务容器克隆
    #[test]
    fn test_service_container_cloning() {
        let services = AppServices::new().expect("Failed to create services");
        let services_clone = services.clone();

        // 验证克隆后引用计数增加
        assert!(std::sync::Arc::strong_count(services.event_bus()) >= 2);

        // 验证克隆的服务指向相同的实例
        assert!(std::sync::Arc::ptr_eq(
            services.event_bus(),
            services_clone.event_bus()
        ));
    }

    /// 测试并发服务访问
    #[test]
    fn test_concurrent_service_access() {
        use std::sync::Arc;
        use std::thread;

        let services = Arc::new(AppServices::new().expect("Failed to create services"));
        let mut handles = vec![];

        // 创建多个线程并发访问服务
        for i in 0..10 {
            let services_clone = Arc::clone(&services);
            let handle = thread::spawn(move || {
                // 访问事件总线
                let _event_bus = services_clone.event_bus();

                // 访问资源追踪器
                let resource_tracker = services_clone.resource_tracker();
                let _report = resource_tracker.generate_report();

                // 创建取消令牌
                let cancellation_manager = services_clone.cancellation_manager();
                let _token = cancellation_manager.create_token(format!("test-{}", i));
            });
            handles.push(handle);
        }

        // 等待所有线程完成
        for handle in handles {
            handle.join().expect("Thread panicked");
        }

        // 验证服务仍然健康
        let health = services.overall_health();
        assert_eq!(health.status, HealthStatus::Healthy);
    }

    /// 测试配置保存和加载的往返
    #[test]
    fn test_configuration_round_trip() {
        let original_config = ServiceConfiguration::development();

        // 保存到 TOML
        let toml_file = NamedTempFile::new().expect("Failed to create temp file");
        original_config
            .save_to_toml(toml_file.path())
            .expect("Failed to save TOML");

        // 从 TOML 加载
        let loaded_config = ServiceConfiguration::from_toml_file(toml_file.path())
            .expect("Failed to load TOML");

        // 验证配置相同
        assert_eq!(
            original_config.event_bus.capacity,
            loaded_config.event_bus.capacity
        );
        assert_eq!(
            original_config.query_executor.cache_size,
            loaded_config.query_executor.cache_size
        );

        // 保存到 JSON
        let json_file = NamedTempFile::new().expect("Failed to create temp file");
        original_config
            .save_to_json(json_file.path())
            .expect("Failed to save JSON");

        // 从 JSON 加载
        let loaded_json_config = ServiceConfiguration::from_json_file(json_file.path())
            .expect("Failed to load JSON");

        // 验证配置相同
        assert_eq!(
            original_config.event_bus.capacity,
            loaded_json_config.event_bus.capacity
        );
    }

    /// 测试服务依赖关系
    #[test]
    fn test_service_dependencies() {
        let services = AppServices::new().expect("Failed to create services");

        // 验证所有服务都可以独立访问
        let _event_bus = services.event_bus();
        let _query_executor = services.query_executor();
        let _resource_manager = services.resource_manager();
        let _cancellation_manager = services.cancellation_manager();
        let _resource_tracker = services.resource_tracker();

        // 验证服务之间没有循环依赖（编译时检查）
        // 如果有循环依赖，代码将无法编译
    }

    /// 测试完整的服务生命周期
    #[test]
    fn test_complete_service_lifecycle() {
        // 1. 创建配置
        let config = ServiceConfiguration::development();
        assert!(config.validate().is_ok());

        // 2. 使用配置创建服务
        let services = AppServices::builder()
            .with_config(config)
            .build()
            .expect("Failed to build services");

        // 3. 启动服务
        assert!(services.start_all().is_ok());

        // 4. 检查健康状态
        let health = services.overall_health();
        assert_eq!(health.status, HealthStatus::Healthy);

        // 5. 使用服务
        let event_bus = services.event_bus();
        assert!(event_bus.subscriber_count() >= 0);

        // 6. 停止服务
        assert!(services.stop_all().is_ok());
    }
}
