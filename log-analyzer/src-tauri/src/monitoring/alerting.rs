//! Alerting system for production monitoring
//!
//! **Feature: performance-optimization, Property 18: Performance Alert Generation**
//! This module implements comprehensive alerting for performance threshold violations,
//! resource constraints, and system health issues with actionable diagnostic information.

use eyre::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use tokio::time::interval;
use tracing::{debug, error, info, warn};

/// Alert severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Alert types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertType {
    PerformanceRegression {
        operation: String,
        current_duration: Duration,
        baseline_duration: Duration,
        regression_percentage: f64,
    },
    HighErrorRate {
        operation: String,
        error_rate: f64,
        threshold: f64,
    },
    ResourceExhaustion {
        resource: String,
        current_usage: f64,
        threshold: f64,
    },
    SystemHealth {
        component: String,
        status: String,
        details: HashMap<String, String>,
    },
    /// Response time violation alert
    /// **Validates: Requirements 4.4** - Alert generation for response time violations
    ResponseTimeViolation {
        operation: String,
        response_time_ms: f64,
        threshold_ms: f64,
        percentile: String,
    },
    /// Search performance degradation
    SearchPerformanceDegradation {
        query_type: String,
        avg_response_time_ms: f64,
        target_response_time_ms: f64,
        sample_count: u64,
    },
    /// Cache performance issue
    CachePerformanceIssue {
        issue_type: String,
        hit_rate: f64,
        target_hit_rate: f64,
        eviction_rate: f64,
    },
    /// State synchronization delay
    StateSyncDelay {
        sync_latency_ms: f64,
        target_latency_ms: f64,
        affected_workspaces: Vec<String>,
    },
}

/// Diagnostic information for alerts
/// **Validates: Requirements 4.4** - Actionable diagnostic information in alerts
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AlertDiagnostics {
    /// Root cause analysis
    pub probable_cause: String,
    /// Recommended actions to resolve the issue
    pub recommended_actions: Vec<String>,
    /// Related metrics that may help diagnose the issue
    pub related_metrics: HashMap<String, f64>,
    /// Links to relevant documentation or runbooks
    pub documentation_links: Vec<String>,
    /// Suggested queries or commands for further investigation
    pub investigation_commands: Vec<String>,
}

/// Alert escalation level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum EscalationLevel {
    /// Initial alert, no escalation
    Initial,
    /// First escalation after repeated occurrences
    Level1,
    /// Second escalation for persistent issues
    Level2,
    /// Critical escalation requiring immediate attention
    Critical,
}

/// Alert notification channel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationChannel {
    /// Log to application logs
    Log,
    /// Send to Sentry
    Sentry,
    /// Emit as Tauri event for frontend
    TauriEvent,
    /// Custom webhook (URL stored in config)
    Webhook { url: String },
}

/// Alert configuration
/// **Validates: Requirements 4.4** - Configurable performance thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertConfig {
    pub name: String,
    pub severity: AlertSeverity,
    pub threshold: f64,
    pub cooldown_duration: Duration,
    pub enabled: bool,
    /// Number of occurrences before escalation
    pub escalation_threshold: u32,
    /// Time window for counting occurrences (for escalation)
    pub escalation_window: Duration,
    /// Notification channels for this alert
    pub notification_channels: Vec<NotificationChannel>,
    /// Whether to auto-resolve when condition clears
    pub auto_resolve: bool,
}

/// Alert instance
/// **Feature: performance-optimization, Property 18: Performance Alert Generation**
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    pub alert_type: AlertType,
    pub severity: AlertSeverity,
    pub message: String,
    pub timestamp: SystemTime,
    pub resolved: bool,
    pub metadata: HashMap<String, String>,
    /// Diagnostic information for troubleshooting
    pub diagnostics: AlertDiagnostics,
    /// Current escalation level
    pub escalation_level: EscalationLevel,
    /// Number of times this alert has occurred
    pub occurrence_count: u32,
    /// Time of last occurrence
    pub last_occurrence: SystemTime,
}

