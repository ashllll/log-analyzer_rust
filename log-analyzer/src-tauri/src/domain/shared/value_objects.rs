//! 通用值对象 (Value Objects)
//!
//! 遵循 DDD (Domain-Driven Design) 原则实现通用值对象：
//! - **不可变性**: 创建后不能修改
//! - **值相等**: 通过值而非标识比较
//! - **自我验证**: 创建时验证自身有效性
//! - **无副作用**: 操作返回新实例
//!
//! 这些值对象可在多个限界上下文中复用。

use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use thiserror::Error;

// ==================== 错误类型 ====================

/// 值对象验证错误
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum ValueError {
    /// 非空字符串为空
    #[error("字符串不能为空")]
    EmptyString,

    /// 字符串长度超出范围
    #[error("字符串长度 {actual} 超出允许范围 [{min}, {max}]")]
    StringLengthOutOfRange {
        min: usize,
        max: usize,
        actual: usize,
    },

    /// 电子邮件格式无效
    #[error("无效的电子邮件格式: {0}")]
    InvalidEmail(String),

    /// URL 格式无效
    #[error("无效的 URL 格式: {0}")]
    InvalidUrl(String),

    /// 文件路径无效
    #[error("无效的文件路径: {0}")]
    InvalidFilePath(String),

    /// 路径遍历攻击检测
    #[error("检测到路径遍历攻击: {0}")]
    PathTraversalDetected(String),

    /// 非正整数
    #[error("值 {0} 不是正整数")]
    NotPositiveInteger(i64),

    /// 整数超出范围
    #[error("整数 {actual} 超出允许范围 [{min}, {max}]")]
    IntegerOutOfRange { min: i64, max: i64, actual: i64 },
}

// ==================== 非空字符串 ====================

/// 非空字符串值对象
///
/// 确保字符串不为空，常用于名称、标题等必填字段。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NonEmptyString(String);

impl NonEmptyString {
    /// 创建非空字符串
    ///
    /// # Errors
    /// 如果字符串为空或只包含空白字符，返回 `ValueError::EmptyString`
    pub fn new(value: String) -> Result<Self, ValueError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(ValueError::EmptyString);
        }
        Ok(Self(trimmed.to_string()))
    }

    /// 从字符串切片创建
    pub fn try_from_str(value: &str) -> Result<Self, ValueError> {
        Self::new(value.to_string())
    }

    /// 获取字符串引用
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// 获取字符串长度
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// 检查是否为空（对于 NonEmptyString 永远返回 false）
    #[allow(clippy::len_without_is_empty)]
    pub fn is_empty(&self) -> bool {
        // NonEmptyString 经过验证永远不会为空
        // 此方法仅为了满足 clippy 警告，始终返回 false
        false
    }

    /// 检查是否包含子串
    pub fn contains(&self, pattern: &str) -> bool {
        self.0.contains(pattern)
    }

    /// 转换为小写（返回新实例）
    pub fn to_lowercase(&self) -> Self {
        Self(self.0.to_lowercase())
    }

    /// 转换为大写（返回新实例）
    pub fn to_uppercase(&self) -> Self {
        Self(self.0.to_uppercase())
    }
}

impl fmt::Display for NonEmptyString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for NonEmptyString {
    type Error = ValueError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for NonEmptyString {
    type Error = ValueError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::try_from_str(value)
    }
}

impl AsRef<str> for NonEmptyString {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

// ==================== 有界字符串 ====================

/// 有长度限制的字符串值对象
///
/// 确保字符串长度在指定范围内。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BoundedString {
    value: String,
    min_len: usize,
    max_len: usize,
}

impl BoundedString {
    /// 创建有界字符串
    ///
    /// # Errors
    /// 如果字符串长度超出范围，返回 `ValueError::StringLengthOutOfRange`
    pub fn new(value: String, min_len: usize, max_len: usize) -> Result<Self, ValueError> {
        let len = value.len();
        if len < min_len || len > max_len {
            return Err(ValueError::StringLengthOutOfRange {
                min: min_len,
                max: max_len,
                actual: len,
            });
        }
        Ok(Self {
            value,
            min_len,
            max_len,
        })
    }

    /// 创建带默认范围的有界字符串 (1-1000 字符)
    pub fn with_default_bounds(value: String) -> Result<Self, ValueError> {
        Self::new(value, 1, 1000)
    }

