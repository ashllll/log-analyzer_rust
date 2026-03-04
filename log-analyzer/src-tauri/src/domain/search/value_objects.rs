//! 搜索领域值对象
//!
//! 定义搜索相关的不可变值对象

use std::fmt;
use std::str::FromStr;
use thiserror::Error;

/// 搜索查询值对象
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SearchQuery {
    /// 原始查询文本
    text: String,
    /// 搜索模式
    mode: SearchMode,
    /// 优先级
    priority: SearchPriority,
}

impl SearchQuery {
    /// 创建新的搜索查询
    pub fn new(text: String) -> Self {
        Self {
            text,
            mode: SearchMode::default(),
            priority: SearchPriority::default(),
        }
    }

    /// 使用指定模式创建查询
    pub fn with_mode(text: String, mode: SearchMode) -> Self {
        Self {
            text,
            mode,
            priority: SearchPriority::default(),
        }
    }

    /// 使用指定模式和优先级创建查询
    pub fn with_options(text: String, mode: SearchMode, priority: SearchPriority) -> Self {
        Self {
            text,
            mode,
            priority,
        }
    }

    /// 获取查询文本
    pub fn text(&self) -> &str {
        &self.text
    }

    /// 获取搜索模式
    pub fn mode(&self) -> SearchMode {
        self.mode
    }

    /// 获取优先级
    pub fn priority(&self) -> SearchPriority {
        self.priority
    }

    /// 检查查询是否为空
    pub fn is_empty(&self) -> bool {
        self.text.trim().is_empty()
    }

    /// 获取查询长度
    pub fn len(&self) -> usize {
        self.text.len()
    }

    /// 是否大小写敏感
    pub fn is_case_sensitive(&self) -> bool {
        matches!(self.mode, SearchMode::Exact)
    }

    /// 是否使用正则表达式
    pub fn is_regex(&self) -> bool {
        matches!(self.mode, SearchMode::Regex)
    }
}

impl fmt::Display for SearchQuery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.text)
    }
}

impl From<String> for SearchQuery {
    fn from(text: String) -> Self {
        Self::new(text)
    }
}

impl From<&str> for SearchQuery {
    fn from(text: &str) -> Self {
        Self::new(text.to_string())
    }
}

/// 搜索模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SearchMode {
    /// 精确匹配（大小写敏感）
    Exact,
    /// 模糊匹配（大小写不敏感）
    #[default]
    Fuzzy,
    /// 正则表达式
    Regex,
    /// 通配符匹配
    Wildcard,
}

impl SearchMode {
    /// 获取模式名称
    pub fn as_str(&self) -> &'static str {
        match self {
            SearchMode::Exact => "exact",
            SearchMode::Fuzzy => "fuzzy",
            SearchMode::Regex => "regex",
            SearchMode::Wildcard => "wildcard",
        }
    }

    /// 是否需要转义特殊字符
    pub fn needs_escaping(&self) -> bool {
        matches!(self, SearchMode::Fuzzy | SearchMode::Wildcard)
    }
}

impl fmt::Display for SearchMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for SearchMode {
    type Err = SearchModeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "exact" => Ok(SearchMode::Exact),
            "fuzzy" => Ok(SearchMode::Fuzzy),
            "regex" => Ok(SearchMode::Regex),
            "wildcard" => Ok(SearchMode::Wildcard),
            _ => Err(SearchModeError::InvalidMode(s.to_string())),
        }
    }
}

/// 搜索优先级
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SearchPriority {
    /// 低优先级（后台搜索）
    Low,
    /// 普通优先级
    #[default]
    Normal,
    /// 高优先级（用户交互）
    High,
    /// 实时优先级（即时响应）
    Realtime,
}

impl SearchPriority {
    /// 获取优先级数值（越大越优先）
    pub fn value(&self) -> u8 {
        match self {
            SearchPriority::Low => 1,
            SearchPriority::Normal => 5,
            SearchPriority::High => 10,
            SearchPriority::Realtime => 20,
        }
    }

    /// 获取超时时间（毫秒）
    pub fn timeout_ms(&self) -> u64 {
        match self {
            SearchPriority::Low => 30000,     // 30秒
            SearchPriority::Normal => 10000,  // 10秒
            SearchPriority::High => 5000,     // 5秒
            SearchPriority::Realtime => 1000, // 1秒
        }
    }
}

impl fmt::Display for SearchPriority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SearchPriority::Low => write!(f, "low"),
            SearchPriority::Normal => write!(f, "normal"),
            SearchPriority::High => write!(f, "high"),
            SearchPriority::Realtime => write!(f, "realtime"),
        }
    }
}

/// 搜索模式错误
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum SearchModeError {
    #[error("无效的搜索模式: {0}")]
    InvalidMode(String),
}

// ==================== 单元测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_query_creation() {
        let query = SearchQuery::new("error".to_string());
        assert_eq!(query.text(), "error");
        assert_eq!(query.mode(), SearchMode::Fuzzy);
        assert!(!query.is_empty());
    }

    #[test]
    fn test_search_query_with_mode() {
        let query = SearchQuery::with_mode("error".to_string(), SearchMode::Exact);
        assert!(query.is_case_sensitive());
        assert!(!query.is_regex());
    }

    #[test]
    fn test_search_query_regex_mode() {
        let query = SearchQuery::with_mode(r"\d+".to_string(), SearchMode::Regex);
        assert!(query.is_regex());
        assert!(!query.is_case_sensitive());
    }

    #[test]
    fn test_search_mode_from_str() {
        assert_eq!(SearchMode::from_str("exact").unwrap(), SearchMode::Exact);
        assert_eq!(SearchMode::from_str("FUZZY").unwrap(), SearchMode::Fuzzy);
        assert_eq!(SearchMode::from_str("regex").unwrap(), SearchMode::Regex);
        assert!(SearchMode::from_str("invalid").is_err());
    }

    #[test]
    fn test_search_priority_timeout() {
        assert_eq!(SearchPriority::Low.timeout_ms(), 30000);
        assert_eq!(SearchPriority::Normal.timeout_ms(), 10000);
        assert_eq!(SearchPriority::High.timeout_ms(), 5000);
        assert_eq!(SearchPriority::Realtime.timeout_ms(), 1000);
    }

    #[test]
    fn test_search_priority_ordering() {
        assert!(SearchPriority::Realtime.value() > SearchPriority::High.value());
        assert!(SearchPriority::High.value() > SearchPriority::Normal.value());
        assert!(SearchPriority::Normal.value() > SearchPriority::Low.value());
    }

    #[test]
    fn test_search_query_empty() {
        let empty_query = SearchQuery::new("".to_string());
        assert!(empty_query.is_empty());

        let whitespace_query = SearchQuery::new("   ".to_string());
        assert!(whitespace_query.is_empty());
    }

    #[test]
    fn test_search_query_from_string() {
        let query: SearchQuery = "test".into();
        assert_eq!(query.text(), "test");
    }
}