/// Alert history for tracking and deduplication
#[derive(Debug, Clone)]
struct AlertHistory {
    last_sent: SystemTime,
    count: u32,
}

/// Production alerting system
pub struct AlertingSystem {
    alert_configs: parking_lot::RwLock<HashMap<String, AlertConfig>>,
    alert_history: parking_lot::RwLock<HashMap<String, AlertHistory>>,
    active_alerts: parking_lot::RwLock<HashMap<String, Alert>>,
}

impl AlertingSystem {
    /// Create a new alerting system
    pub fn new() -> Result<Self> {
        Ok(Self {
            alert_configs: parking_lot::RwLock::new(HashMap::new()),
            alert_history: parking_lot::RwLock::new(HashMap::new()),
            active_alerts: parking_lot::RwLock::new(HashMap::new()),
        })
    }

    /// Initialize alert configurations
    pub async fn initialize_alerts(&self) -> Result<()> {
        info!("Initializing alerting system");

        // Setup default alert configurations
        self.setup_default_alerts().await?;

        // Start alert cleanup task
        let alerting = self.clone();
        tauri::async_runtime::spawn(async move {
            let mut interval = interval(Duration::from_secs(300)); // Check every 5 minutes
            loop {
                interval.tick().await;
                alerting.cleanup_resolved_alerts().await;
            }
        });

        Ok(())
    }

