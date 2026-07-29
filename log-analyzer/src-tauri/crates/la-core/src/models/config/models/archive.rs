//! 归档与文件过滤配置

use super::defaults::*;
use crate::models::config::validator::*;
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
fn default_10mb() -> u64 {
    10 * 1024 * 1024
}
fn default_4_usize() -> usize {
    4
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

// Shared defaults (reused from other config domains)
fn default_10_usize() -> usize {
    10
}
