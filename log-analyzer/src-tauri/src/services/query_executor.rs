use crate::error::Result;
use crate::models::search::*;
use crate::services::pattern_matcher::PatternMatcher;
use crate::services::query_planner::{ExecutionPlan, QueryPlanner};
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
    #[allow(dead_code)]
    validator: QueryValidator,
    planner: QueryPlanner,
    pattern_matcher: Option<PatternMatcher>,
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
            validator: QueryValidator,
            planner: QueryPlanner::new(cache_size),
            pattern_matcher: None,
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

        // 3. 初始化Aho-Corasick模式匹配器（仅用于AND策略）
        self.pattern_matcher =
            if plan.strategy == crate::services::query_planner::SearchStrategy::And {
                let patterns = plan.terms.clone();
                Some(PatternMatcher::new(patterns, true)) // AND策略总是大小写不敏感
            } else {
                None
            };

        // 4. 按优先级排序
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
                // 使用Aho-Corasick算法优化AND逻辑
                if let Some(ref matcher) = self.pattern_matcher {
                    matcher.matches_all(line)
                } else {
                    // 回退到原始实现
                    let line_lower = line.to_lowercase();
                    for term in &plan.terms {
                        if !line_lower.contains(term) {
                            return false;
                        }
                    }
                    true
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
                // 直接使用term_id和terms列表查找值
                let term_value = plan
                    .terms
                    .iter()
                    .find(|t| t.to_lowercase() == mat.as_str().to_lowercase())
                    .cloned()
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
