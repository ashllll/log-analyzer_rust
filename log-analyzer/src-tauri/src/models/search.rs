use serde::{Deserialize, Serialize};

/// 搜索缓存键类型
/// 用于唯一标识搜索查询的缓存条目
pub type SearchCacheKey = (
    String,         // query
    String,         // workspace_id
    Option<String>, // time_start
    Option<String>, // time_end
    Vec<String>,    // levels
    Option<String>, // file_pattern
    bool,           // case_sensitive
    usize,          // max_results
    String,         // query_version
);

/**
 * 查询操作符
 */
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum QueryOperator {
    #[serde(rename = "AND")]
    And,
    #[serde(rename = "OR")]
    Or,
    #[serde(rename = "NOT")]
    Not,
}

/**
 * 关键词来源
 */
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TermSource {
    #[serde(rename = "user")]
    User,
    #[serde(rename = "preset")]
    Preset,
}

/**
 * 单个搜索条件
 */
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchTerm {
    pub id: String,
    pub value: String,
    pub operator: QueryOperator,
    pub source: TermSource,
    #[serde(rename = "presetGroupId")]
    pub preset_group_id: Option<String>,
    #[serde(rename = "isRegex")]
    pub is_regex: bool,
    pub priority: u32,
    pub enabled: bool,
    #[serde(rename = "caseSensitive")]
    pub case_sensitive: bool,
}

/**
 * 查询元数据
 */
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryMetadata {
    #[serde(rename = "createdAt")]
    pub created_at: i64,
    #[serde(rename = "lastModified")]
    pub last_modified: i64,
    #[serde(rename = "executionCount")]
    pub execution_count: u32,
    pub label: Option<String>,
}

/**
 * 时间范围
 */
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: Option<String>,
    pub end: Option<String>,
}

/**
 * 搜索过滤器
 */
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchFilters {
    pub levels: Option<Vec<String>>,
    #[serde(rename = "timeRange")]
    pub time_range: Option<TimeRange>,
    #[serde(rename = "filePattern")]
    pub file_pattern: Option<String>,
}

/**
 * 完整搜索查询
 */
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub id: String,
    pub terms: Vec<SearchTerm>,
    #[serde(rename = "globalOperator")]
    pub global_operator: QueryOperator,
    pub filters: Option<SearchFilters>,
    pub metadata: QueryMetadata,
}

/// 分页搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PagedSearchResult {
    /// 当前页的数据
    pub results: Vec<crate::models::LogEntry>,
    /// 总结果数
    pub total_count: usize,
    /// 当前页索引
    pub page_index: i32,
    /// 每页大小
    pub page_size: usize,
    /// 总页数
    pub total_pages: usize,
    /// 是否还有更多页
    pub has_more: bool,
    /// 搜索摘要
    pub summary: crate::models::SearchResultSummary,
    /// 搜索查询字符串（用于缓存匹配）
    pub query: String,
    /// 缓存的搜索ID
    pub search_id: String,
}

impl PagedSearchResult {
    /// 创建新的分页结果
    pub fn new(
        results: Vec<crate::models::LogEntry>,
        total_count: usize,
        page_index: i32,
        page_size: usize,
        summary: crate::models::SearchResultSummary,
        query: String,
        search_id: String,
    ) -> Self {
        let total_pages = (total_count as f64 / page_size as f64).ceil() as usize;
        let has_more = (page_index as usize + 1) < total_pages;

        Self {
            results,
            total_count,
            page_index,
            page_size,
            total_pages,
            has_more,
            summary,
            query,
            search_id,
        }
    }

    /// 获取指定范围的条目（用于分页）
    pub fn slice_results(entries: &[crate::models::LogEntry], page: i32, size: usize) -> Vec<crate::models::LogEntry> {
        if page < 0 || size == 0 {
            return entries.to_vec();
        }

        let start = (page as usize) * size;
        if start >= entries.len() {
            return vec![];
        }

        let end = (start + size).min(entries.len());
        entries[start..end].to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_deserialize() {
        let query = SearchQuery {
            id: "test-1".to_string(),
            terms: vec![],
            global_operator: QueryOperator::And,
            filters: None,
            metadata: QueryMetadata {
                created_at: 0,
                last_modified: 0,
                execution_count: 0,
                label: None,
            },
        };

        let json = serde_json::to_string(&query).unwrap();
        let deserialized: SearchQuery = serde_json::from_str(&json).unwrap();

        assert_eq!(query.id, deserialized.id);
        assert_eq!(query.terms.len(), deserialized.terms.len());
    }

    #[test]
    fn test_operator_serialization() {
        let json = serde_json::to_string(&QueryOperator::And).unwrap();
        assert_eq!(json, r#""AND""#);

        let json = serde_json::to_string(&QueryOperator::Or).unwrap();
        assert_eq!(json, r#""OR""#);

        let json = serde_json::to_string(&QueryOperator::Not).unwrap();
        assert_eq!(json, r#""NOT""#);
    }

    #[test]
    fn test_source_serialization() {
        let json = serde_json::to_string(&TermSource::User).unwrap();
        assert_eq!(json, r#""user""#);

        let json = serde_json::to_string(&TermSource::Preset).unwrap();
        assert_eq!(json, r#""preset""#);
    }

    #[test]
    fn test_search_term_with_all_fields() {
        let term = SearchTerm {
            id: "term-1".to_string(),
            value: "error".to_string(),
            operator: QueryOperator::And,
            source: TermSource::User,
            preset_group_id: Some("group-1".to_string()),
            is_regex: true,
            priority: 10,
            enabled: true,
            case_sensitive: false,
        };

        let json = serde_json::to_string(&term).unwrap();
        let deserialized: SearchTerm = serde_json::from_str(&json).unwrap();

        assert_eq!(term.id, deserialized.id);
        assert_eq!(term.value, deserialized.value);
        assert_eq!(term.priority, deserialized.priority);
    }
}
