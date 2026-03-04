//! 搜索领域服务
//!
//! 提供搜索相关的领域服务

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;

use super::entities::SearchResult;
use super::value_objects::{SearchMode, SearchQuery};

/// 搜索策略接口
///
/// 定义不同的搜索策略实现
#[async_trait]
pub trait SearchStrategy: Send + Sync {
    /// 策略名称
    fn name(&self) -> &str;

    /// 执行搜索
    async fn search(
        &self,
        query: &SearchQuery,
        content: &str,
    ) -> Result<Vec<SearchResult>, SearchStrategyError>;

    /// 是否支持该搜索模式
    fn supports_mode(&self, mode: SearchMode) -> bool;

    /// 估计结果数量
    fn estimate_results(&self, query: &SearchQuery, content_size: usize) -> usize;
}

/// 搜索策略错误
#[derive(Debug, Clone, thiserror::Error)]
pub enum SearchStrategyError {
    #[error("搜索失败: {0}")]
    SearchFailed(String),

    #[error("不支持的搜索模式: {0}")]
    UnsupportedMode(String),

    #[error("正则表达式错误: {0}")]
    RegexError(String),

    #[error("内容解析错误: {0}")]
    ParseError(String),
}

/// 搜索策略评估器
///
/// 根据查询特征选择最佳搜索策略
pub struct SearchStrategyEvaluator {
    strategies: Vec<Arc<dyn SearchStrategy>>,
}

impl SearchStrategyEvaluator {
    /// 创建策略评估器
    pub fn new() -> Self {
        Self {
            strategies: Vec::new(),
        }
    }

    /// 注册策略
    pub fn register(&mut self, strategy: Arc<dyn SearchStrategy>) {
        self.strategies.push(strategy);
    }

    /// 选择最佳策略
    pub fn select_best(&self, query: &SearchQuery) -> Option<Arc<dyn SearchStrategy>> {
        // 按优先级和模式支持度选择
        self.strategies
            .iter()
            .filter(|s| s.supports_mode(query.mode()))
            .max_by_key(|s| (query.priority().value(), s.name().len()))
            .cloned()
    }

    /// 获取所有策略
    pub fn available_strategies(&self) -> Vec<&str> {
        self.strategies.iter().map(|s| s.name()).collect()
    }
}

impl Default for SearchStrategyEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

/// 搜索结果聚合器
///
/// 聚合多个搜索源的结果
pub struct SearchAggregator {
    /// 最大结果数
    max_results: usize,
    /// 最小分数阈值
    min_score: f32,
    /// 是否去重
    deduplicate: bool,
}

impl SearchAggregator {
    /// 创建聚合器
    pub fn new() -> Self {
        Self {
            max_results: 1000,
            min_score: 0.0,
            deduplicate: true,
        }
    }

    /// 设置最大结果数
    pub fn with_max_results(mut self, max: usize) -> Self {
        self.max_results = max;
        self
    }

    /// 设置最小分数
    pub fn with_min_score(mut self, score: f32) -> Self {
        self.min_score = score;
        self
    }

    /// 设置是否去重
    pub fn with_deduplication(mut self, dedup: bool) -> Self {
        self.deduplicate = dedup;
        self
    }

    /// 聚合结果
    pub fn aggregate(&self, results: Vec<Vec<SearchResult>>) -> AggregatedResults {
        let mut all_results: Vec<SearchResult> = results.into_iter().flatten().collect();

        // 过滤低分结果
        all_results.retain(|r| r.score >= self.min_score);

        // 去重
        if self.deduplicate {
            let mut seen = std::collections::HashSet::new();
            all_results.retain(|r| {
                let key = format!("{}:{}", r.source_file, r.line_number);
                seen.insert(key)
            });
        }

        // 按分数排序
        all_results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // 截断
        let total_count = all_results.len();
        all_results.truncate(self.max_results);

        // 统计来源分布
        let source_distribution = self.calculate_source_distribution(&all_results);

        AggregatedResults {
            results: all_results,
            total_count,
            source_distribution,
        }
    }

    /// 计算来源分布
    fn calculate_source_distribution(&self, results: &[SearchResult]) -> HashMap<String, usize> {
        let mut distribution = HashMap::new();
        for result in results {
            *distribution.entry(result.source_file.clone()).or_insert(0) += 1;
        }
        distribution
    }
}

