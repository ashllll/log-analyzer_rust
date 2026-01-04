//! 服务配置模块
//!
//! 提供配置驱动的服务创建，支持：
//! - TOML/JSON 配置文件加载
//! - 开发和生产环境的默认配置
//! - 配置验证和错误处理

use eyre::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// 服务配置
///
/// 定义所有服务的配置参数，支持从配置文件加载
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfiguration {
    /// 事件总线配置
    #[serde(default)]
    pub event_bus: EventBusConfig,
    /// 查询执行器配置
    #[serde(default)]
    pub query_executor: QueryExecutorConfig,
    /// 缓存配置
    #[serde(default)]
    pub cache: CacheConfig,
    /// 资源管理配置
    #[serde(default)]
    pub resource_management: ResourceManagementConfig,
}

/// 事件总线配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventBusConfig {
    /// 通道容量
    #[serde(default = "default_event_bus_capacity")]
    pub capacity: usize,
}

fn default_event_bus_capacity() -> usize {
    1000
}

impl Default for EventBusConfig {
    fn default() -> Self {
        Self {
            capacity: default_event_bus_capacity(),
        }
    }
}

/// 查询执行器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryExecutorConfig {
    /// 正则表达式缓存大小
    #[serde(default = "default_query_cache_size")]
    pub cache_size: usize,
    /// 最大查询复杂度
    #[serde(default = "default_max_query_complexity")]
    pub max_query_complexity: usize,
}

fn default_query_cache_size() -> usize {
    100
}

fn default_max_query_complexity() -> usize {
    1000
}

impl Default for QueryExecutorConfig {
    fn default() -> Self {
        Self {
            cache_size: default_query_cache_size(),
            max_query_complexity: default_max_query_complexity(),
        }
    }
}

/// 缓存配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// 最大缓存容量
    #[serde(default = "default_cache_capacity")]
    pub max_capacity: u64,
    /// TTL（秒）
    #[serde(default = "default_cache_ttl")]
    pub ttl_seconds: u64,
    /// TTI（秒）
    #[serde(default = "default_cache_tti")]
    pub tti_seconds: u64,
}

fn default_cache_capacity() -> u64 {
    100
}

fn default_cache_ttl() -> u64 {
    300 // 5 分钟
}

fn default_cache_tti() -> u64 {
    60 // 1 分钟
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_capacity: default_cache_capacity(),
            ttl_seconds: default_cache_ttl(),
            tti_seconds: default_cache_tti(),
        }
    }
}

/// 资源管理配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceManagementConfig {
    /// 清理队列最大大小
    #[serde(default = "default_cleanup_queue_size")]
    pub cleanup_queue_size: usize,
    /// 资源泄漏检测超时（秒）
    #[serde(default = "default_leak_detection_timeout")]
    pub leak_detection_timeout_seconds: u64,
    /// 是否启用自动清理
    #[serde(default = "default_auto_cleanup")]
    pub auto_cleanup_enabled: bool,
}

fn default_cleanup_queue_size() -> usize {
    1000
}

fn default_leak_detection_timeout() -> u64 {
    300 // 5 分钟
}

fn default_auto_cleanup() -> bool {
    true
}

impl Default for ResourceManagementConfig {
    fn default() -> Self {
        Self {
            cleanup_queue_size: default_cleanup_queue_size(),
            leak_detection_timeout_seconds: default_leak_detection_timeout(),
            auto_cleanup_enabled: default_auto_cleanup(),
        }
    }
}

impl ServiceConfiguration {
    /// 从 TOML 文件加载配置
    ///
    /// # Arguments
    /// * `path` - 配置文件路径
    ///
    /// # Errors
    /// 如果文件不存在或格式错误，返回错误
    pub fn from_toml_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let config: Self = toml::from_str(&content)
            .with_context(|| format!("Failed to parse TOML config: {}", path.display()))?;

