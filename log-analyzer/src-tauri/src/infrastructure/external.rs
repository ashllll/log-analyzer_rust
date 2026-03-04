//! 外部服务集成模块
//!
//! 提供与外部系统集成的基础设施实现，包括：
//! - 健康检查
//! - 速率限制
//!
//! 注意：HTTP 客户端功能需要启用 `http-client` feature

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, error};

/// 外部服务错误
#[derive(Debug, Error)]
pub enum ExternalServiceError {
    /// 连接失败
    #[error("连接失败: {0}")]
    ConnectionFailed(String),

    /// 超时
    #[error("请求超时")]
    Timeout,

    /// 服务不可用
    #[error("服务不可用: {0}")]
    ServiceUnavailable(String),

    /// 配置错误
    #[error("配置错误: {0}")]
    ConfigurationError(String),
}

/// 健康检查结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    /// 服务名称
    pub service: String,
    /// 是否健康
    pub healthy: bool,
    /// 响应时间（毫秒）
    pub response_time_ms: u64,
    /// 消息
    pub message: Option<String>,
    /// 时间戳
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// 附加元数据
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl HealthCheckResult {
    /// 创建健康的检查结果
    pub fn healthy(service: impl Into<String>, response_time_ms: u64) -> Self {
        Self {
            service: service.into(),
            healthy: true,
            response_time_ms,
            message: None,
            timestamp: chrono::Utc::now(),
            metadata: None,
        }
    }

    /// 创建健康的检查结果（带消息）
    pub fn healthy_with_message(
        service: impl Into<String>,
        response_time_ms: u64,
        message: impl Into<String>,
    ) -> Self {
        Self {
            service: service.into(),
            healthy: true,
            response_time_ms,
            message: Some(message.into()),
            timestamp: chrono::Utc::now(),
            metadata: None,
        }
    }

    /// 创建不健康的检查结果
    pub fn unhealthy(service: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            service: service.into(),
            healthy: false,
            response_time_ms: 0,
            message: Some(message.into()),
            timestamp: chrono::Utc::now(),
            metadata: None,
        }
    }

    /// 创建不健康的检查结果（带元数据）
    pub fn unhealthy_with_metadata(
        service: impl Into<String>,
        message: impl Into<String>,
        metadata: serde_json::Value,
    ) -> Self {
        Self {
            service: service.into(),
            healthy: false,
            response_time_ms: 0,
            message: Some(message.into()),
            timestamp: chrono::Utc::now(),
            metadata: Some(metadata),
        }
    }
}

/// 健康检查器
///
/// 用于检查系统组件的健康状态
pub struct HealthChecker {
    /// 上次检查时间
    last_check: Arc<std::sync::RwLock<Option<Instant>>>,
    /// 缓存的检查结果
    cached_results: Arc<std::sync::RwLock<Vec<HealthCheckResult>>>,
    /// 缓存有效期（秒）
    cache_ttl_secs: u64,
}

impl HealthChecker {
    /// 创建新的健康检查器
    pub fn new(cache_ttl_secs: u64) -> Self {
        Self {
            last_check: Arc::new(std::sync::RwLock::new(None)),
            cached_results: Arc::new(std::sync::RwLock::new(Vec::new())),
            cache_ttl_secs,
        }
    }

    /// 检查数据库连接
    pub async fn check_database(&self, database_path: &str) -> HealthCheckResult {
        let start = Instant::now();

        // 检查数据库文件是否存在
        let path = std::path::Path::new(database_path);
        if !path.exists() {
            return HealthCheckResult::unhealthy("database", "数据库文件不存在");
        }

        // 检查是否可读
        match std::fs::metadata(path) {
            Ok(metadata) => {
                let elapsed = start.elapsed().as_millis() as u64;
                let size_mb = metadata.len() as f64 / (1024.0 * 1024.0);
                HealthCheckResult::healthy_with_message(
                    "database",
                    elapsed,
                    format!("数据库正常 ({:.2} MB)", size_mb),
                )
            }
            Err(e) => HealthCheckResult::unhealthy("database", format!("无法访问数据库: {}", e)),
        }
    }

    /// 检查存储目录
    pub async fn check_storage(&self, storage_path: &str) -> HealthCheckResult {
        let start = Instant::now();

        let path = std::path::Path::new(storage_path);
        if !path.exists() {
            // 尝试创建目录
            match std::fs::create_dir_all(path) {
                Ok(_) => {
                    let elapsed = start.elapsed().as_millis() as u64;
                    return HealthCheckResult::healthy_with_message(
                        "storage",
                        elapsed,
                        "存储目录已创建",
                    );
                }
                Err(e) => {
                    return HealthCheckResult::unhealthy(
                        "storage",
                        format!("无法创建存储目录: {}", e),
                    );
                }
            }
        }

        // 检查是否可写
        let test_file = path.join(".health_check");
        match std::fs::write(&test_file, b"test") {
            Ok(_) => {
                let _ = std::fs::remove_file(&test_file);
                let elapsed = start.elapsed().as_millis() as u64;
                HealthCheckResult::healthy("storage", elapsed)
            }
            Err(e) => HealthCheckResult::unhealthy("storage", format!("存储目录不可写: {}", e)),
        }
    }

