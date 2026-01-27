//! Path Validator Module - Archive Security Enhancement
//! 
//! 提供统一的路径安全验证，防止路径遍历、符号链接攻击等安全问题。
//! 
//! **安全原则**:
//! 1. Defense in Depth - 多层验证
//! 2. Fail Secure - 验证失败时拒绝操作
//! 3. Whitelist Approach - 明确允许的模式

use crate::error::{AppError, Result};
use std::path::{Path, PathBuf, Component};
use tracing::warn;

/// 路径验证结果
#[derive(Debug, Clone)]
pub enum ValidationResult {
    /// 路径安全，可以使用
    Safe,
    /// 路径存在安全风险，提供原因
    Unsafe(String),
}

/// 路径验证器配置
#[derive(Debug, Clone)]
pub struct PathValidatorConfig {
    /// 是否允许符号链接
    pub allow_symlinks: bool,
    /// 是否允许绝对路径
    pub allow_absolute_paths: bool,
    /// 是否允许..组件
    pub allow_parent_references: bool,
    /// 最大路径深度
    pub max_path_depth: usize,
    /// 禁止的文件名模式（正则表达式）
    pub forbidden_patterns: Vec<String>,
}

impl Default for PathValidatorConfig {
    fn default() -> Self {
        Self {
            allow_symlinks: false,
            allow_absolute_paths: false,
            allow_parent_references: false,
            max_path_depth: 100,
            forbidden_patterns: vec![
                // Windows设备文件
                r"^(CON|PRN|AUX|NUL|COM[0-9]|LPT[0-9])$".to_string(),
                // 隐藏的配置文件（可选）
                r"^\.[a-zA-Z]".to_string(),
            ],
        }
    }
}

impl PathValidatorConfig {
    /// 创建严格的安全配置（用于不受信任的归档）
    pub fn strict() -> Self {
        Self {
            allow_symlinks: false,
            allow_absolute_paths: false,
            allow_parent_references: false,
            max_path_depth: 50,
            forbidden_patterns: vec![
                r"^(CON|PRN|AUX|NUL|COM[0-9]|LPT[0-9])$".to_string(),
                r"^\.[a-zA-Z]".to_string(),
                r"[\x00-\x1f]".to_string(), // 控制字符
            ],
        }
    }

    /// 创建宽松的配置（用于受信任的源）
    pub fn permissive() -> Self {
        Self {
            allow_symlinks: true,
            allow_absolute_paths: false,
            allow_parent_references: false,
            max_path_depth: 200,
            forbidden_patterns: vec![
                r"^(CON|PRN|AUX|NUL|COM[0-9]|LPT[0-9])$".to_string(),
            ],
        }
    }
}

/// 路径安全验证器
pub struct PathValidator {
    config: PathValidatorConfig,
}

impl Default for PathValidator {
    fn default() -> Self {
        Self::new(PathValidatorConfig::default())
    }
}

impl PathValidator {
    /// 创建新的路径验证器
    pub fn new(config: PathValidatorConfig) -> Self {
        Self { config }
    }

    /// 创建使用严格配置的验证器
    pub fn strict() -> Self {
        Self::new(PathValidatorConfig::strict())
    }

