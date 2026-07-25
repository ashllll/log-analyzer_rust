//! 服务器、存储、监控、安全、数据库、速率限制与前端配置

use super::defaults::*;
use crate::models::config::validator::*;
use serde::{Deserialize, Serialize};

// ============ 服务器配置 ============

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServerConfig {
    #[serde(default = "default_3000_u16")]
    pub port: u16,

    #[serde(default = "default_localhost")]
    pub host: String,

    #[serde(default = "default_100_usize")]
    pub max_connections: usize,

    #[serde(default = "default_30_u64")]
    pub timeout_seconds: u64,
}

fn default_3000_u16() -> u16 {
    3000
}

fn default_localhost() -> String {
    "localhost".to_string()
}

fn default_100_usize() -> usize {
    100
}

fn default_30_u64() -> u64 {
    30
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 3000,
            host: "localhost".to_string(),
            max_connections: 100,
            timeout_seconds: 30,
        }
    }
}

impl ConfigValidator for ServerConfig {
    fn validate(&self) -> ValidationResult {
        let mut result = ValidationResult::new();

        // 验证端口
        if let Some(err) = validate_port(self.port) {
            result.add_error("port", err.message, err.code);
        }

        // 验证主机名
        if let Some(err) = validate_host(&self.host) {
            result.add_error("host", err.message, err.code);
        }

        // 验证最大连接数
        if let Some(err) = validate_range("max_connections", self.max_connections, 1, 10000) {
            result.add_error(err.field, err.message, err.code);
        }

        // 验证超时时间
        if let Some(err) = validate_range("timeout_seconds", self.timeout_seconds, 1, 3600) {
            result.add_error(err.field, err.message, err.code);
        }

        result
    }

    fn validate_with_defaults(&self) -> (ValidationResult, bool) {
        let result = self.validate();
        let mut config = self.clone();
        let mut modified = false;

        if self.port == 0 {
            config.port = 3000;
            modified = true;
        }

        if self.host.is_empty() {
            config.host = "localhost".to_string();
            modified = true;
        }

        if self.max_connections == 0 || self.max_connections > 10000 {
            config.max_connections = 100;
            modified = true;
        }

        if self.timeout_seconds == 0 || self.timeout_seconds > 3600 {
            config.timeout_seconds = 30;
            modified = true;
        }

        (result, !modified)
    }
}

// ============ 存储配置 ============

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StorageConfig {
    #[serde(default = "default_data_dir")]
    pub data_dir: String,

    #[serde(default = "default_100_u64")]
    pub max_file_size_mb: u64,

    #[serde(default = "default_10_usize")]
    pub max_concurrent_files: usize,

    #[serde(default = "default_true")]
    pub compression_enabled: bool,

    #[serde(default = "default_false")]
    pub encryption_enabled: bool,
}

fn default_data_dir() -> String {
    "./data".to_string()
}

fn default_100_u64() -> u64 {
    100
}

fn default_10_usize() -> usize {
    10
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            data_dir: "./data".to_string(),
            max_file_size_mb: 100,
            max_concurrent_files: 10,
            compression_enabled: true,
            encryption_enabled: false,
        }
    }
}

impl ConfigValidator for StorageConfig {
    fn validate(&self) -> ValidationResult {
        let mut result = ValidationResult::new();

        // 验证数据目录
        if let Some(err) = validate_path("data_dir", &self.data_dir) {
            result.add_error(err.field, err.message, err.code);
        }

        // 验证最大文件大小
        if let Some(err) = validate_range("max_file_size_mb", self.max_file_size_mb, 1, 10000) {
            result.add_error(err.field, err.message, err.code);
        }

        // 验证并发文件数
        if let Some(err) =
            validate_range("max_concurrent_files", self.max_concurrent_files, 1, 1000)
        {
            result.add_error(err.field, err.message, err.code);
        }

        result
    }

    fn validate_with_defaults(&self) -> (ValidationResult, bool) {
        let result = self.validate();
        let mut modified = false;

        if self.data_dir.is_empty() || self.data_dir.contains("..") {
            modified = true;
        }

        if self.max_file_size_mb == 0 || self.max_file_size_mb > 10000 {
            modified = true;
        }

        if self.max_concurrent_files == 0 || self.max_concurrent_files > 1000 {
            modified = true;
        }

        (result, !modified)
    }
}


