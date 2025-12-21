use crate::error::Result;
use crate::models::search::*;
use crate::services::pattern_matcher::PatternMatcher;
use crate::services::query_planner::{ExecutionPlan, PlanTerm, QueryPlanner};
use crate::services::query_validator::QueryValidator;

/**
 * 查询执行器
 *
 * 重构后的执行器，职责拆分为：
 * - QueryValidator: 验证查询
 * - QueryPlanner: 构建执行计划
 * - PatternMatcher: 模式匹配（Aho-Corasick）
 * - QueryExecutor: 执行查询和结果匹配
 */
pub struct QueryExecutor {
    planner: QueryPlanner,
    // 移除未使用的 validator 字段
}

impl QueryExecutor {
    /**
     * 创建新的执行器
     *
     * # 参数
     * * `cache_size` - 正则表达式缓存大小
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
        // 1. 验证查询
        QueryValidator::validate(query)?;

        // 2. 构建执行计划
        let mut plan = self.planner.build_plan(query)?;

        // 3. 按优先级排序
        plan.sort_by_priority();

        Ok(plan)
    }

    /**
     * 测试单行是否匹配
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
                // 收集所有模式，按大小写敏感分组
                let mut all_patterns = Vec::new();
                let mut case_sensitive_flags = Vec::new();

                for term in &plan.terms {
                    all_patterns.push(term.value.clone());
                    case_sensitive_flags.push(term.case_sensitive);
                }

                // 如果有任何大小写敏感模式，需要分别处理
                if case_sensitive_flags.iter().any(|&x| x) {
                    // 混合模式：分别构建两个匹配器
                    let sensitive_patterns: Vec<_> = plan
                        .terms
                        .iter()
                        .filter(|t| t.case_sensitive)
                        .map(|t| t.value.clone())
                        .collect();

                    let insensitive_patterns: Vec<_> = plan
                        .terms
                        .iter()
                        .filter(|t| !t.case_sensitive)
                        .map(|t| t.value.clone())
                        .collect();

                    // 使用Result类型的new方法，需要处理错误
                    let sensitive_matcher = if !sensitive_patterns.is_empty() {
                        PatternMatcher::new(sensitive_patterns, false).unwrap_or_else(|_| {
                            // 如果构建失败，创建一个空匹配器（不会匹配任何内容）
                            PatternMatcher::new(Vec::new(), false).unwrap()
                        })
                    } else {
                        PatternMatcher::new(Vec::new(), false).unwrap()
                    };

                    let insensitive_matcher = if !insensitive_patterns.is_empty() {
                        PatternMatcher::new(insensitive_patterns, true)
                            .unwrap_or_else(|_| PatternMatcher::new(Vec::new(), true).unwrap())
                    } else {
                        PatternMatcher::new(Vec::new(), true).unwrap()
                    };

                    sensitive_matcher.matches_all(line) && insensitive_matcher.matches_all(line)
                } else {
                    // 全部大小写不敏感
                    let patterns = plan.terms.iter().map(|t| t.value.clone()).collect();
                    PatternMatcher::new(patterns, true)
                        .map(|m| m.matches_all(line))
                        .unwrap_or(false)
                }
            }
            crate::services::query_planner::SearchStrategy::Or => {
                // OR 逻辑：匹配任意一个
                for compiled in &plan.regexes {
                    if compiled.regex.is_match(line) {
                        return true;
                    }
                }
                false
            }
            crate::services::query_planner::SearchStrategy::Not => {
                // NOT 逻辑：不能匹配任何一个
                for compiled in &plan.regexes {
                    if compiled.regex.is_match(line) {
                        return false;
                    }
                }
                true
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
     * 匹配并返回详情
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

        for compiled in &plan.regexes {
            if let Some(mat) = compiled.regex.find(line) {
                // 使用term_id定位原始配置，确保保留大小写设置
                let term_value = plan
                    .terms
                    .iter()
                    .find(|t| t.id == compiled.term_id)
                    .map(|t| t.value.clone())
                    .unwrap_or_else(|| mat.as_str().to_string());

                details.push(MatchDetail {
                    term_id: compiled.term_id.clone(),
                    term_value,
                    priority: compiled.priority,
                    match_position: Some((mat.start(), mat.end())),
                });
            }
        }

        // 按优先级排序
        details.sort_by_key(|d| std::cmp::Reverse(d.priority));

        if details.is_empty() {
            None
        } else {
            Some(details)
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
            vec![
                build_term("1", "ERROR", QueryOperator::And, true),
                build_term("2", "timeout", QueryOperator::And, false),
            ],
            QueryOperator::And,
        );

        let plan = executor.execute(&query).expect("plan should build");

        assert!(!executor.matches_line(&plan, "error timeout happened"));
        assert!(executor.matches_line(&plan, "ERROR timeout happened"));
    }
}