    /// Setup default alert configurations
    /// **Validates: Requirements 4.4** - Configurable performance thresholds
    async fn setup_default_alerts(&self) -> Result<()> {
        let mut configs = self.alert_configs.write();

        // Performance regression alerts
        configs.insert(
            "performance_regression".to_string(),
            AlertConfig {
                name: "Performance Regression".to_string(),
                severity: AlertSeverity::Warning,
                threshold: 50.0, // 50% regression threshold
                cooldown_duration: Duration::from_secs(300), // 5 minute cooldown
                enabled: true,
                escalation_threshold: 3,
                escalation_window: Duration::from_secs(900), // 15 minutes
                notification_channels: vec![NotificationChannel::Log, NotificationChannel::Sentry],
                auto_resolve: true,
            },
        );

        // High error rate alerts
        configs.insert(
            "high_error_rate".to_string(),
            AlertConfig {
                name: "High Error Rate".to_string(),
                severity: AlertSeverity::Error,
                threshold: 5.0,                             // 5% error rate threshold
                cooldown_duration: Duration::from_secs(60), // 1 minute cooldown
                enabled: true,
                escalation_threshold: 5,
                escalation_window: Duration::from_secs(300), // 5 minutes
                notification_channels: vec![NotificationChannel::Log, NotificationChannel::Sentry],
                auto_resolve: true,
            },
        );

        // Memory usage alerts
        configs.insert(
            "high_memory_usage".to_string(),
            AlertConfig {
                name: "High Memory Usage".to_string(),
                severity: AlertSeverity::Warning,
                threshold: 80.0, // 80% memory usage threshold
                cooldown_duration: Duration::from_secs(600), // 10 minute cooldown
                enabled: true,
                escalation_threshold: 2,
                escalation_window: Duration::from_secs(1800), // 30 minutes
                notification_channels: vec![NotificationChannel::Log, NotificationChannel::Sentry],
                auto_resolve: true,
            },
        );

        // CPU usage alerts
        configs.insert(
            "high_cpu_usage".to_string(),
            AlertConfig {
                name: "High CPU Usage".to_string(),
                severity: AlertSeverity::Warning,
                threshold: 90.0, // 90% CPU usage threshold
                cooldown_duration: Duration::from_secs(300), // 5 minute cooldown
                enabled: true,
                escalation_threshold: 3,
                escalation_window: Duration::from_secs(900), // 15 minutes
                notification_channels: vec![NotificationChannel::Log, NotificationChannel::Sentry],
                auto_resolve: true,
            },
        );

        // Response time violation alerts (for search operations)
        configs.insert(
            "response_time_violation".to_string(),
            AlertConfig {
                name: "Response Time Violation".to_string(),
                severity: AlertSeverity::Warning,
                threshold: 200.0, // 200ms threshold for search operations
                cooldown_duration: Duration::from_secs(120), // 2 minute cooldown
                enabled: true,
                escalation_threshold: 5,
                escalation_window: Duration::from_secs(600), // 10 minutes
                notification_channels: vec![
                    NotificationChannel::Log,
                    NotificationChannel::Sentry,
                    NotificationChannel::TauriEvent,
                ],
                auto_resolve: true,
            },
        );

        // Search performance degradation
        configs.insert(
            "search_performance_degradation".to_string(),
            AlertConfig {
                name: "Search Performance Degradation".to_string(),
                severity: AlertSeverity::Warning,
                threshold: 500.0, // 500ms average threshold
                cooldown_duration: Duration::from_secs(300), // 5 minute cooldown
                enabled: true,
                escalation_threshold: 3,
                escalation_window: Duration::from_secs(900), // 15 minutes
                notification_channels: vec![NotificationChannel::Log, NotificationChannel::Sentry],
                auto_resolve: true,
            },
        );

        // Cache performance issue
        configs.insert(
            "cache_performance_issue".to_string(),
            AlertConfig {
                name: "Cache Performance Issue".to_string(),
                severity: AlertSeverity::Warning,
                threshold: 50.0,                             // 50% hit rate threshold
                cooldown_duration: Duration::from_secs(600), // 10 minute cooldown
                enabled: true,
                escalation_threshold: 2,
                escalation_window: Duration::from_secs(1800), // 30 minutes
                notification_channels: vec![NotificationChannel::Log, NotificationChannel::Sentry],
                auto_resolve: true,
            },
        );

        // State sync delay
        configs.insert(
            "state_sync_delay".to_string(),
            AlertConfig {
                name: "State Synchronization Delay".to_string(),
                severity: AlertSeverity::Warning,
                threshold: 100.0, // 100ms latency threshold
                cooldown_duration: Duration::from_secs(120), // 2 minute cooldown
                enabled: true,
                escalation_threshold: 5,
                escalation_window: Duration::from_secs(300), // 5 minutes
                notification_channels: vec![
                    NotificationChannel::Log,
                    NotificationChannel::TauriEvent,
                ],
                auto_resolve: true,
            },
        );

        info!("Configured {} default alerts", configs.len());
        Ok(())
    }

    /// Send a performance regression alert
    pub fn send_performance_alert(
        &self,
        operation: &str,
        current_duration: Duration,
        baseline_duration: Duration,
    ) {
        let regression_percentage = ((current_duration.as_millis() as f64
            - baseline_duration.as_millis() as f64)
            / baseline_duration.as_millis() as f64)
            * 100.0;

        let alert_type = AlertType::PerformanceRegression {
            operation: operation.to_string(),
            current_duration,
            baseline_duration,
            regression_percentage,
        };

        let message = format!(
            "Performance regression detected in '{}': {:.1}% slower than baseline ({:.1}ms vs {:.1}ms)",
            operation,
            regression_percentage,
            current_duration.as_millis(),
            baseline_duration.as_millis()
        );

        self.send_alert(
            "performance_regression",
            alert_type,
            message,
            HashMap::new(),
        );
    }

    /// Send a high error rate alert
    pub fn send_error_rate_alert(&self, operation: &str, error_rate: f64, threshold: f64) {
        let alert_type = AlertType::HighErrorRate {
            operation: operation.to_string(),
            error_rate,
            threshold,
        };

        let message = format!(
            "High error rate detected in '{}': {:.1}% (threshold: {:.1}%)",
            operation, error_rate, threshold
        );

        self.send_alert("high_error_rate", alert_type, message, HashMap::new());
    }