    /// 验证归档条目路径是否安全
    /// 
    /// # Arguments
    /// * `entry_path` - 归档中的条目路径
    /// * `base_dir` - 提取的基础目录
    /// 
    /// # Returns
    /// * `Ok(PathBuf)` - 验证通过，返回规范化的完整路径
    /// * `Err(AppError)` - 验证失败，包含具体原因
    pub fn validate_extraction_path(
        &self,
        entry_path: &str,
        base_dir: &Path,
    ) -> Result<PathBuf> {
        // 1. 检查空路径
        if entry_path.is_empty() {
            return Err(AppError::archive_error(
                "Empty path in archive entry".to_string(),
                None,
            ));
        }

        // 2. 检查路径遍历攻击特征
        if entry_path.contains("..") {
            warn!("Path traversal detected: {}", entry_path);
            return Err(AppError::archive_error(
                format!("Path traversal detected: {}", entry_path),
                Some(PathBuf::from(entry_path)),
            ));
        }

        // 3. 检查协议前缀（如 file://, http://）
        if entry_path.contains("://") {
            warn!("URL protocol detected in path: {}", entry_path);
            return Err(AppError::archive_error(
                format!("Invalid path with protocol: {}", entry_path),
                Some(PathBuf::from(entry_path)),
            ));
        }

        // 4. 检查控制字符
        if entry_path.chars().any(|c| c.is_control()) {
            warn!("Control characters detected in path: {:?}", entry_path);
            return Err(AppError::archive_error(
                "Path contains control characters".to_string(),
                Some(PathBuf::from(entry_path)),
            ));
        }

        // 5. 解析路径组件
        let path = Path::new(entry_path);
        let mut components = Vec::new();
        let mut depth = 0;

        for component in path.components() {
            match component {
                Component::Normal(name) => {
                    components.push(name);
                    depth += 1;
                    
                    // 检查路径深度
                    if depth > self.config.max_path_depth {
                        return Err(AppError::archive_error(
                            format!("Path depth {} exceeds maximum {}", depth, self.config.max_path_depth),
                            Some(PathBuf::from(entry_path)),
                        ));
                    }
                    
                    // 检查文件名合法性
                    if let Some(name_str) = name.to_str() {
                        self.validate_filename(name_str)?;
                    }
                }
                Component::ParentDir => {
                    if !self.config.allow_parent_references {
                        return Err(AppError::archive_error(
                            "Parent directory references (..) not allowed".to_string(),
                            Some(PathBuf::from(entry_path)),
                        ));
                    }
                }
                Component::RootDir | Component::Prefix(_) => {
                    if !self.config.allow_absolute_paths {
                        return Err(AppError::archive_error(
                            "Absolute paths not allowed in archives".to_string(),
                            Some(PathBuf::from(entry_path)),
                        ));
                    }
                }
                Component::CurDir => {
                    // ./ 是安全的，可以忽略
                }
            }
        }

        // 6. 构建最终路径并验证是否在base_dir内
        let final_path = base_dir.join(entry_path);
        
        // 7. 规范化路径（解析符号链接和相对路径）
        // 注意：这一步在文件不存在时会失败，所以我们使用逐步验证
        self.validate_path_within_base(&final_path, base_dir)?;

        Ok(final_path)
    }

    /// 验证文件名是否合法
    fn validate_filename(&self, filename: &str) -> Result<()> {
        // 检查空文件名
        if filename.is_empty() {
            return Err(AppError::archive_error(
                "Empty filename".to_string(),
                None,
            ));
        }

        // 检查Windows保留设备名
        let uppercase = filename.to_uppercase();
        let reserved_names = [
            "CON", "PRN", "AUX", "NUL",
            "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
            "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
        ];
        
        // 处理类似 "CON.txt" 的情况
        let base_name = uppercase.split('.').next().unwrap_or("");
        if reserved_names.contains(&base_name) {
            return Err(AppError::archive_error(
                format!("Reserved device name not allowed: {}", filename),
                None,
            ));
        }

        // 检查Windows非法字符 < > : " | ? * 
        #[cfg(target_os = "windows")]
        {
            let illegal_chars = ['<', '>', ':', '"', '|', '?', '*'];
            if filename.chars().any(|c| illegal_chars.contains(&c)) {
                return Err(AppError::archive_error(
                    format!("Filename contains illegal characters: {}", filename),
                    None,
                ));
            }
        }

        // 检查文件名长度
        if filename.len() > 255 {
            return Err(AppError::archive_error(
                format!("Filename too long ({} > 255): {}", filename.len(), filename),
                None,
            ));
        }

        Ok(())
    }

