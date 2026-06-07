//! 配置数据模型
//!
//! 定义所有配置结构体及其验证实现。

use super::validator::*;
use serde::{Deserialize, Serialize};

// ============ 文件过滤模式 ============

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FilterMode {
    #[default]
    Whitelist,
    Blacklist,
}

/// 文件类型过滤配置
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileFilterConfig {
    #[serde(default = "default_false")]
    pub enabled: bool,

    #[serde(default = "default_true")]
    pub binary_detection_enabled: bool,

    #[serde(default)]
    pub mode: FilterMode,

    #[serde(default)]
    pub filename_patterns: Vec<String>,

    #[serde(default)]
    pub allowed_extensions: Vec<String>,

    #[serde(default)]
    pub forbidden_extensions: Vec<String>,
}

fn default_false() -> bool {
    false
}

fn default_true() -> bool {
    true
}

impl Default for FileFilterConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            binary_detection_enabled: true,
            mode: FilterMode::default(),
            filename_patterns: vec![
                "syslog".to_string(),
                "messages".to_string(),
                "system".to_string(),
                "stdout".to_string(),
                "stderr".to_string(),
                "*log*".to_string(),
                "*error*".to_string(),
                "*access*".to_string(),
                "*.20*".to_string(),
            ],
            allowed_extensions: vec!["log", "txt", "json", "xml", "csv"]
                .into_iter()
                .map(|s| s.to_string())
                .collect(),
            forbidden_extensions: vec!["exe", "bat", "sh", "dll", "so"]
                .into_iter()
                .map(|s| s.to_string())
                .collect(),
        }
    }
}

impl ConfigValidator for FileFilterConfig {
    fn validate(&self) -> ValidationResult {
        let mut result = ValidationResult::new();

        // 验证允许的扩展名
        for (i, ext) in self.allowed_extensions.iter().enumerate() {
            if let Some(err) = validate_extension(ext) {
                result.add_error(format!("allowed_extensions[{i}]"), err.message, err.code);
            }
        }

        // 验证禁止的扩展名
        for (i, ext) in self.forbidden_extensions.iter().enumerate() {
            if let Some(err) = validate_extension(ext) {
                result.add_error(format!("forbidden_extensions[{i}]"), err.message, err.code);
            }
        }

        // 验证文件名模式（如果是正则表达式）
        for (i, pattern) in self.filename_patterns.iter().enumerate() {
            // 通配符模式不需要验证
            if pattern.contains('*') || pattern.contains('?') {
                continue;
            }
            // 其他模式尝试作为正则验证
            if let Some(err) = validate_regex_pattern(pattern) {
                result.add_error(format!("filename_patterns[{i}]"), err.message, err.code);
            }
        }

        result
    }

    fn validate_with_defaults(&self) -> (ValidationResult, bool) {
        let mut result = ValidationResult::new();
        let mut has_invalid = false;

        // 过滤掉无效的扩展名
        for (i, ext) in self.allowed_extensions.iter().enumerate() {
            if validate_extension(ext).is_some() {
                result.add_error(
                    format!("allowed_extensions[{i}]"),
                    format!("扩展名 '{ext}' 无效，将被忽略"),
                    "invalid_extension".to_string(),
                );
                has_invalid = true;
            }
        }

        for (i, ext) in self.forbidden_extensions.iter().enumerate() {
            if validate_extension(ext).is_some() {
                result.add_error(
                    format!("forbidden_extensions[{i}]"),
                    format!("扩展名 '{ext}' 无效，将被忽略"),
                    "invalid_extension".to_string(),
                );
                has_invalid = true;
            }
        }

        (result, !has_invalid)
    }
}

fn default_1000_usize() -> usize {
    1000
}

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

