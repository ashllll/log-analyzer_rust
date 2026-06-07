use crate::services::query_planner::{ExecutionPlan, QueryPlanner};
use crate::services::query_validator::QueryValidator;
use la_core::error::Result;
use la_core::models::search::*;
use moka::sync::Cache;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// 辅助函数：哈希 QueryOperator
fn hash_query_operator(op: &QueryOperator, hasher: &mut DefaultHasher) {
    match op {
        QueryOperator::And => 0u8.hash(hasher),
        QueryOperator::Or => 1u8.hash(hasher),
        QueryOperator::Not => 2u8.hash(hasher),
    }
}

/// 公共函数: 计算查询缓存键
///
/// 统一 QueryExecutor 和 GenericQueryExecutor 的缓存键生成逻辑，
/// 确保等价的查询产生相同的缓存键。
///
/// 使用 DefaultHasher 替代 SHA-256，避免密码学安全哈希的额外开销。
/// 缓存键不需要抗碰撞性，DefaultHasher 的性能更优。
pub(crate) fn compute_query_cache_key(query: &SearchQuery) -> u64 {
    let mut hasher = DefaultHasher::new();

    // 哈希查询 ID 和全局操作符
    query.id.hash(&mut hasher);
    hash_query_operator(&query.global_operator, &mut hasher);

    // 哈希过滤器（如果存在）
    if let Some(filters) = &query.filters {
        if let Some(time_range) = &filters.time_range {
            time_range.start.hash(&mut hasher);
            time_range.end.hash(&mut hasher);
        }
        filters.levels.hash(&mut hasher);
        filters.file_pattern.hash(&mut hasher);
    }

    // 按 ID 排序以确保哈希一致性；只排序引用，避免克隆完整查询项
    let mut sorted_terms: Vec<_> = query.terms.iter().collect();
    sorted_terms.sort_unstable_by(|a, b| a.id.cmp(&b.id));

    // 哈希每个搜索项的所有关键字段
    for term in sorted_terms {
        term.id.hash(&mut hasher);
        term.value.hash(&mut hasher);
        hash_query_operator(&term.operator, &mut hasher);
        term.source.hash(&mut hasher);
        term.preset_group_id.hash(&mut hasher);
        term.is_regex.hash(&mut hasher);
        term.priority.hash(&mut hasher);
        term.enabled.hash(&mut hasher);
        term.case_sensitive.hash(&mut hasher);
    }

    hasher.finish()
}

/**
 * 查询计划构建器
 *
 * 职责：验证查询、构建执行计划、缓存计划。
 * 匹配逻辑已移入 `ExecutionPlan` 自身（query_planner 模块），
 * 调用方通过 `plan.matches_line()` / `plan.match_with_details()` 直接使用。
 *
 * 设计决策：使用混合引擎策略自动选择最佳匹配算法：
 * - AhoCorasick: 简单关键词搜索，O(n) 线性复杂度
 * - Standard: 复杂正则/需要前瞻后瞻
 *
 * 性能优化：使用 ExecutionPlan 缓存避免重复构建查询计划
 */
pub struct QueryPlanBuilder {
    planner: QueryPlanner,
    /// ExecutionPlan 缓存 (LRU策略)
    plan_cache: Cache<u64, Arc<ExecutionPlan>>,
}

impl QueryPlanBuilder {
    /**
     * 创建新的计划构建器
     *
     * # 参数
     * * `cache_size` - 引擎缓存大小
     * * `plan_cache_size` - ExecutionPlan 缓存大小
     */
    pub fn new(cache_size: usize) -> Self {
        Self {
            planner: QueryPlanner::new(cache_size),
            plan_cache: Cache::new(1000), // 默认缓存1000个查询计划
        }
    }

    /**
     * 生成查询缓存键
     *
     * 委托给公共函数 compute_query_cache_key，确保与 GenericQueryPlanBuilder 一致性
     */
    fn cache_key(query: &SearchQuery) -> u64 {
        compute_query_cache_key(query)
    }

    /**
     * 构建查询计划
     *
     * # 参数
     * * `query` - 搜索查询
     *
     * # 返回
     * * `Ok(ExecutionPlan)` - 执行计划
     * * `Err(AppError)` - 构建失败
     */
    pub fn build(&mut self, query: &SearchQuery) -> Result<ExecutionPlan> {
        QueryValidator::validate(query)?;

        // 检查缓存
        let cache_key = Self::cache_key(query);
        if let Some(cached_plan) = self.plan_cache.get(&cache_key) {
            // 返回缓存的计划的副本
            return Ok((*cached_plan).clone());
        }

        let mut plan = self.planner.build_plan(query)?;
        plan.sort_by_priority();

        // 缓存计划
        self.plan_cache.insert(cache_key, Arc::new(plan.clone()));

        Ok(plan)
    }
}

// Re-export MatchDetail from la-core
pub use la_core::models::match_detail::MatchDetail;

#[cfg(test)]
mod tests {
    use super::*;

    use la_core::models::search::{QueryMetadata, TermSource};

    fn build_term(
        id: &str,
        value: &str,
        operator: QueryOperator,
        case_sensitive: bool,
    ) -> SearchTerm {
        SearchTerm {
            id: id.to_string(),
            value: value.to_string(),
            operator,
            source: TermSource::User,
            preset_group_id: None,
            is_regex: false,
            priority: 1,
            enabled: true,
            case_sensitive,
        }
    }