    /// 检查内存使用
    pub async fn check_memory(&self) -> HealthCheckResult {
        // 简单的内存检查（在实际项目中应使用系统 API）
        HealthCheckResult::healthy("memory", 0)
    }

    /// 执行所有检查
    pub async fn check_all(
        &self,
        database_path: &str,
        storage_path: &str,
    ) -> Vec<HealthCheckResult> {
        let mut results = Vec::new();

        results.push(self.check_database(database_path).await);
        results.push(self.check_storage(storage_path).await);
        results.push(self.check_memory().await);

        // 更新缓存
        *self.cached_results.write().unwrap() = results.clone();
        *self.last_check.write().unwrap() = Some(Instant::now());

        results
    }

    /// 获取缓存的检查结果
    pub fn get_cached_results(&self) -> Option<Vec<HealthCheckResult>> {
        let last_check = self.last_check.read().unwrap();
        if let Some(last) = *last_check {
            if last.elapsed().as_secs() < self.cache_ttl_secs {
                return Some(self.cached_results.read().unwrap().clone());
            }
        }
        None
    }
}

impl Default for HealthChecker {
    fn default() -> Self {
        Self::new(60) // 默认缓存 60 秒
    }
}

/// API 速率限制器
///
/// 使用令牌桶算法实现
pub struct RateLimiter {
    /// 服务名称
    service: String,
    /// 最大令牌数
    max_tokens: u32,
    /// 当前令牌数
    tokens: AtomicU32,
    /// 令牌恢复间隔（毫秒）
    refill_interval_ms: u64,
    /// 每次恢复的令牌数
    refill_amount: u32,
    /// 上次恢复时间
    last_refill: AtomicU64,
    /// 被拒绝的请求数
    rejected_count: AtomicU64,
}

impl RateLimiter {
    /// 创建新的速率限制器
    pub fn new(
        service: impl Into<String>,
        max_tokens: u32,
        refill_interval_ms: u64,
        refill_amount: u32,
    ) -> Self {
        Self {
            service: service.into(),
            max_tokens,
            tokens: AtomicU32::new(max_tokens),
            refill_interval_ms,
            refill_amount,
            last_refill: AtomicU64::new(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64,
            ),
            rejected_count: AtomicU64::new(0),
        }
    }

    /// 获取当前时间戳（毫秒）
    fn current_time_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }

    /// 尝试获取令牌
    ///
    /// 返回 true 表示获取成功，false 表示被限流
    pub fn try_acquire(&self) -> bool {
        // 先尝试恢复令牌
        self.try_refill();

        loop {
            let current = self.tokens.load(Ordering::Acquire);
            if current == 0 {
                self.rejected_count.fetch_add(1, Ordering::Relaxed);
                debug!(
                    service = %self.service,
                    rejected_count = self.rejected_count.load(Ordering::Relaxed),
                    "请求被限流"
                );
                return false;
            }

            match self.tokens.compare_exchange(
                current,
                current - 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    debug!(
                        service = %self.service,
                        remaining_tokens = current - 1,
                        "令牌获取成功"
                    );
                    return true;
                }
                Err(_) => continue,
            }
        }
    }

    /// 尝试恢复令牌
    fn try_refill(&self) {
        let now = Self::current_time_ms();
        let last = self.last_refill.load(Ordering::Acquire);

        if now - last >= self.refill_interval_ms {
            // 尝试更新上次恢复时间
            match self
                .last_refill
                .compare_exchange(last, now, Ordering::AcqRel, Ordering::Acquire)
            {
                Ok(_) => {
                    // 成功更新，恢复令牌
                    let current = self.tokens.load(Ordering::Acquire);
                    let new_value = (current + self.refill_amount).min(self.max_tokens);
                    self.tokens.store(new_value, Ordering::Release);

                    debug!(
                        service = %self.service,
                        tokens_before = current,
                        tokens_after = new_value,
                        "令牌已恢复"
                    );
                }
                Err(_) => {
                    // 其他线程已经更新，跳过
                }
            }
        }
    }

    /// 获取当前可用令牌数
    pub fn available_tokens(&self) -> u32 {
        self.tokens.load(Ordering::Acquire)
    }

    /// 获取被拒绝的请求数
    pub fn rejected_count(&self) -> u64 {
        self.rejected_count.load(Ordering::Relaxed)
    }

    /// 重置速率限制器
    pub fn reset(&self) {
        self.tokens.store(self.max_tokens, Ordering::Release);
        self.rejected_count.store(0, Ordering::Release);
        debug!(service = %self.service, "速率限制器已重置");
    }
}

