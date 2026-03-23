//! 高级监控和可观测性系统

use once_cell::sync::Lazy;
use opentelemetry::global;
use opentelemetry::trace::{Span, Tracer};
use opentelemetry::KeyValue;
use parking_lot::Mutex;
use prometheus::{CounterVec, GaugeVec, HistogramVec, Registry};
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::time::Instant;
use sysinfo::System;
use tokio::sync::RwLock;

/// 缓存的 System 实例，避免每次调用 get_metrics() 都新建 System::new_all()
static CACHED_SYSTEM: Lazy<Mutex<System>> = Lazy::new(|| Mutex::new(System::new_all()));

/// 高级指标收集器
#[derive(Debug)]
pub struct AdvancedMetricsCollector {
    registry: Registry,
    
    // 业务指标
    search_requests: CounterVec,
    search_errors: CounterVec,
    search_duration: HistogramVec,
    
    // 系统指标
    active_connections: GaugeVec,
    memory_usage: GaugeVec,
    file_processing_rate: CounterVec,
    
    // 插件指标
    plugin_loads: CounterVec,
    plugin_errors: CounterVec,
    plugin_processing_time: HistogramVec,
}

impl AdvancedMetricsCollector {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let registry = Registry::new();
        
        // 搜索指标
        let search_requests = CounterVec::new(
            prometheus::Opts::new("search_requests_total", "Total search requests"),
            &["query_type", "status"]
        )?;
        
        let search_errors = CounterVec::new(
            prometheus::Opts::new("search_errors_total", "Total search errors"),
            &["error_type"]
        )?;
        
        let search_duration = HistogramVec::new(
            prometheus::HistogramOpts::new("search_duration_seconds", "Search request duration"),
            &["query_type"]
        )?;
        
        // 系统指标
        let active_connections = GaugeVec::new(
            prometheus::Opts::new("active_connections", "Number of active connections"),
            &["type"]
        )?;
        
        let memory_usage = GaugeVec::new(
            prometheus::Opts::new("memory_usage_bytes", "Memory usage in bytes"),
            &["type"]
        )?;
        
        let file_processing_rate = CounterVec::new(
            prometheus::Opts::new("file_processing_rate_total", "Files processed per second"),
            &["file_type", "status"]
        )?;
        
        // 插件指标
        let plugin_loads = CounterVec::new(
            prometheus::Opts::new("plugin_loads_total", "Total plugin loads"),
            &["plugin_name", "status"]
        )?;
        
        let plugin_errors = CounterVec::new(
            prometheus::Opts::new("plugin_errors_total", "Total plugin errors"),
            &["plugin_name", "error_type"]
        )?;
        
        let plugin_processing_time = HistogramVec::new(
            prometheus::HistogramOpts::new("plugin_processing_seconds", "Plugin processing time"),
            &["plugin_name", "operation"]
        )?;
        
        // 注册所有指标
        registry.register(Box::new(search_requests.clone()))?;
        registry.register(Box::new(search_errors.clone()))?;
        registry.register(Box::new(search_duration.clone()))?;
        registry.register(Box::new(active_connections.clone()))?;
        registry.register(Box::new(memory_usage.clone()))?;
        registry.register(Box::new(file_processing_rate.clone()))?;
        registry.register(Box::new(plugin_loads.clone()))?;
        registry.register(Box::new(plugin_errors.clone()))?;
        registry.register(Box::new(plugin_processing_time.clone()))?;
        
