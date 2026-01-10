use crate::error::{AppError, Result};
use crate::models::search::*;
use regex::Regex;
use std::collections::HashMap;

/**
 * 查询计划构建器
 *
 * 负责构建搜索查询的执行计划
 *
 * 设计决策：所有搜索默认使用正则表达式，与 ripgrep 等工具保持一致。
 * 用户如需精确匹配，可使用 \Q...\E 语法（Perl 风格）。
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
     * 智能判断搜索词是否应该使用单词边界匹配
     *
     * 启发式规则：
     * 1. 用户手动输入了 \b → 立即使用单词边界
     * 2. 常见日志关键词（ERROR, WARN, INFO, DE H, DE N）→ 自动单词边界
     * 3. 短的字母数字组合（≤10字符，且主要是字母+空格）→ 自动单词边界
     * 4. 包含空格的短语（如 "database error"）→ 自动单词边界
     * 5. 其他情况 → 保持子串匹配（兼容现有行为）
     *
     * # 参数
     * * `term` - 搜索项
     *
     * # 返回
     * * `true` - 应该使用单词边界
     * * `false` - 保持子串匹配
     */
    fn should_use_word_boundary(term: &SearchTerm) -> bool {
        // 规则0: 正则表达式模式不自动添加边界
        if term.is_regex {
            return false;
        }

        let value = &term.value;

        // 规则1: 用户手动输入了 \b，尊重用户意图
        if value.contains(r"\b") {
            return true;
        }

        // 规则2: 常见日志关键词（大小写不敏感）
        let common_log_keywords = [
            "ERROR", "WARN", "INFO", "DEBUG", "FATAL", "DE H", "DE N", "DE E",
            "DE W", // 安卓日志常见模式
            "TRACE", "CRITICAL",
        ];
        let upper_value = value.to_uppercase();
        if common_log_keywords.iter().any(|&kw| upper_value == kw) {
            return true;
        }

        // 规则3: 短的字母数字组合
        let is_short = value.len() <= 10;
        let is_simple_pattern = value.chars().all(|c| c.is_alphanumeric() || c == ' ');
        if is_short && is_simple_pattern {
            return true;
        }

        // 规则4: 包含空格的短语（通常是精确词组搜索）
        if value.contains(' ') && value.len() <= 30 {
            // 但不能包含太多特殊字符（可能是正则表达式）
            let special_char_count = value
                .chars()
                .filter(|&c| !c.is_alphanumeric() && c != ' ')
                .count();
            if special_char_count == 0 {
                return true;
            }
        }

        // 规则5: 默认保持子串匹配（向后兼容）
        false
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
            // 正则表达式：保持原样
            term.value.clone()
        } else {
            let escaped = regex::escape(&term.value);

            // 自动检测是否需要单词边界
            let pattern_with_boundaries = if Self::should_use_word_boundary(term) {
                // 自动添加单词边界
                format!(r"\b{}\b", escaped)
            } else {
                // 保持子串匹配
                escaped
            };

            if term.case_sensitive {
                pattern_with_boundaries
            } else {
                format!("(?i:{})", pattern_with_boundaries)
            }
        };

        // 缓存键（不包含match_mode，因为是自动检测）
        let cache_key = format!("{}:{}", term.value, term.case_sensitive);

        // 检查缓存
        if let Some(cached) = self.regex_cache.get(&cache_key) {
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

        self.regex_cache.insert(cache_key, regex.clone());
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
        assert!(plan.terms.iter().all(|term| !term.case_sensitive));
        assert!(!plan.fuzzy_enabled); // 默认不启用模糊匹配
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
            fuzzy_enabled: Some(false),
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
            fuzzy_enabled: Some(false),
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
            fuzzy_enabled: Some(false),
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
                    fuzzy_enabled: false,
                },
                PlanTerm {
                    id: "2".to_string(),
                    value: "test2".to_string(),
                    case_sensitive: false,
                    fuzzy_enabled: false,
                },
                PlanTerm {
                    id: "3".to_string(),
                    value: "test3".to_string(),
                    case_sensitive: false,
                    fuzzy_enabled: false,
                },
            ],
            fuzzy_enabled: false,
        };

        plan.sort_by_priority();

        assert_eq!(plan.regexes[0].priority, 3);
        assert_eq!(plan.regexes[1].priority, 2);
        assert_eq!(plan.regexes[2].priority, 1);
    }

    // ========== 自动单词边界检测测试 ==========

    #[test]
    fn test_auto_detect_common_keywords() {
        let test_cases = vec![
            ("DE H", true), // 常见安卓日志关键词
            ("DE N", true),
            ("ERROR", true), // 通用日志级别
            ("WARN", true),
            ("INFO", true),
            ("de h", true), // 大小写不敏感
            ("error", true),
        ];

        for (value, expected) in test_cases {
            let term = create_test_term(value, QueryOperator::And);
            assert_eq!(
                QueryPlanner::should_use_word_boundary(&term),
                expected,
                "Failed for: {}",
                value
            );
        }
    }

    #[test]
    fn test_auto_detect_short_patterns() {
        let term = create_test_term("test", QueryOperator::And);
        assert!(QueryPlanner::should_use_word_boundary(&term));

        let term2 = create_test_term("CODE", QueryOperator::And);
        assert!(QueryPlanner::should_use_word_boundary(&term2));
    }

    #[test]
    fn test_auto_detect_phrases() {
        let term = create_test_term("database error", QueryOperator::And);
        assert!(QueryPlanner::should_use_word_boundary(&term));

        let term2 = create_test_term("connection timeout", QueryOperator::And);
        assert!(QueryPlanner::should_use_word_boundary(&term2));
    }

    #[test]
    fn test_keep_substring_for_special_chars() {
        let test_cases = vec!["user_123", "file-name.log", "http://api", "test:value"];

        for value in test_cases {
            let term = create_test_term(value, QueryOperator::And);
            assert!(
                !QueryPlanner::should_use_word_boundary(&term),
                "Should not use word boundary for: {}",
                value
            );
        }
    }

    #[test]
    fn test_detect_manual_boundaries() {
        let term = SearchTerm {
            id: "test".to_string(),
            value: r"\bCODE\b".to_string(), // 用户手动输入
            operator: QueryOperator::And,
            source: TermSource::User,
            preset_group_id: None,
            is_regex: false,
            priority: 1,
            enabled: true,
            case_sensitive: false,
            fuzzy_enabled: Some(false),
        };

        assert!(QueryPlanner::should_use_word_boundary(&term));
    }

    #[test]
    fn test_regex_mode_no_auto_boundaries() {
        let term = SearchTerm {
            id: "test".to_string(),
            value: r"\d+".to_string(),
            operator: QueryOperator::And,
            source: TermSource::User,
            preset_group_id: None,
            is_regex: true, // 正则模式
            priority: 1,
            enabled: true,
            case_sensitive: false,
            fuzzy_enabled: Some(false),
        };

        assert!(!QueryPlanner::should_use_word_boundary(&term));
    }

    #[test]
    fn test_android_log_search() {
        // 端到端测试：DE H 不匹配 CODE HDEF
        let mut planner = QueryPlanner::new(100);
        let term = create_test_term("DE H", QueryOperator::And);

        let regex = planner.compile_regex(&term).unwrap();

        // ✅ 应该匹配：独立的 "DE H"
        assert!(regex.is_match("DE H occurred"));
        assert!(regex.is_match("found DE H here"));
        assert!(regex.is_match("DE H at start"));

        // ❌ 不应该匹配：子串但不独立
        assert!(!regex.is_match("CODE HDEF"));
        assert!(!regex.is_match("testDEHtest"));
        assert!(!regex.is_match("CODEH-DEF"));
    }

    #[test]
    fn test_word_boundary_with_or_operator() {
        // 测试 OR 操作符场景
        let mut planner = QueryPlanner::new(100);
        let query = SearchQuery {
            id: "test".to_string(),
            terms: vec![
                create_test_term("DE H", QueryOperator::Or),
                create_test_term("DE N", QueryOperator::Or),
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

        // 测试 AND 逻辑（OR操作符下，所有term应该都匹配）
        assert!(plan.regexes[0].regex.is_match("DE H found"));
        assert!(plan.regexes[1].regex.is_match("DE N here"));

        // 确保不会错误匹配
        assert!(!plan.regexes[0].regex.is_match("CODE HDEF"));
        assert!(!plan.regexes[1].regex.is_match("CODE HDEF"));
    }

    #[test]
    fn test_long_phrase_no_boundary() {
        // 测试超过30字符的短语不自动添加边界
        let term = create_test_term(
            "This is a very long phrase that exceeds thirty characters",
            QueryOperator::And,
        );
        assert!(!QueryPlanner::should_use_word_boundary(&term));
    }

    #[test]
    fn test_url_no_boundary() {
        // 测试URL不添加边界
        let term = create_test_term("http://api.example.com", QueryOperator::And);
        assert!(!QueryPlanner::should_use_word_boundary(&term));
    }
}