// ============ 监控配置 ============

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MonitoringConfig {
    #[serde(default = "default_info_level")]
    pub log_level: String,

    #[serde(default = "default_true")]
    pub metrics_enabled: bool,

    #[serde(default = "default_true")]
    pub tracing_enabled: bool,

    #[serde(default = "default_log_file")]
    pub log_file: String,

    #[serde(default = "default_5_usize")]
    pub max_log_files: usize,
}

fn default_info_level() -> String {
    "info".to_string()
}

fn default_log_file() -> String {
    "./logs/app.log".to_string()
}

fn default_5_usize() -> usize {
    5
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            log_level: "info".to_string(),
            metrics_enabled: true,
            tracing_enabled: true,
            log_file: "./logs/app.log".to_string(),
            max_log_files: 5,
        }
    }
}

impl ConfigValidator for MonitoringConfig {
    fn validate(&self) -> ValidationResult {
        let mut result = ValidationResult::new();

        // 验证日志级别
        if let Some(err) = validate_log_level(&self.log_level) {
            result.add_error("log_level", err.message, err.code);
        }

        // 验证日志文件路径
        if let Some(err) = validate_path("log_file", &self.log_file) {
            result.add_error(err.field, err.message, err.code);
        }

        // 验证最大日志文件数
        if let Some(err) = validate_range("max_log_files", self.max_log_files, 1, 100) {
            result.add_error(err.field, err.message, err.code);
        }

        result
    }

    fn validate_with_defaults(&self) -> (ValidationResult, bool) {
        let result = self.validate();
        let mut modified = false;

        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&self.log_level.to_lowercase().as_str()) {
            modified = true;
        }

        if self.max_log_files == 0 || self.max_log_files > 100 {
            modified = true;
        }

        (result, !modified)
    }
}

// ============ 安全配置 ============

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SecurityConfig {
    #[serde(default = "default_false")]
    pub auth_enabled: bool,

    #[serde(default = "default_none")]
    pub api_key: Option<String>,

    #[serde(default = "default_true")]
    pub rate_limit_enabled: bool,

    #[serde(default = "default_100_u64")]
    pub rate_limit_per_minute: u64,

    #[serde(default = "default_true")]
    pub cors_enabled: bool,

    #[serde(default = "default_wildcard")]
    pub allowed_origins: Vec<String>,
}

fn default_none<T>() -> Option<T> {
    None
}

fn default_wildcard() -> Vec<String> {
    vec!["*".to_string()]
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            auth_enabled: false,
            api_key: None,
            rate_limit_enabled: true,
            rate_limit_per_minute: 100,
            cors_enabled: true,
            allowed_origins: vec!["*".to_string()],
        }
    }
}

impl ConfigValidator for SecurityConfig {
    fn validate(&self) -> ValidationResult {
        let mut result = ValidationResult::new();

        // 验证 API 密钥长度
        if let Some(ref api_key) = self.api_key {
            if api_key.len() < 16 {
                result.add_error(
                    "api_key",
                    "API 密钥长度至少为 16 个字符",
                    "api_key_too_short",
                );
            }
            if api_key.len() > 256 {
                result.add_error(
                    "api_key",
                    "API 密钥长度不能超过 256 个字符",
                    "api_key_too_long",
                );
            }
        }

        // 验证速率限制
        if let Some(err) = validate_range(
            "rate_limit_per_minute",
            self.rate_limit_per_minute,
            1,
            10000,
        ) {
            result.add_error(err.field, err.message, err.code);
        }

        // 验证允许的源
        for (i, origin) in self.allowed_origins.iter().enumerate() {
            if origin != "*" {
                // 如果不是通配符，验证是否为有效的 URL 格式
                if !origin.starts_with("http://") && !origin.starts_with("https://") {
                    result.add_error(
                        format!("allowed_origins[{i}]"),
                        format!("来源 '{origin}' 必须以 http:// 或 https:// 开头"),
                        "invalid_origin",
                    );
                }
            }
        }

        result
    }

    fn validate_with_defaults(&self) -> (ValidationResult, bool) {
        let result = self.validate();
        let mut modified = false;

        if self.rate_limit_per_minute == 0 || self.rate_limit_per_minute > 10000 {
            modified = true;
        }

        (result, !modified)
    }
}