    /// 获取字符串引用
    pub fn as_str(&self) -> &str {
        &self.value
    }

    /// 获取长度
    pub fn len(&self) -> usize {
        self.value.len()
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    /// 获取最小长度限制
    pub fn min_len(&self) -> usize {
        self.min_len
    }

    /// 获取最大长度限制
    pub fn max_len(&self) -> usize {
        self.max_len
    }
}

impl fmt::Display for BoundedString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

// ==================== 电子邮件 ====================

/// 电子邮件值对象
///
/// 验证电子邮件格式的有效性。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Email(String);

impl Email {
    /// 创建电子邮件
    ///
    /// # Errors
    /// 如果格式无效，返回 `ValueError::InvalidEmail`
    pub fn new(value: String) -> Result<Self, ValueError> {
        let trimmed = value.trim().to_lowercase();

        // 基本验证：必须包含 @ 且格式正确
        if !Self::is_valid(&trimmed) {
            return Err(ValueError::InvalidEmail(value));
        }

        Ok(Self(trimmed))
    }

    /// 验证电子邮件格式
    fn is_valid(email: &str) -> bool {
        // 检查基本格式要求
        if email.is_empty() || email.len() > 254 {
            return false;
        }

        // 必须包含且只包含一个 @
        let at_count = email.matches('@').count();
        if at_count != 1 {
            return false;
        }

        let parts: Vec<&str> = email.split('@').collect();
        let local = parts[0];
        let domain = parts[1];

        // 本地部分验证
        if local.is_empty() || local.len() > 64 {
            return false;
        }

        // 域名部分验证
        if domain.is_empty() || domain.len() > 253 {
            return false;
        }

        // 域名必须包含至少一个点
        if !domain.contains('.') {
            return false;
        }

        // 检查顶级域名
        let domain_parts: Vec<&str> = domain.rsplit('.').collect();
        if domain_parts.is_empty() || domain_parts[0].len() < 2 {
            return false;
        }

        // 检查是否包含非法字符
        let valid_chars = |c: char| c.is_alphanumeric() || ".-_+".contains(c);
        if !local.chars().all(valid_chars) {
            return false;
        }

        true
    }

    /// 获取电子邮件字符串
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// 获取域名部分
    pub fn domain(&self) -> &str {
        self.0.split('@').nth(1).unwrap_or("")
    }

    /// 获取本地部分（@ 前）
    pub fn local_part(&self) -> &str {
        self.0.split('@').next().unwrap_or("")
    }
}

impl fmt::Display for Email {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for Email {
    type Error = ValueError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl FromStr for Email {
    type Err = ValueError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s.to_string())
    }
}

// ==================== URL ====================

/// URL 值对象
///
/// 验证 URL 格式的有效性。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Url {
    value: String,
    scheme: String,
    host: String,
}

impl Url {
    /// 支持的 URL 协议
    const ALLOWED_SCHEMES: [&'static str; 4] = ["http", "https", "ftp", "file"];

    /// 创建 URL
    ///
    /// # Errors
    /// 如果格式无效，返回 `ValueError::InvalidUrl`
    pub fn new(value: String) -> Result<Self, ValueError> {
        let trimmed = value.trim();

        if !Self::is_valid(trimmed) {
            return Err(ValueError::InvalidUrl(value));
        }

        let (scheme, host) = Self::parse_components(trimmed);

        Ok(Self {
            value: trimmed.to_string(),
            scheme,
            host,
        })
    }

    /// 验证 URL 格式
    fn is_valid(url: &str) -> bool {
        if url.is_empty() || url.len() > 2048 {
            return false;
        }

        // 检查协议
        let scheme_end = url.find("://");
        if scheme_end.is_none() {
            return false;
        }

        let scheme = &url[..scheme_end.unwrap()];
        if !Self::ALLOWED_SCHEMES.contains(&scheme) {
            return false;
        }

        // 检查主机部分
        let rest = &url[scheme_end.unwrap() + 3..];
        if rest.is_empty() {
            return false;
        }

        // 主机部分应该在第一个 / 或 ? 或 # 之前
        let host_end = rest
            .find('/')
            .or_else(|| rest.find('?'))
            .or_else(|| rest.find('#'))
            .unwrap_or(rest.len());

        let host = &rest[..host_end];
        if host.is_empty() {
            return false;
        }

        // 检查主机格式（可以是域名或 IP）
        if !host.contains('.') && !host.contains(':') && host != "localhost" {
            return false;
        }

        true
    }

    /// 解析 URL 组件
    fn parse_components(url: &str) -> (String, String) {
        let scheme_end = url.find("://").unwrap_or(0);
        let scheme = url[..scheme_end].to_string();

        let rest = &url[scheme_end + 3..];
        let host_end = rest
            .find('/')
            .or_else(|| rest.find('?'))
            .or_else(|| rest.find('#'))
            .unwrap_or(rest.len());

        let host = rest[..host_end].to_string();

        (scheme, host)
    }

    /// 获取 URL 字符串
    pub fn as_str(&self) -> &str {
        &self.value
    }

    /// 获取协议
    pub fn scheme(&self) -> &str {
        &self.scheme
    }

    /// 获取主机
    pub fn host(&self) -> &str {
        &self.host
    }

    /// 检查是否为 HTTPS
    pub fn is_https(&self) -> bool {
        self.scheme == "https"
    }

    /// 检查是否为本地地址
    pub fn is_localhost(&self) -> bool {
        self.host.starts_with("localhost") || self.host.starts_with("127.")
    }
}

impl fmt::Display for Url {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl TryFrom<String> for Url {
    type Error = ValueError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl FromStr for Url {
    type Err = ValueError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s.to_string())
    }
}

// ==================== 文件路径 ====================

/// 文件路径值对象
///
/// 验证文件路径的安全性和有效性。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FilePath {
    path: PathBuf,
    is_absolute: bool,
}