    fn build_query(terms: Vec<SearchTerm>, global_operator: QueryOperator) -> SearchQuery {
        SearchQuery {
            id: "q1".to_string(),
            terms,
            global_operator,
            filters: None,
            metadata: QueryMetadata {
                created_at: 0,
                last_modified: 0,
                execution_count: 0,
                label: None,
            },
        }
    }

    #[test]
    fn test_matches_line_and_mixed_case_sensitivity() {
        let mut builder = QueryPlanBuilder::new(10);
        let query = build_query(
            vec![build_term("t1", "Error", QueryOperator::And, false)],
            QueryOperator::And,
        );
        let plan = builder.build(&query).unwrap();

        assert!(
            plan.matches_line("error: something failed"),
            "Should match lowercase error"
        );
        assert!(
            plan.matches_line("ERROR: something failed"),
            "Should match uppercase ERROR"
        );
        assert!(
            plan.matches_line("Error: something failed"),
            "Should match mixed case Error"
        );
    }

    #[test]
    fn test_matches_line_with_case_sensitive() {
        let mut builder = QueryPlanBuilder::new(10);
        let query = build_query(
            vec![build_term("t1", "Error", QueryOperator::And, true)],
            QueryOperator::And,
        );
        let plan = builder.build(&query).unwrap();

        assert!(plan.matches_line("Error: something"));
        assert!(!plan.matches_line("error: something"));
        assert!(!plan.matches_line("ERROR: something"));
    }

    #[test]
    fn test_matches_line_not_operator() {
        let mut builder = QueryPlanBuilder::new(10);
        let query = build_query(
            vec![build_term("t1", "debug", QueryOperator::Not, false)],
            QueryOperator::Not,
        );
        let plan = builder.build(&query).unwrap();

        assert!(!plan.matches_line("debug: starting")); // Should NOT match
        assert!(plan.matches_line("info: running")); // Should match
    }

    #[test]
    fn test_matches_line_or_operator() {
        let mut builder = QueryPlanBuilder::new(10);
        let query = build_query(
            vec![
                build_term("t1", "Error", QueryOperator::Or, false),
                build_term("t2", "Warn", QueryOperator::Or, false),
            ],
            QueryOperator::Or,
        );
        let plan = builder.build(&query).unwrap();

        assert!(plan.matches_line("error: something"));
        assert!(plan.matches_line("warning: something"));
        assert!(plan.matches_line("Error: something"));
        assert!(!plan.matches_line("info: something"));
    }

    #[test]
    fn test_filter_lines() {
        let mut builder = QueryPlanBuilder::new(10);
        let query = build_query(
            vec![build_term("t1", "error", QueryOperator::And, false)],
            QueryOperator::And,
        );
        let plan = builder.build(&query).unwrap();

        let lines = [
            "error: first".to_string(),
            "info: second".to_string(),
            "error: third".to_string(),
        ];

        let filtered: Vec<&String> = lines
            .iter()
            .filter(|line| plan.matches_line(line))
            .collect();
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_match_with_details() {
        let mut builder = QueryPlanBuilder::new(10);
        let query = build_query(
            vec![
                build_term("t1", "Error", QueryOperator::And, false),
                build_term("t2", "failed", QueryOperator::And, false),
            ],
            QueryOperator::And,
        );
        let plan = builder.build(&query).unwrap();

        let details = plan.match_with_details("Error: operation failed");
        assert!(details.is_some());
        let details = details.unwrap();
        assert!(!details.is_empty());
    }

    #[test]
    fn test_match_with_details_all_keywords() {
        let mut builder = QueryPlanBuilder::new(10);
        let query = build_query(
            vec![
                build_term("t1", "error", QueryOperator::And, false),
                build_term("t2", "timeout", QueryOperator::And, false),
                build_term("t3", "warning", QueryOperator::And, false),
            ],
            QueryOperator::And,
        );
        let plan = builder.build(&query).unwrap();

        let line = "error: timeout occurred, warning: system overloaded";
        let details = plan.match_with_details(line);

        assert!(details.is_some());
        let details = details.unwrap();
        assert_eq!(details.len(), 3, "Should match all 3 keywords");
    }

    #[test]
    fn test_match_with_details_repeated_keywords() {
        let mut builder = QueryPlanBuilder::new(10);
        let query = build_query(
            vec![
                build_term("t1", "error", QueryOperator::And, false),
                build_term("t2", "error", QueryOperator::And, false),
            ],
            QueryOperator::And,
        );
        let plan = builder.build(&query).unwrap();

        let line = "error error error";
        let details = plan.match_with_details(line);

        assert!(details.is_some());
        let details = details.unwrap();
        assert!(
            details.len() >= 2,
            "Should match at least 2 keyword occurrences, got {}",
            details.len()
        );
    }

    #[test]
    fn test_match_with_details_aho_corasiick_multi_keyword() {
        let mut builder = QueryPlanBuilder::new(10);
        let query = build_query(
            vec![build_term(
                "t1",
                "error|warning|info",
                QueryOperator::And,
                false,
            )],
            QueryOperator::And,
        );
        let plan = builder.build(&query).unwrap();

        let line = "error found, warning issued, info logged";
        let details = plan.match_with_details(line);

        assert!(details.is_some());
        let details = details.unwrap();
        assert_eq!(
            details.len(),
            3,
            "Should match all 3 keywords from Aho-Corasick pattern"
        );
    }
}
