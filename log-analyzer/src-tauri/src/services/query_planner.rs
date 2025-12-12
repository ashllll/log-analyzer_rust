use crate::error::{AppError, Result};
use crate::models::search::*;
use regex::Regex;
use std::collections::HashMap;

/**
 * 查询计划构建器
 *
 * 负责构建搜索查询的执行计划
 */
pub struct QueryPlanner {
    cache_size: usize,
    regex_cache: HashMap<String, Regex>,
}

impl QueryPlanner {
    /**
     * 创建新的计划构建器
     *
     * # 参数
     * * `cache_size` - 正则表达式缓存大小
     */
    pub fn new(cache_size: usize) -> Self {
        Self {
            cache_size,
            regex_cache: HashMap::new(),
        }
    }

    /**
     * 构建执行计划
     *
     * # 参数
     * * `query` - 搜索查询
     *
     * # 返回
     * * `Ok(ExecutionPlan)` - 执行计划
     * * `Err(AppError)` - 构建失败
     */
    pub fn build_plan(&mut self, query: &SearchQuery) -> Result<ExecutionPlan> {
        let enabled_terms: Vec<&SearchTerm> = query.terms.iter().filter(|t| t.enabled).collect();

        // 决定策略（优先级由全局配置决定）
        let strategy = match query.global_operator {
            QueryOperator::And => SearchStrategy::And,
            QueryOperator::Or => SearchStrategy::Or,
            QueryOperator::Not => SearchStrategy::Not,
        };

        // 编译正则表达式
        let mut regexes = Vec::new();
        let mut terms_list = Vec::new();

        for term in &enabled_terms {
            let regex = self.compile_regex(term)?;
            regexes.push(CompiledRegex {
                regex,
                term_id: term.id.clone(),
                priority: term.priority,
            });

            terms_list.push(PlanTerm {
                id: term.id.clone(),
                value: term.value.clone(),
                case_sensitive: term.case_sensitive,
            });
        }

        Ok(ExecutionPlan {
            strategy,
            regexes,
            term_count: enabled_terms.len(),
            terms: terms_list,
        })
    }

    /**
     * 编译正则表达式（带缓存）
     *
     * # 参数
     * * `term` - 搜索项
     *
     * # 返回
     * * `Ok(Regex)` - 编译后的正则表达式
     * * `Err(AppError)` - 编译失败
     */
    fn compile_regex(&mut self, term: &SearchTerm) -> Result<Regex> {
        let pattern = if term.is_regex {
            term.value.clone()
        } else {
            let escaped = regex::escape(&term.value);
            if term.case_sensitive {
                escaped
            } else {
                format!("(?i:{})", escaped)
            }
        };

        // 检查缓存
        if let Some(cached) = self.regex_cache.get(&pattern) {
            return Ok(cached.clone());
        }

        // 编译新的正则表达式
        let regex = Regex::new(&pattern)
            .map_err(|e| AppError::validation_error(format!("Invalid pattern: {}", e)))?;

        // 添加到缓存
        if self.regex_cache.len() >= self.cache_size {
            // 缓存满了，清空一半
            let keys: Vec<String> = self.regex_cache.keys().cloned().collect();
            for key in keys.iter().take(self.cache_size / 2) {
                self.regex_cache.remove(key);
            }
        }

        self.regex_cache.insert(pattern.clone(), regex.clone());
        Ok(regex)
    }
}

/**
 * 执行计划
 */
#[derive(Debug, Clone)]
pub struct ExecutionPlan {
    pub strategy: SearchStrategy,
    pub regexes: Vec<CompiledRegex>,
    #[allow(dead_code)]
    pub term_count: usize,
    pub terms: Vec<PlanTerm>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlanTerm {
    pub id: String,
    pub value: String,
    pub case_sensitive: bool,
}

impl ExecutionPlan {
    /**
     * 获取搜索项数量
     */
    #[allow(dead_code)]
    pub fn get_term_count(&self) -> usize {
        self.term_count
    }