// ============ 搜索配置 ============

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SearchConfig {
    #[serde(default = "default_1000_usize")]
    pub max_results: usize,

    #[serde(default = "default_10_u64")]
    pub timeout_seconds: u64,

    #[serde(default = "default_10_usize")]
    pub max_concurrent_searches: usize,

    #[serde(default = "default_true")]
    pub fuzzy_search_enabled: bool,

    #[serde(default = "default_false")]
    pub case_sensitive: bool,

    #[serde(default = "default_true")]
    pub regex_enabled: bool,

    #[serde(default = "default_1000_usize")]
    pub regex_cache_size: usize,
}

fn default_10_u64() -> u64 {
    10
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            max_results: 1000,
            timeout_seconds: 10,
            max_concurrent_searches: 10,
            fuzzy_search_enabled: true,
            case_sensitive: false,
            regex_enabled: true,
            regex_cache_size: 1000,
        }
    }
}

impl ConfigValidator for SearchConfig {
    fn validate(&self) -> ValidationResult {
        let mut result = ValidationResult::new();

        // 验证最大结果数
        if let Some(err) = validate_range("max_results", self.max_results, 1, 1_000_000) {
            result.add_error(err.field, err.message, err.code);
        }

        // 验证超时时间
        if let Some(err) = validate_range("timeout_seconds", self.timeout_seconds, 1, 3600) {
            result.add_error(err.field, err.message, err.code);
        }

        // 验证并发搜索数
        if let Some(err) = validate_range(
            "max_concurrent_searches",
            self.max_concurrent_searches,
            1,
            100,
        ) {
            result.add_error(err.field, err.message, err.code);
        }

        if let Some(err) = validate_range("regex_cache_size", self.regex_cache_size, 1, 100000) {
            result.add_error(err.field, err.message, err.code);
        }

        result
    }

