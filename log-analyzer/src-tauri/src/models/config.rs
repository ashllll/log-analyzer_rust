//! 统一配置管理系统
//!
//! 使用 `config` crate 实现行业标准的配置管理：
//! - 多层配置：默认值 → 配置文件 → 环境变量
//! - 支持 JSON、TOML、YAML 等格式
//! - 环境变量自动解析和映射
//! - 配置验证
//!
//! # 配置优先级
//!
//! 1. 环境变量（最高优先级，覆盖所有）
//! 2. 用户配置文件（config.json）
//! 3. 默认值（最低优先级）

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

// ============ 配置错误类型 ============

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("配置加载失败: {0}")]
    LoadError(#[from] config::ConfigError),

    #[error("配置验证失败: {0}")]
    ValidationError(String),

    #[error("配置文件不存在: {0}")]
    FileNotFound(PathBuf),
}

// ============ 配置结构定义 ============

/// 文件过滤模式
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

/// 高级搜索特性配置
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AdvancedFeaturesConfig {
    #[serde(default = "default_true")]
    pub enable_filter_engine: bool,

    #[serde(default = "default_true")]
    pub enable_regex_engine: bool,

    #[serde(default = "default_true")]
    pub enable_time_partition: bool,

    #[serde(default = "default_true")]
    pub enable_autocomplete: bool,

    #[serde(default = "default_1000_usize")]
    pub regex_cache_size: usize,

    #[serde(default = "default_100_usize")]
    pub autocomplete_limit: usize,

    #[serde(default = "default_3600_u64")]
    pub time_partition_size_secs: u64,
}

fn default_1000_usize() -> usize {
    1000
}

fn default_3600_u64() -> u64 {
    3600
}

impl Default for AdvancedFeaturesConfig {
    fn default() -> Self {
        Self {
            enable_filter_engine: true,
            enable_regex_engine: true,
            enable_time_partition: true,
            enable_autocomplete: true,
            regex_cache_size: 1000,
            autocomplete_limit: 100,
            time_partition_size_secs: 3600,
        }
    }
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
        }
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

impl ArchiveProcessingConfig {
    /// 验证配置有效性
    pub fn validate(&self) -> Result<(), ConfigError> {
        // 验证嵌套深度
        if self.max_nesting_depth == 0 || self.max_nesting_depth > 30 {
            return Err(ConfigError::ValidationError(format!(
                "max_nesting_depth must be between 1 and 30, got {}",
                self.max_nesting_depth
            )));
        }

        // 验证文件大小策略限制
        if self.file_size_policy.full_extraction_limit == 0 {
            return Err(ConfigError::ValidationError(
                "full_extraction_limit must be positive".to_string(),
            ));
        }

        if self.file_size_policy.streaming_search_limit == 0 {
            return Err(ConfigError::ValidationError(
                "streaming_search_limit must be positive".to_string(),
            ));
        }

        // 验证可读性评分范围
        if self.min_readability_score < 0.0 || self.min_readability_score > 1.0 {
            return Err(ConfigError::ValidationError(format!(
                "min_readability_score must be between 0.0 and 1.0, got {}",
                self.min_readability_score
            )));
        }

        Ok(())
    }

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
fn default_1gb() -> u64 {
    1024 * 1024 * 1024
}
fn default_10gb() -> u64 {
    10 * 1024 * 1024 * 1024
}
fn default_50gb() -> u64 {
    50 * 1024 * 1024 * 1024
}
fn default_100_0_f64() -> f64 {
    100.0
}
fn default_1000000_0_f64() -> f64 {
    1_000_000.0
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
            max_file_size: 100 * 1024 * 1024,
            max_total_size: 1024 * 1024 * 1024,
            max_file_count: 1000,
            max_extraction_depth: 10,
            max_archive_total_size: 10 * 1024 * 1024 * 1024,
            max_compression_ratio: 100.0,
            max_workspace_size: 50 * 1024 * 1024 * 1024,
            exponential_backoff_threshold: 1_000_000.0,
            path_shorten_threshold: 0.8,
            checkpoint_byte_interval: 1024 * 1024 * 1024,
            temp_file_ttl_seconds: 24 * 60 * 60,
            max_resource_release_seconds: 5,
            streaming_buffer_size: 64 * 1024,
            directory_batch_size: 10,
            max_parallel_files: 4,
            gz_streaming_threshold: 10 * 1024 * 1024,
            file_copy_timeout_seconds: 300,
            copy_buffer_size: 64 * 1024,
        }
    }
}

// ============ 缓存配置 ============

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CacheConfig {
    #[serde(default = "default_1000_usize")]
    pub regex_cache_size: usize,

    #[serde(default = "default_100_usize")]
    pub autocomplete_limit: usize,

    #[serde(default = "default_100_usize")]
    pub max_cache_capacity: usize,

    #[serde(default = "default_300_u64")]
    pub cache_ttl_seconds: u64,

    #[serde(default = "default_60_u64_cache")]
    pub cache_tti_seconds: u64,

    #[serde(default = "default_10kb")]
    pub compression_threshold: u64,

    #[serde(default = "default_true")]
    pub compression_enabled: bool,

    #[serde(default = "default_1000_usize")]
    pub access_window_size: usize,

    #[serde(default = "default_5_usize")]
    pub preload_threshold: usize,

    #[serde(default = "default_0_7_f64")]
    pub min_hit_rate_threshold: f64,

    #[serde(default = "default_10_0_f64")]
    pub max_avg_access_time_ms: f64,

    #[serde(default = "default_100_0_f64")]
    pub max_avg_load_time_ms: f64,

    #[serde(default = "default_10_0_f64")]
    pub max_eviction_rate_per_min: f64,
}