/// 速率限制器配置
#[derive(Debug, Clone)]
pub struct RateLimiterConfig {
    /// 服务名称
    pub service: String,
    /// 最大令牌数
    pub max_tokens: u32,
    /// 令牌恢复间隔（毫秒）
    pub refill_interval_ms: u64,
    /// 每次恢复的令牌数
    pub refill_amount: u32,
}

impl RateLimiterConfig {
    /// 创建每秒限制的配置
    pub fn per_second(service: impl Into<String>, max_per_second: u32) -> Self {
        Self {
            service: service.into(),
            max_tokens: max_per_second,
            refill_interval_ms: 1000,
            refill_amount: max_per_second,
        }
    }

    /// 创建每分钟限制的配置
    pub fn per_minute(service: impl Into<String>, max_per_minute: u32) -> Self {
        Self {
            service: service.into(),
            max_tokens: max_per_minute,
            refill_interval_ms: 60000,
            refill_amount: max_per_minute,
        }
    }

    /// 构建速率限制器
    pub fn build(&self) -> RateLimiter {
        RateLimiter::new(
            &self.service,
            self.max_tokens,
            self.refill_interval_ms,
            self.refill_amount,
        )
    }
}

/// 外部服务管理器
///
/// 管理所有外部服务相关的功能
pub struct ExternalServiceManager {
    /// 健康检查器
    health_checker: HealthChecker,
    /// 速率限制器映射
    rate_limiters: Arc<std::sync::RwLock<std::collections::HashMap<String, Arc<RateLimiter>>>>,
}

impl ExternalServiceManager {
    /// 创建新的外部服务管理器
    pub fn new(health_cache_ttl_secs: u64) -> Self {
        Self {
            health_checker: HealthChecker::new(health_cache_ttl_secs),
            rate_limiters: Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// 获取健康检查器
    pub fn health_checker(&self) -> &HealthChecker {
        &self.health_checker
    }

    /// 注册速率限制器
    pub fn register_rate_limiter(&self, config: RateLimiterConfig) {
        let limiter = Arc::new(config.build());
        let mut limiters = self.rate_limiters.write().unwrap();
        limiters.insert(limiter.service.clone(), limiter);
    }

    /// 获取速率限制器
    pub fn get_rate_limiter(&self, service: &str) -> Option<Arc<RateLimiter>> {
        let limiters = self.rate_limiters.read().unwrap();
        limiters.get(service).cloned()
    }

    /// 检查所有服务健康状态
    pub async fn health_check_all(
        &self,
        database_path: &str,
        storage_path: &str,
    ) -> Vec<HealthCheckResult> {
        self.health_checker
            .check_all(database_path, storage_path)
            .await
    }
}

impl Default for ExternalServiceManager {
    fn default() -> Self {
        Self::new(60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_check_result() {
        let healthy = HealthCheckResult::healthy("test-service", 50);
        assert!(healthy.healthy);
        assert_eq!(healthy.response_time_ms, 50);
        assert!(healthy.message.is_none());

        let unhealthy = HealthCheckResult::unhealthy("test-service", "Connection refused");
        assert!(!unhealthy.healthy);
        assert_eq!(unhealthy.message, Some("Connection refused".to_string()));
    }

    #[test]
    fn test_rate_limiter() {
        let limiter = RateLimiter::new("test", 5, 1000, 1);

        // 应该可以获取 5 次令牌
        for i in 0..5 {
            assert!(limiter.try_acquire(), "第 {} 次获取应该成功", i + 1);
        }

        // 第 6 次应该失败
        assert!(!limiter.try_acquire(), "第 6 次获取应该失败");
        assert_eq!(limiter.rejected_count(), 1);

        // 重置
        limiter.reset();
        assert_eq!(limiter.available_tokens(), 5);
        assert_eq!(limiter.rejected_count(), 0);
    }

    #[test]
    fn test_rate_limiter_config() {
        let config = RateLimiterConfig::per_second("api", 10);
        assert_eq!(config.max_tokens, 10);
        assert_eq!(config.refill_interval_ms, 1000);

        let config = RateLimiterConfig::per_minute("search", 60);
        assert_eq!(config.max_tokens, 60);
        assert_eq!(config.refill_interval_ms, 60000);
    }

    #[test]
    fn test_external_service_manager() {
        let manager = ExternalServiceManager::new(60);

        // 注册速率限制器
        manager.register_rate_limiter(RateLimiterConfig::per_second("api", 10));

        // 获取速率限制器
        let limiter = manager.get_rate_limiter("api");
        assert!(limiter.is_some());

        // 不存在的服务
        let limiter = manager.get_rate_limiter("nonexistent");
        assert!(limiter.is_none());
    }
}