        tracing::info!("Loaded service configuration from: {}", path.display());
        Ok(config)
    }

    /// 从 JSON 文件加载配置
    ///
    /// # Arguments
    /// * `path` - 配置文件路径
    ///
    /// # Errors
    /// 如果文件不存在或格式错误，返回错误
    pub fn from_json_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let config: Self = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse JSON config: {}", path.display()))?;

        tracing::info!("Loaded service configuration from: {}", path.display());
        Ok(config)
    }

    /// 保存配置到 TOML 文件
    ///
    /// # Arguments
    /// * `path` - 配置文件路径
    ///
    /// # Errors
    /// 如果写入失败，返回错误
    pub fn save_to_toml<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        let content = toml::to_string_pretty(self).context("Failed to serialize config to TOML")?;

        std::fs::write(path, content)
            .with_context(|| format!("Failed to write config file: {}", path.display()))?;

        tracing::info!("Saved service configuration to: {}", path.display());
        Ok(())
    }

    /// 保存配置到 JSON 文件
    ///
    /// # Arguments
    /// * `path` - 配置文件路径
    ///
    /// # Errors
    /// 如果写入失败，返回错误
    pub fn save_to_json<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        let content =
            serde_json::to_string_pretty(self).context("Failed to serialize config to JSON")?;

        std::fs::write(path, content)
            .with_context(|| format!("Failed to write config file: {}", path.display()))?;

        tracing::info!("Saved service configuration to: {}", path.display());
        Ok(())
    }

    /// 创建开发环境默认配置
    pub fn development() -> Self {
        Self {
            event_bus: EventBusConfig { capacity: 1000 },
            query_executor: QueryExecutorConfig {
                cache_size: 50,
                max_query_complexity: 500,
            },
            cache: CacheConfig {
                max_capacity: 50,
                ttl_seconds: 180, // 3 分钟
                tti_seconds: 60,
            },
            resource_management: ResourceManagementConfig {
                cleanup_queue_size: 500,
                leak_detection_timeout_seconds: 180,
                auto_cleanup_enabled: true,
            },
        }
    }

    /// 创建生产环境默认配置
    pub fn production() -> Self {
        Self {
            event_bus: EventBusConfig { capacity: 2000 },
            query_executor: QueryExecutorConfig {
                cache_size: 200,
                max_query_complexity: 2000,
            },
            cache: CacheConfig {
                max_capacity: 200,
                ttl_seconds: 600, // 10 分钟
                tti_seconds: 120, // 2 分钟
            },
            resource_management: ResourceManagementConfig {
                cleanup_queue_size: 2000,
                leak_detection_timeout_seconds: 600,
                auto_cleanup_enabled: true,
            },
        }
    }

    /// 验证配置
    ///
    /// # Errors
    /// 如果配置无效，返回错误
    pub fn validate(&self) -> Result<()> {
        // 验证事件总线容量
        if self.event_bus.capacity == 0 {
            eyre::bail!("Event bus capacity must be greater than 0");
        }

        // 验证查询执行器配置
        if self.query_executor.cache_size == 0 {
            eyre::bail!("Query executor cache size must be greater than 0");
        }
        if self.query_executor.max_query_complexity == 0 {
            eyre::bail!("Max query complexity must be greater than 0");
        }

        // 验证缓存配置
        if self.cache.max_capacity == 0 {
            eyre::bail!("Cache max capacity must be greater than 0");
        }
        if self.cache.ttl_seconds == 0 {
            eyre::bail!("Cache TTL must be greater than 0");
        }
        if self.cache.tti_seconds == 0 {
            eyre::bail!("Cache TTI must be greater than 0");
        }

        // 验证资源管理配置
        if self.resource_management.cleanup_queue_size == 0 {
            eyre::bail!("Cleanup queue size must be greater than 0");
        }
        if self.resource_management.leak_detection_timeout_seconds == 0 {
            eyre::bail!("Leak detection timeout must be greater than 0");
        }

        tracing::debug!("Service configuration validated successfully");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_configuration() {
        let config = ServiceConfiguration::default();
        assert_eq!(config.event_bus.capacity, 1000);
        assert_eq!(config.query_executor.cache_size, 100);
        assert_eq!(config.cache.max_capacity, 100);
    }

    #[test]
    fn test_development_configuration() {
        let config = ServiceConfiguration::development();
        assert_eq!(config.event_bus.capacity, 1000);
        assert_eq!(config.query_executor.cache_size, 50);
        assert_eq!(config.cache.ttl_seconds, 180);
    }

    #[test]
    fn test_production_configuration() {
        let config = ServiceConfiguration::production();
        assert_eq!(config.event_bus.capacity, 2000);
        assert_eq!(config.query_executor.cache_size, 200);
        assert_eq!(config.cache.ttl_seconds, 600);
    }

    #[test]
    fn test_configuration_validation() {
        let config = ServiceConfiguration::default();
        assert!(config.validate().is_ok());

        // 测试无效配置
        let mut invalid_config = config.clone();
        invalid_config.event_bus.capacity = 0;
        assert!(invalid_config.validate().is_err());
    }

    #[test]
    fn test_toml_serialization() {
        let config = ServiceConfiguration::development();
        let toml_str = toml::to_string(&config).expect("Failed to serialize to TOML");

        // 验证可以反序列化
        let deserialized: ServiceConfiguration =
            toml::from_str(&toml_str).expect("Failed to deserialize from TOML");

        assert_eq!(config.event_bus.capacity, deserialized.event_bus.capacity);
    }

    #[test]
    fn test_json_serialization() {
        let config = ServiceConfiguration::production();
        let json_str = serde_json::to_string(&config).expect("Failed to serialize to JSON");

        // 验证可以反序列化
        let deserialized: ServiceConfiguration =
            serde_json::from_str(&json_str).expect("Failed to deserialize from JSON");

        assert_eq!(config.event_bus.capacity, deserialized.event_bus.capacity);
    }

    #[test]
    fn test_load_from_toml_file() {
        let config = ServiceConfiguration::development();

        // 创建临时文件
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let toml_content = toml::to_string(&config).expect("Failed to serialize");
        temp_file
            .write_all(toml_content.as_bytes())
            .expect("Failed to write to temp file");

        // 从文件加载
        let loaded_config = ServiceConfiguration::from_toml_file(temp_file.path())
            .expect("Failed to load from TOML file");

        assert_eq!(config.event_bus.capacity, loaded_config.event_bus.capacity);
    }

    #[test]
    fn test_load_from_json_file() {
        let config = ServiceConfiguration::production();

        // 创建临时文件
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let json_content = serde_json::to_string(&config).expect("Failed to serialize");
        temp_file
            .write_all(json_content.as_bytes())
            .expect("Failed to write to temp file");

        // 从文件加载
        let loaded_config = ServiceConfiguration::from_json_file(temp_file.path())
            .expect("Failed to load from JSON file");

        assert_eq!(config.event_bus.capacity, loaded_config.event_bus.capacity);
    }

    #[test]
    fn test_save_and_load_toml() {
        let config = ServiceConfiguration::development();
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");

        // 保存
        config
            .save_to_toml(temp_file.path())
            .expect("Failed to save to TOML");

        // 加载
        let loaded_config = ServiceConfiguration::from_toml_file(temp_file.path())
            .expect("Failed to load from TOML");

        assert_eq!(config.event_bus.capacity, loaded_config.event_bus.capacity);
    }

    #[test]
    fn test_save_and_load_json() {
        let config = ServiceConfiguration::production();
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");

        // 保存
        config
            .save_to_json(temp_file.path())
            .expect("Failed to save to JSON");

        // 加载
        let loaded_config = ServiceConfiguration::from_json_file(temp_file.path())
            .expect("Failed to load from JSON");

        assert_eq!(config.event_bus.capacity, loaded_config.event_bus.capacity);
    }
}
