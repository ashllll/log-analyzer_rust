//! 验证的数据结构
//!
//! 使用validator框架提供结构化验证

use sanitize_filename::sanitize;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use unicode_normalization::UnicodeNormalization;
use validator::{Validate, ValidationError};

/// 验证的工作区配置
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ValidatedWorkspaceConfig {
    /// 工作区ID - 必须是有效的标识符
    #[validate(length(min = 1, max = 100, message = "Workspace ID must be 1-100 characters"))]
    #[validate(regex(
        path = "*WORKSPACE_ID_REGEX",
        message = "Workspace ID contains invalid characters"
    ))]
    #[validate(custom(function = "validate_workspace_id_format"))]
    pub workspace_id: String,

    /// 工作区名称
    #[validate(length(
        min = 1,
        max = 200,
        message = "Workspace name must be 1-200 characters"
    ))]
    #[validate(custom(function = "validate_workspace_name"))]
    pub name: String,

    /// 工作区描述
    #[validate(length(max = 1000, message = "Description must be less than 1000 characters"))]
    pub description: Option<String>,

    /// 工作区路径 - 必须是有效的路径
    #[validate(custom(function = "validate_workspace_path"))]
    pub path: String,

    /// 最大文件大小（字节）
    #[validate(range(min = 1, max = 1073741824, message = "Max file size must be 1B-1GB"))]
    // 1GB
    pub max_file_size: u64,

    /// 最大文件数量
    #[validate(range(min = 1, max = 100000, message = "Max file count must be 1-100000"))]
    pub max_file_count: u32,

    /// 是否启用实时监听
    pub enable_watch: bool,

    /// 自定义标签 - 每个标签都要验证
    #[validate(length(max = 50, message = "Too many tags"))]
    #[validate(custom(function = "validate_tags"))]
    pub tags: Vec<String>,

    /// 扩展配置 - 验证键值对
    #[validate(custom(function = "validate_metadata"))]
    pub metadata: HashMap<String, String>,

    /// 联系邮箱（可选）
    #[validate(email(message = "Invalid email format"))]
    pub contact_email: Option<String>,

    /// 项目URL（可选）
    #[validate(url(message = "Invalid URL format"))]
    pub project_url: Option<String>,
}

/// 验证的搜索查询
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ValidatedSearchQuery {
    /// 查询字符串
    #[validate(length(min = 1, max = 1000, message = "Query must be 1-1000 characters"))]
    #[validate(custom(function = "validate_search_query_content"))]
    pub query: String,

    /// 工作区ID
    #[validate(length(min = 1, max = 100, message = "Workspace ID must be 1-100 characters"))]
    #[validate(regex(path = "*WORKSPACE_ID_REGEX", message = "Invalid workspace ID format"))]
    #[validate(custom(function = "validate_workspace_id_format"))]
    pub workspace_id: String,

    /// 最大结果数
    #[validate(range(min = 1, max = 100000, message = "Max results must be 1-100000"))]
    pub max_results: Option<usize>,

    /// 是否区分大小写
    pub case_sensitive: bool,

    /// 是否使用正则表达式
    pub use_regex: bool,

    /// 文件模式过滤 - 支持glob模式
    #[validate(length(max = 200, message = "File pattern too long"))]
    #[validate(custom(function = "validate_file_pattern"))]
    pub file_pattern: Option<String>,

    /// 时间范围开始
    #[validate(custom(function = "validate_timestamp"))]
    pub time_start: Option<String>,

    /// 时间范围结束
    #[validate(custom(function = "validate_timestamp"))]
    pub time_end: Option<String>,

    /// 日志级别过滤
    #[validate(length(max = 10, message = "Too many log levels"))]
    #[validate(custom(function = "validate_log_levels"))]
    pub log_levels: Vec<String>,

    /// 搜索优先级（1-10）
    #[validate(range(min = 1, max = 10, message = "Priority must be 1-10"))]
    pub priority: Option<u8>,

    /// 超时时间（秒）
    #[validate(range(min = 1, max = 300, message = "Timeout must be 1-300 seconds"))]
    pub timeout_seconds: Option<u32>,
}

