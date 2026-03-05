//! 规格模式 (Specification Pattern)
//!
//! 封装业务规则，用于验证、查询和构建逻辑。
//! 遵循 DDD 原则，使业务规则可复用、可组合。

use chrono::{DateTime, Utc};

use crate::domain::log_analysis::entities::LogEntry;
use crate::domain::log_analysis::repositories::Workspace;
use crate::domain::log_analysis::value_objects::LogLevel;

/// 规格接口
///
/// 定义规格的基本操作
pub trait Specification<T> {
    /// 检查候选对象是否满足规格
    fn is_satisfied_by(&self, candidate: &T) -> bool;

    /// 与另一个规格取交集
    fn and<S: Specification<T>>(self, other: S) -> AndSpec<Self, S>
    where
        Self: Sized,
    {
        AndSpec {
            first: self,
            second: other,
        }
    }

    /// 与另一个规格取并集
    fn or<S: Specification<T>>(self, other: S) -> OrSpec<Self, S>
    where
        Self: Sized,
    {
        OrSpec {
            first: self,
            second: other,
        }
    }

    /// 取反规格
    fn not(self) -> NotSpec<Self>
    where
        Self: Sized,
    {
        NotSpec { spec: self }
    }
}

/// 与规格 - 两个规格都必须满足
pub struct AndSpec<F, S> {
    first: F,
    second: S,
}

impl<T, F: Specification<T>, S: Specification<T>> Specification<T> for AndSpec<F, S> {
    fn is_satisfied_by(&self, candidate: &T) -> bool {
        self.first.is_satisfied_by(candidate) && self.second.is_satisfied_by(candidate)
    }
}

/// 或规格 - 任一规格满足即可
pub struct OrSpec<F, S> {
    first: F,
    second: S,
}

impl<T, F: Specification<T>, S: Specification<T>> Specification<T> for OrSpec<F, S> {
    fn is_satisfied_by(&self, candidate: &T) -> bool {
        self.first.is_satisfied_by(candidate) || self.second.is_satisfied_by(candidate)
    }
}

/// 非规格 - 取反
pub struct NotSpec<S> {
    spec: S,
}

impl<T, S: Specification<T>> Specification<T> for NotSpec<S> {
    fn is_satisfied_by(&self, candidate: &T) -> bool {
        !self.spec.is_satisfied_by(candidate)
    }
}

// ==================== 日志条目规格 ====================

/// 日志级别规格
pub struct LogLevelSpecification {
    min_severity: u8,
}

impl LogLevelSpecification {
    pub fn new(min_severity: u8) -> Self {
        Self { min_severity }
    }

    pub fn error_or_above() -> Self {
        Self::new(LogLevel::Error.severity())
    }

    pub fn warn_or_above() -> Self {
        Self::new(LogLevel::Warn.severity())
    }
}

impl Specification<LogEntry> for LogLevelSpecification {
    fn is_satisfied_by(&self, candidate: &LogEntry) -> bool {
        candidate.level.severity() >= self.min_severity
    }
}

/// 时间范围规格
pub struct TimeRangeSpecification {
    start: DateTime<Utc>,
    end: DateTime<Utc>,
}

impl TimeRangeSpecification {
    pub fn new(start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        Self { start, end }
    }

    pub fn last_hours(hours: i64) -> Self {
        let end = Utc::now();
        let start = end - chrono::Duration::hours(hours);
        Self { start, end }
    }

    pub fn last_days(days: i64) -> Self {
        let end = Utc::now();
        let start = end - chrono::Duration::days(days);
        Self { start, end }
    }

    pub fn today() -> Self {
        let now = Utc::now();
        let start = now.date_naive().and_hms_opt(0, 0, 0).unwrap();
        let start = DateTime::from_naive_utc_and_offset(start, Utc);
        Self { start, end: now }
    }
}

impl Specification<LogEntry> for TimeRangeSpecification {
    fn is_satisfied_by(&self, candidate: &LogEntry) -> bool {
        let ts = candidate.timestamp.as_datetime();
        *ts >= self.start && *ts <= self.end
    }
}

/// 关键词匹配规格
pub struct KeywordSpecification {
    keywords: Vec<String>,
    case_sensitive: bool,
}

impl KeywordSpecification {
    pub fn new(keywords: Vec<String>, case_sensitive: bool) -> Self {
        Self {
            keywords,
            case_sensitive,
        }
    }

    pub fn single(keyword: String, case_sensitive: bool) -> Self {
        Self::new(vec![keyword], case_sensitive)
    }
}

