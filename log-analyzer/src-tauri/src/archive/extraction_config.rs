//! 提取配置模块
//!
//! 提供统一的配置结构体用于归档提取操作，封装所有限制参数和安全配置。

use serde::{Deserialize, Serialize};

/// 提取配置 - 封装所有提取限制参数
///
/// 该结构体统一管理归档提取时的各种限制，包括文件大小、数量限制、
/// 嵌套深度等，避免使用多个独立参数传递。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExtractionConfig {
    /// 配置版本（用于兼容性处理）
    pub version: u32,
    /// 提取限制
    pub limits: ExtractionLimits,
    /// 安全配置
    pub security: SecurityConfig,
    /// 读取文件时的最大大小（用于预览）
    pub max_read_size: u64,
    /// 缓冲区大小
    pub buffer_size: usize,
}

/// 提取限制参数
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExtractionLimits {
    /// 单个文件最大大小（字节），默认 100MB
    pub max_file_size: u64,
    /// 解压后总大小限制（字节），默认 1GB
    pub max_total_size: u64,
    /// 解压文件数量限制，默认 1000
    pub max_file_count: usize,
    /// 最大解压深度（防止zip炸弹），默认 5
    pub max_depth: u32,
}

/// 安全配置
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// 是否允许符号链接
    pub allow_symlinks: bool,
    /// 是否允许绝对路径
    pub allow_absolute_paths: bool,
    /// 是否允许父目录遍历 (..)
    pub allow_parent_traversal: bool,
    /// 路径黑名单
    pub path_blacklist: Vec<String>,
    /// 允许的文件扩展名白名单（空表示允许所有）
    pub allowed_extensions: Vec<String>,
    /// 禁止的文件扩展名黑名单
    pub forbidden_extensions: Vec<String>,
}

impl Default for ExtractionConfig {
    fn default() -> Self {
        Self {
            version: 1,
            limits: ExtractionLimits::default(),
            security: SecurityConfig::default(),
            max_read_size: 10 * 1024 * 1024, // 10MB
            buffer_size: 64 * 1024,          // 64KB
        }
    }
}

impl ExtractionLimits {
    /// 创建带有自定义限制的配置
    pub fn with_limits(max_file_size: u64, max_total_size: u64, max_file_count: usize) -> Self {
        Self {
            max_file_size,
            max_total_size,
            max_file_count,
            ..Default::default()
        }
    }
}

impl Default for ExtractionLimits {
    fn default() -> Self {
        Self {
            max_file_size: 100 * 1024 * 1024,   // 100MB
            max_total_size: 1024 * 1024 * 1024, // 1GB
            max_file_count: 1000,
            max_depth: 5,
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            allow_symlinks: false,
            allow_absolute_paths: false,
            allow_parent_traversal: false,
            path_blacklist: vec![
                "/etc".to_string(),
                "/Windows".to_string(),
                "/System Volume Information".to_string(),
                "C:\\Windows".to_string(),
                "C:\\Program Files".to_string(),
                "C:\\Program Files (x86)".to_string(),
            ],
            allowed_extensions: Vec::new(), // 空表示允许所有
            forbidden_extensions: vec![
                "exe".to_string(),
                "dll".to_string(),
                "bat".to_string(),
                "sh".to_string(),
            ],
        }
    }
}

impl ExtractionConfig {
    /// 使用默认限制创建配置
    pub fn new() -> Self {
        Self::default()
    }

    /// 创建带有自定义限制的配置
    pub fn with_limits(max_file_size: u64, max_total_size: u64, max_file_count: usize) -> Self {
        Self {
            limits: ExtractionLimits::with_limits(max_file_size, max_total_size, max_file_count),
            ..Default::default()
        }
    }

    /// 从旧格式参数迁移（向后兼容）
    pub fn migrate_from_legacy(
        max_file_size: u64,
        max_total_size: u64,
        max_file_count: usize,
    ) -> Self {
        Self::with_limits(max_file_size, max_total_size, max_file_count)
    }