/// 验证的文件导入配置
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ValidatedImportConfig {
    /// 源路径
    #[validate(custom(function = "validate_import_path"))]
    pub source_path: String,

    /// 目标工作区ID
    #[validate(length(min = 1, max = 100, message = "Workspace ID must be 1-100 characters"))]
    #[validate(custom(function = "validate_workspace_id_format"))]
    pub workspace_id: String,

    /// 是否递归导入
    pub recursive: bool,

    /// 文件扩展名过滤
    #[validate(length(max = 20, message = "Too many file extensions"))]
    pub allowed_extensions: Vec<String>,

    /// 排除模式
    #[validate(length(max = 10, message = "Too many exclude patterns"))]
    pub exclude_patterns: Vec<String>,

    /// 最大导入大小（字节）
    #[validate(range(min = 1, max = 1073741824, message = "Max import size must be 1B-1GB"))]
    // 1GB
    pub max_import_size: u64,

    /// 是否覆盖现有文件
    pub overwrite_existing: bool,
}

/// 验证的压缩文件提取配置
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ValidatedArchiveExtractionConfig {
    /// 压缩文件路径
    #[validate(custom(function = "validate_archive_path"))]
    pub archive_path: String,

    /// 提取目标目录
    #[validate(custom(function = "validate_extraction_target_path"))]
    pub target_path: String,

    /// 单个文件最大大小（字节）- 默认100MB
    #[validate(range(min = 1, max = 104857600, message = "Max file size must be 1B-100MB"))]
    pub max_file_size: u64,

    /// 提取后总大小限制（字节）- 默认1GB
    #[validate(range(min = 1, max = 1073741824, message = "Max total size must be 1B-1GB"))]
    pub max_total_size: u64,

    /// 最大文件数量限制 - 默认1000个文件
    #[validate(range(min = 1, max = 1000, message = "Max file count must be 1-1000"))]
    pub max_file_count: usize,

    /// 是否允许覆盖现有文件
    pub allow_overwrite: bool,

    /// 是否验证提取的文件名安全性
    pub validate_filenames: bool,

    /// 允许的文件扩展名（空表示允许所有）
    #[validate(length(max = 50, message = "Too many allowed extensions"))]
    pub allowed_extensions: Vec<String>,

    /// 禁止的文件扩展名
    #[validate(length(max = 20, message = "Too many forbidden extensions"))]
    pub forbidden_extensions: Vec<String>,
}

impl Default for ValidatedArchiveExtractionConfig {
    fn default() -> Self {
        Self {
            archive_path: String::new(),
            target_path: String::new(),
            max_file_size: 104_857_600,    // 100MB
            max_total_size: 1_073_741_824, // 1GB
            max_file_count: 1000,
            allow_overwrite: false,
            validate_filenames: true,
            allowed_extensions: Vec::new(),
            forbidden_extensions: vec![
                "exe".to_string(),
                "bat".to_string(),
                "cmd".to_string(),
                "com".to_string(),
                "scr".to_string(),
                "pif".to_string(),
                "sh".to_string(),
                "bash".to_string(),
                "zsh".to_string(),
            ],
        }
    }
}

// 正则表达式常量
lazy_static::lazy_static! {
    pub static ref WORKSPACE_ID_REGEX: regex::Regex = regex::Regex::new(r"^[a-zA-Z0-9_-]+$").unwrap();
    pub static ref EMAIL_REGEX: regex::Regex = regex::Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();
    pub static ref URL_REGEX: regex::Regex = regex::Regex::new(r"^https?://[^\s/$.?#].[^\s]*$").unwrap();
    pub static ref FILE_PATTERN_REGEX: regex::Regex = regex::Regex::new(r"^[a-zA-Z0-9.*_-]+$").unwrap();
    pub static ref LOG_LEVEL_REGEX: regex::Regex = regex::Regex::new(r"^(TRACE|DEBUG|INFO|WARN|ERROR|FATAL)$").unwrap();
}

/// 验证工作区ID格式
fn validate_workspace_id_format(id: &str) -> Result<(), ValidationError> {
    if !WORKSPACE_ID_REGEX.is_match(id) {
        return Err(ValidationError::new("Invalid workspace ID format"));
    }

    // 额外检查：不能以连字符或下划线开头/结尾
    if id.starts_with('-') || id.starts_with('_') || id.ends_with('-') || id.ends_with('_') {
        return Err(ValidationError::new(
            "Workspace ID cannot start or end with - or _",
        ));
    }

    // 检查连续的特殊字符
    if id.contains("--") || id.contains("__") || id.contains("-_") || id.contains("_-") {
        return Err(ValidationError::new(
            "Workspace ID cannot contain consecutive special characters",
        ));
    }

    Ok(())
}