impl FilePath {
    /// 创建文件路径
    ///
    /// # Errors
    /// 如果路径无效或存在安全问题，返回相应的 `ValueError`
    pub fn new(path: String) -> Result<Self, ValueError> {
        let trimmed = path.trim();

        if trimmed.is_empty() {
            return Err(ValueError::InvalidFilePath("路径不能为空".to_string()));
        }

        // 检查路径长度
        if trimmed.len() > 4096 {
            return Err(ValueError::InvalidFilePath(
                "路径长度超过 4096 字符".to_string(),
            ));
        }

        // 检查路径遍历攻击
        if Self::has_path_traversal(trimmed) {
            return Err(ValueError::PathTraversalDetected(path));
        }

        // 检查非法字符（Windows）
        #[cfg(windows)]
        {
            let invalid_chars = ['<', '>', '"', '|', '?', '*'];
            for ch in invalid_chars {
                if trimmed.contains(ch) {
                    return Err(ValueError::InvalidFilePath(format!(
                        "路径包含非法字符: {}",
                        ch
                    )));
                }
            }
        }

        let path_buf = PathBuf::from(trimmed);
        let is_absolute = path_buf.is_absolute();

        Ok(Self {
            path: path_buf,
            is_absolute,
        })
    }

    /// 检查路径遍历攻击
    fn has_path_traversal(path: &str) -> bool {
        // 检查 ../ 和 ..\ 模式
        let normalized = path.replace('\\', "/");
        let components: Vec<&str> = normalized.split('/').collect();

        let mut depth = 0;
        for component in components {
            if component == ".." {
                if depth == 0 {
                    return true; // 试图跳出根目录
                }
                depth -= 1;
            } else if !component.is_empty() && component != "." {
                depth += 1;
            }
        }

        false
    }

    /// 获取路径字符串
    pub fn as_str(&self) -> String {
        self.path.to_string_lossy().to_string()
    }

    /// 获取 Path 引用
    pub fn as_path(&self) -> &Path {
        &self.path
    }

    /// 获取文件名
    pub fn file_name(&self) -> Option<&str> {
        self.path.file_name().and_then(|name| name.to_str())
    }

    /// 获取扩展名
    pub fn extension(&self) -> Option<&str> {
        self.path.extension().and_then(|ext| ext.to_str())
    }

    /// 获取父目录
    pub fn parent(&self) -> Option<FilePath> {
        self.path.parent().map(|p| FilePath {
            path: p.to_path_buf(),
            is_absolute: self.is_absolute,
        })
    }

    /// 检查是否为绝对路径
    pub fn is_absolute(&self) -> bool {
        self.is_absolute
    }

    /// 检查是否为相对路径
    pub fn is_relative(&self) -> bool {
        !self.is_absolute
    }

