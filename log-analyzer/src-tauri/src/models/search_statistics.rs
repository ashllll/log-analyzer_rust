use serde::{Deserialize, Serialize};

/// 关键词统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordStatistics {
    /// 关键词文本
    pub keyword: String,
    /// 该关键词匹配的行数
    #[serde(rename = "matchCount")]
    pub match_count: usize,
    /// 占总结果的百分比
    #[serde(rename = "matchPercentage")]
    pub match_percentage: f32,
}

impl KeywordStatistics {
    /// 创建新的关键词统计信息
    pub fn new(keyword: String, match_count: usize, total_matches: usize) -> Self {
        let match_percentage = if total_matches > 0 {
            (match_count as f32 / total_matches as f32) * 100.0
        } else {
            0.0
        };

        Self {
            keyword,
            match_count,
            match_percentage,
        }
    }
}

/// 搜索结果摘要信息
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchResultSummary {
    /// 总匹配行数
    #[serde(rename = "totalMatches")]
    pub total_matches: usize,
    /// 关键词统计数组
    #[serde(rename = "keywordStats")]
    pub keyword_stats: Vec<KeywordStatistics>,
    /// 搜索耗时(毫秒)
    #[serde(rename = "searchDurationMs")]
    pub search_duration_ms: u64,
    /// 是否因超限截断
    pub truncated: bool,
}

impl SearchResultSummary {
    /// 创建新的搜索结果摘要
    pub fn new(
        total_matches: usize,
        keyword_stats: Vec<KeywordStatistics>,
        search_duration_ms: u64,
        truncated: bool,
    ) -> Self {
        Self {
            total_matches,
            keyword_stats,
            search_duration_ms,
            truncated,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyword_statistics_creation() {
        let stats = KeywordStatistics::new("error".to_string(), 10, 100);
        assert_eq!(stats.keyword, "error");
        assert_eq!(stats.match_count, 10);
        assert_eq!(stats.match_percentage, 10.0);
    }

    #[test]
    fn test_keyword_statistics_zero_total() {
        let stats = KeywordStatistics::new("error".to_string(), 0, 0);
        assert_eq!(stats.match_count, 0);
        assert_eq!(stats.match_percentage, 0.0);
    }

    #[test]
    fn test_search_result_summary_creation() {
        let stats = vec![KeywordStatistics::new("error".to_string(), 10, 100)];
        let summary = SearchResultSummary::new(100, stats, 45, false);

        assert_eq!(summary.total_matches, 100);
        assert_eq!(summary.keyword_stats.len(), 1);
        assert_eq!(summary.search_duration_ms, 45);
        assert!(!summary.truncated);
    }

    #[test]
    fn test_search_result_summary_default() {
        let summary = SearchResultSummary::default();
        assert_eq!(summary.total_matches, 0);
        assert_eq!(summary.keyword_stats.len(), 0);
        assert_eq!(summary.search_duration_ms, 0);
        assert!(!summary.truncated);
    }
}