/// 验证工作区名称
fn validate_workspace_name(name: &str) -> Result<(), ValidationError> {
    // Unicode规范化
    let normalized: String = name.nfc().collect();
    if normalized != name {
        return Err(ValidationError::new("Name contains non-normalized Unicode"));
    }

    // 检查是否只包含空白字符
    if name.trim().is_empty() {
        return Err(ValidationError::new("Name cannot be only whitespace"));
    }

    // 检查是否包含控制字符
    if name.chars().any(|c| c.is_control()) {
        return Err(ValidationError::new("Name contains control characters"));
    }

    Ok(())
}

/// 验证标签列表
fn validate_tags(tags: &[String]) -> Result<(), ValidationError> {
    for tag in tags {
        // 每个标签长度限制
        if tag.len() > 50 {
            return Err(ValidationError::new("Tag is too long (max 50 characters)"));
        }

        // 标签不能为空
        if tag.trim().is_empty() {
            return Err(ValidationError::new("Tag cannot be empty"));
        }

        // Unicode规范化
        let normalized: String = tag.nfc().collect();
        if normalized != *tag {
            return Err(ValidationError::new("Tag contains non-normalized Unicode"));
        }

        // 标签只能包含字母、数字、连字符和下划线
        if !tag
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == ' ')
        {
            return Err(ValidationError::new("Tag contains invalid characters"));
        }
    }

    // 检查重复标签
    let mut unique_tags = std::collections::HashSet::new();
    for tag in tags {
        if !unique_tags.insert(tag.to_lowercase()) {
            return Err(ValidationError::new("Duplicate tags detected"));
        }
    }

    Ok(())
}

/// 验证元数据
fn validate_metadata(metadata: &HashMap<String, String>) -> Result<(), ValidationError> {
    // 限制元数据条目数量
    if metadata.len() > 100 {
        return Err(ValidationError::new("Too many metadata entries (max 100)"));
    }

    for (key, value) in metadata {
        // 键长度限制
        if key.len() > 100 {
            return Err(ValidationError::new(
                "Metadata key is too long (max 100 characters)",
            ));
        }

        // 值长度限制
        if value.len() > 1000 {
            return Err(ValidationError::new(
                "Metadata value is too long (max 1000 characters)",
            ));
        }

        // 键只能包含字母、数字、连字符和下划线
        if !key
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(ValidationError::new(
                "Metadata key contains invalid characters",
            ));
        }

        // 键不能为空
        if key.trim().is_empty() {
            return Err(ValidationError::new("Metadata key cannot be empty"));
        }
    }

    Ok(())
}

/// 验证搜索查询内容
fn validate_search_query_content(query: &str) -> Result<(), ValidationError> {
    // 检查是否只包含空白字符
    if query.trim().is_empty() {
        return Err(ValidationError::new("Query cannot be only whitespace"));
    }

    // Unicode规范化
    let normalized: String = query.nfc().collect();
    if normalized != query {
        return Err(ValidationError::new(
            "Query contains non-normalized Unicode",
        ));
    }

    // 检查是否包含NULL字符
    if query.contains('\0') {
        return Err(ValidationError::new("Query contains NULL characters"));
    }

    // 检查查询复杂度（防止ReDoS攻击）
    let special_char_count = query
        .chars()
        .filter(|c| "*+?[]{}()^$|\\".contains(*c))
        .count();
    if special_char_count > 50 {
        return Err(ValidationError::new(
            "Query is too complex (too many special characters)",
        ));
    }

    Ok(())
}

/// 验证文件模式
fn validate_file_pattern(pattern: &str) -> Result<(), ValidationError> {
    if pattern.is_empty() {
        return Ok(()); // 空模式是允许的
    }

    // 检查基本的glob模式语法
    if !FILE_PATTERN_REGEX.is_match(pattern) {
        return Err(ValidationError::new("Invalid file pattern format"));
    }

    // 检查是否包含路径遍历
    if pattern.contains("..") || pattern.contains('/') || pattern.contains('\\') {
        return Err(ValidationError::new(
            "File pattern cannot contain path separators",
        ));
    }

    Ok(())
}

