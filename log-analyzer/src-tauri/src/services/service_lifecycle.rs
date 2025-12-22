//! 服务生命周期管理
//!
//! 提供统一的服务生命周期接口，包括：
//! - 启动和停止服务
//! - 健康检查
//! - 优雅关闭

use eyre::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;

/// 服务特征 - 定义服务的生命周期接口
///
/// 所有需要生命周期管理的服务都应实现此特征
pub trait Service: Send + Sync {
    /// 服务名称
    fn name(&self) -> &str;

    /// 启动服务
    ///
    /// # Errors
    /// 如果启动失败，返回错误
    fn start(&self) -> Result<()> {
        tracing::info!("Starting service: {}", self.name());
        Ok(())
    }

    /// 停止服务
    ///
    /// # Errors
    /// 如果停止失败，返回错误
    fn stop(&self) -> Result<()> {
        tracing::info!("Stopping service: {}", self.name());
        Ok(())
    }

    /// 健康检查
    ///
    /// # Errors
    /// 如果健康检查失败，返回错误
    fn health_check(&self) -> Result<ServiceHealth> {
        Ok(ServiceHealth {
            service_name: self.name().to_string(),
            is_healthy: true,
            last_check: SystemTime::now(),
            status: HealthStatus::Healthy,
            details: HashMap::new(),
            message: None,
        })
    }

    /// 是否正在运行
    fn is_running(&self) -> bool {
        true
    }
}

/// 健康状态枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    /// 健康
    Healthy,
    /// 降级（部分功能不可用）
    Degraded,
    /// 不健康
    Unhealthy,
    /// 未知
    Unknown,
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "Healthy"),
            HealthStatus::Degraded => write!(f, "Degraded"),
            HealthStatus::Unhealthy => write!(f, "Unhealthy"),
            HealthStatus::Unknown => write!(f, "Unknown"),
        }
    }
}

/// 服务健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHealth {
    /// 服务名称
    pub service_name: String,
    /// 是否健康
    pub is_healthy: bool,
    /// 最后检查时间
    #[serde(with = "system_time_serde")]
    pub last_check: SystemTime,
    /// 健康状态
    pub status: HealthStatus,
    /// 详细信息
    pub details: HashMap<String, String>,
    /// 可选的消息
    pub message: Option<String>,
}

impl ServiceHealth {
    /// 创建健康状态
    pub fn healthy(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
            is_healthy: true,
            last_check: SystemTime::now(),
            status: HealthStatus::Healthy,
            details: HashMap::new(),
            message: None,
        }
    }

    /// 创建不健康状态
    pub fn unhealthy(service_name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
            is_healthy: false,
            last_check: SystemTime::now(),
            status: HealthStatus::Unhealthy,
            details: HashMap::new(),
            message: Some(message.into()),
        }
    }

    /// 创建降级状态
    pub fn degraded(service_name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
            is_healthy: true,
            last_check: SystemTime::now(),
            status: HealthStatus::Degraded,
            details: HashMap::new(),
            message: Some(message.into()),
        }
    }

    /// 添加详细信息
    pub fn with_detail(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.details.insert(key.into(), value.into());
        self
    }

    /// 添加消息
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}

// SystemTime 序列化辅助模块
mod system_time_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::{SystemTime, UNIX_EPOCH};

    pub fn serialize<S>(time: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let duration = time
            .duration_since(UNIX_EPOCH)
            .map_err(serde::ser::Error::custom)?;
        duration.as_secs().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(UNIX_EPOCH + std::time::Duration::from_secs(secs))
    }
}

/// 服务生命周期管理器
///
/// 管理所有服务的启动、停止和健康检查
pub struct ServiceLifecycleManager {
    services: Vec<Box<dyn Service>>,
}

impl ServiceLifecycleManager {
    /// 创建新的生命周期管理器
    pub fn new() -> Self {
        Self {
            services: Vec::new(),
        }
    }

    /// 注册服务
    pub fn register(&mut self, service: Box<dyn Service>) {
        tracing::info!("Registering service: {}", service.name());
        self.services.push(service);
    }

    /// 启动所有服务
    ///
    /// # Errors
    /// 如果任何服务启动失败，返回错误
    pub fn start_all(&self) -> Result<()> {
        tracing::info!("Starting all services");
        for service in &self.services {
            service.start()?;
        }
        tracing::info!("All services started successfully");
        Ok(())
    }