    /// 拼接子路径（返回新实例）
    pub fn join(&self, child: &str) -> Result<Self, ValueError> {
        // 验证子路径安全性
        if Self::has_path_traversal(child) {
            return Err(ValueError::PathTraversalDetected(child.to_string()));
        }

        let new_path = self.path.join(child);
        Self::new(new_path.to_string_lossy().to_string())
    }
}

impl fmt::Display for FilePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.path.display())
    }
}

impl TryFrom<String> for FilePath {
    type Error = ValueError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for FilePath {
    type Error = ValueError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value.to_string())
    }
}

impl AsRef<Path> for FilePath {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

// ==================== 正整数 ====================

/// 正整数值对象
///
/// 确保值为正整数（大于 0）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PositiveInteger(i64);

impl PositiveInteger {
    /// 创建正整数
    ///
    /// # Errors
    /// 如果值不是正整数，返回 `ValueError::NotPositiveInteger`
    pub fn new(value: i64) -> Result<Self, ValueError> {
        if value <= 0 {
            return Err(ValueError::NotPositiveInteger(value));
        }
        Ok(Self(value))
    }

    /// 创建带范围限制的正整数
    ///
    /// # Errors
    /// 如果值超出范围，返回 `ValueError::IntegerOutOfRange`
    pub fn with_bounds(value: i64, min: i64, max: i64) -> Result<Self, ValueError> {
        if value < min || value > max {
            return Err(ValueError::IntegerOutOfRange {
                min,
                max,
                actual: value,
            });
        }
        Self::new(value)
    }

    /// 获取值
    pub fn value(&self) -> i64 {
        self.0
    }

    /// 转换为 usize
    pub fn as_usize(&self) -> usize {
        self.0 as usize
    }

    /// 加法（返回新实例）
    pub fn add(&self, other: PositiveInteger) -> Self {
        Self(self.0 + other.0)
    }

    /// 减法（返回新实例，如果结果非正则返回 None）
    pub fn sub(&self, other: PositiveInteger) -> Option<Self> {
        let result = self.0 - other.0;
        if result > 0 {
            Some(Self(result))
        } else {
            None
        }
    }

    /// 乘法（返回新实例）
    pub fn mul(&self, other: PositiveInteger) -> Self {
        Self(self.0 * other.0)
    }
}

impl fmt::Display for PositiveInteger {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<i64> for PositiveInteger {
    type Error = ValueError;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<i32> for PositiveInteger {
    type Error = ValueError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        Self::new(i64::from(value))
    }
}

impl TryFrom<usize> for PositiveInteger {
    type Error = ValueError;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        // 尝试将 usize 转换为 i64
        if value > i64::MAX as usize {
            return Err(ValueError::NotPositiveInteger(-1)); // 使用 -1 表示溢出
        }
        Self::new(value as i64)
    }
}

impl From<PositiveInteger> for i64 {
    fn from(value: PositiveInteger) -> Self {
        value.0
    }
}

impl From<PositiveInteger> for usize {
    fn from(value: PositiveInteger) -> Self {
        value.0 as usize
    }
}

// ==================== 单元测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== NonEmptyString 测试 ====================

    #[test]
    fn test_non_empty_string_valid() {
        let nes = NonEmptyString::new("Hello, World!".to_string()).unwrap();
        assert_eq!(nes.as_str(), "Hello, World!");
        assert_eq!(nes.len(), 13);
    }

    #[test]
    fn test_non_empty_string_trims_whitespace() {
        let nes = NonEmptyString::new("  hello  ".to_string()).unwrap();
        assert_eq!(nes.as_str(), "hello");
    }

    #[test]
    fn test_non_empty_string_rejects_empty() {
        assert_eq!(
            NonEmptyString::new("".to_string()),
            Err(ValueError::EmptyString)
        );
        assert_eq!(
            NonEmptyString::new("   ".to_string()),
            Err(ValueError::EmptyString)
        );
    }

    #[test]
    fn test_non_empty_string_case_conversion() {
        let nes = NonEmptyString::new("Hello".to_string()).unwrap();
        assert_eq!(nes.to_lowercase().as_str(), "hello");
        assert_eq!(nes.to_uppercase().as_str(), "HELLO");
    }

    #[test]
    fn test_non_empty_string_try_from() {
        let nes = NonEmptyString::try_from_str("test").unwrap();
        assert_eq!(nes.as_str(), "test");

        let result = NonEmptyString::try_from_str("");
        assert!(result.is_err());
    }