    /// 设置最大深度
    pub fn with_max_depth(mut self, max_depth: u32) -> Self {
        self.limits.max_depth = max_depth;
        self
    }

    /// 更新安全配置
    pub fn with_security(mut self, security: SecurityConfig) -> Self {
        self.security = security;
        self
    }

    /// 验证配置有效性
    ///
    /// 检查配置参数是否合理，避免无意义的限制。
    ///
    /// # 返回
    ///
    /// * `Ok(())` - 配置有效
    /// * `Err(String)` - 配置无效，返回错误信息
    pub fn validate(&self) -> Result<(), String> {
        if self.limits.max_file_size == 0 {
            return Err("max_file_size 不能为 0".to_string());
        }
        if self.limits.max_total_size == 0 {
            return Err("max_total_size 不能为 0".to_string());
        }
        if self.limits.max_file_count == 0 {
            return Err("max_file_count 不能为 0".to_string());
        }
        if self.limits.max_depth == 0 {
            return Err("max_depth 不能为 0".to_string());
        }
        if self.limits.max_file_size > self.limits.max_total_size {
            return Err("max_file_size 不能大于 max_total_size".to_string());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extraction_config_default() {
        let config = ExtractionConfig::default();

        assert_eq!(config.version, 1);
        assert_eq!(config.limits.max_file_size, 100 * 1024 * 1024);
        assert_eq!(config.limits.max_total_size, 1024 * 1024 * 1024);
        assert_eq!(config.limits.max_file_count, 1000);
        assert_eq!(config.limits.max_depth, 5);
        assert_eq!(config.max_read_size, 10 * 1024 * 1024);
        assert_eq!(config.buffer_size, 64 * 1024);
    }

    #[test]
    fn test_extraction_config_with_limits() {
        let config = ExtractionConfig::with_limits(
            50 * 1024 * 1024,  // 50MB
            500 * 1024 * 1024, // 500MB
            500,
        );

        assert_eq!(config.limits.max_file_size, 50 * 1024 * 1024);
        assert_eq!(config.limits.max_total_size, 500 * 1024 * 1024);
        assert_eq!(config.limits.max_file_count, 500);
        assert_eq!(config.limits.max_depth, 5); // 默认值保持不变
    }

    #[test]
    fn test_extraction_config_validate_success() {
        let config = ExtractionConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_extraction_config_validate_zero_file_size() {
        let config = ExtractionConfig {
            limits: ExtractionLimits {
                max_file_size: 0,
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_extraction_config_validate_file_size_exceeds_total() {
        let config = ExtractionConfig {
            limits: ExtractionLimits {
                max_file_size: 200 * 1024 * 1024,  // 200MB
                max_total_size: 100 * 1024 * 1024, // 100MB
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_extraction_config_with_max_depth() {
        let config = ExtractionConfig::default().with_max_depth(10);

        assert_eq!(config.limits.max_depth, 10);
    }

    #[test]
    fn test_security_config_default() {
        let security = SecurityConfig::default();

        assert!(!security.allow_symlinks);
        assert!(!security.allow_absolute_paths);
        assert!(!security.allow_parent_traversal);
        assert!(!security.path_blacklist.is_empty());
        assert!(security.allowed_extensions.is_empty());
        assert!(!security.forbidden_extensions.is_empty());
    }

    #[test]
    fn test_extraction_config_migrate_from_legacy() {
        let config =
            ExtractionConfig::migrate_from_legacy(100 * 1024 * 1024, 1024 * 1024 * 1024, 1000);

        assert_eq!(config.limits.max_file_size, 100 * 1024 * 1024);
        assert_eq!(config.limits.max_total_size, 1024 * 1024 * 1024);
        assert_eq!(config.limits.max_file_count, 1000);
        assert_eq!(config.version, 1);
    }
}
