use crate::services::query_planner::{ExecutionPlan, QueryPlanner};
use crate::services::query_validator::QueryValidator;
use crate::services::regex_engine::RegexEngine;
use crate::services::traits::{QueryPlanning, QueryValidation};
use la_core::error::Result;
use la_core::models::search::*;
use moka::sync::Cache;
use sha2::{Digest, Sha256};
use std::sync::Arc;

/// 辅助函数：哈希 QueryOperator
fn hash_query_operator(op: &QueryOperator, hasher: &mut Sha256) {
    match op {
        QueryOperator::And => hasher.update([0u8]),
        QueryOperator::Or => hasher.update([1u8]),
        QueryOperator::Not => hasher.update([2u8]),
    }
}

/// 公共函数: 计算查询缓存键
///
/// 统一 QueryExecutor 和 GenericQueryExecutor 的缓存键生成逻辑，
/// 确保等价的查询产生相同的缓存键。
///
/// 使用 SHA-256 替代 DefaultHasher，确保跨 Rust 版本稳定。
/// 哈希所有查询字段:
/// - 查询 ID, 全局操作符
/// - 过滤器 (时间范围, 日志级别, 文件模式)
/// - 搜索项 (按 ID 排序后哈希所有字段)
pub(crate) fn compute_query_cache_key(query: &SearchQuery) -> String {
    let mut hasher = Sha256::new();

    // 哈希查询 ID 和全局操作符
    hasher.update(query.id.as_bytes());
    hash_query_operator(&query.global_operator, &mut hasher);

    // 哈希过滤器（如果存在）
    if let Some(filters) = &query.filters {
        if let Some(time_range) = &filters.time_range {
            if let Some(start) = &time_range.start {
                hasher.update(start.as_bytes());
            }
            if let Some(end) = &time_range.end {
                hasher.update(end.as_bytes());
            }
        }
        if let Some(levels) = &filters.levels {
            for level in levels {
                hasher.update(level.as_bytes());
            }
        }
        if let Some(fp) = &filters.file_pattern {
            hasher.update(fp.as_bytes());
        }
    }

    // 按 ID 排序以确保哈希一致性（无论术语顺序如何，等价查询产生相同哈希）
    let mut sorted_terms = query.terms.clone();
    sorted_terms.sort_by(|a, b| a.id.cmp(&b.id));

    // 哈希每个搜索项的所有关键字段
    for term in &sorted_terms {
        hasher.update(term.id.as_bytes());
        hasher.update(term.value.as_bytes());
        hash_query_operator(&term.operator, &mut hasher);
        hasher.update(&[term.source.clone() as u8]);
        if let Some(pid) = &term.preset_group_id {
            hasher.update(pid.as_bytes());
        }
        hasher.update(&[term.is_regex as u8]);
        hasher.update(&term.priority.to_le_bytes());
        hasher.update(&[term.enabled as u8]);
        hasher.update(&[term.case_sensitive as u8]);
    }

    // 返回 16 进制哈希值作为缓存键（取前 8 字节 / 64 位）
    format!(
        "{:016x}",
        u64::from_le_bytes(hasher.finalize()[..8].try_into().unwrap())
    )
}

/**
 * 查询执行器
 *
 * 重构后的执行器，职责拆分为：
 * - QueryValidator: 验证查询
 * - QueryPlanner: 构建执行计划（使用混合引擎）
 * - QueryExecutor: 执行查询和结果匹配
 *
 * 设计决策：使用混合引擎策略自动选择最佳匹配算法：
 * - AhoCorasick: 简单关键词搜索，O(n) 线性复杂度
 * - Standard: 复杂正则/需要前瞻后瞻
 *
 * 性能优化：使用 ExecutionPlan 缓存避免重复构建查询计划
 */
pub struct QueryExecutor {
    planner: QueryPlanner,
    /// ExecutionPlan 缓存 (LRU策略)
    plan_cache: Cache<String, Arc<ExecutionPlan>>,
}

impl QueryExecutor {
    /**
     * 创建新的执行器
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
     * 委托给公共函数 compute_query_cache_key，确保与 GenericQueryExecutor 一致性
     */
    fn cache_key(query: &SearchQuery) -> String {
        compute_query_cache_key(query)
    }