    // ==================== BoundedString 测试 ====================

    #[test]
    fn test_bounded_string_valid() {
        let bs = BoundedString::new("Hello".to_string(), 1, 10).unwrap();
        assert_eq!(bs.as_str(), "Hello");
        assert_eq!(bs.len(), 5);
    }

    #[test]
    fn test_bounded_string_rejects_too_short() {
        let result = BoundedString::new("Hi".to_string(), 5, 10);
        assert_eq!(
            result,
            Err(ValueError::StringLengthOutOfRange {
                min: 5,
                max: 10,
                actual: 2
            })
        );
    }

    #[test]
    fn test_bounded_string_rejects_too_long() {
        let result = BoundedString::new("Hello World".to_string(), 1, 5);
        assert_eq!(
            result,
            Err(ValueError::StringLengthOutOfRange {
                min: 1,
                max: 5,
                actual: 11
            })
        );
    }

    #[test]
    fn test_bounded_string_bounds() {
        let bs = BoundedString::new("test".to_string(), 1, 100).unwrap();
        assert_eq!(bs.min_len(), 1);
        assert_eq!(bs.max_len(), 100);
    }

    // ==================== Email 测试 ====================

    #[test]
    fn test_email_valid() {
        let email = Email::new("user@example.com".to_string()).unwrap();
        assert_eq!(email.as_str(), "user@example.com");
        assert_eq!(email.local_part(), "user");
        assert_eq!(email.domain(), "example.com");
    }

    #[test]
    fn test_email_normalizes_case() {
        let email = Email::new("USER@Example.COM".to_string()).unwrap();
        assert_eq!(email.as_str(), "user@example.com");
    }

    #[test]
    fn test_email_rejects_invalid() {
        // 缺少 @
        assert!(Email::new("userexample.com".to_string()).is_err());

        // 缺少域名
        assert!(Email::new("user@".to_string()).is_err());

        // 缺少本地部分
        assert!(Email::new("@example.com".to_string()).is_err());

        // 缺少顶级域名
        assert!(Email::new("user@example".to_string()).is_err());

        // 空
        assert!(Email::new("".to_string()).is_err());
    }

    #[test]
    fn test_email_from_str() {
        let email: Email = "test@example.org".parse().unwrap();
        assert_eq!(email.as_str(), "test@example.org");
    }

    // ==================== Url 测试 ====================

    #[test]
    fn test_url_valid() {
        let url = Url::new("https://example.com/path".to_string()).unwrap();
        assert_eq!(url.as_str(), "https://example.com/path");
        assert_eq!(url.scheme(), "https");
        assert_eq!(url.host(), "example.com");
        assert!(url.is_https());
    }

    #[test]
    fn test_url_http() {
        let url = Url::new("http://example.com".to_string()).unwrap();
        assert_eq!(url.scheme(), "http");
        assert!(!url.is_https());
    }

    #[test]
    fn test_url_localhost() {
        let url = Url::new("http://localhost:8080/api".to_string()).unwrap();
        assert!(url.is_localhost());
    }

    #[test]
    fn test_url_rejects_invalid() {
        // 缺少协议
        assert!(Url::new("example.com".to_string()).is_err());

        // 不支持的协议
        assert!(Url::new("javascript:alert(1)".to_string()).is_err());

        // 空
        assert!(Url::new("".to_string()).is_err());
    }

    #[test]
    fn test_url_rejects_unsupported_scheme() {
        assert!(Url::new("ftp://files.example.com".to_string()).is_ok());
        assert!(Url::new("file://localhost/path/to/file".to_string()).is_ok());
        assert!(Url::new("javascript:void(0)".to_string()).is_err());
        assert!(Url::new("data:text/plain,hello".to_string()).is_err());
    }

    // ==================== FilePath 测试 ====================

    #[test]
    fn test_file_path_valid() {
        // 使用跨平台路径测试
        #[cfg(windows)]
        let path = FilePath::new(r"C:\Users\user\file.txt".to_string()).unwrap();
        #[cfg(not(windows))]
        let path = FilePath::new("/home/user/file.txt".to_string()).unwrap();

        assert_eq!(path.file_name(), Some("file.txt"));
        assert_eq!(path.extension(), Some("txt"));
        assert!(path.is_absolute());
    }