impl Specification<LogEntry> for KeywordSpecification {
    fn is_satisfied_by(&self, candidate: &LogEntry) -> bool {
        let message = if self.case_sensitive {
            candidate.message.as_str().to_string()
        } else {
            candidate.message.as_str().to_lowercase()
        };

        self.keywords.iter().any(|keyword| {
            let search_term = if self.case_sensitive {
                keyword.clone()
            } else {
                keyword.to_lowercase()
            };
            message.contains(&search_term)
        })
    }
}

/// 源文件规格
pub struct SourceFileSpecification {
    allowed_patterns: Vec<String>,
}

impl SourceFileSpecification {
    pub fn new(patterns: Vec<String>) -> Self {
        Self {
            allowed_patterns: patterns,
        }
    }

    pub fn single(pattern: String) -> Self {
        Self::new(vec![pattern])
    }
}

impl Specification<LogEntry> for SourceFileSpecification {
    fn is_satisfied_by(&self, candidate: &LogEntry) -> bool {
        if self.allowed_patterns.is_empty() {
            return true;
        }

        self.allowed_patterns.iter().any(|pattern| {
            if pattern.contains('*') {
                // 简单的通配符匹配
                let parts: Vec<&str> = pattern.split('*').collect();
                if parts.len() == 2 {
                    candidate.source_file.starts_with(parts[0])
                        && candidate.source_file.ends_with(parts[1])
                } else {
                    candidate.source_file.contains(&pattern.replace('*', ""))
                }
            } else {
                candidate.source_file == *pattern
            }
        })
    }
}

/// 标签规格
pub struct TagSpecification {
    required_tags: Vec<String>,
    match_all: bool,
}

impl TagSpecification {
    pub fn any(tags: Vec<String>) -> Self {
        Self {
            required_tags: tags,
            match_all: false,
        }
    }

    pub fn all(tags: Vec<String>) -> Self {
        Self {
            required_tags: tags,
            match_all: true,
        }
    }
}

impl Specification<LogEntry> for TagSpecification {
    fn is_satisfied_by(&self, candidate: &LogEntry) -> bool {
        if self.required_tags.is_empty() {
            return true;
        }

        if self.match_all {
            self.required_tags.iter().all(|tag| candidate.has_tag(tag))
        } else {
            self.required_tags.iter().any(|tag| candidate.has_tag(tag))
        }
    }
}

// ==================== 工作区规格 ====================

/// 工作区状态规格
pub struct WorkspaceStatusSpecification {
    allowed_statuses: Vec<String>,
}

impl WorkspaceStatusSpecification {
    pub fn new(statuses: Vec<String>) -> Self {
        Self {
            allowed_statuses: statuses,
        }
    }

    pub fn ready_only() -> Self {
        Self::new(vec!["READY".to_string()])
    }

    pub fn active() -> Self {
        Self::new(vec!["READY".to_string(), "SCANNING".to_string()])
    }
}

impl Specification<Workspace> for WorkspaceStatusSpecification {
    fn is_satisfied_by(&self, candidate: &Workspace) -> bool {
        self.allowed_statuses
            .contains(&candidate.status.as_str().to_string())
    }
}

/// 工作区路径规格（用于路径过滤）
pub struct WorkspacePathFilterSpecification {
    base_path: String,
}

impl WorkspacePathFilterSpecification {
    pub fn new(base_path: String) -> Self {
        Self { base_path }
    }

    pub fn within(path: &str) -> Self {
        Self::new(path.to_string())
    }
}

impl Specification<Workspace> for WorkspacePathFilterSpecification {
    fn is_satisfied_by(&self, candidate: &Workspace) -> bool {
        candidate.path.starts_with(&self.base_path)
    }
}

// ==================== 验证规格 ====================

/// 工作区名称验证规格
pub struct WorkspaceNameSpecification {}

impl WorkspaceNameSpecification {
    pub fn new() -> Self {
        Self {}
    }

    /// 验证名称是否有效
    pub fn validate(name: &str) -> Result<(), String> {
        if name.is_empty() {
            return Err("工作区名称不能为空".to_string());
        }

        if name.len() > 100 {
            return Err("工作区名称不能超过100个字符".to_string());
        }

        // 检查非法字符
        let invalid_chars = ['/', '\\', ':', '*', '?', '"', '<', '>', '|'];
        for ch in invalid_chars {
            if name.contains(ch) {
                return Err(format!("工作区名称包含非法字符: {}", ch));
            }
        }

        Ok(())
    }
}

impl Default for WorkspaceNameSpecification {
    fn default() -> Self {
        Self::new()
    }
}