impl Default for SearchAggregator {
    fn default() -> Self {
        Self::new()
    }
}

/// 聚合结果
#[derive(Debug)]
pub struct AggregatedResults {
    /// 结果列表
    pub results: Vec<SearchResult>,
    /// 总数量（截断前）
    pub total_count: usize,
    /// 来源分布
    pub source_distribution: HashMap<String, usize>,
}

impl AggregatedResults {
    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.results.is_empty()
    }

    /// 结果数量
    pub fn len(&self) -> usize {
        self.results.len()
    }

    /// 是否被截断
    pub fn is_truncated(&self) -> bool {
        self.total_count > self.results.len()
    }

    /// 获取前 N 个结果
    pub fn top(&self, n: usize) -> &[SearchResult] {
        let end = n.min(self.results.len());
        &self.results[..end]
    }
}

// ==================== 基础搜索策略实现 ====================

/// 精确匹配策略
pub struct ExactMatchStrategy;

impl ExactMatchStrategy {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ExactMatchStrategy {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SearchStrategy for ExactMatchStrategy {
    fn name(&self) -> &str {
        "exact"
    }

    async fn search(
        &self,
        query: &SearchQuery,
        content: &str,
    ) -> Result<Vec<SearchResult>, SearchStrategyError> {
        let search_term = query.text();
        let mut results = Vec::new();

        for (line_num, line) in content.lines().enumerate() {
            if let Some(pos) = line.find(search_term) {
                let mut result =
                    SearchResult::new(line_num + 1, line.to_string(), "unknown".to_string());
                result.add_highlight(pos, pos + search_term.len());
                results.push(result);
            }
        }

        Ok(results)
    }

    fn supports_mode(&self, mode: SearchMode) -> bool {
        matches!(mode, SearchMode::Exact | SearchMode::Fuzzy)
    }

    fn estimate_results(&self, query: &SearchQuery, content_size: usize) -> usize {
        // 粗略估计：每 100 字节可能有一个匹配
        content_size / 100.max(query.text().len())
    }
}

/// 模糊匹配策略
pub struct FuzzyMatchStrategy {
    case_insensitive: bool,
}

impl FuzzyMatchStrategy {
    pub fn new(case_insensitive: bool) -> Self {
        Self { case_insensitive }
    }
}

impl Default for FuzzyMatchStrategy {
    fn default() -> Self {
        Self::new(true)
    }
}

#[async_trait]
impl SearchStrategy for FuzzyMatchStrategy {
    fn name(&self) -> &str {
        "fuzzy"
    }

    async fn search(
        &self,
        query: &SearchQuery,
        content: &str,
    ) -> Result<Vec<SearchResult>, SearchStrategyError> {
        let search_term = if self.case_insensitive {
            query.text().to_lowercase()
        } else {
            query.text().to_string()
        };

        let mut results = Vec::new();

        for (line_num, line) in content.lines().enumerate() {
            let search_line = if self.case_insensitive {
                line.to_lowercase()
            } else {
                line.to_string()
            };

            if let Some(pos) = search_line.find(&search_term) {
                let mut result =
                    SearchResult::new(line_num + 1, line.to_string(), "unknown".to_string());
                result.add_highlight(pos, pos + search_term.len());
                results.push(result);
            }
        }

        Ok(results)
    }

    fn supports_mode(&self, mode: SearchMode) -> bool {
        matches!(mode, SearchMode::Fuzzy)
    }

    fn estimate_results(&self, query: &SearchQuery, content_size: usize) -> usize {
        content_size / 80.max(query.text().len())
    }
}

// ==================== 单元测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_aggregator_basic() {
        let aggregator = SearchAggregator::new();

        let results1 = vec![SearchResult::new(
            1,
            "error 1".to_string(),
            "file1.log".to_string(),
        )];
        let results2 = vec![SearchResult::new(
            2,
            "error 2".to_string(),
            "file2.log".to_string(),
        )];

        let aggregated = aggregator.aggregate(vec![results1, results2]);

        assert_eq!(aggregated.len(), 2);
        assert!(!aggregated.is_empty());
    }