    /**
     * 获取所有搜索项
     */
    #[allow(dead_code)]
    pub fn get_terms(&self) -> &[PlanTerm] {
        &self.terms
    }

    /**
     * 按优先级排序正则表达式
     */
    pub fn sort_by_priority(&mut self) {
        self.regexes.sort_by_key(|r| std::cmp::Reverse(r.priority));
    }
}

/**
 * 搜索策略
 */
#[derive(Debug, Clone, PartialEq)]
pub enum SearchStrategy {
    And,
    Or,
    Not,
}

/**
 * 编译后的正则表达式
 */
#[derive(Debug, Clone)]
pub struct CompiledRegex {
    pub regex: Regex,
    pub term_id: String,
    pub priority: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::search::{QueryMetadata, TermSource};

    fn create_test_term(value: &str, operator: QueryOperator) -> SearchTerm {
        SearchTerm {
            id: "test".to_string(),
            value: value.to_string(),
            operator,
            source: TermSource::User,
            preset_group_id: None,
            is_regex: false,
            priority: 1,
            enabled: true,
            case_sensitive: false,
        }
    }

    #[test]
    fn test_build_and_plan() {
        let mut planner = QueryPlanner::new(100);
        let query = SearchQuery {
            id: "test".to_string(),
            terms: vec![
                create_test_term("error", QueryOperator::And),
                create_test_term("timeout", QueryOperator::And),
            ],
            global_operator: QueryOperator::And,
            filters: None,
            metadata: QueryMetadata {
                created_at: 0,
                last_modified: 0,
                execution_count: 0,
                label: None,
            },
        };

        let plan = planner.build_plan(&query).unwrap();
        assert_eq!(plan.strategy, SearchStrategy::And);
        assert_eq!(plan.term_count, 2);
        assert_eq!(plan.terms.len(), 2);
        assert!(plan
            .terms
            .iter()
            .all(|term| matches!(term.case_sensitive, false)));
    }

    #[test]
    fn test_build_or_plan() {
        let mut planner = QueryPlanner::new(100);
        let query = SearchQuery {
            id: "test".to_string(),
            terms: vec![
                create_test_term("error", QueryOperator::Or),
                create_test_term("warning", QueryOperator::Or),
            ],
            global_operator: QueryOperator::Or,
            filters: None,
            metadata: QueryMetadata {
                created_at: 0,
                last_modified: 0,
                execution_count: 0,
                label: None,
            },
        };

        let plan = planner.build_plan(&query).unwrap();
        assert_eq!(plan.strategy, SearchStrategy::Or);
        assert_eq!(plan.term_count, 2);
        assert_eq!(plan.terms.len(), 2);
    }

    #[test]
    fn test_build_not_plan() {
        let mut planner = QueryPlanner::new(100);
        let query = SearchQuery {
            id: "test".to_string(),
            terms: vec![create_test_term("debug", QueryOperator::Not)],
            global_operator: QueryOperator::Not,
            filters: None,
            metadata: QueryMetadata {
                created_at: 0,
                last_modified: 0,
                execution_count: 0,
                label: None,
            },
        };

        let plan = planner.build_plan(&query).unwrap();
        assert_eq!(plan.strategy, SearchStrategy::Not);
        assert_eq!(plan.term_count, 1);
        assert_eq!(plan.terms.len(), 1);
    }

    #[test]
    fn test_plan_respects_global_operator_overrides() {
        let mut planner = QueryPlanner::new(100);
        let query = SearchQuery {
            id: "test".to_string(),
            terms: vec![
                create_test_term("error", QueryOperator::And),
                create_test_term("timeout", QueryOperator::Not),
            ],
            // 全局 OR 应覆盖局部 operator 设置
            global_operator: QueryOperator::Or,
            filters: None,
            metadata: QueryMetadata {
                created_at: 0,
                last_modified: 0,
                execution_count: 0,
                label: None,
            },
        };

        let plan = planner.build_plan(&query).unwrap();
        assert_eq!(plan.strategy, SearchStrategy::Or);
        assert_eq!(plan.term_count, 2);
        assert_eq!(plan.regexes.len(), 2);
        assert_eq!(plan.terms.len(), 2);
    }