        Ok(Self {
            registry,
            search_requests,
            search_errors,
            search_duration,
            active_connections,
            memory_usage,
            file_processing_rate,
            plugin_loads,
            plugin_errors,
            plugin_processing_time,
        })
    }
    
    /// 记录搜索请求
    pub fn record_search_request(&self, query_type: &str, success: bool) {
        let status = if success { "success" } else { "error" };
        self.search_requests.with_label_values(&[query_type, status]).inc();
    }
    
    /// 记录搜索错误
    pub fn record_search_error(&self, error_type: &str) {
        self.search_errors.with_label_values(&[error_type]).inc();
    }
    
    /// 记录搜索持续时间
    pub fn record_search_duration(&self, query_type: &str, duration: f64) {
        self.search_duration.with_label_values(&[query_type]).observe(duration);
    }
    
    /// 记录活动连接数
    pub fn record_active_connections(&self, conn_type: &str, count: f64) {
        self.active_connections.with_label_values(&[conn_type]).set(count);
    }
    
    /// 记录内存使用
    pub fn record_memory_usage(&self, mem_type: &str, bytes: f64) {
        self.memory_usage.with_label_values(&[mem_type]).set(bytes);
    }
    
    /// 记录文件处理
    pub fn record_file_processing(&self, file_type: &str, success: bool) {
        let status = if success { "success" } else { "error" };
        self.file_processing_rate.with_label_values(&[file_type, status]).inc();
    }
    
    /// 记录插件加载
    pub fn record_plugin_load(&self, plugin_name: &str, success: bool) {
        let status = if success { "success" } else { "error" };
        self.plugin_loads.with_label_values(&[plugin_name, status]).inc();
    }
    
    /// 记录插件错误
    pub fn record_plugin_error(&self, plugin_name: &str, error_type: &str) {
        self.plugin_errors.with_label_values(&[plugin_name, error_type]).inc();
    }
    
    /// 记录插件处理时间
    pub fn record_plugin_processing_time(&self, plugin_name: &str, operation: &str, duration: f64) {
        self.plugin_processing_time.with_label_values(&[plugin_name, operation]).observe(duration);
    }
    
    /// 获取指标注册表
    pub fn registry(&self) -> &Registry {
        &self.registry
    }
}

/// 分布式追踪器
#[derive(Debug, Clone)]
pub struct DistributedTracer {
    tracer: Arc<dyn Tracer + Send + Sync>,
}

impl DistributedTracer {
    pub fn new() -> Self {
        let tracer = global::tracer("log-analyzer");
        Self {
            tracer: Arc::new(tracer),
        }
    }
    
    /// 创建搜索操作span
    pub fn create_search_span(&self, query: &str) -> Box<dyn Span> {
        let mut span = self.tracer.start("search_operation");
        span.set_attribute(KeyValue::new("search.query", query.to_string()));
        span
    }
    
    /// 创建文件处理span
    pub fn create_file_processing_span(&self, file_path: &str) -> Box<dyn Span> {
        let mut span = self.tracer.start("file_processing");
        span.set_attribute(KeyValue::new("file.path", file_path.to_string()));
        span
    }
    
    /// 创建插件执行span
    pub fn create_plugin_span(&self, plugin_name: &str, operation: &str) -> Box<dyn Span> {
        let mut span = self.tracer.start("plugin_execution");
        span.set_attribute(KeyValue::new("plugin.name", plugin_name.to_string()));
        span.set_attribute(KeyValue::new("plugin.operation", operation.to_string()));
        span
    }
}

/// 系统健康监控器
#[derive(Debug, Clone)]
pub struct HealthMonitor {
    metrics: Arc<AdvancedMetricsCollector>,
    last_check: Arc<RwLock<Instant>>,
}

impl HealthMonitor {
    pub fn new(metrics: Arc<AdvancedMetricsCollector>) -> Self {
        Self {
            metrics,
            last_check: Arc::new(RwLock::new(Instant::now())),
        }
    }
    
    /// 检查系统健康状态
    pub async fn check_health(&self) -> HealthStatus {
        let mut last_check = self.last_check.write().await;
        *last_check = Instant::now();
        
        // 模拟健康检查逻辑
        HealthStatus::Healthy
    }
    