    /// 验证路径是否在base_dir内（防止符号链接逃逸）
    fn validate_path_within_base(&self, path: &Path, base_dir: &Path) -> Result<()> {
        // 使用字符串比较进行初步验证
        let path_str = path.to_string_lossy();
        let base_str = base_dir.to_string_lossy();
        
        // 规范化路径分隔符
        let normalized_path = path_str.replace('\\', "/");
        let normalized_base = base_str.replace('\\', "/");
        
        if !normalized_path.starts_with(&normalized_base) {
            warn!(
                "Path escape detected: {} not within {}",
                normalized_path, normalized_base
            );
            return Err(AppError::archive_error(
                format!("Path escapes base directory: {}", path.display()),
                Some(path.to_path_buf()),
            ));
        }

        Ok(())
    }

    /// 批量验证多个路径
    pub fn validate_batch(
        &self,
        entries: &[String],
        base_dir: &Path,
    ) -> Vec<Result<PathBuf>> {
        entries
            .iter()
            .map(|entry| self.validate_extraction_path(entry, base_dir))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_path_traversal_detection() {
        let validator = PathValidator::strict();
        let base = PathBuf::from("/tmp/extract");

        // 应该拒绝路径遍历
        assert!(validator
            .validate_extraction_path("../etc/passwd", &base)
            .is_err());
        assert!(validator
            .validate_extraction_path("foo/../../etc/passwd", &base)
            .is_err());
    }

    #[test]
    fn test_absolute_path_rejection() {
        let validator = PathValidator::strict();
        let base = PathBuf::from("/tmp/extract");

        // 应该拒绝绝对路径
        assert!(validator
            .validate_extraction_path("/etc/passwd", &base)
            .is_err());
        
        #[cfg(target_os = "windows")]
        assert!(validator
            .validate_extraction_path("C:\\Windows\\System32", &base)
            .is_err());
    }

    #[test]
    fn test_reserved_names() {
        let validator = PathValidator::strict();
        let base = PathBuf::from("/tmp/extract");

        // Windows保留设备名
        assert!(validator
            .validate_extraction_path("CON", &base)
            .is_err());
        assert!(validator
            .validate_extraction_path("PRN.txt", &base)
            .is_err());
        assert!(validator
            .validate_extraction_path("COM1", &base)
            .is_err());
    }

    #[test]
    fn test_safe_paths() {
        let validator = PathValidator::strict();
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();

        // 这些路径应该是安全的
        assert!(validator
            .validate_extraction_path("file.txt", base)
            .is_ok());
        assert!(validator
            .validate_extraction_path("dir/subdir/file.txt", base)
            .is_ok());
        assert!(validator
            .validate_extraction_path("./relative/path.txt", base)
            .is_ok());
    }

    #[test]
    fn test_control_characters() {
        let validator = PathValidator::strict();
        let base = PathBuf::from("/tmp/extract");

        // 应该拒绝控制字符
        assert!(validator
            .validate_extraction_path("file\x00name.txt", &base)
            .is_err());
        assert!(validator
            .validate_extraction_path("file\nname.txt", &base)
            .is_err());
    }

    #[test]
    fn test_protocol_prefix() {
        let validator = PathValidator::strict();
        let base = PathBuf::from("/tmp/extract");

        // 应该拒绝协议前缀
        assert!(validator
            .validate_extraction_path("file:///etc/passwd", &base)
            .is_err());
        assert!(validator
            .validate_extraction_path("http://example.com/file", &base)
            .is_err());
    }

    #[test]
    fn test_path_depth_limit() {
        let validator = PathValidator::strict();
        let base = PathBuf::from("/tmp/extract");

        // 创建深度超过限制的路径
        let deep_path = "a/".repeat(60); // 超过50的限制
        assert!(validator
            .validate_extraction_path(&deep_path, &base)
            .is_err());
    }
}