    /**
     * 执行查询
     *
     * # 参数
     * * `query` - 搜索查询
     *
     * # 返回
     * * `Ok(ExecutionPlan)` - 执行计划
     * * `Err(AppError)` - 执行失败
     */
    pub fn execute(&mut self, query: &SearchQuery) -> Result<ExecutionPlan> {
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

    /**
     * 测试单行是否匹配（使用混合引擎）
     *
     * # 参数
     * * `plan` - 执行计划
     * * `line` - 要测试的文本行
     *
     * # 返回
     * * `true` - 如果行匹配
     * * `false` - 否则
     */
    pub fn matches_line(&self, plan: &ExecutionPlan, line: &str) -> bool {
        match plan.strategy {
            crate::services::query_planner::SearchStrategy::And => {
                for term_id in plan.execution_term_ids() {
                    let term_matches = if let Some(engine) = plan.get_engine_for_term(term_id) {
                        Self::engine_is_match(&engine, line)
                    } else {
                        false
                    };

                    if !term_matches {
                        return false;
                    }
                }
                true
            }
            crate::services::query_planner::SearchStrategy::Or => {
                if let Some(engine) = &plan.fast_or_engine {
                    return Self::engine_is_match(engine.as_ref(), line);
                }

                for term_id in plan.execution_term_ids() {
                    let term_matches = if let Some(engine) = plan.get_engine_for_term(term_id) {
                        Self::engine_is_match(&engine, line)
                    } else {
                        false
                    };

                    if term_matches {
                        return true;
                    }
                }
                false
            }
            crate::services::query_planner::SearchStrategy::Not => {
                for term_id in plan.execution_term_ids() {
                    let term_matches = if let Some(engine) = plan.get_engine_for_term(term_id) {
                        Self::engine_is_match(&engine, line)
                    } else {
                        false
                    };

                    if term_matches {
                        return false;
                    }
                }
                true
            }
        }
    }

    fn engine_is_match(engine: &RegexEngine, text: &str) -> bool {
        engine.is_match(text)
    }

    /**
     * 批量过滤日志行
     *
     * # 参数
     * * `plan` - 执行计划
     * * `lines` - 要过滤的文本行列表
     *
     * # 返回
     * * 匹配的行列表
     */
    #[allow(dead_code)]
    pub fn filter_lines<'a>(&self, plan: &ExecutionPlan, lines: &'a [String]) -> Vec<&'a String> {
        lines
            .iter()
            .filter(|line| self.matches_line(plan, line))
            .collect()
    }

    /**
     * 匹配并返回详情（使用混合引擎）
     *
     * # 参数
     * * `plan` - 执行计划
     * * `line` - 要匹配的文本行
     *
     * # 返回
     * * `Some(Vec<MatchDetail>)` - 所有匹配详情
     * * `None` - 不匹配
     */
    pub fn match_with_details(&self, plan: &ExecutionPlan, line: &str) -> Option<Vec<MatchDetail>> {
        if !self.matches_line(plan, line) {
            return None;
        }

        let mut details = Vec::new();

        match plan.strategy {
            crate::services::query_planner::SearchStrategy::Or => {
                // OR 策略：只要有一个 engine 找到匹配即可，与 matches_line 保持一致。
                // 收集所有有匹配的 engine 的 details，而非要求所有 engine 都有结果。
                for compiled in &plan.engines {
                    let matches: Vec<_> = Self::engine_find_all(&compiled.engine, line);
                    if !matches.is_empty() {
                        for mat in matches {
                            let term_value = plan
                                .terms
                                .iter()
                                .find(|t| t.id == compiled.term_id)
                                .map(|t| t.value.clone())
                                .unwrap_or_else(|| {
                                    // 使用 get() 避免多字节 UTF-8 字符边界处的 panic
                                    line.get(mat.start..mat.end).unwrap_or_default().to_string()
                                });

                            details.push(MatchDetail {
                                term_id: compiled.term_id.clone(),
                                term_value,
                                priority: compiled.priority,
                                match_position: Some((
                                    Self::byte_offset_to_char_index(line, mat.start),
                                    Self::byte_offset_to_char_index(line, mat.end),
                                )),
                            });
                        }
                    }
                }
                // OR 策略下行已通过 matches_line 验证，即使 details 为空也返回 Some([])
                details.sort_by_key(|d| std::cmp::Reverse(d.priority));
                Some(details)
            }
            crate::services::query_planner::SearchStrategy::Not => {
                // Not 策略语义：行通过匹配验证是因为"没有"任何排除关键词出现，
                // 因此 details 为空是预期的正确语义（不存在需要高亮的匹配项）。
                // 返回 Some(vec![]) 而非 None，表示该行确实通过了匹配验证，
                // 只是 Not 操作符下的匹配结果不会产生高亮信息。
                // 这与完全不匹配的情况（返回 None）不同，调用方可通过此区分：
                //   - None: 行被排除（不匹配任何条件）
                //   - Some([]): 行通过 Not 策略（没有匹配到排除关键词）
                Some(vec![])
            }
            crate::services::query_planner::SearchStrategy::And => {
                // And 策略：收集所有 engine 的 details。
                for compiled in &plan.engines {
                    for mat in Self::engine_find_all(&compiled.engine, line) {
                        let term_value = plan
                            .terms
                            .iter()
                            .find(|t| t.id == compiled.term_id)
                            .map(|t| t.value.clone())
                            .unwrap_or_else(|| {
                                // 使用 get() 避免多字节 UTF-8 字符边界处的 panic
                                line.get(mat.start..mat.end).unwrap_or_default().to_string()
                            });

                        details.push(MatchDetail {
                            term_id: compiled.term_id.clone(),
                            term_value,
                            priority: compiled.priority,
                            match_position: Some((
                                Self::byte_offset_to_char_index(line, mat.start),
                                Self::byte_offset_to_char_index(line, mat.end),
                            )),
                        });
                    }
                }
                details.sort_by_key(|d| std::cmp::Reverse(d.priority));
                if details.is_empty() {
                    // And 策略下 details 为空意味着引擎实现与 matches_line 不一致，返回 None 是合理的防御。
                    None
                } else {
                    Some(details)
                }
            }
        }
    }

    /// 将 UTF-8 byte offset 转换为字符索引（char count），供前端 JavaScript 字符串切片使用。
    ///
    /// 原因：regex/aho-corasick 返回的 match start/end 是 UTF-8 byte offsets，
    /// 而前端 `String.slice()` 使用 UTF-16 code unit indices。对于 ASCII 文本两者一致，
    /// 但对于多字节 UTF-8 字符（如中文）会产生偏移。通过转换为 char count，
    /// 确保前后端高亮位置精确对齐。
    fn byte_offset_to_char_index(s: &str, byte_offset: usize) -> usize {
        s[..byte_offset.min(s.len())].chars().count()
    }

    fn engine_find_all(
        engine: &RegexEngine,
        text: &str,
    ) -> Vec<crate::services::regex_engine::MatchResult> {
        engine.find_iter(text).collect()
    }
}