impl Specification<Workspace> for WorkspaceNameSpecification {
    fn is_satisfied_by(&self, candidate: &Workspace) -> bool {
        WorkspaceNameSpecification::validate(&candidate.name).is_ok()
    }
}

/// 工作区路径验证规格
pub struct WorkspacePathValidationSpecification {}

impl WorkspacePathValidationSpecification {
    pub fn new() -> Self {
        Self {}
    }

    /// 验证路径是否有效
    pub fn validate(path: &str) -> Result<(), String> {
        if path.is_empty() {
            return Err("工作区路径不能为空".to_string());
        }

        // 检查路径遍历攻击
        if path.contains("..") {
            return Err("工作区路径不能包含 '..'".to_string());
        }

        // 路径长度检查
        if path.len() > 500 {
            return Err("工作区路径过长".to_string());
        }

        Ok(())
    }
}

impl Default for WorkspacePathValidationSpecification {
    fn default() -> Self {
        Self::new()
    }
}

impl Specification<Workspace> for WorkspacePathValidationSpecification {
    fn is_satisfied_by(&self, candidate: &Workspace) -> bool {
        WorkspacePathValidationSpecification::validate(&candidate.path).is_ok()
    }
}

/// 搜索查询验证规格
pub struct SearchQuerySpecification {
    min_length: usize,
    max_length: usize,
}

impl SearchQuerySpecification {
    pub fn new() -> Self {
        Self {
            min_length: 1,
            max_length: 1000,
        }
    }

    pub fn with_limits(min: usize, max: usize) -> Self {
        Self {
            min_length: min,
            max_length: max,
        }
    }

    /// 验证查询是否有效
    pub fn validate(&self, query: &str) -> Result<(), String> {
        let trimmed = query.trim();

        if trimmed.len() < self.min_length {
            return Err(format!("搜索查询至少需要 {} 个字符", self.min_length));
        }

        if trimmed.len() > self.max_length {
            return Err(format!("搜索查询不能超过 {} 个字符", self.max_length));
        }

        // 检查是否只包含空白字符
        if trimmed.chars().all(|c| c.is_whitespace()) {
            return Err("搜索查询不能只包含空白字符".to_string());
        }

        Ok(())
    }
}

impl Default for SearchQuerySpecification {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_entry(level: LogLevel, message: &str) -> LogEntry {
        LogEntry::new(
            Utc::now(),
            level,
            message.to_string(),
            "test.log".to_string(),
            1,
        )
    }

    #[test]
    fn test_log_level_specification() {
        let spec = LogLevelSpecification::error_or_above();

        let error_entry = create_test_entry(LogLevel::Error, "error message");
        let warn_entry = create_test_entry(LogLevel::Warn, "warn message");
        let info_entry = create_test_entry(LogLevel::Info, "info message");
        let fatal_entry = create_test_entry(LogLevel::Fatal, "fatal message");

        assert!(spec.is_satisfied_by(&error_entry));
        assert!(spec.is_satisfied_by(&fatal_entry)); // Fatal severity is 5 >= 4
        assert!(!spec.is_satisfied_by(&warn_entry)); // Warn severity is 3 < 4
        assert!(!spec.is_satisfied_by(&info_entry));
    }

    #[test]
    fn test_keyword_specification() {
        let spec = KeywordSpecification::new(vec!["error".to_string(), "fail".to_string()], false);

        let matching_entry = create_test_entry(LogLevel::Info, "This is an ERROR message");
        let non_matching_entry = create_test_entry(LogLevel::Info, "This is a success message");

        assert!(spec.is_satisfied_by(&matching_entry));
        assert!(!spec.is_satisfied_by(&non_matching_entry));
    }

    #[test]
    fn test_and_specification() {
        let level_spec = LogLevelSpecification::warn_or_above();
        let keyword_spec = KeywordSpecification::single("error".to_string(), false);

        let combined = level_spec.and(keyword_spec);

        let matching_entry = create_test_entry(LogLevel::Error, "This is an error");
        let level_ok_keyword_fail = create_test_entry(LogLevel::Error, "This is fine");
        let level_fail_keyword_ok = create_test_entry(LogLevel::Info, "This is an error");

        assert!(combined.is_satisfied_by(&matching_entry));
        assert!(!combined.is_satisfied_by(&level_ok_keyword_fail));
        assert!(!combined.is_satisfied_by(&level_fail_keyword_ok));
    }

