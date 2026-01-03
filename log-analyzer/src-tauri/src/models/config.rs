//! 配置相关数据结构
//!
//! 本模块定义了应用配置等核心配置结构。

use serde::{Deserialize, Serialize};

/// 应用配置
///
/// 存储关键词组和工作区等全局配置信息。
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppConfig {
    /// 关键词分组配置
    pub keyword_groups: serde_json::Value,
    /// 工作区配置
    pub workspaces: serde_json::Value,
    /// 高级搜索特性配置
    #[serde(default)]
    pub advanced_features: AdvancedFeaturesConfig,
    /// 文件类型过滤配置
    #[serde(default)]
    pub file_filter: FileFilterConfig,
}

/// 文件类型过滤配置（三层检测策略）
///
/// 防御性设计：
/// - 默认禁用第2层智能过滤（enabled = false）
/// - 默认启用第1层二进制检测（binary_detection_enabled = true）
/// - 任何配置错误都会降级到默认行为（允许所有文件）
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileFilterConfig {
    /// 是否启用第2层智能过滤（第1层二进制检测始终启用）
    #[serde(default = "default_file_filter_disabled")]
    pub enabled: bool,

    /// 第1层：二进制文件检测（默认启用）
    #[serde(default = "default_binary_detection_enabled")]
    pub binary_detection_enabled: bool,

    /// 第2层：过滤模式（whitelist 或 blacklist）
    #[serde(default)]
    pub mode: FilterMode,

    /// 文件名 Glob 模式列表（支持无后缀日志）
    #[serde(default)]
    pub filename_patterns: Vec<String>,

    /// 扩展名白名单
    #[serde(default)]
    pub allowed_extensions: Vec<String>,

    /// 扩展名黑名单
    #[serde(default)]
    pub forbidden_extensions: Vec<String>,
}

/// 文件过滤模式
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FilterMode {
    Whitelist,
    Blacklist,
}

impl Default for FilterMode {
    fn default() -> Self {
        FilterMode::Whitelist
    }
}

fn default_file_filter_disabled() -> bool {
    false  // 默认禁用第2层过滤（向后兼容）
}

fn default_binary_detection_enabled() -> bool {
    true  // 默认启用第1层二进制检测
}

impl Default for FileFilterConfig {
    fn default() -> Self {
        Self {
            enabled: default_file_filter_disabled(),
            binary_detection_enabled: default_binary_detection_enabled(),
            mode: FilterMode::default(),
            filename_patterns: vec![
                // 系统日志（无后缀）
                "syslog".to_string(),
                "messages".to_string(),
                "system".to_string(),
                // 容器/应用日志（无后缀）
                "stdout".to_string(),
                "stderr".to_string(),
                // 通用日志模式
                "*log*".to_string(),
                "*error*".to_string(),
                "*access*".to_string(),
                // 带日期的日志
                "*.20*".to_string(),
            ],
            allowed_extensions: vec![
                "log".to_string(),
                "txt".to_string(),
                "json".to_string(),
                "xml".to_string(),
                "csv".to_string(),
            ],
            forbidden_extensions: vec![
                "exe".to_string(),
                "bat".to_string(),
                "sh".to_string(),
                "dll".to_string(),
                "so".to_string(),
            ],
        }
    }
}

/// 高级搜索特性配置
///
/// 控制各种高级搜索特性的启用/禁用状态和参数设置。
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct AdvancedFeaturesConfig {
    /// 是否启用位图索引过滤器（RoaringBitmap）
    #[serde(default = "default_enabled")]
    pub enable_filter_engine: bool,

    /// 是否启用正则表达式搜索引擎（LRU缓存）
    #[serde(default = "default_enabled")]
    pub enable_regex_engine: bool,

    /// 是否启用时间分区索引（时序优化）
    #[serde(default = "default_enabled")]
    pub enable_time_partition: bool,

    /// 是否启用自动补全引擎（Trie树）
    #[serde(default = "default_enabled")]
    pub enable_autocomplete: bool,

    /// 正则表达式缓存大小（默认1000）
    #[serde(default = "default_regex_cache_size")]
    pub regex_cache_size: usize,

    /// 自动补全建议数量（默认100）
    #[serde(default = "default_autocomplete_limit")]
    pub autocomplete_limit: usize,

    /// 时间分区大小（秒，默认3600 = 1小时）
    #[serde(default = "default_partition_size")]
    pub time_partition_size_secs: u64,
}

fn default_enabled() -> bool {
    true
}

fn default_regex_cache_size() -> usize {
    1000
}

fn default_autocomplete_limit() -> usize {
    100
}

fn default_partition_size() -> u64 {
    3600
}