    fn validate_with_defaults(&self) -> (ValidationResult, bool) {
        let result = self.validate();
        let mut modified = false;

        if self.max_results == 0 || self.max_results > 1_000_000 {
            modified = true;
        }

        if self.timeout_seconds == 0 || self.timeout_seconds > 3600 {
            modified = true;
        }

        if self.max_concurrent_searches == 0 || self.max_concurrent_searches > 100 {
            modified = true;
        }

        if self.regex_cache_size == 0 || self.regex_cache_size > 100000 {
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

// ============ 压缩解压配置 ============

/// 文件大小处理策略
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FileSizeStrategy {
    /// 完全解压后搜索
    #[default]
    FullExtraction,
    /// 流式搜索（适用于大文件）
    StreamingSearch,
    /// 跳过大文件
    Skip,
}

/// 嵌套压缩包策略配置
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NestedArchivePolicy {
    /// 文件数量阈值（超过此数量则限制嵌套深度）
    #[serde(default = "default_5000_usize")]
    pub file_count_threshold: usize,

    /// 总大小阈值（字节）
    #[serde(default = "default_20gb")]
    pub total_size_threshold: u64,

    /// 压缩比阈值（检测压缩炸弹）
    #[serde(default = "default_100_0_f64")]
    pub compression_ratio_threshold: f64,

    /// 指数退避阈值（ratio^depth）
    #[serde(default = "default_1000000_0_f64")]
    pub exponential_backoff_threshold: f64,

    /// 启用压缩炸弹检测
    #[serde(default = "default_true")]
    pub enable_zip_bomb_detection: bool,
}

fn default_5000_usize() -> usize {
    5000
}

fn default_20gb() -> u64 {
    20 * 1024 * 1024 * 1024
}

fn default_100_0_f64() -> f64 {
    100.0
}

fn default_1000000_0_f64() -> f64 {
    1_000_000.0
}

impl Default for NestedArchivePolicy {
    fn default() -> Self {
        Self {
            file_count_threshold: 5000,
            total_size_threshold: 20 * 1024 * 1024 * 1024,
            compression_ratio_threshold: 100.0,
            exponential_backoff_threshold: 1_000_000.0,
            enable_zip_bomb_detection: true,
        }
    }
}

impl ConfigValidator for NestedArchivePolicy {
    fn validate(&self) -> ValidationResult {
        let mut result = ValidationResult::new();

        if let Some(err) =
            validate_range("file_count_threshold", self.file_count_threshold, 1, 100000)
        {
            result.add_error(err.field, err.message, err.code);
        }

        if let Some(err) = validate_range(
            "total_size_threshold",
            self.total_size_threshold,
            1,
            u64::MAX,
        ) {
            result.add_error(err.field, err.message, err.code);
        }

        if let Some(err) = validate_range(
            "compression_ratio_threshold",
            self.compression_ratio_threshold,
            1.0,
            10000.0,
        ) {
            result.add_error(err.field, err.message, err.code);
        }

        if let Some(err) = validate_range(
            "exponential_backoff_threshold",
            self.exponential_backoff_threshold,
            1.0,
            1e12,
        ) {
            result.add_error(err.field, err.message, err.code);
        }

        result
    }

    fn validate_with_defaults(&self) -> (ValidationResult, bool) {
        (self.validate(), true)
    }
}

/// 文件大小策略配置
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileSizePolicy {
    /// 处理策略
    #[serde(default)]
    pub strategy: FileSizeStrategy,

    /// 完全解压的文件大小限制（字节）
    #[serde(default = "default_500mb")]
    pub full_extraction_limit: u64,

    /// 流式搜索的文件大小限制（字节）
    #[serde(default = "default_2gb")]
    pub streaming_search_limit: u64,

    /// 超过限制时的行为（warn/error/silent）
    #[serde(default = "default_warn_action")]
    pub oversized_file_action: String,
}

fn default_500mb() -> u64 {
    500 * 1024 * 1024
}

fn default_2gb() -> u64 {
    2 * 1024 * 1024 * 1024
}

fn default_warn_action() -> String {
    "warn".to_string()
}

impl Default for FileSizePolicy {
    fn default() -> Self {
        Self {
            strategy: FileSizeStrategy::default(),
            full_extraction_limit: 500 * 1024 * 1024,
            streaming_search_limit: 2 * 1024 * 1024 * 1024,
            oversized_file_action: "warn".to_string(),
        }
    }
}

impl ConfigValidator for FileSizePolicy {
    fn validate(&self) -> ValidationResult {
        let mut result = ValidationResult::new();

        // 验证文件大小限制
        if self.full_extraction_limit == 0 {
            result.add_error(
                "full_extraction_limit",
                "完全解压限制必须大于 0",
                "invalid_limit",
            );
        }

        if self.streaming_search_limit == 0 {
            result.add_error(
                "streaming_search_limit",
                "流式搜索限制必须大于 0",
                "invalid_limit",
            );
        }

        // 验证流式搜索限制应该大于完全解压限制
        if self.streaming_search_limit <= self.full_extraction_limit {
            result.add_error(
                "streaming_search_limit",
                "流式搜索限制必须大于完全解压限制",
                "invalid_limit_order",
            );
        }

        // 验证动作类型
        let valid_actions = ["warn", "error", "skip", "silent"];
        if !valid_actions.contains(&self.oversized_file_action.as_str()) {
            result.add_error(
                "oversized_file_action",
                format!(
                    "无效的动作类型: {}, 必须是以下之一: {:?}",
                    self.oversized_file_action, valid_actions
                ),
                "invalid_action",
            );
        }

        result
    }

    fn validate_with_defaults(&self) -> (ValidationResult, bool) {
        let result = self.validate();
        let mut modified = false;

        if self.full_extraction_limit == 0 {
            modified = true;
        }

        if self.streaming_search_limit == 0
            || self.streaming_search_limit <= self.full_extraction_limit
        {
            modified = true;
        }

        (result, !modified)
    }
}

/// 扩展的压缩包处理配置
///
/// 这个配置扩展了基础的 ArchiveConfig，增加了：
/// - 大文件处理策略
/// - 嵌套压缩包智能处理
/// - 流式搜索支持
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ArchiveProcessingConfig {
    /// 最大文件大小（字节），0 表示无限制
    #[serde(default = "default_10gb")]
    pub max_file_size: u64,

    /// 最大总大小（字节），0 表示无限制
    #[serde(default = "default_zero_u64")]
    pub max_total_size: u64,

    /// 最大文件数量，0 表示无限制
    #[serde(default = "default_zero_usize")]
    pub max_file_count: usize,

    /// 最大嵌套深度
    #[serde(default = "default_15_usize")]
    pub max_nesting_depth: usize,

    /// 嵌套压缩包策略
    #[serde(default)]
    pub nested_archive_policy: NestedArchivePolicy,

    /// 文件大小策略
    #[serde(default)]
    pub file_size_policy: FileSizePolicy,

    /// 启用智能文件类型检测
    #[serde(default = "default_true")]
    pub enable_intelligent_file_filter: bool,

    /// 内容采样大小（字节）
    #[serde(default = "default_10kb_v2")]
    pub content_sample_size: u64,

    /// 最小文本可读性评分（0.0-1.0）
    #[serde(default = "default_0_3_f64")]
    pub min_readability_score: f64,

    /// 启用进度报告
    #[serde(default = "default_true")]
    pub enable_progress_reporting: bool,

    /// 进度报告间隔（毫秒）
    #[serde(default = "default_500_u64")]
    pub progress_report_interval_ms: u64,
}

fn default_zero_u64() -> u64 {
    0
}

fn default_zero_usize() -> usize {
    0
}

fn default_15_usize() -> usize {
    15
}

fn default_10gb() -> u64 {
    10 * 1024 * 1024 * 1024
}

fn default_10kb_v2() -> u64 {
    10 * 1024
}

fn default_0_3_f64() -> f64 {
    0.3
}

fn default_500_u64() -> u64 {
    500
}

impl Default for ArchiveProcessingConfig {
    fn default() -> Self {
        Self {
            max_file_size: 10 * 1024 * 1024 * 1024, // 10GB
            max_total_size: 0,                      // 无限制
            max_file_count: 0,                      // 无限制
            max_nesting_depth: 15,
            nested_archive_policy: NestedArchivePolicy::default(),
            file_size_policy: FileSizePolicy::default(),
            enable_intelligent_file_filter: true,
            content_sample_size: 10 * 1024,
            min_readability_score: 0.3,
            enable_progress_reporting: true,
            progress_report_interval_ms: 500,
        }
    }
}

impl ConfigValidator for ArchiveProcessingConfig {
    fn validate(&self) -> ValidationResult {
        let mut result = ValidationResult::new();

        // 验证嵌套深度
        if let Some(err) = validate_range("max_nesting_depth", self.max_nesting_depth, 1, 50) {
            result.add_error(err.field, err.message, err.code);
        }

        // 验证内容采样大小
        if let Some(err) = validate_range(
            "content_sample_size",
            self.content_sample_size,
            1,
            10 * 1024 * 1024,
        ) {
            result.add_error(err.field, err.message, err.code);
        }

        // 验证可读性评分范围
        if let Some(err) = validate_range(
            "min_readability_score",
            self.min_readability_score,
            0.0,
            1.0,
        ) {
            result.add_error(err.field, err.message, err.code);
        }

        // 验证进度报告间隔
        if let Some(err) = validate_range(
            "progress_report_interval_ms",
            self.progress_report_interval_ms,
            100,
            60000,
        ) {
            result.add_error(err.field, err.message, err.code);
        }

        // 嵌套验证
        result.merge(self.nested_archive_policy.validate());
        result.merge(self.file_size_policy.validate());

        result
    }

    fn validate_with_defaults(&self) -> (ValidationResult, bool) {
        let mut result = self.validate();
        let mut modified = false;

        if self.max_nesting_depth == 0 || self.max_nesting_depth > 50 {
            modified = true;
        }

        if self.min_readability_score < 0.0 || self.min_readability_score > 1.0 {
            modified = true;
        }

        if self.progress_report_interval_ms < 100 || self.progress_report_interval_ms > 60000 {
            modified = true;
        }

        let (policy_result, policy_valid) = self.file_size_policy.validate_with_defaults();
        result.merge(policy_result);
        if !policy_valid {
            modified = true;
        }

        (result, !modified)
    }
}

impl ArchiveProcessingConfig {
    /// 根据文件大小决定处理策略
    pub fn decide_file_strategy(&self, file_size: u64) -> FileSizeStrategy {
        let config = &self.file_size_policy;

        // 检查是否超过最大文件大小限制
        if self.max_file_size > 0 && file_size > self.max_file_size {
            match config.oversized_file_action.as_str() {
                "skip" => return FileSizeStrategy::Skip,
                "error" => return FileSizeStrategy::Skip,
                _ => return FileSizeStrategy::StreamingSearch,
            }
        }

        // 根据文件大小决定策略
        if file_size <= config.full_extraction_limit {
            FileSizeStrategy::FullExtraction
        } else if file_size <= config.streaming_search_limit {
            FileSizeStrategy::StreamingSearch
        } else {
            match config.strategy {
                FileSizeStrategy::Skip => FileSizeStrategy::Skip,
                _ => FileSizeStrategy::StreamingSearch,
            }
        }
    }

    /// 计算动态嵌套深度限制
    pub fn calculate_dynamic_depth_limit(
        &self,
        current_file_count: usize,
        current_total_size: u64,
    ) -> usize {
        let policy = &self.nested_archive_policy;

        let mut depth_limit = self.max_nesting_depth;

        // 根据文件数量调整
        if current_file_count > policy.file_count_threshold {
            let reduction = ((current_file_count - policy.file_count_threshold) / 1000).min(5);
            depth_limit = depth_limit.saturating_sub(reduction);
        }

        // 根据总大小调整
        if current_total_size > policy.total_size_threshold {
            let reduction = ((current_total_size - policy.total_size_threshold)
                / (1024 * 1024 * 1024))
                .min(3) as usize;
            depth_limit = depth_limit.saturating_sub(reduction);
        }

        depth_limit.max(1) // 至少允许1层
    }

    /// 检查是否为潜在的压缩炸弹
    pub fn is_potential_zip_bomb(&self, compression_ratio: f64, depth: usize) -> bool {
        if !self.nested_archive_policy.enable_zip_bomb_detection {
            return false;
        }

        let policy = &self.nested_archive_policy;

        // 检查压缩比
        if compression_ratio > policy.compression_ratio_threshold {
            return true;
        }

        // 检查指数退避阈值
        let exponential_factor = compression_ratio.powi(depth as i32);
        if exponential_factor > policy.exponential_backoff_threshold {
            return true;
        }

        false
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ArchiveConfig {
    // 安全限制
    #[serde(default = "default_100mb")]
    pub max_file_size: u64,

    #[serde(default = "default_1gb")]
    pub max_total_size: u64,

    #[serde(default = "default_1000_usize")]
    pub max_file_count: usize,

    #[serde(default = "default_10_usize")]
    pub max_extraction_depth: usize,

    #[serde(default = "default_10gb")]
    pub max_archive_total_size: u64,

    #[serde(default = "default_100_0_f64")]
    pub max_compression_ratio: f64,

    #[serde(default = "default_50gb")]
    pub max_workspace_size: u64,

    #[serde(default = "default_1000000_0_f64")]
    pub exponential_backoff_threshold: f64,

    #[serde(default = "default_0_8_f64")]
    pub path_shorten_threshold: f64,

    // 内部配置
    #[serde(default = "default_1gb")]
    pub checkpoint_byte_interval: u64,

    #[serde(default = "default_86400_u64")]
    pub temp_file_ttl_seconds: u64,

    #[serde(default = "default_5_u64")]
    pub max_resource_release_seconds: u64,

    #[serde(default = "default_64kb")]
    pub streaming_buffer_size: u64,

    #[serde(default = "default_10_usize")]
    pub directory_batch_size: usize,

    #[serde(default = "default_4_usize")]
    pub max_parallel_files: usize,

    #[serde(default = "default_10mb")]
    pub gz_streaming_threshold: u64,

    #[serde(default = "default_300_u64")]
    pub file_copy_timeout_seconds: u64,

    #[serde(default = "default_64kb")]
    pub copy_buffer_size: u64,
}

// 辅助默认值函数
fn default_100mb() -> u64 {
    100 * 1024 * 1024
}

fn default_300_u64() -> u64 {
    300
}
fn default_1gb() -> u64 {
    1024 * 1024 * 1024
}
fn default_50gb() -> u64 {
    50 * 1024 * 1024 * 1024
}
fn default_0_8_f64() -> f64 {
    0.8
}
fn default_86400_u64() -> u64 {
    24 * 60 * 60
}
fn default_64kb() -> u64 {
    64 * 1024
}
fn default_10mb() -> u64 {
    10 * 1024 * 1024
}
fn default_4_usize() -> usize {
    4
}
fn default_5_u64() -> u64 {
    5
}

impl Default for ArchiveConfig {
    fn default() -> Self {
        Self {
            max_file_size: 100 * 1024 * 1024 * 1024, // 100GB (统一: 支持大文件)
            max_total_size: 500 * 1024 * 1024 * 1024, // 500GB (统一: 支持大文件)
            max_file_count: 1000,
            max_extraction_depth: 10,
            max_archive_total_size: 500 * 1024 * 1024 * 1024, // 500GB (统一: 支持大文件)
            max_compression_ratio: 100.0,
            max_workspace_size: 500 * 1024 * 1024 * 1024, // 500GB (统一: 支持大文件)
            exponential_backoff_threshold: 1_000_000.0,
            path_shorten_threshold: 0.8,
            checkpoint_byte_interval: 1024 * 1024 * 1024,
            temp_file_ttl_seconds: 24 * 60 * 60,
            max_resource_release_seconds: 5,
            streaming_buffer_size: 1024 * 1024, // 1MB (优化: 从 64KB 增大)
            directory_batch_size: 10,
            max_parallel_files: 4,
            gz_streaming_threshold: 10 * 1024 * 1024,
            file_copy_timeout_seconds: 300,
            copy_buffer_size: 1024 * 1024, // 1MB (优化: 从 64KB 增大)
        }
    }
}

impl ConfigValidator for ArchiveConfig {
    fn validate(&self) -> ValidationResult {
        let mut result = ValidationResult::new();

        // 验证提取深度
        if let Some(err) = validate_range("max_extraction_depth", self.max_extraction_depth, 1, 50)
        {
            result.add_error(err.field, err.message, err.code);
        }

        // 验证文件数量
        if let Some(err) = validate_range("max_file_count", self.max_file_count, 1, 100000) {
            result.add_error(err.field, err.message, err.code);
        }

        // 验证压缩比
        if let Some(err) = validate_range(
            "max_compression_ratio",
            self.max_compression_ratio,
            1.0,
            10000.0,
        ) {
            result.add_error(err.field, err.message, err.code);
        }

        // 验证路径缩短阈值
        if let Some(err) = validate_range(
            "path_shorten_threshold",
            self.path_shorten_threshold,
            0.0,
            1.0,
        ) {
            result.add_error(err.field, err.message, err.code);
        }

        // 验证超时时间
        if let Some(err) = validate_range(
            "temp_file_ttl_seconds",
            self.temp_file_ttl_seconds,
            60,
            86400 * 30,
        ) {
            result.add_error(err.field, err.message, err.code);
        }

        if let Some(err) = validate_range(
            "max_resource_release_seconds",
            self.max_resource_release_seconds,
            1,
            3600,
        ) {
            result.add_error(err.field, err.message, err.code);
        }

        if let Some(err) = validate_range(
            "file_copy_timeout_seconds",
            self.file_copy_timeout_seconds,
            1,
            3600 * 24,
        ) {
            result.add_error(err.field, err.message, err.code);
        }

        // 验证缓冲区大小
        if let Some(err) = validate_range(
            "streaming_buffer_size",
            self.streaming_buffer_size,
            1024,
            100 * 1024 * 1024,
        ) {
            result.add_error(err.field, err.message, err.code);
        }

        if let Some(err) = validate_range(
            "copy_buffer_size",
            self.copy_buffer_size,
            1024,
            100 * 1024 * 1024,
        ) {
            result.add_error(err.field, err.message, err.code);
        }

        // 验证并发数
        if let Some(err) = validate_range("max_parallel_files", self.max_parallel_files, 1, 100) {
            result.add_error(err.field, err.message, err.code);
        }

        if let Some(err) =
            validate_range("directory_batch_size", self.directory_batch_size, 1, 10000)
        {
            result.add_error(err.field, err.message, err.code);
        }

        result
    }

    fn validate_with_defaults(&self) -> (ValidationResult, bool) {
        let result = self.validate();
        let mut modified = false;

        if self.max_extraction_depth == 0 || self.max_extraction_depth > 50 {
            modified = true;
        }

        if self.max_file_count == 0 || self.max_file_count > 100000 {
            modified = true;
        }

        if self.max_compression_ratio < 1.0 || self.max_compression_ratio > 10000.0 {
            modified = true;
        }

        (result, !modified)
    }
}

// ============ 任务管理器配置 ============

/// 任务管理器配置（可外部化）
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct TaskManagerConfig {
    /// 完成任务的保留时间（秒）
    #[serde(default = "default_300_u64_task")]
    pub completed_task_ttl: u64,

    /// 失败任务的保留时间（秒）
    #[serde(default = "default_1800_u64_task")]
    pub failed_task_ttl: u64,

    /// 清理检查间隔（秒）
    #[serde(default = "default_60_u64_task")]
    pub cleanup_interval: u64,

    /// 操作超时时间（秒）
    #[serde(default = "default_30_u64_task")]
    pub operation_timeout: u64,

    /// 最大并发任务数
    #[serde(default = "default_10_usize_task")]
    pub max_concurrent_tasks: usize,
}

fn default_300_u64_task() -> u64 {
    300 // 5 分钟
}

fn default_1800_u64_task() -> u64 {
    1800 // 30 分钟
}

fn default_60_u64_task() -> u64 {
    60 // 1 分钟
}

fn default_30_u64_task() -> u64 {
    30 // 30 秒
}

fn default_10_usize_task() -> usize {
    10
}

impl Default for TaskManagerConfig {
    fn default() -> Self {
        Self {
            completed_task_ttl: 300,
            failed_task_ttl: 1800,
            cleanup_interval: 60,
            operation_timeout: 30,
            max_concurrent_tasks: 10,
        }
    }
}

impl ConfigValidator for TaskManagerConfig {
    fn validate(&self) -> ValidationResult {
        let mut result = ValidationResult::new();

        // 验证 TTL
        if let Some(err) =
            validate_range("completed_task_ttl", self.completed_task_ttl, 1, 86400 * 7)
        {
            result.add_error(err.field, err.message, err.code);
        }

        if let Some(err) = validate_range("failed_task_ttl", self.failed_task_ttl, 1, 86400 * 30) {
            result.add_error(err.field, err.message, err.code);
        }

        // 验证清理间隔
        if let Some(err) = validate_range("cleanup_interval", self.cleanup_interval, 1, 3600) {
            result.add_error(err.field, err.message, err.code);
        }

        // 验证超时时间
        if let Some(err) = validate_range("operation_timeout", self.operation_timeout, 1, 3600) {
            result.add_error(err.field, err.message, err.code);
        }

        // 验证并发数
        if let Some(err) =
            validate_range("max_concurrent_tasks", self.max_concurrent_tasks, 1, 1000)
        {
            result.add_error(err.field, err.message, err.code);
        }

        result
    }

    fn validate_with_defaults(&self) -> (ValidationResult, bool) {
        let result = self.validate();
        let mut modified = false;

        if self.completed_task_ttl == 0 || self.completed_task_ttl > 86400 * 7 {
            modified = true;
        }

        if self.failed_task_ttl == 0 || self.failed_task_ttl > 86400 * 30 {
            modified = true;
        }

        if self.max_concurrent_tasks == 0 || self.max_concurrent_tasks > 1000 {
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

// ============ 统一应用配置根结构 ============

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AppConfig {
    // 向后兼容字段
    #[serde(default)]
    pub keyword_groups: serde_json::Value,

    #[serde(default)]
    pub workspaces: serde_json::Value,

    #[serde(default)]
    pub file_filter: FileFilterConfig,

    // 系统配置
    #[serde(default)]
    pub server: ServerConfig,

    #[serde(default)]
    pub storage: StorageConfig,

    #[serde(default)]
    pub search: SearchConfig,

    #[serde(default)]
    pub monitoring: MonitoringConfig,

    #[serde(default)]
    pub security: SecurityConfig,

    #[serde(default)]
    pub archive: ArchiveConfig,

    #[serde(default)]
    pub archive_processing: ArchiveProcessingConfig,

    #[serde(default)]
    pub task_manager: TaskManagerConfig,

    #[serde(default)]
    pub database: DatabaseConfig,

    #[serde(default)]
    pub rate_limit: RateLimitConfig,

    #[serde(default)]
    pub frontend: FrontendConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            keyword_groups: serde_json::json!([]),
            workspaces: serde_json::json!([]),
            file_filter: FileFilterConfig::default(),
            server: ServerConfig::default(),
            storage: StorageConfig::default(),
            search: SearchConfig::default(),
            monitoring: MonitoringConfig::default(),
            security: SecurityConfig::default(),
            archive: ArchiveConfig::default(),
            archive_processing: ArchiveProcessingConfig::default(),
            task_manager: TaskManagerConfig::default(),
            database: DatabaseConfig::default(),
            rate_limit: RateLimitConfig::default(),
            frontend: FrontendConfig::default(),
        }
    }
}

impl ConfigValidator for AppConfig {
    fn validate(&self) -> ValidationResult {
        let mut result = ValidationResult::new();

        // 验证所有子配置
        result.merge(self.file_filter.validate());
        result.merge(self.server.validate());
        result.merge(self.storage.validate());
        result.merge(self.search.validate());
        result.merge(self.monitoring.validate());
        result.merge(self.security.validate());
        result.merge(self.archive.validate());
        result.merge(self.archive_processing.validate());
        result.merge(self.task_manager.validate());
        result.merge(self.database.validate());
        result.merge(self.rate_limit.validate());
        result.merge(self.frontend.validate());

        result
    }

    fn validate_with_defaults(&self) -> (ValidationResult, bool) {
        let mut result = ValidationResult::new();
        let mut all_valid = true;

        let validations = [
            ("file_filter", self.file_filter.validate_with_defaults()),
            ("server", self.server.validate_with_defaults()),
            ("storage", self.storage.validate_with_defaults()),
            ("search", self.search.validate_with_defaults()),
            ("monitoring", self.monitoring.validate_with_defaults()),
            ("security", self.security.validate_with_defaults()),
            ("archive", self.archive.validate_with_defaults()),
            (
                "archive_processing",
                self.archive_processing.validate_with_defaults(),
            ),
            ("task_manager", self.task_manager.validate_with_defaults()),
            ("database", self.database.validate_with_defaults()),
            ("rate_limit", self.rate_limit.validate_with_defaults()),
            ("frontend", self.frontend.validate_with_defaults()),
        ];

        for (name, (sub_result, is_valid)) in validations {
            result.merge(sub_result);
            if !is_valid {
                all_valid = false;
                tracing::warn!("配置节 '{}' 包含无效值，将使用默认值", name);
            }
        }

        (result, all_valid)
    }
}