    #[test]
    fn test_or_specification() {
        let error_spec = KeywordSpecification::single("error".to_string(), false);
        let warning_spec = KeywordSpecification::single("warning".to_string(), false);

        let combined = error_spec.or(warning_spec);

        let error_entry = create_test_entry(LogLevel::Info, "An error occurred");
        let warning_entry = create_test_entry(LogLevel::Info, "A warning was issued");
        let neither_entry = create_test_entry(LogLevel::Info, "All is well");

        assert!(combined.is_satisfied_by(&error_entry));
        assert!(combined.is_satisfied_by(&warning_entry));
        assert!(!combined.is_satisfied_by(&neither_entry));
    }

    #[test]
    fn test_not_specification() {
        let error_spec = KeywordSpecification::single("error".to_string(), false);
        let not_error = error_spec.not();

        let error_entry = create_test_entry(LogLevel::Info, "An error occurred");
        let success_entry = create_test_entry(LogLevel::Info, "All is well");

        assert!(!not_error.is_satisfied_by(&error_entry));
        assert!(not_error.is_satisfied_by(&success_entry));
    }

    #[test]
    fn test_time_range_specification() {
        let now = Utc::now();
        let spec = TimeRangeSpecification::new(
            now - chrono::Duration::hours(1),
            now + chrono::Duration::seconds(10),
        );

        // 当前时间的条目应该满足
        let recent_entry = LogEntry::new(
            now,
            LogLevel::Info,
            "recent".to_string(),
            "test.log".to_string(),
            1,
        );
        assert!(spec.is_satisfied_by(&recent_entry));

        // 创建一个旧条目（超出时间范围）
        let old_entry = LogEntry::new(
            now - chrono::Duration::hours(2),
            LogLevel::Info,
            "old message".to_string(),
            "test.log".to_string(),
            1,
        );
        assert!(!spec.is_satisfied_by(&old_entry));
    }

    #[test]
    fn test_source_file_specification() {
        let spec =
            SourceFileSpecification::new(vec!["test.log".to_string(), "error*.log".to_string()]);

        let matching_entry = create_test_entry(LogLevel::Info, "test");
        assert!(spec.is_satisfied_by(&matching_entry));

        // 测试不匹配的文件
        let spec2 = SourceFileSpecification::new(vec!["other.log".to_string()]);
        assert!(!spec2.is_satisfied_by(&matching_entry));
    }

    #[test]
    fn test_workspace_name_validation() {
        assert!(WorkspaceNameSpecification::validate("Valid Name").is_ok());
        assert!(WorkspaceNameSpecification::validate("生产环境日志").is_ok());
        assert!(WorkspaceNameSpecification::validate("").is_err());
        assert!(WorkspaceNameSpecification::validate("Name/With/Slash").is_err());
        assert!(WorkspaceNameSpecification::validate("Name\\With\\Backslash").is_err());
    }

    #[test]
    fn test_workspace_path_validation() {
        assert!(WorkspacePathValidationSpecification::validate("/valid/path").is_ok());
        assert!(WorkspacePathValidationSpecification::validate("C:\\valid\\path").is_ok());
        assert!(WorkspacePathValidationSpecification::validate("").is_err());
        assert!(WorkspacePathValidationSpecification::validate("/path/with/../traversal").is_err());
    }

    #[test]
    fn test_search_query_validation() {
        let spec = SearchQuerySpecification::new();

        assert!(spec.validate("valid query").is_ok());
        assert!(spec.validate("").is_err());
        assert!(spec.validate("   ").is_err());
        assert!(spec.validate(&"a".repeat(1001)).is_err());
    }

    #[test]
    fn test_tag_specification() {
        let any_spec = TagSpecification::any(vec!["important".to_string(), "urgent".to_string()]);
        let all_spec = TagSpecification::all(vec!["important".to_string(), "reviewed".to_string()]);

        let mut entry_with_important = create_test_entry(LogLevel::Info, "test");
        entry_with_important.add_tag("important".to_string());

        let mut entry_with_both = create_test_entry(LogLevel::Info, "test");
        entry_with_both.add_tag("important".to_string());
        entry_with_both.add_tag("reviewed".to_string());

        let entry_with_none = create_test_entry(LogLevel::Info, "test");

        assert!(any_spec.is_satisfied_by(&entry_with_important));
        assert!(any_spec.is_satisfied_by(&entry_with_both));
        assert!(!any_spec.is_satisfied_by(&entry_with_none));

        assert!(!all_spec.is_satisfied_by(&entry_with_important));
        assert!(all_spec.is_satisfied_by(&entry_with_both));
        assert!(!all_spec.is_satisfied_by(&entry_with_none));
    }
}
