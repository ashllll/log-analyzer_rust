//! 监控和可观测性模块
//!
//! 提供系统监控、性能指标收集和分布式追踪功能
//!
//! # 功能特性
//!
//! - **指标收集**: Counter 和 Histogram 指标类型
//! - **分布式追踪**: 通过 `telemetry` feature 启用 OpenTelemetry 集成
//! - **日志记录**: 结构化日志，支持环境变量配置
//!
//! # 启用 OpenTelemetry
//!
//! 在 `Cargo.toml` 中启用 `telemetry` feature:
//!
//! ```toml
//! [dependencies]
//! log-analyzer = { features = ["telemetry"] }
//! ```
//!
//! 然后设置环境变量配置 OTLP 端点:
//!
//! ```bash
//! export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
//! ```
//!
//! 推荐使用 Jaeger 或 Grafana Tempo 作为追踪后端。

use std::sync::Arc;
use tracing::{info, instrument};
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

pub mod metrics;

/// 初始化监控系统
///
/// # 错误
///
/// 当 tracing 订阅器初始化失败时返回错误
pub fn init_monitoring() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 初始化日志和追踪
    init_tracing()?;

    // 初始化指标
    metrics::init_metrics()?;

    info!("Monitoring system initialized");
    Ok(())
}

/// 初始化追踪系统
///
/// 根据 `telemetry` feature 决定是否启用 OpenTelemetry 集成
fn init_tracing() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    #[cfg(feature = "telemetry")]
    {
        // 启用 OpenTelemetry 遥测
        init_tracing_with_telemetry(env_filter)
    }

    #[cfg(not(feature = "telemetry"))]
    {
        // 仅使用标准 tracing
        init_tracing_basic(env_filter)
    }
}

/// 基础 tracing 初始化（不启用 OpenTelemetry）
#[cfg(not(feature = "telemetry"))]
fn init_tracing_basic(
    env_filter: EnvFilter,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let subscriber = Registry::default()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer());

    tracing::subscriber::set_global_default(subscriber)?;

    info!("Tracing initialized (basic mode, OpenTelemetry disabled)");
    Ok(())
}

/// 带 OpenTelemetry 的 tracing 初始化
///
/// # 配置
///
/// 通过环境变量配置:
/// - `OTEL_EXPORTER_OTLP_ENDPOINT`: OTLP 收集器地址 (默认: http://localhost:4317)
/// - `OTEL_SERVICE_NAME`: 服务名称 (默认: log-analyzer)
///
/// # 示例
///
/// ```bash
/// # 启动 Jaeger 作为 OTLP 后端
/// docker run -d --name jaeger \
///   -p 4317:4317 \
///   -p 16686:16686 \
///   jaegertracing/all-in-one:latest
///
/// # 配置应用
/// export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
/// export OTEL_SERVICE_NAME=log-analyzer
/// ```
#[cfg(feature = "telemetry")]
fn init_tracing_with_telemetry(
    env_filter: EnvFilter,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use opentelemetry::trace::TracerProvider;
    use opentelemetry_otlp::WithExportConfig;
    use opentelemetry_sdk::trace::Sampler;
    use tracing_opentelemetry::OpenTelemetryLayer;

    // 获取服务名称
    let service_name =
        std::env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| "log-analyzer".to_string());

    // 获取 OTLP 端点
    let otlp_endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:4317".to_string());

    info!(
        service_name = %service_name,
        endpoint = %otlp_endpoint,
        "Initializing OpenTelemetry tracing"
    );

    // 创建 OTLP trace 导出器
    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(&otlp_endpoint)
        .build_span_exporter()?;

    // 创建追踪提供者
    let tracer_provider = opentelemetry_sdk::trace::TracerProvider::builder()
        .with_config(
            opentelemetry_sdk::trace::Config::default()
                .with_sampler(Sampler::AlwaysOn)
                .with_resource(opentelemetry_sdk::Resource::new(vec![
                    opentelemetry::KeyValue::new(
                        opentelemetry_semantic_conventions::resource::SERVICE_NAME,
                        service_name.clone(),
                    ),
                    opentelemetry::KeyValue::new(
                        opentelemetry_semantic_conventions::resource::SERVICE_VERSION,
                        env!("CARGO_PKG_VERSION"),
                    ),
                ])),
        )
        .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
        .build();

    // 获取 tracer
    let tracer = tracer_provider.tracer(service_name);

    // 创建 OpenTelemetry layer
    let telemetry_layer = OpenTelemetryLayer::new(tracer);

    // 构建订阅器
    let subscriber = Registry::default()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
        .with(telemetry_layer);

    tracing::subscriber::set_global_default(subscriber)?;

    info!("Tracing initialized with OpenTelemetry enabled");
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_collector() {
        let collector = MetricsCollector::new();

        collector.record_search();
        collector.record_search();
        assert_eq!(collector.search_counter.get(), 2);

        collector.record_error("test_error");
        assert_eq!(collector.error_counter.get(), 1);

        collector.record_search_duration(std::time::Duration::from_millis(100));
        assert_eq!(collector.search_duration.count(), 1);
    }

    #[test]
    fn test_health_check() {
        let health = HealthCheck {
            service_name: "test-service".to_string(),
            status: HealthStatus::Healthy,
            details: Some("All systems operational".to_string()),
            timestamp: std::time::SystemTime::now(),
        };

        assert_eq!(health.service_name, "test-service");
        assert_eq!(health.status, HealthStatus::Healthy);
        assert!(health.details.is_some());
    }

    #[test]
    fn test_health_status_equality() {
        assert_eq!(HealthStatus::Healthy, HealthStatus::Healthy);
        assert_ne!(HealthStatus::Healthy, HealthStatus::Degraded);
        assert_ne!(HealthStatus::Degraded, HealthStatus::Unhealthy);
    }
}
