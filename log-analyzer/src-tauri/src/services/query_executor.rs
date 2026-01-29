use crate::error::Result;
use crate::models::search::*;
use crate::services::query_planner::{ExecutionPlan, QueryPlanner};
use crate::services::query_validator::QueryValidator;
use crate::services::regex_engine::RegexEngine;

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
 */
pub struct QueryExecutor {
    planner: QueryPlanner,
}

impl QueryExecutor {
    /**
     * 创建新的执行器
     *
     * # 参数
     * * `cache_size` - 引擎缓存大小
     */
    pub fn new(cache_size: usize) -> Self {
        Self {
            planner: QueryPlanner::new(cache_size),
        }
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

        let mut plan = self.planner.build_plan(query)?;

        plan.sort_by_priority();

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
                for term in &plan.terms {
                    let term_matches = if let Some(compiled) =
                        plan.engines.iter().find(|e| e.term_id == term.id)
                    {
                        Self::engine_is_match(&compiled.engine, line)
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
                for term in &plan.terms {
                    let term_matches = if let Some(compiled) =
                        plan.engines.iter().find(|e| e.term_id == term.id)
                    {
                        Self::engine_is_match(&compiled.engine, line)
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
                for term in &plan.terms {
                    let term_matches = if let Some(compiled) =
                        plan.engines.iter().find(|e| e.term_id == term.id)
                    {
                        Self::engine_is_match(&compiled.engine, line)
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
        match engine {
            RegexEngine::AhoCorasick(e) => {
                e.find_iter(text).next().is_some()
            }
            RegexEngine::Automata(e) => {
                e.is_match(text)
            }
            RegexEngine::Standard(e) => {
                e.is_match(text)
            }
        }
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
     * * `Some(Vec<MatchDetail>)` - 匹配详情
     * * `None` - 不匹配
     */
    pub fn match_with_details(&self, plan: &ExecutionPlan, line: &str) -> Option<Vec<MatchDetail>> {
        if !self.matches_line(plan, line) {
            return None;
        }

        let mut details = Vec::new();

        for compiled in &plan.engines {
            let mat = Self::engine_find(&compiled.engine, line);

            if let Some(m) = mat {
                let term_value = plan
                    .terms
                    .iter()
                    .find(|t| t.id == compiled.term_id)
                    .map(|t| t.value.clone())
                    .unwrap_or_else(|| line[m.start..m.end].to_string());

                details.push(MatchDetail {
                    term_id: compiled.term_id.clone(),
                    term_value,
                    priority: compiled.priority,
                    match_position: Some((m.start, m.end)),
                });
            }
        }

        details.sort_by_key(|d| std::cmp::Reverse(d.priority));

        if details.is_empty() {
            None
        } else {
            Some(details)
        }
    }

    fn engine_find<'a>(engine: &RegexEngine, text: &'a str) -> Option<crate::services::regex_engine::MatchResult> {
        match engine {
            RegexEngine::AhoCorasick(e) => e.find_iter(text).next(),
            RegexEngine::Automata(e) => e.find_iter(text).next(),
            RegexEngine::Standard(e) => e.find_iter(text).next(),
        }
    }
}

/**
 * 匹配结果详情
 */
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MatchDetail {
    /// 匹配的搜索项ID
    pub term_id: String,
    /// 匹配的关键词值
    pub term_value: String,
    /// 优先级
    pub priority: u32,
    /// 匹配位置（可选）
    pub match_position: Option<(usize, usize)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::models::search::{QueryMetadata, TermSource};

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

        assert!(executor.matches_line(&plan, "error: something failed"), "Should match lowercase error");
        assert!(executor.matches_line(&plan, "ERROR: something failed"), "Should match uppercase ERROR");
        assert!(executor.matches_line(&plan, "Error: something failed"), "Should match mixed case Error");
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
        assert!(executor.matches_line(&plan, "info: running"));   // Should match
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
        assert!(details.len() >= 1);
    }

    #[test]
    fn test_aho_corasiick_multi_keyword_matching() {
        let mut executor = QueryExecutor::new(10);
        let query = build_query(
            vec![build_term("t1", "error|warning|info", QueryOperator::And, false)],
            QueryOperator::And,
        );
        let plan = executor.execute(&query).unwrap();

        assert!(executor.matches_line(&plan, "error: test"), "Should match 'error'");
        assert!(executor.matches_line(&plan, "warning: test"), "Should match 'warning'");
        assert!(executor.matches_line(&plan, "info: test"), "Should match 'info'");
        assert!(!executor.matches_line(&plan, "debug: test"), "Should not match 'debug'");
    }
}
