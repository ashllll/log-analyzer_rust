/**
 * 生产监控模块
 * 
 * 提供全面的性能监控、错误追踪和指标收集
 */

pub mod sentry_config;

pub use sentry_config::{
    error_monitoring, performance, init_sentry_monitoring, SentryMonitoringConfig,
};