// Re-export MatchDetail from la-core
pub use la_core::models::match_detail::MatchDetail;

/// 泛型查询执行器
///
/// 这个版本允许注入不同的验证器和规划器实现，实现依赖倒置原则。
/// 通过使用泛型参数，可以在编译时确定具体实现，获得零成本抽象。
///
/// # 类型参数
/// * `V` - 验证器类型，必须实现 `QueryValidation` trait
/// * `P` - 规划器类型，必须实现 `QueryPlanning` trait
///
/// # 示例
/// ```rust,ignore
/// use log_analyzer::services::{GenericQueryExecutor, QueryValidator, QueryPlannerAdapter};
///
/// let executor = GenericQueryExecutor::new(
///     QueryValidator,
///     QueryPlannerAdapter::new()
/// );
/// ```
pub struct GenericQueryExecutor<V, P> {
    validator: V,
    planner: P,
    /// ExecutionPlan 缓存 (LRU策略)
    plan_cache: Cache<String, Arc<ExecutionPlan>>,
}

impl<V, P> GenericQueryExecutor<V, P>
where
    V: QueryValidation,
    P: QueryPlanning,
{
    /// 创建新的泛型执行器
    ///
    /// # 参数
    /// * `validator` - 查询验证器
    /// * `planner` - 查询规划器
    pub fn new(validator: V, planner: P) -> Self {
        Self {
            validator,
            planner,
            plan_cache: Cache::new(1000),
        }
    }

    /// 生成查询缓存键
    ///
    /// 委托给公共函数 compute_query_cache_key，确保与 QueryExecutor 一致性
    fn cache_key(query: &SearchQuery) -> String {
        compute_query_cache_key(query)
    }

    /// 执行查询（使用泛型验证器和规划器）
    ///
    /// # 参数
    /// * `query` - 搜索查询
    ///
    /// # 返回
    /// * `Ok(ExecutionPlan)` - 执行计划
    /// * `Err(AppError)` - 执行失败
    pub fn execute(&self, query: &SearchQuery) -> Result<ExecutionPlan> {
        let cache_key = Self::cache_key(query);

        // 检查缓存
        if let Some(cached_plan) = self.plan_cache.get(&cache_key) {
            return Ok((*cached_plan).clone());
        }

        // 验证查询
        let validation_result = self.validator.validate(query);
        if !validation_result.is_valid {
            return Err(crate::error::AppError::validation_error(
                validation_result.errors.join(", "),
            ));
        }

        // 构建计划
        // 使用 build_execution_plan 方法获取完整的执行计划
        let plan = self.planner.build_execution_plan(query)?;

        // 缓存计划
        self.plan_cache.insert(cache_key, Arc::new(plan.clone()));

        Ok(plan)
    }
}

