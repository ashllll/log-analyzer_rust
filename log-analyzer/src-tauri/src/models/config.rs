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