    /// Send a resource exhaustion alert
    pub fn send_resource_alert(&self, resource: &str, current_usage: f64, threshold: f64) {
        let alert_type = AlertType::ResourceExhaustion {
            resource: resource.to_string(),
            current_usage,
            threshold,
        };

        let message = format!(
            "High {} usage: {:.1}% (threshold: {:.1}%)",
            resource, current_usage, threshold
        );

        let alert_config_key = match resource {
            "memory" => "high_memory_usage",
            "cpu" => "high_cpu_usage",
            _ => "resource_exhaustion",
        };

        self.send_alert(alert_config_key, alert_type, message, HashMap::new());
    }

    /// Send a system health alert
    pub fn send_system_health_alert(
        &self,
        component: &str,
        status: &str,
        details: HashMap<String, String>,
    ) {
        let alert_type = AlertType::SystemHealth {
            component: component.to_string(),
            status: status.to_string(),
            details: details.clone(),
        };

        let message = format!("System health issue in '{}': {}", component, status);

        let diagnostics = AlertDiagnostics {
            probable_cause: format!("Component '{}' reported status: {}", component, status),
            recommended_actions: vec![
                format!("Check {} logs for detailed error information", component),
                "Review recent configuration changes".to_string(),
                "Verify system resources are adequate".to_string(),
            ],
            related_metrics: HashMap::new(),
            documentation_links: vec![],
            investigation_commands: vec![],
        };

        self.send_alert_with_diagnostics(
            "system_health",
            alert_type,
            message,
            details,
            diagnostics,
        );
    }

    /// Send a response time violation alert
    /// **Validates: Requirements 4.4** - Alert generation for response time violations
    pub fn send_response_time_alert(
        &self,
        operation: &str,
        response_time_ms: f64,
        threshold_ms: f64,
        percentile: &str,
    ) {
        let alert_type = AlertType::ResponseTimeViolation {
            operation: operation.to_string(),
            response_time_ms,
            threshold_ms,
            percentile: percentile.to_string(),
        };

        let message = format!(
            "Response time violation in '{}': {} percentile is {:.1}ms (threshold: {:.1}ms)",
            operation, percentile, response_time_ms, threshold_ms
        );

        let diagnostics = AlertDiagnostics {
            probable_cause: "Search query execution time exceeded acceptable threshold".to_string(),
            recommended_actions: vec![
                "Review query complexity and consider simplification".to_string(),
                "Check if index optimization is needed".to_string(),
                "Verify cache hit rates are acceptable".to_string(),
                "Consider adding specialized indexes for frequent query patterns".to_string(),
            ],
            related_metrics: {
                let mut metrics = HashMap::new();
                metrics.insert("response_time_ms".to_string(), response_time_ms);
                metrics.insert("threshold_ms".to_string(), threshold_ms);
                metrics
            },
            documentation_links: vec![],
            investigation_commands: vec![
                "Check query timing breakdown in metrics".to_string(),
                "Review recent query patterns".to_string(),
            ],
        };

        self.send_alert_with_diagnostics(
            "response_time_violation",
            alert_type,
            message,
            HashMap::new(),
            diagnostics,
        );
    }