// ============ 数据库配置 ============

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DatabaseConfig {
    #[serde(default = "default_30_u64_v3")]
    pub connection_timeout_seconds: u64,

    #[serde(default = "default_600_u64")]
    pub idle_timeout_seconds: u64,

    #[serde(default = "default_1800_u64")]
    pub max_lifetime_seconds: u64,

    #[serde(default = "default_1000_usize")]
    pub event_buffer_size: usize,

    #[serde(default = "default_1000_usize")]
    pub channel_capacity: usize,

    #[serde(default = "default_100000_usize")]
    pub max_cached_results: usize,

    #[serde(default = "default_8kb")]
    pub read_buffer_size: u64,

    #[serde(default = "default_64kb")]
    pub streaming_builder_buffer_size: u64,

    #[serde(default = "default_64kb")]
    pub buffer_size: u64,
}

fn default_30_u64_v3() -> u64 {
    30
}
fn default_600_u64() -> u64 {
    600
}
fn default_1800_u64() -> u64 {
    1800
}
fn default_100000_usize() -> usize {
    100_000
}
fn default_8kb() -> u64 {
    8 * 1024
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            connection_timeout_seconds: 30,
            idle_timeout_seconds: 600,
            max_lifetime_seconds: 1800,
            event_buffer_size: 1000,
            channel_capacity: 1000,
            max_cached_results: 100_000,
            read_buffer_size: 1024 * 1024, // 1MB (优化: 从 8KB 增大)
            streaming_builder_buffer_size: 1024 * 1024, // 1MB (优化: 从 64KB 增大)
            buffer_size: 1024 * 1024,      // 1MB (优化: 从 64KB 增大)
        }
    }
}

impl ConfigValidator for DatabaseConfig {
    fn validate(&self) -> ValidationResult {
        let mut result = ValidationResult::new();

        // 验证连接超时
        if let Some(err) = validate_range(
            "connection_timeout_seconds",
            self.connection_timeout_seconds,
            1,
            300,
        ) {
            result.add_error(err.field, err.message, err.code);
        }

        // 验证空闲超时
        if let Some(err) = validate_range(
            "idle_timeout_seconds",
            self.idle_timeout_seconds,
            1,
            3600 * 24,
        ) {
            result.add_error(err.field, err.message, err.code);
        }

        // 验证最大生命周期
        if let Some(err) = validate_range(
            "max_lifetime_seconds",
            self.max_lifetime_seconds,
            1,
            3600 * 24 * 7,
        ) {
            result.add_error(err.field, err.message, err.code);
        }

        // 验证缓冲区大小
        if let Some(err) = validate_range("event_buffer_size", self.event_buffer_size, 1, 100000) {
            result.add_error(err.field, err.message, err.code);
        }

        if let Some(err) = validate_range("channel_capacity", self.channel_capacity, 1, 100000) {
            result.add_error(err.field, err.message, err.code);
        }

        if let Some(err) =
            validate_range("max_cached_results", self.max_cached_results, 1, 10_000_000)
        {
            result.add_error(err.field, err.message, err.code);
        }

        // 验证读取缓冲区
        if let Some(err) = validate_range(
            "read_buffer_size",
            self.read_buffer_size,
            1024,
            100 * 1024 * 1024,
        ) {
            result.add_error(err.field, err.message, err.code);
        }

        if let Some(err) = validate_range(
            "streaming_builder_buffer_size",
            self.streaming_builder_buffer_size,
            1024,
            100 * 1024 * 1024,
        ) {
            result.add_error(err.field, err.message, err.code);
        }

        if let Some(err) = validate_range("buffer_size", self.buffer_size, 1024, 100 * 1024 * 1024)
        {
            result.add_error(err.field, err.message, err.code);
        }

        result
    }

    fn validate_with_defaults(&self) -> (ValidationResult, bool) {
        let result = self.validate();
        let mut modified = false;

        if self.connection_timeout_seconds == 0 || self.connection_timeout_seconds > 300 {
            modified = true;
        }

        if self.max_cached_results == 0 || self.max_cached_results > 10_000_000 {
            modified = true;
        }

        (result, !modified)
    }
}