fn default_300_u64() -> u64 {
    300
}
fn default_60_u64_cache() -> u64 {
    60
}
fn default_10kb() -> u64 {
    10 * 1024
}
fn default_0_7_f64() -> f64 {
    0.7
}
fn default_10_0_f64() -> f64 {
    10.0
}
// default_100_0_f64 is defined at line 436 (ArchiveConfig section)

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            regex_cache_size: 1000,
            autocomplete_limit: 100,
            max_cache_capacity: 100,
            cache_ttl_seconds: 300,
            cache_tti_seconds: 60,
            compression_threshold: 10 * 1024,
            compression_enabled: true,
            access_window_size: 1000,
            preload_threshold: 5,
            min_hit_rate_threshold: 0.7,
            max_avg_access_time_ms: 10.0,
            max_avg_load_time_ms: 100.0,
            max_eviction_rate_per_min: 10.0,
        }
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
            read_buffer_size: 8 * 1024,
            streaming_builder_buffer_size: 64 * 1024,
            buffer_size: 64 * 1024,
        }
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
fn default_ws_url() -> String {
    "ws://localhost:8080/ws".to_string()
}
fn default_10_u64() -> u64 {
    10
}
fn default_60_u64() -> u64 {
    60
}
fn default_2_u64() -> u64 {
    2
}
fn default_120_u64() -> u64 {
    120
}
fn default_20_u64() -> u64 {
    20
}
fn default_200_u64() -> u64 {
    200
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

// ============ 统一应用配置根结构 ============

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AppConfig {
    // 向后兼容字段
    #[serde(default)]
    pub keyword_groups: serde_json::Value,

    #[serde(default)]
    pub workspaces: serde_json::Value,

    #[serde(default)]
    pub advanced_features: AdvancedFeaturesConfig,

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
    pub cache: CacheConfig,

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
            advanced_features: AdvancedFeaturesConfig::default(),
            file_filter: FileFilterConfig::default(),
            server: ServerConfig::default(),
            storage: StorageConfig::default(),
            search: SearchConfig::default(),
            monitoring: MonitoringConfig::default(),
            security: SecurityConfig::default(),
            archive: ArchiveConfig::default(),
            archive_processing: ArchiveProcessingConfig::default(),
            cache: CacheConfig::default(),
            task_manager: TaskManagerConfig::default(),
            database: DatabaseConfig::default(),
            rate_limit: RateLimitConfig::default(),
            frontend: FrontendConfig::default(),
        }
    }
}

// ============ 配置加载器 ============

pub struct ConfigLoader {
    config: AppConfig,
}

impl ConfigLoader {
    /// 从文件加载配置
    ///
    /// 支持 JSON 格式配置文件，优先级：
    /// 1. 默认值
    /// 2. 配置文件
    /// 3. 环境变量
    pub fn load(config_path: Option<PathBuf>) -> Result<Self, ConfigError> {
        let mut config_builder = config::Config::builder()
            .add_source(config::File::with_name("config").required(false))
            .add_source(
                config::Environment::with_prefix("LOG_ANALYZER")
                    .prefix_separator("_")
                    .separator("__")
                    .list_separator(",")
                    .try_parsing(true),
            );

        // 如果提供了配置文件路径，添加该配置源
        if let Some(path) = config_path {
            if path.exists() {
                config_builder = config_builder.add_source(config::File::from(path));
            }
        }

        // 尝试加载配置
        let config: AppConfig = config_builder.build()?.try_deserialize()?;

        Ok(Self { config })
    }

    /// 获取配置引用
    pub fn get_config(&self) -> &AppConfig {
        &self.config
    }

    /// 获取配置可变引用
    pub fn get_config_mut(&mut self) -> &mut AppConfig {
        &mut self.config
    }

    /// 获取单个配置节
    pub fn get_archive_config(&self) -> &ArchiveConfig {
        &self.config.archive
    }

    pub fn get_archive_processing_config(&self) -> &ArchiveProcessingConfig {
        &self.config.archive_processing
    }

    pub fn get_search_config(&self) -> &SearchConfig {
        &self.config.search
    }

    pub fn get_cache_config(&self) -> &CacheConfig {
        &self.config.cache
    }

    pub fn get_task_manager_config(&self) -> &TaskManagerConfig {
        &self.config.task_manager
    }

    pub fn get_database_config(&self) -> &DatabaseConfig {
        &self.config.database
    }

    pub fn get_rate_limit_config(&self) -> &RateLimitConfig {
        &self.config.rate_limit
    }

    pub fn get_frontend_config(&self) -> &FrontendConfig {
        &self.config.frontend
    }
}

// 导出配置节
pub use crate::models::config::ConfigLoader as AppConfigLoader;
