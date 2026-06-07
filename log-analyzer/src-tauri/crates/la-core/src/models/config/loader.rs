//! 配置加载器
//!
//! 支持多层配置加载：默认值 -> 配置文件 -> 环境变量。

use super::models::*;
use super::validator::{ConfigError, ConfigValidator, ValidationResult};
use std::path::PathBuf;

// ============ 配置加载器 ============

pub struct ConfigLoader {
    config: AppConfig,
    validation_result: Option<ValidationResult>,
}

impl ConfigLoader {
    /// 从文件加载配置
    ///
    /// 支持 JSON 格式配置文件，优先级：
    /// 1. 默认值
    /// 2. 配置文件
    /// 3. 环境变量
    ///
    /// 加载时会自动验证配置，无效配置会使用默认值
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
            } else {
                return Err(ConfigError::FileNotFound(
                    path.to_string_lossy().to_string(),
                ));
            }
        }

        // 尝试加载配置
        let config: AppConfig = config_builder
            .build()
            .map_err(|e| ConfigError::LoadError(e.to_string()))?
            .try_deserialize()
            .map_err(|e| ConfigError::LoadError(e.to_string()))?;

        // 验证配置
        let validation_result = config.validate();

        if !validation_result.is_valid {
            tracing::warn!(
                "配置验证失败: {}",
                validation_result
                    .errors
                    .iter()
                    .map(|e| format!("{}: {}", e.field, e.message))
                    .collect::<Vec<_>>()
                    .join("; ")
            );
        }

        Ok(Self {
            config,
            validation_result: Some(validation_result),
        })
    }

    /// 获取配置引用
    pub fn get_config(&self) -> &AppConfig {
        &self.config
    }

    /// 获取配置可变引用
    pub fn get_config_mut(&mut self) -> &mut AppConfig {
        &mut self.config
    }

    /// 获取验证结果
    pub fn get_validation_result(&self) -> Option<&ValidationResult> {
        self.validation_result.as_ref()
    }

    /// 验证配置是否有效
    pub fn is_valid(&self) -> bool {
        self.validation_result
            .as_ref()
            .map(|r| r.is_valid)
            .unwrap_or(true)
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