    #[test]
    fn test_file_path_relative() {
        let path = FilePath::new("documents/file.txt".to_string()).unwrap();
        assert!(path.is_relative());
        assert!(!path.is_absolute());
    }

    #[test]
    fn test_file_path_rejects_empty() {
        assert!(FilePath::new("".to_string()).is_err());
    }

    #[test]
    fn test_file_path_rejects_traversal() {
        // 试图跳出根目录
        assert!(FilePath::new("../../../etc/passwd".to_string()).is_err());
        // 跨平台路径遍历测试
        #[cfg(windows)]
        assert!(FilePath::new(r"C:\home\..\..\..\etc\passwd".to_string()).is_err());
        #[cfg(not(windows))]
        assert!(FilePath::new("/home/../../../etc/passwd".to_string()).is_err());
    }

    #[test]
    fn test_file_path_join() {
        #[cfg(windows)]
        let path = FilePath::new(r"C:\Users\user".to_string()).unwrap();
        #[cfg(not(windows))]
        let path = FilePath::new("/home/user".to_string()).unwrap();

        let joined = path.join("documents/file.txt").unwrap();
        assert!(joined.as_str().contains("documents"));
    }

    #[test]
    fn test_file_path_parent() {
        #[cfg(windows)]
        let path = FilePath::new(r"C:\Users\user\file.txt".to_string()).unwrap();
        #[cfg(not(windows))]
        let path = FilePath::new("/home/user/file.txt".to_string()).unwrap();

        let parent = path.parent().unwrap();
        assert!(parent.as_str().ends_with("user"));
    }

    // ==================== PositiveInteger 测试 ====================

    #[test]
    fn test_positive_integer_valid() {
        let pi = PositiveInteger::new(42).unwrap();
        assert_eq!(pi.value(), 42);
        assert_eq!(pi.as_usize(), 42);
    }

    #[test]
    fn test_positive_integer_rejects_zero() {
        assert_eq!(
            PositiveInteger::new(0),
            Err(ValueError::NotPositiveInteger(0))
        );
    }

    #[test]
    fn test_positive_integer_rejects_negative() {
        assert_eq!(
            PositiveInteger::new(-5),
            Err(ValueError::NotPositiveInteger(-5))
        );
    }

    #[test]
    fn test_positive_integer_with_bounds() {
        let pi = PositiveInteger::with_bounds(50, 1, 100).unwrap();
        assert_eq!(pi.value(), 50);

        assert!(PositiveInteger::with_bounds(0, 1, 100).is_err());
        assert!(PositiveInteger::with_bounds(101, 1, 100).is_err());
    }

    #[test]
    fn test_positive_integer_arithmetic() {
        let a = PositiveInteger::new(5).unwrap();
        let b = PositiveInteger::new(3).unwrap();

        assert_eq!(a.add(b).value(), 8);
        assert_eq!(a.sub(b).unwrap().value(), 2);
        assert!(b.sub(a).is_none()); // 结果非正
        assert_eq!(a.mul(b).value(), 15);
    }

    #[test]
    fn test_positive_integer_conversions() {
        let pi = PositiveInteger::new(42).unwrap();

        // Into traits
        let val: i64 = pi.into();
        assert_eq!(val, 42);

        let pi2 = PositiveInteger::new(42).unwrap();
        let val2: usize = pi2.into();
        assert_eq!(val2, 42);

        // TryFrom traits
        let from_i32: PositiveInteger = 10i32.try_into().unwrap();
        assert_eq!(from_i32.value(), 10);

        let from_usize: PositiveInteger = 20usize.try_into().unwrap();
        assert_eq!(from_usize.value(), 20);
    }

    // ==================== 值相等测试 ====================

    #[test]
    fn test_value_equality() {
        // 相同值的 NonEmptyString 应该相等
        let a = NonEmptyString::new("test".to_string()).unwrap();
        let b = NonEmptyString::new("test".to_string()).unwrap();
        assert_eq!(a, b);

        // 相同值的 Email 应该相等
        let email_a = Email::new("user@example.com".to_string()).unwrap();
        let email_b = Email::new("user@example.com".to_string()).unwrap();
        assert_eq!(email_a, email_b);

        // 相同值的 PositiveInteger 应该相等
        let int_a = PositiveInteger::new(42).unwrap();
        let int_b = PositiveInteger::new(42).unwrap();
        assert_eq!(int_a, int_b);
    }
}
