/*!
 * Sentry 性能监控配置
 *
 * 提供生产环境的错误追踪和性能监控
 *
 * **Validates: Requirements 7.1, 7.3, 7.4**
 */

#![allow(dead_code)]

use sentry::{ClientOptions, IntoDsn};
use std::env;

/// Sentry 监控配置
#[allow(dead_code)]
pub struct SentryMonitoringConfig {
    /// Sentry DSN (Data Source Name)
    pub dsn: Option<String>,
    /// 环境名称 (development, staging, production)
    pub environment: String,
    /// 应用版本
    pub release: String,
    /// 采样率 (0.0 - 1.0)
    pub traces_sample_rate: f32,
    /// 性能监控采样率
    pub profiles_sample_rate: f32,
    /// 是否启用调试模式
    pub debug: bool,
}

impl Default for SentryMonitoringConfig {
    fn default() -> Self {
        Self {
            dsn: env::var("SENTRY_DSN").ok(),
            environment: env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()),
            release: env!("CARGO_PKG_VERSION").to_string(),
            traces_sample_rate: 0.1, // 10% 采样率
            profiles_sample_rate: 0.1,
            debug: cfg!(debug_assertions),
        }
    }
}

impl SentryMonitoringConfig {
    /// 创建生产环境配置
    pub fn production() -> Self {
        Self {
            environment: "production".to_string(),
            traces_sample_rate: 0.05, // 生产环境降低采样率
            profiles_sample_rate: 0.05,
            debug: false,
            ..Default::default()
        }
    }

    /// 创建开发环境配置
    pub fn development() -> Self {
        Self {
            environment: "development".to_string(),
            traces_sample_rate: 1.0, // 开发环境 100% 采样
            profiles_sample_rate: 1.0,
            debug: true,
            ..Default::default()
        }
    }
}

/// 初始化 Sentry 监控
///
/// 配置 Sentry 客户端并集成 tracing
pub fn init_sentry_monitoring(config: SentryMonitoringConfig) -> Option<sentry::ClientInitGuard> {
    let dsn = config.dsn.as_ref()?.clone();

    let options = ClientOptions {
        dsn: dsn.into_dsn().ok()?,
        environment: Some(config.environment.into()),
        release: Some(config.release.into()),
        traces_sample_rate: config.traces_sample_rate,
        debug: config.debug,
        ..Default::default()
    };

    let guard = sentry::init(options);

    // 简单的 tracing 初始化（不使用 sentry_tracing）
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tracing::info!(
        "Sentry monitoring initialized for environment: {}",
        guard.options().environment.as_ref().unwrap()
    );

    Some(guard)
}

/// 性能监控辅助函数
pub mod performance {
    use sentry::protocol::Event;
    use std::time::Instant;

    /// 性能事务
    pub struct PerformanceTransaction {
        name: String,
        start_time: Instant,
        transaction: Option<sentry::Transaction>,
    }

    impl PerformanceTransaction {
        /// 开始新的性能事务
        pub fn start(name: impl Into<String>) -> Self {
            let name = name.into();
            let transaction =
                sentry::start_transaction(sentry::TransactionContext::new(&name, "performance"));

            Self {
                name,
                start_time: Instant::now(),
                transaction: Some(transaction),
            }
        }

        /// 完成事务
        pub fn finish(mut self) {
            let duration = self.start_time.elapsed();

            if let Some(transaction) = self.transaction.take() {
                transaction.finish();
            }

            tracing::info!(
                "Performance transaction '{}' completed in {:?}",
                self.name,
                duration
            );
        }

        /// 完成事务并返回持续时间
        pub fn finish_with_duration(mut self) -> std::time::Duration {
            let duration = self.start_time.elapsed();

            if let Some(transaction) = self.transaction.take() {
                transaction.finish();
            }

            duration
        }
    }

    /// 记录性能指标
    pub fn record_metric(name: &str, value: f64, unit: &str) {
        tracing::info!(
            metric_name = name,
            metric_value = value,
            metric_unit = unit,
            "Performance metric recorded"
        );

        // 发送自定义事件到 Sentry
        sentry::capture_event(Event {
            message: Some(format!("Metric: {} = {} {}", name, value, unit)),
            level: sentry::Level::Info,
            ..Default::default()
        });
    }

    /// 记录缓存性能指标
    pub fn record_cache_metrics(hit_rate: f64, eviction_count: u64, size: usize) {
        record_metric("cache.hit_rate", hit_rate, "percentage");
        record_metric("cache.eviction_count", eviction_count as f64, "count");
        record_metric("cache.size", size as f64, "entries");
    }

    /// 记录搜索性能指标
    pub fn record_search_metrics(query_time_ms: f64, result_count: usize, files_scanned: usize) {
        record_metric("search.query_time", query_time_ms, "milliseconds");
        record_metric("search.result_count", result_count as f64, "count");
        record_metric("search.files_scanned", files_scanned as f64, "count");
    }
}

/// 错误监控辅助函数
pub mod error_monitoring {
    use sentry::protocol::Event;

    /// 捕获错误并发送到 Sentry
    pub fn capture_error(error: &dyn std::error::Error, context: &str) {
        tracing::error!(
            error = %error,
            context = context,
            "Error captured for monitoring"
        );

        sentry::capture_event(Event {
            message: Some(format!("{}: {}", context, error)),
            level: sentry::Level::Error,
            ..Default::default()
        });
    }

    /// 捕获警告
    pub fn capture_warning(message: &str, context: &str) {
        tracing::warn!(
            message = message,
            context = context,
            "Warning captured for monitoring"
        );

        sentry::capture_event(Event {
            message: Some(format!("{}: {}", context, message)),
            level: sentry::Level::Warning,
            ..Default::default()
        });
    }

    /// 设置用户上下文
    pub fn set_user_context(user_id: &str, workspace_id: Option<&str>) {
        sentry::configure_scope(|scope| {
            scope.set_user(Some(sentry::User {
                id: Some(user_id.to_string()),
                ..Default::default()
            }));

            if let Some(ws_id) = workspace_id {
                scope.set_tag("workspace_id", ws_id);
            }
        });
    }

    /// 添加面包屑（用于追踪用户操作路径）
    pub fn add_breadcrumb(category: &str, message: &str, level: sentry::Level) {
        sentry::add_breadcrumb(sentry::Breadcrumb {
            category: Some(category.to_string()),
            message: Some(message.to_string()),
            level,
            ..Default::default()
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_creation() {
        let config = SentryMonitoringConfig::default();
        assert_eq!(config.environment, "development");
        assert!(config.traces_sample_rate > 0.0);
    }

    #[test]
    fn test_production_config() {
        let config = SentryMonitoringConfig::production();
        assert_eq!(config.environment, "production");
        assert_eq!(config.traces_sample_rate, 0.05);
        assert!(!config.debug);
    }

    #[test]
    fn test_development_config() {
        let config = SentryMonitoringConfig::development();
        assert_eq!(config.environment, "development");
        assert_eq!(config.traces_sample_rate, 1.0);
        assert!(config.debug);
    }
}