    /// Send a search performance degradation alert
    pub fn send_search_degradation_alert(
        &self,
        query_type: &str,
        avg_response_time_ms: f64,
        target_response_time_ms: f64,
        sample_count: u64,
    ) {
        let alert_type = AlertType::SearchPerformanceDegradation {
            query_type: query_type.to_string(),
            avg_response_time_ms,
            target_response_time_ms,
            sample_count,
        };

        let message = format!(
            "Search performance degradation for '{}': avg {:.1}ms (target: {:.1}ms) over {} queries",
            query_type, avg_response_time_ms, target_response_time_ms, sample_count
        );

        let diagnostics = AlertDiagnostics {
            probable_cause: "Sustained search performance below target levels".to_string(),
            recommended_actions: vec![
                "Analyze query patterns for optimization opportunities".to_string(),
                "Consider index rebuilding or optimization".to_string(),
                "Review system resource utilization".to_string(),
                "Check for concurrent load affecting performance".to_string(),
            ],
            related_metrics: {
                let mut metrics = HashMap::new();
                metrics.insert("avg_response_time_ms".to_string(), avg_response_time_ms);
                metrics.insert(
                    "target_response_time_ms".to_string(),
                    target_response_time_ms,
                );
                metrics.insert("sample_count".to_string(), sample_count as f64);
                metrics
            },
            documentation_links: vec![],
            investigation_commands: vec![],
        };

        self.send_alert_with_diagnostics(
            "search_performance_degradation",
            alert_type,
            message,
            HashMap::new(),
            diagnostics,
        );
    }

    /// Send a cache performance issue alert
    pub fn send_cache_performance_alert(
        &self,
        issue_type: &str,
        hit_rate: f64,
        target_hit_rate: f64,
        eviction_rate: f64,
    ) {
        let alert_type = AlertType::CachePerformanceIssue {
            issue_type: issue_type.to_string(),
            hit_rate,
            target_hit_rate,
            eviction_rate,
        };

        let message = format!(
            "Cache performance issue ({}): hit rate {:.1}% (target: {:.1}%), eviction rate: {:.1}/min",
            issue_type, hit_rate * 100.0, target_hit_rate * 100.0, eviction_rate
        );

        let diagnostics = AlertDiagnostics {
            probable_cause: match issue_type {
                "low_hit_rate" => "Cache is not effectively serving repeated requests".to_string(),
                "high_eviction" => "Cache is thrashing due to insufficient capacity".to_string(),
                _ => "Cache performance is below expected levels".to_string(),
            },
            recommended_actions: vec![
                "Consider increasing cache capacity".to_string(),
                "Review TTL settings for cached items".to_string(),
                "Analyze access patterns for cache optimization".to_string(),
                "Check for cache key collisions or inefficient key strategies".to_string(),
            ],
            related_metrics: {
                let mut metrics = HashMap::new();
                metrics.insert("hit_rate".to_string(), hit_rate);
                metrics.insert("target_hit_rate".to_string(), target_hit_rate);
                metrics.insert("eviction_rate".to_string(), eviction_rate);
                metrics
            },
            documentation_links: vec![],
            investigation_commands: vec![],
        };

        self.send_alert_with_diagnostics(
            "cache_performance_issue",
            alert_type,
            message,
            HashMap::new(),
            diagnostics,
        );
    }

    /// Send a state synchronization delay alert
    pub fn send_state_sync_delay_alert(
        &self,
        sync_latency_ms: f64,
        target_latency_ms: f64,
        affected_workspaces: Vec<String>,
    ) {
        let alert_type = AlertType::StateSyncDelay {
            sync_latency_ms,
            target_latency_ms,
            affected_workspaces: affected_workspaces.clone(),
        };

        let message = format!(
            "State synchronization delay: {:.1}ms (target: {:.1}ms), affecting {} workspace(s)",
            sync_latency_ms,
            target_latency_ms,
            affected_workspaces.len()
        );

        let diagnostics = AlertDiagnostics {
            probable_cause: "WebSocket or event propagation latency exceeds acceptable threshold"
                .to_string(),
            recommended_actions: vec![
                "Check WebSocket connection health".to_string(),
                "Review Redis pub/sub performance".to_string(),
                "Verify network connectivity between components".to_string(),
                "Consider reducing event payload sizes".to_string(),
            ],
            related_metrics: {
                let mut metrics = HashMap::new();
                metrics.insert("sync_latency_ms".to_string(), sync_latency_ms);
                metrics.insert("target_latency_ms".to_string(), target_latency_ms);
                metrics.insert(
                    "affected_workspace_count".to_string(),
                    affected_workspaces.len() as f64,
                );
                metrics
            },
            documentation_links: vec![],
            investigation_commands: vec![],
        };

        let mut metadata = HashMap::new();
        metadata.insert(
            "affected_workspaces".to_string(),
            affected_workspaces.join(","),
        );

        self.send_alert_with_diagnostics(
            "state_sync_delay",
            alert_type,
            message,
            metadata,
            diagnostics,
        );
    }

