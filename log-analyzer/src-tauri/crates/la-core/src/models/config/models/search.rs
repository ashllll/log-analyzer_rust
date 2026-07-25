//! 搜索与任务管理器配置

use super::defaults::*;
use crate::models::config::validator::*;
use serde::{Deserialize, Serialize};
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