/// 验证日志级别列表
fn validate_log_levels(levels: &[String]) -> Result<(), ValidationError> {
    for level in levels {
        let upper_level = level.to_uppercase();
        if !LOG_LEVEL_REGEX.is_match(&upper_level) {
            return Err(ValidationError::new("Invalid log level"));
        }
    }

    // 检查重复级别
    let mut unique_levels = std::collections::HashSet::new();
    for level in levels {
        if !unique_levels.insert(level.to_uppercase()) {
            return Err(ValidationError::new("Duplicate log levels detected"));
        }
    }

    Ok(())
}

/// 验证工作区路径 - 增强版本，包含Unicode规范化和全面的安全检查
fn validate_workspace_path(path: &str) -> Result<(), ValidationError> {
    if path.is_empty() {
        return Err(ValidationError::new("Path cannot be empty"));
    }

    if path.len() > 500 {
        return Err(ValidationError::new("Path too long (max 500 characters)"));
    }

    // Unicode规范化 - 防止Unicode欺骗攻击
    let normalized_path: String = path.nfc().collect();
    if normalized_path != path {
        return Err(ValidationError::new(
            "Path contains non-normalized Unicode characters",
        ));
    }

    // 全面的路径遍历攻击检查
    if path.contains("..") || path.contains("~") || path.contains("./") || path.contains(".\\") {
        return Err(ValidationError::new("Path contains traversal sequences"));
    }

    // 检查绝对路径（在某些上下文中可能不安全）
    let path_buf = Path::new(path);
    if path_buf.is_absolute() {
        // 允许绝对路径，但记录警告
        tracing::warn!("Absolute path detected: {}", path);
    }

    // 检查路径组件
    for component in path_buf.components() {
        match component {
            std::path::Component::ParentDir => {
                return Err(ValidationError::new(
                    "Path contains parent directory references",
                ));
            }
            std::path::Component::Normal(os_str) => {
                if let Some(str_component) = os_str.to_str() {
                    // 使用sanitize-filename验证每个组件
                    let sanitized = sanitize(str_component);
                    if sanitized != str_component {
                        return Err(ValidationError::new(
                            "Path component contains invalid characters",
                        ));
                    }

                    // 检查保留名称（Windows）
                    if cfg!(target_os = "windows") {
                        let reserved_names = [
                            "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5",
                            "COM6", "COM7", "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5",
                            "LPT6", "LPT7", "LPT8", "LPT9",
                        ];
                        let upper_component = str_component.to_uppercase();
                        if reserved_names.contains(&upper_component.as_str()) {
                            return Err(ValidationError::new(
                                "Path contains Windows reserved name",
                            ));
                        }
                    }
                }
            }
            _ => {} // 其他组件类型（Root, Prefix等）通常是安全的
        }
    }

    // 检查路径长度限制（不同操作系统有不同限制）
    let max_path_length = if cfg!(target_os = "windows") {
        260
    } else {
        4096
    };
    if path.len() > max_path_length {
        return Err(ValidationError::new("Path exceeds maximum length"));
    }

    Ok(())
}

