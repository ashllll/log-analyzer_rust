//! 监控和可观测性模块
//!
//! 提供系统监控、性能指标收集和分布式追踪功能

use std::sync::Arc;
use tracing::{info, instrument};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};

pub mod metrics;
pub mod tracing;

/// 初始化监控系统
pub fn init_monitoring() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    init_tracing()?;
    
    // 初始化指标
    metrics::init_metrics()?;
    
    info!("Monitoring system initialized");
    Ok(())
}

/// 初始化追踪系统
fn init_tracing() -> Result<(), Box<dyn std::error::Error>> {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let subscriber = Registry::default()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_opentelemetry::layer());

    tracing::subscriber::set_global_default(subscriber)?;
    
    Ok(())
}

/// 系统健康检查
#[derive(Debug, Clone)]
pub struct HealthCheck {
    pub service_name: String,
    pub status: HealthStatus,
    pub details: Option<String>,
    pub timestamp: std::time::SystemTime,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// 性能指标收集器
#[derive(Debug, Clone)]
pub struct MetricsCollector {
    search_counter: Arc<metrics::Counter>,
    error_counter: Arc<metrics::Counter>,
    search_duration: Arc<metrics::Histogram>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            search_counter: Arc::new(metrics::Counter::new("searches_total")),
            error_counter: Arc::new(metrics::Counter::new("errors_total")),
            search_duration: Arc::new(metrics::Histogram::new("search_duration_seconds")),
        }
    }

    #[instrument(skip(self))]
    pub fn record_search(&self) {
        self.search_counter.increment(1);
    }

    #[instrument(skip(self))]
    pub fn record_error(&self, error_type: &str) {
        self.error_counter.increment(1);
        tracing::error!(error_type = %error_type, "Error occurred");
    }

    #[instrument(skip(self))]
    pub fn record_search_duration(&self, duration: std::time::Duration) {
        self.search_duration.record(duration.as_secs_f64());
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}