    /// Send an alert with deduplication, cooldown, and diagnostics
    /// **Validates: Requirements 4.4** - Actionable diagnostic information in alerts
    fn send_alert_with_diagnostics(
        &self,
        config_key: &str,
        alert_type: AlertType,
        message: String,
        metadata: HashMap<String, String>,
        diagnostics: AlertDiagnostics,
    ) {
        // Check if alert is enabled
        let config = {
            let configs = self.alert_configs.read();
            configs.get(config_key).cloned()
        };

        let config = match config {
            Some(config) if config.enabled => config,
            _ => {
                debug!("Alert '{}' is disabled or not configured", config_key);
                return;
            }
        };

        // Check cooldown period and track occurrences for escalation
        let alert_key = format!("{}:{}", config_key, self.get_alert_key(&alert_type));
        let (should_send, occurrence_count, escalation_level) = {
            let mut history = self.alert_history.write();
            match history.get_mut(&alert_key) {
                Some(entry) => {
                    let elapsed = SystemTime::now()
                        .duration_since(entry.last_sent)
                        .unwrap_or_default();
                    if elapsed >= config.cooldown_duration {
                        entry.last_sent = SystemTime::now();
                        entry.count += 1;

                        // Determine escalation level based on occurrence count
                        let escalation = if entry.count >= config.escalation_threshold * 3 {
                            EscalationLevel::Critical
                        } else if entry.count >= config.escalation_threshold * 2 {
                            EscalationLevel::Level2
                        } else if entry.count >= config.escalation_threshold {
                            EscalationLevel::Level1
                        } else {
                            EscalationLevel::Initial
                        };

                        (true, entry.count, escalation)
                    } else {
                        (false, entry.count, EscalationLevel::Initial)
                    }
                }
                None => {
                    history.insert(
                        alert_key.clone(),
                        AlertHistory {
                            last_sent: SystemTime::now(),
                            count: 1,
                        },
                    );
                    (true, 1, EscalationLevel::Initial)
                }
            }
        };

        if !should_send {
            return;
        }

        // Adjust severity based on escalation level
        let effective_severity = match escalation_level {
            EscalationLevel::Critical => AlertSeverity::Critical,
            EscalationLevel::Level2 => {
                if config.severity == AlertSeverity::Info {
                    AlertSeverity::Warning
                } else if config.severity == AlertSeverity::Warning {
                    AlertSeverity::Error
                } else {
                    config.severity.clone()
                }
            }
            _ => config.severity.clone(),
        };

        // Create alert with diagnostics
        let alert = Alert {
            id: uuid::Uuid::new_v4().to_string(),
            alert_type: alert_type.clone(),
            severity: effective_severity.clone(),
            message: message.clone(),
            timestamp: SystemTime::now(),
            resolved: false,
            metadata,
            diagnostics,
            escalation_level: escalation_level.clone(),
            occurrence_count,
            last_occurrence: SystemTime::now(),
        };

        // Store active alert
        {
            let mut active_alerts = self.active_alerts.write();
            active_alerts.insert(alert.id.clone(), alert.clone());
        }

        // Send to monitoring systems
        self.dispatch_alert(&alert);

        // Log alert with escalation info
        let escalation_info = if escalation_level != EscalationLevel::Initial {
            format!(" [ESCALATED: {:?}]", escalation_level)
        } else {
            String::new()
        };

        match effective_severity {
            AlertSeverity::Info => info!(
                alert_id = alert.id,
                message = format!("{}{}", message, escalation_info),
                "Alert sent"
            ),
            AlertSeverity::Warning => warn!(
                alert_id = alert.id,
                message = format!("{}{}", message, escalation_info),
                "Alert sent"
            ),
            AlertSeverity::Error | AlertSeverity::Critical => {
                error!(
                    alert_id = alert.id,
                    message = format!("{}{}", message, escalation_info),
                    "Alert sent"
                )
            }
        }
    }