    #[test]
    fn test_compile_regex_cache() {
        let mut planner = QueryPlanner::new(2); // 小缓存用于测试

        let term1 = create_test_term("error", QueryOperator::And);
        let term2 = create_test_term("timeout", QueryOperator::And);

        // 第一次编译
        let regex1 = planner.compile_regex(&term1).unwrap();
        let _regex2 = planner.compile_regex(&term2).unwrap();

        assert!(planner.regex_cache.len() == 2);

        // 第二次应该命中缓存
        let regex1_cached = planner.compile_regex(&term1).unwrap();
        assert_eq!(regex1.as_str(), regex1_cached.as_str());
    }

    #[test]
    fn test_compile_regex_case_insensitive() {
        let mut planner = QueryPlanner::new(100);

        let term = SearchTerm {
            id: "test".to_string(),
            value: "error".to_string(),
            operator: QueryOperator::And,
            source: TermSource::User,
            preset_group_id: None,
            is_regex: false,
            priority: 1,
            enabled: true,
            case_sensitive: false,
        };

        let regex = planner.compile_regex(&term).unwrap();
        assert!(regex.is_match("ERROR"));
        assert!(regex.is_match("error"));
    }

    #[test]
    fn test_compile_regex_case_sensitive() {
        let mut planner = QueryPlanner::new(100);

        let term = SearchTerm {
            id: "test".to_string(),
            value: "error".to_string(),
            operator: QueryOperator::And,
            source: TermSource::User,
            preset_group_id: None,
            is_regex: false,
            priority: 1,
            enabled: true,
            case_sensitive: true,
        };

        let regex = planner.compile_regex(&term).unwrap();
        assert!(!regex.is_match("ERROR"));
        assert!(regex.is_match("error"));
    }

    #[test]
    fn test_compile_regex_with_regex_term() {
        let mut planner = QueryPlanner::new(100);

        let term = SearchTerm {
            id: "test".to_string(),
            value: r"\d+".to_string(),
            operator: QueryOperator::And,
            source: TermSource::User,
            preset_group_id: None,
            is_regex: true,
            priority: 1,
            enabled: true,
            case_sensitive: false,
        };

        let regex = planner.compile_regex(&term).unwrap();
        assert!(regex.is_match("123"));
        assert!(!regex.is_match("abc"));
    }

    #[test]
    fn test_plan_sort_by_priority() {
        let mut plan = ExecutionPlan {
            strategy: SearchStrategy::And,
            regexes: vec![
                CompiledRegex {
                    regex: Regex::new("test1").unwrap(),
                    term_id: "1".to_string(),
                    priority: 1,
                },
                CompiledRegex {
                    regex: Regex::new("test2").unwrap(),
                    term_id: "2".to_string(),
                    priority: 3,
                },
                CompiledRegex {
                    regex: Regex::new("test3").unwrap(),
                    term_id: "3".to_string(),
                    priority: 2,
                },
            ],
            term_count: 3,
            terms: vec![
                PlanTerm {
                    id: "1".to_string(),
                    value: "test1".to_string(),
                    case_sensitive: false,
                },
                PlanTerm {
                    id: "2".to_string(),
                    value: "test2".to_string(),
                    case_sensitive: false,
                },
                PlanTerm {
                    id: "3".to_string(),
                    value: "test3".to_string(),
                    case_sensitive: false,
                },
            ],
        };

        plan.sort_by_priority();

        assert_eq!(plan.regexes[0].priority, 3);
        assert_eq!(plan.regexes[1].priority, 2);
        assert_eq!(plan.regexes[2].priority, 1);
    }
}