// ============ 速率限制配置 ============

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RateLimitConfig {
    #[serde(default = "default_60_u64")]
    pub search_per_minute: u64,

    #[serde(default = "default_5_u64")]
    pub search_max_burst: u64,

    #[serde(default = "default_10_u64")]
    pub import_per_minute: u64,

    #[serde(default = "default_2_u64")]
    pub import_max_burst: u64,

    #[serde(default = "default_120_u64")]
    pub workspace_per_minute: u64,

    #[serde(default = "default_20_u64")]
    pub workspace_max_burst: u64,

    #[serde(default = "default_200_u64")]
    pub default_per_minute: u64,

    #[serde(default = "default_30_u64")]
    pub default_max_burst: u64,
}

fn default_60_u64() -> u64 {
    60
}
fn default_120_u64() -> u64 {
    120
}
fn default_200_u64() -> u64 {
    200
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            search_per_minute: 60,
            search_max_burst: 5,
            import_per_minute: 10,
            import_max_burst: 2,
            workspace_per_minute: 120,
            workspace_max_burst: 20,
            default_per_minute: 200,
            default_max_burst: 30,
        }
    }
}

impl ConfigValidator for RateLimitConfig {
    fn validate(&self) -> ValidationResult {
        let mut result = ValidationResult::new();

        // 验证搜索速率限制
        if let Some(err) = validate_range("search_per_minute", self.search_per_minute, 1, 10000) {
            result.add_error(err.field, err.message, err.code);
        }

        if let Some(err) = validate_range("search_max_burst", self.search_max_burst, 1, 1000) {
            result.add_error(err.field, err.message, err.code);
        }

        // 验证导入速率限制
        if let Some(err) = validate_range("import_per_minute", self.import_per_minute, 1, 1000) {
            result.add_error(err.field, err.message, err.code);
        }

        if let Some(err) = validate_range("import_max_burst", self.import_max_burst, 1, 100) {
            result.add_error(err.field, err.message, err.code);
        }

        // 验证工作区速率限制
        if let Some(err) =
            validate_range("workspace_per_minute", self.workspace_per_minute, 1, 1000)
        {
            result.add_error(err.field, err.message, err.code);
        }

        if let Some(err) = validate_range("workspace_max_burst", self.workspace_max_burst, 1, 100) {
            result.add_error(err.field, err.message, err.code);
        }

        // 验证默认速率限制
        if let Some(err) = validate_range("default_per_minute", self.default_per_minute, 1, 10000) {
            result.add_error(err.field, err.message, err.code);
        }

        if let Some(err) = validate_range("default_max_burst", self.default_max_burst, 1, 1000) {
            result.add_error(err.field, err.message, err.code);
        }

        result
    }

    fn validate_with_defaults(&self) -> (ValidationResult, bool) {
        let result = self.validate();
        let mut modified = false;

        let fields = [
            ("search_per_minute", self.search_per_minute, 1, 10000),
            ("search_max_burst", self.search_max_burst, 1, 1000),
            ("import_per_minute", self.import_per_minute, 1, 1000),
            ("import_max_burst", self.import_max_burst, 1, 100),
            ("workspace_per_minute", self.workspace_per_minute, 1, 1000),
            ("workspace_max_burst", self.workspace_max_burst, 1, 100),
            ("default_per_minute", self.default_per_minute, 1, 10000),
            ("default_max_burst", self.default_max_burst, 1, 1000),
        ];

        for (name, value, min, max) in fields {
            if value < min || value > max {
                tracing::warn!(
                    "配置字段 {} 的值 {} 超出范围 [{}-{}], 将使用默认值",
                    name,
                    value,
                    min,
                    max
                );
                modified = true;
            }
        }

        (result, !modified)
    }
}