    /// Send an alert with deduplication and cooldown (legacy method for backward compatibility)
    fn send_alert(
        &self,
        config_key: &str,
        alert_type: AlertType,
        message: String,
        metadata: HashMap<String, String>,
    ) {
        self.send_alert_with_diagnostics(
            config_key,
            alert_type,
            message,
            metadata,
            AlertDiagnostics::default(),
        );
    }

    /// Dispatch alert to monitoring systems
    fn dispatch_alert(&self, alert: &Alert) {
        // Send to Sentry
        let sentry_level = match alert.severity {
            AlertSeverity::Info => sentry::Level::Info,
            AlertSeverity::Warning => sentry::Level::Warning,
            AlertSeverity::Error => sentry::Level::Error,
            AlertSeverity::Critical => sentry::Level::Fatal,
        };

        sentry::with_scope(
            |scope| {
                scope.set_tag("alert_type", format!("{:?}", alert.alert_type));
                scope.set_tag("alert_severity", format!("{:?}", alert.severity));
                scope.set_tag("alert_id", &alert.id);
                scope.set_tag("escalation_level", format!("{:?}", alert.escalation_level));

                // Add metadata as extra context
                for (key, value) in &alert.metadata {
                    scope.set_extra(key, value.clone().into());
                }

                // Add diagnostics as extra context
                scope.set_extra(
                    "probable_cause",
                    alert.diagnostics.probable_cause.clone().into(),
                );
                scope.set_extra(
                    "recommended_actions",
                    serde_json::json!(alert.diagnostics.recommended_actions),
                );
                scope.set_extra("occurrence_count", (alert.occurrence_count as u64).into());

                // Add alert type specific context
                match &alert.alert_type {
                    AlertType::PerformanceRegression {
                        operation,
                        current_duration,
                        baseline_duration,
                        regression_percentage,
                    } => {
                        scope.set_extra("operation", operation.clone().into());
                        scope.set_extra(
                            "current_duration_ms",
                            (current_duration.as_millis() as u64).into(),
                        );
                        scope.set_extra(
                            "baseline_duration_ms",
                            (baseline_duration.as_millis() as u64).into(),
                        );
                        scope.set_extra("regression_percentage", (*regression_percentage).into());
                    }
                    AlertType::HighErrorRate {
                        operation,
                        error_rate,
                        threshold,
                    } => {
                        scope.set_extra("operation", operation.clone().into());
                        scope.set_extra("error_rate", (*error_rate).into());
                        scope.set_extra("threshold", (*threshold).into());
                    }
                    AlertType::ResourceExhaustion {
                        resource,
                        current_usage,
                        threshold,
                    } => {
                        scope.set_extra("resource", resource.clone().into());
                        scope.set_extra("current_usage", (*current_usage).into());
                        scope.set_extra("threshold", (*threshold).into());
                    }
                    AlertType::SystemHealth {
                        component,
                        status,
                        details,
                    } => {
                        scope.set_extra("component", component.clone().into());
                        scope.set_extra("status", status.clone().into());
                        for (key, value) in details {
                            scope.set_extra(&format!("detail_{}", key), value.clone().into());
                        }
                    }
                    AlertType::ResponseTimeViolation {
                        operation,
                        response_time_ms,
                        threshold_ms,
                        percentile,
                    } => {
                        scope.set_extra("operation", operation.clone().into());
                        scope.set_extra("response_time_ms", (*response_time_ms).into());
                        scope.set_extra("threshold_ms", (*threshold_ms).into());
                        scope.set_extra("percentile", percentile.clone().into());
                    }
                    AlertType::SearchPerformanceDegradation {
                        query_type,
                        avg_response_time_ms,
                        target_response_time_ms,
                        sample_count,
                    } => {
                        scope.set_extra("query_type", query_type.clone().into());
                        scope.set_extra("avg_response_time_ms", (*avg_response_time_ms).into());
                        scope.set_extra(
                            "target_response_time_ms",
                            (*target_response_time_ms).into(),
                        );
                        scope.set_extra("sample_count", (*sample_count).into());
                    }
                    AlertType::CachePerformanceIssue {
                        issue_type,
                        hit_rate,
                        target_hit_rate,
                        eviction_rate,
                    } => {
                        scope.set_extra("issue_type", issue_type.clone().into());
                        scope.set_extra("hit_rate", (*hit_rate).into());
                        scope.set_extra("target_hit_rate", (*target_hit_rate).into());
                        scope.set_extra("eviction_rate", (*eviction_rate).into());
                    }
                    AlertType::StateSyncDelay {
                        sync_latency_ms,
                        target_latency_ms,
                        affected_workspaces,
                    } => {
                        scope.set_extra("sync_latency_ms", (*sync_latency_ms).into());
                        scope.set_extra("target_latency_ms", (*target_latency_ms).into());
                        scope.set_extra(
                            "affected_workspace_count",
                            (affected_workspaces.len() as u64).into(),
                        );
                    }
                }
            },
            || {
                sentry::capture_message(&alert.message, sentry_level);
            },
        );
    }