    /// 停止所有服务
    ///
    /// # Errors
    /// 如果任何服务停止失败，返回错误
    pub fn stop_all(&self) -> Result<()> {
        tracing::info!("Stopping all services");
        for service in self.services.iter().rev() {
            // 反向停止服务
            if let Err(e) = service.stop() {
                tracing::error!("Failed to stop service {}: {}", service.name(), e);
                // 继续停止其他服务
            }
        }
        tracing::info!("All services stopped");
        Ok(())
    }

    /// 检查所有服务的健康状态
    pub fn check_all_health(&self) -> Vec<ServiceHealth> {
        self.services
            .iter()
            .map(|service| match service.health_check() {
                Ok(health) => health,
                Err(e) => {
                    tracing::error!("Health check failed for service {}: {}", service.name(), e);
                    ServiceHealth::unhealthy(service.name(), format!("Health check failed: {}", e))
                }
            })
            .collect()
    }

    /// 获取整体健康状态
    pub fn overall_health(&self) -> OverallHealth {
        let health_checks = self.check_all_health();
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

    /// 获取服务数量
    pub fn service_count(&self) -> usize {
        self.services.len()
    }
}

impl Default for ServiceLifecycleManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 整体健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverallHealth {
    /// 整体状态
    pub status: HealthStatus,
    /// 总服务数
    pub total_services: usize,
    /// 健康服务数
    pub healthy_services: usize,
    /// 降级服务数
    pub degraded_services: usize,
    /// 不健康服务数
    pub unhealthy_services: usize,
    /// 各服务健康状态
    pub service_health: Vec<ServiceHealth>,
    /// 时间戳
    #[serde(with = "system_time_serde")]
    pub timestamp: SystemTime,
}

impl OverallHealth {
    /// 打印健康报告
    pub fn print_report(&self) {
        println!("\n=== Service Health Report ===");
        println!("Overall Status: {}", self.status);
        println!(
            "Services: {} total, {} healthy, {} degraded, {} unhealthy",
            self.total_services,
            self.healthy_services,
            self.degraded_services,
            self.unhealthy_services
        );
        println!("\nService Details:");
        for health in &self.service_health {
            println!(
                "  - {}: {} {}",
                health.service_name,
                health.status,
                health
                    .message
                    .as_ref()
                    .map(|m| format!("({})", m))
                    .unwrap_or_default()
            );
            if !health.details.is_empty() {
                for (key, value) in &health.details {
                    println!("      {}: {}", key, value);
                }
            }
        }
        println!("=============================\n");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 测试服务实现
    struct TestService {
        name: String,
        should_fail: bool,
    }

    impl Service for TestService {
        fn name(&self) -> &str {
            &self.name
        }

        fn start(&self) -> Result<()> {
            if self.should_fail {
                eyre::bail!("Service start failed");
            }
            Ok(())
        }

        fn stop(&self) -> Result<()> {
            if self.should_fail {
                eyre::bail!("Service stop failed");
            }
            Ok(())
        }

        fn health_check(&self) -> Result<ServiceHealth> {
            if self.should_fail {
                Ok(ServiceHealth::unhealthy(&self.name, "Service is unhealthy"))
            } else {
                Ok(ServiceHealth::healthy(&self.name))
            }
        }
    }

    #[test]
    fn test_service_health_creation() {
        let health = ServiceHealth::healthy("test-service");
        assert!(health.is_healthy);
        assert_eq!(health.status, HealthStatus::Healthy);
        assert_eq!(health.service_name, "test-service");

        let unhealthy = ServiceHealth::unhealthy("test-service", "Something went wrong");
        assert!(!unhealthy.is_healthy);
        assert_eq!(unhealthy.status, HealthStatus::Unhealthy);
        assert_eq!(unhealthy.message, Some("Something went wrong".to_string()));

        let degraded = ServiceHealth::degraded("test-service", "Partial failure");
        assert!(degraded.is_healthy);
        assert_eq!(degraded.status, HealthStatus::Degraded);
    }

    #[test]
    fn test_service_health_with_details() {
        let health = ServiceHealth::healthy("test-service")
            .with_detail("version", "1.0.0")
            .with_detail("uptime", "100s")
            .with_message("All systems operational");

        assert_eq!(health.details.get("version"), Some(&"1.0.0".to_string()));
        assert_eq!(health.details.get("uptime"), Some(&"100s".to_string()));
        assert_eq!(health.message, Some("All systems operational".to_string()));
    }

    #[test]
    fn test_lifecycle_manager_registration() {
        let mut manager = ServiceLifecycleManager::new();
        assert_eq!(manager.service_count(), 0);

        manager.register(Box::new(TestService {
            name: "service1".to_string(),
            should_fail: false,
        }));
        assert_eq!(manager.service_count(), 1);

        manager.register(Box::new(TestService {
            name: "service2".to_string(),
            should_fail: false,
        }));
        assert_eq!(manager.service_count(), 2);
    }

    #[test]
    fn test_lifecycle_manager_start_all() {
        let mut manager = ServiceLifecycleManager::new();
        manager.register(Box::new(TestService {
            name: "service1".to_string(),
            should_fail: false,
        }));
        manager.register(Box::new(TestService {
            name: "service2".to_string(),
            should_fail: false,
        }));

        assert!(manager.start_all().is_ok());
    }

    #[test]
    fn test_lifecycle_manager_start_failure() {
        let mut manager = ServiceLifecycleManager::new();
        manager.register(Box::new(TestService {
            name: "service1".to_string(),
            should_fail: false,
        }));
        manager.register(Box::new(TestService {
            name: "service2".to_string(),
            should_fail: true,
        }));

        assert!(manager.start_all().is_err());
    }

    #[test]
    fn test_lifecycle_manager_stop_all() {
        let mut manager = ServiceLifecycleManager::new();
        manager.register(Box::new(TestService {
            name: "service1".to_string(),
            should_fail: false,
        }));
        manager.register(Box::new(TestService {
            name: "service2".to_string(),
            should_fail: false,
        }));

        assert!(manager.stop_all().is_ok());
    }

    #[test]
    fn test_lifecycle_manager_health_check() {
        let mut manager = ServiceLifecycleManager::new();
        manager.register(Box::new(TestService {
            name: "service1".to_string(),
            should_fail: false,
        }));
        manager.register(Box::new(TestService {
            name: "service2".to_string(),
            should_fail: true,
        }));

        let health_checks = manager.check_all_health();
        assert_eq!(health_checks.len(), 2);

        let healthy_count = health_checks
            .iter()
            .filter(|h| h.status == HealthStatus::Healthy)
            .count();
        assert_eq!(healthy_count, 1);

        let unhealthy_count = health_checks
            .iter()
            .filter(|h| h.status == HealthStatus::Unhealthy)
            .count();
        assert_eq!(unhealthy_count, 1);
    }

    #[test]
    fn test_overall_health() {
        let mut manager = ServiceLifecycleManager::new();
        manager.register(Box::new(TestService {
            name: "service1".to_string(),
            should_fail: false,
        }));
        manager.register(Box::new(TestService {
            name: "service2".to_string(),
            should_fail: false,
        }));

        let overall = manager.overall_health();
        assert_eq!(overall.status, HealthStatus::Healthy);
        assert_eq!(overall.total_services, 2);
        assert_eq!(overall.healthy_services, 2);
        assert_eq!(overall.degraded_services, 0);
        assert_eq!(overall.unhealthy_services, 0);
    }

    #[test]
    fn test_overall_health_with_failures() {
        let mut manager = ServiceLifecycleManager::new();
        manager.register(Box::new(TestService {
            name: "service1".to_string(),
            should_fail: false,
        }));
        manager.register(Box::new(TestService {
            name: "service2".to_string(),
            should_fail: true,
        }));

        let overall = manager.overall_health();
        assert_eq!(overall.status, HealthStatus::Unhealthy);
        assert_eq!(overall.total_services, 2);
        assert_eq!(overall.healthy_services, 1);
        assert_eq!(overall.unhealthy_services, 1);
    }

    #[test]
    fn test_health_status_display() {
        assert_eq!(HealthStatus::Healthy.to_string(), "Healthy");
        assert_eq!(HealthStatus::Degraded.to_string(), "Degraded");
        assert_eq!(HealthStatus::Unhealthy.to_string(), "Unhealthy");
        assert_eq!(HealthStatus::Unknown.to_string(), "Unknown");
    }

    #[test]
    fn test_service_health_serialization() {
        let health = ServiceHealth::healthy("test-service")
            .with_detail("version", "1.0.0")
            .with_message("OK");

        // 测试 JSON 序列化
        let json = serde_json::to_string(&health).expect("Failed to serialize");
        let deserialized: ServiceHealth =
            serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(health.service_name, deserialized.service_name);
        assert_eq!(health.is_healthy, deserialized.is_healthy);
        assert_eq!(health.status, deserialized.status);
    }
}