// ============ 前端配置 ============

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FrontendConfig {
    // 超时配置
    #[serde(default = "default_30000_u64")]
    pub default_ipc_timeout_ms: u64,

    #[serde(default = "default_30000_u64")]
    pub query_execution_timeout_ms: u64,

    #[serde(default = "default_5000_u64")]
    pub query_validation_timeout_ms: u64,

    #[serde(default = "default_1000_u64")]
    pub config_save_debounce_ms: u64,

    #[serde(default = "default_5000_u64")]
    pub optimistic_update_timeout_ms: u64,

    #[serde(default = "default_50_u64")]
    pub batch_update_delay_ms: u64,

    #[serde(default = "default_1000_u64")]
    pub default_retry_delay_ms: u64,

    #[serde(default = "default_5000_u64")]
    pub max_search_retry_delay_ms: u64,

    #[serde(default = "default_300000_u64")]
    pub query_cache_stale_time_ms: u64,

    #[serde(default = "default_30000_u64")]
    pub retry_exponential_backoff_limit_ms: u64,

    // WebSocket配置
    // NOTE: 当前为纯 Tauri 桌面应用，使用 Tauri Events 进行前后端通信，
    // 无需 WebSocket。以下字段保留以兼容已有配置文件，暂不生效。
    #[serde(default = "default_ws_url")]
    pub websocket_url: String,

    #[serde(default = "default_1000_u64")]
    pub websocket_reconnect_interval_ms: u64,

    #[serde(default = "default_10_u64")]
    pub websocket_max_reconnect_attempts: u64,

    #[serde(default = "default_30000_u64")]
    pub websocket_heartbeat_interval_ms: u64,

    #[serde(default = "default_10000_u64")]
    pub websocket_connection_timeout_ms: u64,

    #[serde(default = "default_true")]
    pub websocket_auto_reconnect: bool,

    #[serde(default = "default_30000_u64")]
    pub websocket_max_reconnect_wait_ms: u64,

    // UI配置
    #[serde(default = "default_1000_usize")]
    pub log_truncate_threshold: usize,

    #[serde(default = "default_50_usize")]
    pub context_length: usize,

    #[serde(default = "default_100_usize")]
    pub event_bus_max_cache: usize,

    #[serde(default = "default_3000_u16")]
    pub vite_dev_server_port: u16,

    #[serde(default = "default_300000_u64")]
    pub react_query_stale_time_ms: u64,

    #[serde(default = "default_600000_u64")]
    pub react_query_gc_time_ms: u64,

    #[serde(default = "default_30000_u64")]
    pub max_retry_delay_ms: u64,

    // 虚拟滚动配置
    #[serde(default = "default_10_usize")]
    pub virtual_scroll_overscan: usize,

    #[serde(default = "default_60_u64")]
    pub virtual_scroll_estimated_row_height: u64,
}

fn default_30000_u64() -> u64 {
    30_000
}
fn default_5000_u64() -> u64 {
    5_000
}
fn default_1000_u64() -> u64 {
    1_000
}
fn default_10000_u64() -> u64 {
    10_000
}
fn default_300000_u64() -> u64 {
    300_000
}
fn default_600000_u64() -> u64 {
    600_000
}
fn default_50_usize() -> usize {
    50
}
fn default_50_u64() -> u64 {
    50
}
fn default_2_u64() -> u64 {
    2
}
fn default_20_u64() -> u64 {
    20
}
fn default_ws_url() -> String {
    "ws://localhost:8080/ws".to_string()
}

impl Default for FrontendConfig {
    fn default() -> Self {
        Self {
            default_ipc_timeout_ms: 30_000,
            query_execution_timeout_ms: 30_000,
            query_validation_timeout_ms: 5_000,
            config_save_debounce_ms: 1_000,
            optimistic_update_timeout_ms: 5_000,
            batch_update_delay_ms: 50,
            default_retry_delay_ms: 1_000,
            max_search_retry_delay_ms: 5_000,
            query_cache_stale_time_ms: 300_000,
            retry_exponential_backoff_limit_ms: 30_000,
            websocket_url: "ws://localhost:8080/ws".to_string(),
            websocket_reconnect_interval_ms: 1_000,
            websocket_max_reconnect_attempts: 10,
            websocket_heartbeat_interval_ms: 30_000,
            websocket_connection_timeout_ms: 10_000,
            websocket_auto_reconnect: true,
            websocket_max_reconnect_wait_ms: 30_000,
            log_truncate_threshold: 1000,
            context_length: 50,
            event_bus_max_cache: 100,
            vite_dev_server_port: 3000,
            react_query_stale_time_ms: 300_000,
            react_query_gc_time_ms: 600_000,
            max_retry_delay_ms: 30_000,
            virtual_scroll_overscan: 10,
            virtual_scroll_estimated_row_height: 60,
        }
    }
}