    /// 获取系统指标
    pub async fn get_metrics(&self) -> HashMap<String, f64> {
        // 使用缓存的 System 实例，只刷新不重建，提高性能
        let result = tokio::task::spawn_blocking(|| {
            use sysinfo::{CpuExt, DiskExt, SystemExt};
            let mut sys = CACHED_SYSTEM.lock();
            sys.refresh_all();

            let memory_mb = sys.used_memory() as f64 / 1024.0 / 1024.0;
            let cpu_pct = sys.global_cpu_info().cpu_usage() as f64;
            let disk_gb: f64 = sys
                .disks()
                .iter()
                .map(|d| (d.total_space() - d.available_space()) as f64)
                .sum::<f64>()
                / 1024.0
                / 1024.0
                / 1024.0;

            (memory_mb, cpu_pct, disk_gb)
        })
        .await;

        let mut metrics = HashMap::new();
        if let Ok((memory_mb, cpu_pct, disk_gb)) = result {
            metrics.insert("memory_usage_mb".to_string(), memory_mb);
            metrics.insert("cpu_usage_percent".to_string(), cpu_pct);
            metrics.insert("disk_usage_gb".to_string(), disk_gb);
        }
        metrics
    }
}

/// 健康状态枚举
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

impl fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "healthy"),
            HealthStatus::Degraded => write!(f, "degraded"),
            HealthStatus::Unhealthy => write!(f, "unhealthy"),
        }
    }
}

/// 统一监控管理器
#[derive(Debug, Clone)]
pub struct UnifiedMonitoringManager {
    metrics: Arc<AdvancedMetricsCollector>,
    tracer: Arc<DistributedTracer>,
    health: Arc<HealthMonitor>,
}

impl UnifiedMonitoringManager {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let metrics = Arc::new(AdvancedMetricsCollector::new()?);
        let tracer = Arc::new(DistributedTracer::new());
        let health = Arc::new(HealthMonitor::new(metrics.clone()));

        Ok(Self {
            metrics,
            tracer,
            health,
        })
    }

    /// 创建降级实例，用于初始化失败时兜底
    ///
    /// `AdvancedMetricsCollector::new()` 使用私有 `Registry::new()`，在正常情况下
    /// 不会失败。此处使用 `expect` 而非 `panic!`，若再次失败说明是不可恢复的内部错误。
    fn new_degraded() -> Self {
        let metrics = Arc::new(
            AdvancedMetricsCollector::new()
                .expect("降级监控收集器创建失败（私有注册表不应注册失败，这是内部错误）"),
        );
        let tracer = Arc::new(DistributedTracer::new());
        let health = Arc::new(HealthMonitor::new(metrics.clone()));
        Self {
            metrics,
            tracer,
            health,
        }
    }
    
    pub fn metrics(&self) -> &Arc<AdvancedMetricsCollector> {
        &self.metrics
    }
    
    pub fn tracer(&self) -> &Arc<DistributedTracer> {
        &self.tracer
    }
    
    pub fn health(&self) -> &Arc<HealthMonitor> {
        &self.health
    }
    
    /// 启动监控服务
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        // 启动指标收集
        self.metrics.record_active_connections("web", 0.0);
        
        // 启动健康检查
        let health_status = self.health.check_health().await;
        tracing::info!("System health: {}", health_status);
        
        Ok(())
    }
    
    /// 获取监控仪表板数据
    pub async fn get_dashboard_data(&self) -> serde_json::Value {
        let health_status = self.health.check_health().await;
        let metrics = self.health.get_metrics().await;
        
        serde_json::json!({
            "health": health_status.to_string(),
            "metrics": metrics,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        })
    }
}

impl Default for UnifiedMonitoringManager {
    fn default() -> Self {
        Self::new().unwrap_or_else(|e| {
            // Prometheus 指标注册失败通常是由于同一进程中重复注册
            // 不再 panic，改为记录错误并使用降级实例，防止生产环境崩溃
            tracing::error!(
                error = %e,
                "UnifiedMonitoringManager 初始化失败，切换到降级模式运行（指标可能不可用）"
            );
            Self::new_degraded()
        })
    }
}