/// 验证导入路径 - 增强版本，包含安全性和可访问性检查
fn validate_import_path(path: &str) -> Result<(), ValidationError> {
    // 首先进行基本路径验证
    validate_workspace_path(path)?;

    let path_buf = Path::new(path);

    // 检查路径是否存在（对于导入来说必须存在）
    if !path_buf.exists() {
        return Err(ValidationError::new("Import path does not exist"));
    }

    // 尝试规范化路径以检测符号链接和路径遍历
    match path_buf.canonicalize() {
        Ok(canonical_path) => {
            // 检查规范化后的路径是否与原路径一致（检测符号链接）
            if canonical_path != path_buf {
                tracing::warn!(
                    "Import path contains symbolic links: {} -> {}",
                    path_buf.display(),
                    canonical_path.display()
                );
            }

            // 可以在这里添加额外的安全检查，比如确保路径在允许的根目录内
            // 这里暂时记录日志，实际部署时可以根据需要启用严格检查
        }
        Err(_e) => {
            return Err(ValidationError::new("Cannot canonicalize path"));
        }
    }

    // 检查权限和可访问性
    match path_buf.metadata() {
        Ok(metadata) => {
            // 检查是否为目录或文件
            if !metadata.is_file() && !metadata.is_dir() {
                return Err(ValidationError::new(
                    "Path is neither a file nor a directory",
                ));
            }

            // 检查权限（Unix系统）
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let permissions = metadata.permissions();
                let mode = permissions.mode();

                // 检查是否可读
                if mode & 0o400 == 0 {
                    return Err(ValidationError::new("Import path is not readable"));
                }

                // 如果是目录，检查是否可执行（可进入）
                if metadata.is_dir() && mode & 0o100 == 0 {
                    return Err(ValidationError::new("Import directory is not accessible"));
                }
            }

            // 检查文件大小（防止导入过大的单个文件）
            if metadata.is_file() && metadata.len() > 1_073_741_824 {
                // 1GB
                return Err(ValidationError::new("Import file is too large (max 1GB)"));
            }
        }
        Err(_e) => {
            return Err(ValidationError::new("Cannot access import path metadata"));
        }
    }

    Ok(())
}

/// 验证时间戳格式
fn validate_timestamp(timestamp: &str) -> Result<(), ValidationError> {
    use chrono::{DateTime, Utc};

    if timestamp.is_empty() {
        return Ok(()); // 空时间戳是允许的
    }

    // 尝试解析ISO 8601格式
    if DateTime::parse_from_rfc3339(timestamp).is_ok() {
        return Ok(());
    }

    // 尝试解析UTC格式
    if timestamp.parse::<DateTime<Utc>>().is_ok() {
        return Ok(());
    }

    // 尝试解析Unix时间戳
    if let Ok(timestamp_num) = timestamp.parse::<i64>() {
        if timestamp_num > 0 && timestamp_num < 4102444800 {
            // 2100年之前
            return Ok(());
        }
    }

    Err(ValidationError::new("Invalid timestamp format"))
}

/// 验证压缩文件路径
fn validate_archive_path(path: &str) -> Result<(), ValidationError> {
    // 首先进行基本路径验证
    validate_workspace_path(path)?;

    let path_buf = Path::new(path);

    // 检查文件是否存在
    if !path_buf.exists() {
        return Err(ValidationError::new("Archive file does not exist"));
    }

    // 检查是否为文件（不是目录）
    if !path_buf.is_file() {
        return Err(ValidationError::new("Archive path must be a file"));
    }

    // 检查文件扩展名是否为支持的压缩格式
    let supported_extensions = ["zip", "rar", "tar", "gz", "tgz", "tar.gz", "tar.bz2", "bz2"];
    let file_name = path_buf
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| ValidationError::new("Invalid archive filename"))?;

    let has_valid_extension = supported_extensions
        .iter()
        .any(|ext| file_name.to_lowercase().ends_with(&format!(".{}", ext)));

    if !has_valid_extension {
        return Err(ValidationError::new("Unsupported archive format"));
    }

    // 检查文件大小（防止处理过大的压缩文件）
    if let Ok(metadata) = path_buf.metadata() {
        if metadata.len() > 2_147_483_648 {
            // 2GB
            return Err(ValidationError::new("Archive file is too large (max 2GB)"));
        }
    }

    Ok(())
}

/// 验证提取目标路径
fn validate_extraction_target_path(path: &str) -> Result<(), ValidationError> {
    // 基本路径验证
    validate_workspace_path(path)?;

    let path_buf = Path::new(path);

    // 如果路径存在，必须是目录
    if path_buf.exists() && !path_buf.is_dir() {
        return Err(ValidationError::new(
            "Extraction target must be a directory",
        ));
    }

    // 检查父目录是否存在且可写
    if let Some(parent) = path_buf.parent() {
        if parent.exists() {
            // 尝试检查写权限（Unix系统）
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(metadata) = parent.metadata() {
                    let permissions = metadata.permissions();
                    let mode = permissions.mode();

                    // 检查是否可写
                    if mode & 0o200 == 0 {
                        return Err(ValidationError::new(
                            "Extraction target directory is not writable",
                        ));
                    }
                }
            }
        } else {
            return Err(ValidationError::new("Parent directory does not exist"));
        }
    }

    Ok(())
}