impl ConfigValidator for FrontendConfig {
    fn validate(&self) -> ValidationResult {
        let mut result = ValidationResult::new();

        // 验证超时配置
        let timeout_fields = [
            (
                "default_ipc_timeout_ms",
                self.default_ipc_timeout_ms,
                1000,
                300000,
            ),
            (
                "query_execution_timeout_ms",
                self.query_execution_timeout_ms,
                1000,
                600000,
            ),
            (
                "query_validation_timeout_ms",
                self.query_validation_timeout_ms,
                100,
                60000,
            ),
            (
                "config_save_debounce_ms",
                self.config_save_debounce_ms,
                0,
                10000,
            ),
            (
                "optimistic_update_timeout_ms",
                self.optimistic_update_timeout_ms,
                1000,
                60000,
            ),
            ("batch_update_delay_ms", self.batch_update_delay_ms, 0, 5000),
            (
                "default_retry_delay_ms",
                self.default_retry_delay_ms,
                100,
                60000,
            ),
            (
                "max_search_retry_delay_ms",
                self.max_search_retry_delay_ms,
                1000,
                300000,
            ),
            (
                "query_cache_stale_time_ms",
                self.query_cache_stale_time_ms,
                1000,
                3600000,
            ),
            (
                "retry_exponential_backoff_limit_ms",
                self.retry_exponential_backoff_limit_ms,
                1000,
                300000,
            ),
        ];

        for (name, value, min, max) in timeout_fields {
            if let Some(err) = validate_range(name, value, min, max) {
                result.add_error(err.field, err.message, err.code);
            }
        }

        // 验证 WebSocket URL
        if !self.websocket_url.starts_with("ws://") && !self.websocket_url.starts_with("wss://") {
            result.add_error(
                "websocket_url",
                "WebSocket URL 必须以 ws:// 或 wss:// 开头",
                "invalid_websocket_url",
            );
        }

        // 验证 WebSocket 配置
        let ws_fields = [
            (
                "websocket_reconnect_interval_ms",
                self.websocket_reconnect_interval_ms,
                100,
                60000,
            ),
            (
                "websocket_max_reconnect_attempts",
                self.websocket_max_reconnect_attempts,
                0,
                100,
            ),
            (
                "websocket_heartbeat_interval_ms",
                self.websocket_heartbeat_interval_ms,
                1000,
                300000,
            ),
            (
                "websocket_connection_timeout_ms",
                self.websocket_connection_timeout_ms,
                1000,
                300000,
            ),
            (
                "websocket_max_reconnect_wait_ms",
                self.websocket_max_reconnect_wait_ms,
                1000,
                600000,
            ),
        ];

        for (name, value, min, max) in ws_fields {
            if let Some(err) = validate_range(name, value, min, max) {
                result.add_error(err.field, err.message, err.code);
            }
        }

        // 验证 UI 配置
        if let Some(err) = validate_range(
            "log_truncate_threshold",
            self.log_truncate_threshold,
            1,
            100000,
        ) {
            result.add_error(err.field, err.message, err.code);
        }

        if let Some(err) = validate_range("context_length", self.context_length, 0, 10000) {
            result.add_error(err.field, err.message, err.code);
        }

        if let Some(err) = validate_range("event_bus_max_cache", self.event_bus_max_cache, 1, 10000)
        {
            result.add_error(err.field, err.message, err.code);
        }

        // 验证端口
        if self.vite_dev_server_port == 0 {
            result.add_error("vite_dev_server_port", "端口号不能为 0", "invalid_port");
        }

        // 验证虚拟滚动配置
        if let Some(err) = validate_range(
            "virtual_scroll_overscan",
            self.virtual_scroll_overscan,
            0,
            1000,
        ) {
            result.add_error(err.field, err.message, err.code);
        }

        if let Some(err) = validate_range(
            "virtual_scroll_estimated_row_height",
            self.virtual_scroll_estimated_row_height,
            1,
            1000,
        ) {
            result.add_error(err.field, err.message, err.code);
        }

        result
    }

    fn validate_with_defaults(&self) -> (ValidationResult, bool) {
        let result = self.validate();
        let mut modified = false;

        if self.vite_dev_server_port == 0 {
            modified = true;
        }

        if self.log_truncate_threshold == 0 || self.log_truncate_threshold > 100000 {
            modified = true;
        }

        if self.virtual_scroll_estimated_row_height == 0
            || self.virtual_scroll_estimated_row_height > 1000
        {
            modified = true;
        }

        (result, !modified)
    }
}


// Shared defaults (reused from other config domains)


