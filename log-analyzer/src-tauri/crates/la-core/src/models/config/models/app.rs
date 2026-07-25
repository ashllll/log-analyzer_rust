//! 统一应用配置根结构

use super::archive::*;
use super::search::*;
use super::network::*;
use crate::models::config::validator::*;
use serde::{Deserialize, Serialize};

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