/// 验证提取的文件名安全性
pub fn validate_extracted_filename(filename: &str) -> Result<(), ValidationError> {
    if filename.is_empty() {
        return Err(ValidationError::new("Filename cannot be empty"));
    }

    // Unicode规范化
    let normalized: String = filename.nfc().collect();
    if normalized != filename {
        return Err(ValidationError::new(
            "Filename contains non-normalized Unicode",
        ));
    }

    // 使用sanitize-filename清理文件名
    let sanitized = sanitize(filename);
    if sanitized != filename {
        return Err(ValidationError::new("Filename contains invalid characters"));
    }

    // 检查路径遍历
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return Err(ValidationError::new(
            "Filename contains path traversal sequences",
        ));
    }

    // 检查隐藏文件（可选，根据需求）
    if filename.starts_with('.') {
        tracing::warn!("Hidden file detected: {}", filename);
    }

    // 检查文件名长度
    if filename.len() > 255 {
        return Err(ValidationError::new(
            "Filename is too long (max 255 characters)",
        ));
    }

    Ok(())
}

/// 验证结果包装器
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ValidationResult<T> {
    pub data: T,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl<T> ValidationResult<T> {
    pub fn new(data: T) -> Self {
        Self {
            data,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn with_errors(mut self, errors: Vec<String>) -> Self {
        self.errors = errors;
        self
    }

    pub fn with_warnings(mut self, warnings: Vec<String>) -> Self {
        self.warnings = warnings;
        self
    }

    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }
}

/// 验证工具函数
pub fn validate_workspace_config(config: &ValidatedWorkspaceConfig) -> ValidationResult<()> {
    let mut result = ValidationResult::new(());

    match config.validate() {
        Ok(_) => {
            // 额外的业务逻辑验证
            if config.max_file_size > 100_000_000 && config.max_file_count > 10000 {
                result
                    .warnings
                    .push("Large file size and count limits may impact performance".to_string());
            }

            if config.tags.len() > 20 {
                result
                    .warnings
                    .push("Many tags may impact search performance".to_string());
            }
        }
        Err(errors) => {
            result.errors = errors
                .field_errors()
                .iter()
                .flat_map(|(field, errors)| {
                    let field_name = field.to_string();
                    errors.iter().map(move |e| {
                        format!(
                            "{}: {}",
                            field_name,
                            e.message.as_ref().unwrap_or(&"Invalid value".into())
                        )
                    })
                })
                .collect();
        }
    }

    result
}

/// 验证搜索查询
pub fn validate_search_query(query: &ValidatedSearchQuery) -> ValidationResult<()> {
    let mut result = ValidationResult::new(());

    match query.validate() {
        Ok(_) => {
            // 额外验证
            if query.use_regex {
                if let Err(_) = regex::Regex::new(&query.query) {
                    result
                        .errors
                        .push("Invalid regular expression syntax".to_string());
                }
            }

            if let (Some(start), Some(end)) = (&query.time_start, &query.time_end) {
                // 验证时间范围逻辑
                if start >= end {
                    result
                        .errors
                        .push("Start time must be before end time".to_string());
                }
            }

            if query.max_results.unwrap_or(1000) > 50000 {
                result
                    .warnings
                    .push("Large result sets may impact performance".to_string());
            }
        }
        Err(errors) => {
            result.errors = errors
                .field_errors()
                .iter()
                .flat_map(|(field, errors)| {
                    let field_name = field.to_string();
                    errors.iter().map(move |e| {
                        format!(
                            "{}: {}",
                            field_name,
                            e.message.as_ref().unwrap_or(&"Invalid value".into())
                        )
                    })
                })
                .collect();
        }
    }

    result
}

/// 验证压缩文件提取配置
pub fn validate_archive_extraction_config(
    config: &ValidatedArchiveExtractionConfig,
) -> ValidationResult<()> {
    let mut result = ValidationResult::new(());

    match config.validate() {
        Ok(_) => {
            // 额外的业务逻辑验证

            // 检查限制的合理性
            if config.max_file_size > config.max_total_size {
                result
                    .errors
                    .push("Max file size cannot exceed max total size".to_string());
            }

            // 检查文件数量和大小的组合是否合理
            let theoretical_min_total = config.max_file_count as u64 * 1024; // 假设每个文件至少1KB
            if theoretical_min_total > config.max_total_size {
                result
                    .warnings
                    .push("File count and size limits may be incompatible".to_string());
            }

            // 检查扩展名冲突
            for allowed_ext in &config.allowed_extensions {
                if config.forbidden_extensions.contains(allowed_ext) {
                    result.errors.push(format!(
                        "Extension '{}' is both allowed and forbidden",
                        allowed_ext
                    ));
                }
            }

            // 性能警告
            if config.max_file_count > 500 {
                result
                    .warnings
                    .push("Large file count may impact extraction performance".to_string());
            }

            if config.max_total_size > 500_000_000 {
                // 500MB
                result
                    .warnings
                    .push("Large total size may impact system performance".to_string());
            }

            // 安全警告
            if !config.validate_filenames {
                result
                    .warnings
                    .push("Filename validation is disabled - security risk".to_string());
            }

            if config.forbidden_extensions.is_empty() {
                result.warnings.push(
                    "No forbidden extensions specified - consider security implications"
                        .to_string(),
                );
            }
        }
        Err(errors) => {
            result.errors = errors
                .field_errors()
                .iter()
                .flat_map(|(field, errors)| {
                    let field_name = field.to_string();
                    errors.iter().map(move |e| {
                        format!(
                            "{}: {}",
                            field_name,
                            e.message.as_ref().unwrap_or(&"Invalid value".into())
                        )
                    })
                })
                .collect();
        }
    }

    result
}

/// 综合验证服务 - 处理复杂的嵌套验证场景
pub struct ValidationService {
    _max_validation_depth: usize,
    enable_warnings: bool,
}

impl ValidationService {
    pub fn new() -> Self {
        Self {
            _max_validation_depth: 10,
            enable_warnings: true,
        }
    }

    pub fn with_config(max_depth: usize, enable_warnings: bool) -> Self {
        Self {
            _max_validation_depth: max_depth,
            enable_warnings,
        }
    }

    /// 验证嵌套数据结构
    pub fn validate_nested<T: Validate>(&self, data: &T, context: &str) -> ValidationResult<()> {
        let mut result = ValidationResult::new(());

        match data.validate() {
            Ok(_) => {
                if self.enable_warnings {
                    result
                        .warnings
                        .push(format!("Validation passed for context: {}", context));
                }
            }
            Err(errors) => {
                result.errors = self.format_validation_errors(&errors, context);
            }
        }

        result
    }

    /// 批量验证多个对象
    pub fn validate_batch<T: Validate>(&self, items: &[T], context: &str) -> ValidationResult<()> {
        let mut result = ValidationResult::new(());

        for (index, item) in items.iter().enumerate() {
            let item_context = format!("{}[{}]", context, index);
            let item_result = self.validate_nested(item, &item_context);

            result.errors.extend(item_result.errors);
            if self.enable_warnings {
                result.warnings.extend(item_result.warnings);
            }
        }

        result
    }

    /// 格式化验证错误消息
    fn format_validation_errors(
        &self,
        errors: &validator::ValidationErrors,
        context: &str,
    ) -> Vec<String> {
        errors
            .field_errors()
            .iter()
            .flat_map(|(field, field_errors)| {
                field_errors.iter().map(move |error| {
                    let message = error
                        .message
                        .as_ref()
                        .map(|m| m.to_string())
                        .unwrap_or_else(|| "Invalid value".to_string());
                    format!("{}.{}: {}", context, field, message)
                })
            })
            .collect()
    }
}

impl Default for ValidationService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_config_validation() {
        let config = ValidatedWorkspaceConfig {
            workspace_id: "test-workspace".to_string(),
            name: "Test Workspace".to_string(),
            description: Some("A test workspace".to_string()),
            path: "/valid/path".to_string(),
            max_file_size: 1000000,
            max_file_count: 1000,
            enable_watch: true,
            tags: vec!["test".to_string()],
            metadata: HashMap::new(),
            contact_email: None,
            project_url: None,
        };

        let result = validate_workspace_config(&config);
        assert!(result.is_valid());
    }

    #[test]
    fn test_invalid_workspace_id() {
        let config = ValidatedWorkspaceConfig {
            workspace_id: "invalid/id".to_string(), // 包含无效字符
            name: "Test".to_string(),
            description: None,
            path: "/valid/path".to_string(),
            max_file_size: 1000,
            max_file_count: 10,
            enable_watch: false,
            tags: vec![],
            metadata: HashMap::new(),
            contact_email: None,
            project_url: None,
        };

        let result = validate_workspace_config(&config);
        assert!(!result.is_valid());
        // 错误消息格式为 "field_name: error_message"
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.contains("workspace_id") || e.contains("Invalid")),
            "Expected error about workspace_id, got: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_path_traversal_validation() {
        assert!(validate_workspace_path("../etc/passwd").is_err());
        assert!(validate_workspace_path("valid/path").is_ok());
        assert!(validate_workspace_path("").is_err());
    }

    #[test]
    fn test_timestamp_validation() {
        assert!(validate_timestamp("2023-01-01T00:00:00Z").is_ok());
        assert!(validate_timestamp("1640995200").is_ok()); // Unix timestamp
        assert!(validate_timestamp("invalid-timestamp").is_err());
        assert!(validate_timestamp("").is_ok()); // Empty is allowed
    }

    #[test]
    fn test_search_query_validation() {
        let query = ValidatedSearchQuery {
            query: "test search".to_string(),
            workspace_id: "test-workspace".to_string(),
            max_results: Some(1000),
            case_sensitive: false,
            use_regex: false,
            file_pattern: None,
            time_start: None,
            time_end: None,
            log_levels: vec!["INFO".to_string()],
            priority: Some(5),
            timeout_seconds: Some(30),
        };

        let result = validate_search_query(&query);
        assert!(result.is_valid());
    }

    #[test]
    fn test_regex_validation() {
        let query = ValidatedSearchQuery {
            query: "[invalid regex".to_string(),
            workspace_id: "test-workspace".to_string(),
            max_results: Some(100),
            case_sensitive: false,
            use_regex: true, // 启用正则表达式验证
            file_pattern: None,
            time_start: None,
            time_end: None,
            log_levels: vec![],
            priority: Some(5),
            timeout_seconds: Some(30),
        };

        let result = validate_search_query(&query);
        assert!(!result.is_valid());
        assert!(result
            .errors
            .iter()
            .any(|e| e.contains("Invalid regular expression")));
    }

    #[test]
    fn test_archive_extraction_config_validation() {
        let config = ValidatedArchiveExtractionConfig {
            archive_path: "/path/to/test.zip".to_string(),
            target_path: "/path/to/extract".to_string(),
            max_file_size: 10_000_000,   // 10MB
            max_total_size: 100_000_000, // 100MB
            max_file_count: 100,
            allow_overwrite: false,
            validate_filenames: true,
            allowed_extensions: vec!["txt".to_string(), "log".to_string()],
            forbidden_extensions: vec!["exe".to_string(), "bat".to_string()],
        };

        // 注意：这个测试会失败，因为路径不存在，但我们主要测试结构验证
        let result = validate_archive_extraction_config(&config);
        // 应该有错误，因为文件路径不存在
        assert!(!result.is_valid());
    }

    #[test]
    fn test_filename_validation() {
        // 有效文件名
        assert!(validate_extracted_filename("valid_file.txt").is_ok());
        assert!(validate_extracted_filename("file-123.log").is_ok());

        // 无效文件名
        assert!(validate_extracted_filename("").is_err());
        assert!(validate_extracted_filename("../etc/passwd").is_err());
        assert!(validate_extracted_filename("file\\with\\backslash").is_err());
        assert!(validate_extracted_filename("file/with/slash").is_err());

        // 过长的文件名
        let long_name = "a".repeat(300);
        assert!(validate_extracted_filename(&long_name).is_err());
    }

    #[test]
    fn test_unicode_normalization() {
        // 测试Unicode规范化
        let non_normalized = "café"; // 可能包含组合字符
        let normalized: String = non_normalized.chars().nfc().collect();

        // 如果字符串已经规范化，应该通过验证
        assert!(validate_extracted_filename(&normalized).is_ok());
    }
}