/// 标准查询执行器类型别名
///
/// 为了向后兼容，提供默认的具体类型。
/// 使用 QueryPlannerAdapter 作为规划器实现，它提供了 QueryPlanning trait 的实现。
pub type StandardQueryExecutor = GenericQueryExecutor<QueryValidator, super::QueryPlannerAdapter>;

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
        let mut executor = QueryExecutor::new(10);
        let query = build_query(
            vec![build_term("t1", "Error", QueryOperator::And, false)],
            QueryOperator::And,
        );
        let plan = executor.execute(&query).unwrap();

        assert!(
            executor.matches_line(&plan, "error: something failed"),
            "Should match lowercase error"
        );
        assert!(
            executor.matches_line(&plan, "ERROR: something failed"),
            "Should match uppercase ERROR"
        );
        assert!(
            executor.matches_line(&plan, "Error: something failed"),
            "Should match mixed case Error"
        );
    }

    #[test]
    fn test_matches_line_with_case_sensitive() {
        let mut executor = QueryExecutor::new(10);
        let query = build_query(
            vec![build_term("t1", "Error", QueryOperator::And, true)],
            QueryOperator::And,
        );
        let plan = executor.execute(&query).unwrap();

        assert!(executor.matches_line(&plan, "Error: something"));
        assert!(!executor.matches_line(&plan, "error: something"));
        assert!(!executor.matches_line(&plan, "ERROR: something"));
    }

    #[test]
    fn test_matches_line_not_operator() {
        let mut executor = QueryExecutor::new(10);
        let query = build_query(
            vec![build_term("t1", "debug", QueryOperator::Not, false)],
            QueryOperator::Not,
        );
        let plan = executor.execute(&query).unwrap();

        assert!(!executor.matches_line(&plan, "debug: starting")); // Should NOT match
        assert!(executor.matches_line(&plan, "info: running")); // Should match
    }

    #[test]
    fn test_matches_line_or_operator() {
        let mut executor = QueryExecutor::new(10);
        let query = build_query(
            vec![
                build_term("t1", "Error", QueryOperator::Or, false),
                build_term("t2", "Warn", QueryOperator::Or, false),
            ],
            QueryOperator::Or,
        );
        let plan = executor.execute(&query).unwrap();

        assert!(executor.matches_line(&plan, "error: something"));
        assert!(executor.matches_line(&plan, "warning: something"));
        assert!(executor.matches_line(&plan, "Error: something"));
        assert!(!executor.matches_line(&plan, "info: something"));
    }

    #[test]
    fn test_filter_lines() {
        let mut executor = QueryExecutor::new(10);
        let query = build_query(
            vec![build_term("t1", "error", QueryOperator::And, false)],
            QueryOperator::And,
        );
        let plan = executor.execute(&query).unwrap();

        let lines = vec![
            "error: first".to_string(),
            "info: second".to_string(),
            "error: third".to_string(),
        ];

        let filtered = executor.filter_lines(&plan, &lines);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_match_with_details() {
        let mut executor = QueryExecutor::new(10);
        let query = build_query(
            vec![
                build_term("t1", "Error", QueryOperator::And, false),
                build_term("t2", "failed", QueryOperator::And, false),
            ],
            QueryOperator::And,
        );
        let plan = executor.execute(&query).unwrap();

        let details = executor.match_with_details(&plan, "Error: operation failed");
        assert!(details.is_some());
        let details = details.unwrap();
        assert!(!details.is_empty());
    }

    #[test]
    fn test_match_with_details_all_keywords() {
        let mut executor = QueryExecutor::new(10);
        let query = build_query(
            vec![
                build_term("t1", "error", QueryOperator::And, false),
                build_term("t2", "timeout", QueryOperator::And, false),
                build_term("t3", "warning", QueryOperator::And, false),
            ],
            QueryOperator::And,
        );
        let plan = executor.execute(&query).unwrap();

        let line = "error: timeout occurred, warning: system overloaded";
        let details = executor.match_with_details(&plan, line);

        assert!(details.is_some());
        let details = details.unwrap();
        assert_eq!(details.len(), 3, "Should match all 3 keywords");
    }

    #[test]
    fn test_match_with_details_repeated_keywords() {
        let mut executor = QueryExecutor::new(10);
        let query = build_query(
            vec![
                build_term("t1", "error", QueryOperator::And, false),
                build_term("t2", "error", QueryOperator::And, false),
            ],
            QueryOperator::And,
        );
        let plan = executor.execute(&query).unwrap();

        let line = "error error error";
        let details = executor.match_with_details(&plan, line);

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
        let mut executor = QueryExecutor::new(10);
        let query = build_query(
            vec![build_term(
                "t1",
                "error|warning|info",
                QueryOperator::And,
                false,
            )],
            QueryOperator::And,
        );
        let plan = executor.execute(&query).unwrap();

        let line = "error found, warning issued, info logged";
        let details = executor.match_with_details(&plan, line);

        assert!(details.is_some());
        let details = details.unwrap();
        assert_eq!(
            details.len(),
            3,
            "Should match all 3 keywords from Aho-Corasick pattern"
        );
    }
}