    /// Generate a unique key for alert deduplication
    fn get_alert_key(&self, alert_type: &AlertType) -> String {
        match alert_type {
            AlertType::PerformanceRegression { operation, .. } => {
                format!("perf_regression_{}", operation)
            }
            AlertType::HighErrorRate { operation, .. } => format!("error_rate_{}", operation),
            AlertType::ResourceExhaustion { resource, .. } => format!("resource_{}", resource),
            AlertType::SystemHealth { component, .. } => format!("health_{}", component),
            AlertType::ResponseTimeViolation {
                operation,
                percentile,
                ..
            } => {
                format!("response_time_{}_{}", operation, percentile)
            }
            AlertType::SearchPerformanceDegradation { query_type, .. } => {
                format!("search_degradation_{}", query_type)
            }
            AlertType::CachePerformanceIssue { issue_type, .. } => {
                format!("cache_issue_{}", issue_type)
            }
            AlertType::StateSyncDelay { .. } => "state_sync_delay".to_string(),
        }
    }

    /// Resolve an alert
    pub fn resolve_alert(&self, alert_id: &str) {
        let mut active_alerts = self.active_alerts.write();
        if let Some(alert) = active_alerts.get_mut(alert_id) {
            alert.resolved = true;
            info!(alert_id = alert_id, "Alert resolved");
        }
    }

    /// Get active alerts
    pub fn get_active_alerts(&self) -> Vec<Alert> {
        self.active_alerts
            .read()
            .values()
            .filter(|alert| !alert.resolved)
            .cloned()
            .collect()
    }

    /// Cleanup resolved alerts
    async fn cleanup_resolved_alerts(&self) {
        let cutoff = SystemTime::now() - Duration::from_secs(3600); // Keep resolved alerts for 1 hour

        let mut active_alerts = self.active_alerts.write();
        let original_len = active_alerts.len();

        active_alerts.retain(|_, alert| !alert.resolved || alert.timestamp > cutoff);

        let removed = original_len - active_alerts.len();
        if removed > 0 {
            debug!("Cleaned up {} resolved alerts", removed);
        }
    }
}

impl Clone for AlertingSystem {
    fn clone(&self) -> Self {
        Self {
            alert_configs: parking_lot::RwLock::new(self.alert_configs.read().clone()),
            alert_history: parking_lot::RwLock::new(HashMap::new()), // Don't clone history
            active_alerts: parking_lot::RwLock::new(self.active_alerts.read().clone()),
        }
    }
}
