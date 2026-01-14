//! 配置管理基础设施层
//!
//! 采用分层架构：
//! - 应用层配置 (用户界面)
//! - 领域层配置 (业务规则)
//! - 基础设施层配置 (持久化)

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
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
}

/// 全局配置根结构
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
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

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            storage: StorageConfig::default(),
            search: SearchConfig::default(),
            monitoring: MonitoringConfig::default(),
            security: SecurityConfig::default(),
        }
    }
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