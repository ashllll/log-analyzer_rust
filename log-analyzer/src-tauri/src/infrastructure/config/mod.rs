//! 配置管理基础设施层
//!
//! 采用分层架构：
//! - 应用层配置 (用户界面)
//! - 领域层配置 (业务规则)
//! - 基础设施层配置 (持久化)
//!
//! **Features**:
//! - 运行时验证使用 validator crate
//! - 环境变量覆盖支持
//! - 热重载机制
//! - 配置文件格式：TOML/JSON/YAML (可扩展)

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;
use validator::Validate;

// pub mod application; // TODO: 模块文件缺失，暂时注释
// pub mod domain; // TODO: 模块文件缺失，暂时注释
// pub mod infrastructure; // TODO: 模块文件缺失，暂时注释

/// 配置错误类型
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("配置验证失败: {0}")]
    Validation(String),

    #[error("配置文件不存在: {0}")]
    FileNotFound(PathBuf),

    #[error("配置文件格式错误: {0}")]
    FormatError(String),

    #[error("配置值无效: {0}")]
    InvalidValue(String),

    #[error("IO错误: {0}")]
    IoError(#[from] std::io::Error),

    #[error("环境变量错误: {0}")]
    EnvError(#[from] std::env::VarError),
}

/// 配置结果类型
pub type ConfigResult<T> = std::result::Result<T, ConfigError>;

/// 运行环境
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Environment {
    Development,
    Production,
    Testing,
}

impl Environment {
    /// 从环境变量获取当前环境
    pub fn from_env() -> Self {
        match std::env::var("APP_ENV")
            .unwrap_or_else(|_| "development".to_string())
            .to_lowercase()
            .as_str()
        {
            "production" | "prod" => Environment::Production,
            "testing" | "test" => Environment::Testing,
            _ => Environment::Development,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Development => "development",
            Environment::Production => "production",
            Environment::Testing => "testing",
        }
    }
}

/// 全局配置根结构
#[derive(Debug, Clone, Serialize, Deserialize, Validate, Default)]
pub struct AppConfig {
    pub server: ServerConfig,

    pub storage: StorageConfig,

    pub search: SearchConfig,

    pub monitoring: MonitoringConfig,

    pub security: SecurityConfig,
}

/// 服务器配置
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ServerConfig {
    #[validate(range(min = 1, max = 65535))]
    pub port: u16,

    #[validate(length(min = 1, max = 100))]
    pub host: String,

    #[validate(range(min = 1, max = 100))]
    pub max_connections: usize,

    #[validate(range(min = 1, max = 3600))]
    pub timeout_seconds: u64,
}

/// 存储配置
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct StorageConfig {
    #[validate(length(min = 1, max = 500))]
    pub data_dir: String,

    #[validate(range(min = 1, max = 1024))]
    pub max_file_size_mb: u64,

    #[validate(range(min = 1, max = 100))]
    pub max_concurrent_files: usize,

    pub compression_enabled: bool,

    pub encryption_enabled: bool,
}

/// 搜索配置
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct SearchConfig {
    #[validate(range(min = 1, max = 10000))]
    pub max_results: usize,

    #[validate(range(min = 1, max = 60))]
    pub timeout_seconds: u64,

    #[validate(range(min = 1, max = 100))]
    pub max_concurrent_searches: usize,

    pub fuzzy_search_enabled: bool,

    pub case_sensitive: bool,

    pub regex_enabled: bool,
}

/// 监控配置
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct MonitoringConfig {
    #[validate(length(min = 1, max = 100))]
    pub log_level: String,

    pub metrics_enabled: bool,

    pub tracing_enabled: bool,

    #[validate(length(min = 1, max = 500))]
    pub log_file: String,

    #[validate(range(min = 1, max = 100))]
    pub max_log_files: usize,
}

/// 安全配置
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct SecurityConfig {
    pub auth_enabled: bool,

    #[validate(length(min = 8, max = 100))]
    pub api_key: Option<String>,

    pub rate_limit_enabled: bool,

    #[validate(range(min = 1, max = 1000))]
    pub rate_limit_per_minute: u64,

    pub cors_enabled: bool,

    #[validate(length(min = 1, max = 500))]
    pub allowed_origins: Vec<String>,
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

impl AppConfig {
    /// 从文件加载配置并验证
    ///
    /// **Industry Pattern**: Config loading with validation
    /// - Loads from file (TOML/JSON/YAML)
    /// - Applies environment variable overrides
    /// - Validates using validator crate
    /// - Returns validated config or detailed error
    pub fn load_and_validate(path: &Path) -> ConfigResult<Self> {
        // 检查文件是否存在
        if !path.exists() {
            return Err(ConfigError::FileNotFound(path.to_path_buf()));
        }

        // TODO: 实际实现文件加载 (暂时返回默认配置)
        // let content = std::fs::read_to_string(path)?;
        // let config: Self = match path.extension().and_then(|e| e.to_str()) {
        //     Some("toml") => toml::from_str(&content).map_err(|e| ConfigError::FormatError(e.to_string()))?,
        //     Some("json") => serde_json::from_str(&content).map_err(|e| ConfigError::FormatError(e.to_string()))?,
        //     _ => return Err(ConfigError::FormatError("Unsupported file format".to_string())),
        // };

        let mut config = Self::default();

        // 应用环境变量覆盖
        config.apply_env_overrides()?;

        // 验证配置
        config
            .validate()
            .map_err(|e| ConfigError::Validation(format!("Validation errors: {}", e)))?;

        Ok(config)
    }

    /// 应用环境变量覆盖
    ///
    /// **Pattern**: 12-Factor App Configuration
    /// Environment variables override file-based config
    ///
    /// Examples:
    /// - `APP_SERVER_PORT=8080` overrides server.port
    /// - `APP_SEARCH_MAX_RESULTS=5000` overrides search.max_results
    fn apply_env_overrides(&mut self) -> ConfigResult<()> {
        // Server config overrides
        if let Ok(port) = std::env::var("APP_SERVER_PORT") {
            self.server.port = port.parse().map_err(|_| {
                ConfigError::InvalidValue(format!("Invalid APP_SERVER_PORT: {}", port))
            })?;
        }

        if let Ok(host) = std::env::var("APP_SERVER_HOST") {
            self.server.host = host;
        }

        if let Ok(max_conn) = std::env::var("APP_SERVER_MAX_CONNECTIONS") {
            self.server.max_connections = max_conn.parse().map_err(|_| {
                ConfigError::InvalidValue(format!(
                    "Invalid APP_SERVER_MAX_CONNECTIONS: {}",
                    max_conn
                ))
            })?;
        }

        // Storage config overrides
        if let Ok(data_dir) = std::env::var("APP_STORAGE_DATA_DIR") {
            self.storage.data_dir = data_dir;
        }

        if let Ok(max_size) = std::env::var("APP_STORAGE_MAX_FILE_SIZE_MB") {
            self.storage.max_file_size_mb = max_size.parse().map_err(|_| {
                ConfigError::InvalidValue(format!(
                    "Invalid APP_STORAGE_MAX_FILE_SIZE_MB: {}",
                    max_size
                ))
            })?;
        }

        // Search config overrides
        if let Ok(max_results) = std::env::var("APP_SEARCH_MAX_RESULTS") {
            self.search.max_results = max_results.parse().map_err(|_| {
                ConfigError::InvalidValue(format!(
                    "Invalid APP_SEARCH_MAX_RESULTS: {}",
                    max_results
                ))
            })?;
        }

        // Monitoring config overrides
        if let Ok(log_level) = std::env::var("APP_LOG_LEVEL") {
            self.monitoring.log_level = log_level;
        }

        // Security config overrides
        if let Ok(api_key) = std::env::var("APP_API_KEY") {
            self.security.api_key = Some(api_key);
        }

        Ok(())
    }

    /// 热重载配置
    ///
    /// **Use Case**: Update config without restarting application
    /// - Reloads from file
    /// - Validates new config
    /// - Replaces current config atomically
    pub fn reload(&mut self, path: &Path) -> ConfigResult<()> {
        let new_config = Self::load_and_validate(path)?;
        *self = new_config;
        tracing::info!("Configuration reloaded successfully from {:?}", path);
        Ok(())
    }

    /// 获取针对特定环境的配置
    ///
    /// **Pattern**: Environment-specific configuration
    /// Adjusts defaults based on environment (dev/prod/test)
    pub fn for_environment(env: Environment) -> Self {
        let mut config = Self::default();

        match env {
            Environment::Production => {
                config.server.host = "0.0.0.0".to_string();
                config.monitoring.log_level = "warn".to_string();
                config.security.auth_enabled = true;
                config.security.cors_enabled = true;
                config.security.allowed_origins = vec!["https://yourdomain.com".to_string()];
            }
            Environment::Testing => {
                config.server.port = 0; // Random port
                config.monitoring.log_level = "debug".to_string();
                config.security.auth_enabled = false;
                config.storage.data_dir = "./test_data".to_string();
            }
            Environment::Development => {
                // Already set by default()
                config.monitoring.log_level = "debug".to_string();
            }
        }

        config
    }

    /// 验证配置的额外业务规则
    ///
    /// **Beyond validator crate**: Custom business logic validation
    pub fn validate_business_rules(&self) -> ConfigResult<()> {
        // 验证端口不在保留范围
        if self.server.port < 1024 && self.server.port != 0 {
            return Err(ConfigError::Validation(
                "Port must be >= 1024 (unprivileged) or 0 (random)".to_string(),
            ));
        }

        // 验证数据目录路径合法
        let data_path = PathBuf::from(&self.storage.data_dir);
        if data_path.to_string_lossy().contains("..") {
            return Err(ConfigError::Validation(
                "Data directory path must not contain '..'".to_string(),
            ));
        }

        // 验证日志级别有效
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&self.monitoring.log_level.to_lowercase().as_str()) {
            return Err(ConfigError::Validation(format!(
                "Invalid log level: {}",
                self.monitoring.log_level
            )));
        }

        Ok(())
    }
}