    #[test]
    fn test_search_aggregator_deduplication() {
        let aggregator = SearchAggregator::new().with_deduplication(true);

        let mut result1 = SearchResult::new(1, "error".to_string(), "file.log".to_string());
        result1.id = "same-id".to_string();

        let mut result2 = SearchResult::new(1, "error".to_string(), "file.log".to_string());
        result2.id = "same-id".to_string();

        let aggregated = aggregator.aggregate(vec![vec![result1], vec![result2]]);

        // 应该去重
        assert_eq!(aggregated.len(), 1);
    }

    #[test]
    fn test_search_aggregator_score_filter() {
        let aggregator = SearchAggregator::new().with_min_score(50.0);

        let mut high_score = SearchResult::new(1, "match".to_string(), "file.log".to_string());
        high_score.set_score(80.0);

        let mut low_score = SearchResult::new(2, "weak match".to_string(), "file.log".to_string());
        low_score.set_score(30.0);

        let aggregated = aggregator.aggregate(vec![vec![high_score, low_score]]);

        assert_eq!(aggregated.len(), 1);
        assert_eq!(aggregated.results[0].score, 80.0);
    }

    #[test]
    fn test_search_aggregator_max_results() {
        let aggregator = SearchAggregator::new().with_max_results(2);

        let results: Vec<SearchResult> = (1..=5)
            .map(|i| SearchResult::new(i, format!("error {}", i), "file.log".to_string()))
            .collect();

        let aggregated = aggregator.aggregate(vec![results]);

        assert_eq!(aggregated.len(), 2);
        assert_eq!(aggregated.total_count, 5);
        assert!(aggregated.is_truncated());
    }

    #[test]
    fn test_search_aggregator_top() {
        let aggregator = SearchAggregator::new();

        let results: Vec<SearchResult> = (1..=10)
            .map(|i| {
                let mut r = SearchResult::new(i, format!("error {}", i), "file.log".to_string());
                r.set_score(i as f32 * 10.0);
                r
            })
            .collect();

        let aggregated = aggregator.aggregate(vec![results]);

        let top3 = aggregated.top(3);
        assert_eq!(top3.len(), 3);
        // 按分数降序，应该是最高的三个
    }

    #[test]
    fn test_exact_match_strategy() {
        let strategy = ExactMatchStrategy::new();
        let _query = SearchQuery::with_mode("error".to_string(), SearchMode::Exact);

        assert!(strategy.supports_mode(SearchMode::Exact));
        assert!(strategy.supports_mode(SearchMode::Fuzzy));
        assert!(!strategy.supports_mode(SearchMode::Regex));
    }

    #[test]
    fn test_fuzzy_match_strategy() {
        let strategy = FuzzyMatchStrategy::new(true);
        let _query = SearchQuery::with_mode("ERROR".to_string(), SearchMode::Fuzzy);

        assert!(strategy.supports_mode(SearchMode::Fuzzy));
        assert!(!strategy.supports_mode(SearchMode::Exact));
    }

    #[test]
    fn test_strategy_evaluator() {
        let mut evaluator = SearchStrategyEvaluator::new();

        evaluator.register(Arc::new(ExactMatchStrategy::new()));
        evaluator.register(Arc::new(FuzzyMatchStrategy::new(true)));

        let strategies = evaluator.available_strategies();
        assert_eq!(strategies.len(), 2);

        let query = SearchQuery::with_mode("test".to_string(), SearchMode::Fuzzy);
        let selected = evaluator.select_best(&query);
        assert!(selected.is_some());
    }

    #[tokio::test]
    async fn test_exact_match_search() {
        let strategy = ExactMatchStrategy::new();
        let query = SearchQuery::with_mode("error".to_string(), SearchMode::Exact);

        let content = "line 1: info message\nline 2: error occurred\nline 3: another error";
        let results = strategy.search(&query, content).await.unwrap();

        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_fuzzy_match_search() {
        let strategy = FuzzyMatchStrategy::new(true);
        let query = SearchQuery::with_mode("ERROR".to_string(), SearchMode::Fuzzy);

        let content = "line 1: info\nline 2: error message";
        let results = strategy.search(&query, content).await.unwrap();

        assert_eq!(results.len(), 1);
        assert!(results[0].has_highlights());
    }
}
