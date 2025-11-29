use crate::models::search::*;
use regex::Regex;
use std::collections::HashMap;

/**
 * 查询执行器
 */
#[allow(dead_code)]  // cache_size 和 regex_cache 字段用于缓存优化
pub struct QueryExecutor {
    cache_size: usize,
    regex_cache: HashMap<String, Regex>,
}

impl QueryExecutor {
    /**
     * 创建新的执行器
     */
    pub fn new(cache_size: usize) -> Self {
        Self {
            cache_size,
            regex_cache: HashMap::new(),
        }
    }

    /**
     * 执行查询
     */
    pub fn execute(&mut self, query: &SearchQuery) -> Result<ExecutionPlan, ExecutionError> {
        // 1. 验证
        self.validate(query)?;

        // 2. 构建执行计划
        let plan = self.build_execution_plan(query)?;

        Ok(plan)
    }

    /**
     * 验证查询
     */
    fn validate(&self, query: &SearchQuery) -> Result<(), ExecutionError> {
        if query.terms.is_empty() {
            return Err(ExecutionError::EmptyQuery);
        }

        let enabled_terms: Vec<_> = query.terms.iter().filter(|t| t.enabled).collect();

        if enabled_terms.is_empty() {
            return Err(ExecutionError::NoEnabledTerms);
        }

        // 验证每个项
        for term in &enabled_terms {
            self.validate_term(term)?;
        }

        Ok(())
    }

    /**
     * 验证单个项
     */
    fn validate_term(&self, term: &SearchTerm) -> Result<(), ExecutionError> {
        if term.value.is_empty() {
            return Err(ExecutionError::EmptyTermValue(term.id.clone()));
        }

        if term.value.len() > 100 {
            return Err(ExecutionError::TermValueTooLong(term.id.clone()));
        }

        if term.is_regex {
            Regex::new(&term.value)
                .map_err(|e| ExecutionError::InvalidRegex(term.id.clone(), e.to_string()))?;
        }

        Ok(())
    }

    /**
     * 构建执行计划
     */
    fn build_execution_plan(
        &mut self,
        query: &SearchQuery,
    ) -> Result<ExecutionPlan, ExecutionError> {
        let enabled_terms: Vec<&SearchTerm> =
            query.terms.iter().filter(|t| t.enabled).collect();

        // 按操作符分组
        let mut and_terms = Vec::new();
        let mut or_terms = Vec::new();
        let mut not_terms = Vec::new();

        for term in &enabled_terms {
            match term.operator {
                QueryOperator::And => and_terms.push(*term),
                QueryOperator::Or => or_terms.push(*term),
                QueryOperator::Not => not_terms.push(*term),
            }
        }

        // 决定策略（优先级：AND > OR > NOT）
        let strategy = if !and_terms.is_empty() {
            SearchStrategy::And
        } else if !or_terms.is_empty() {
            SearchStrategy::Or
        } else {
            SearchStrategy::Not
        };

        // 编译正则表达式
        let mut regexes = Vec::new();
        let terms_list: Vec<String>;  // 将在match中初始化

        match strategy {
            SearchStrategy::And => {
                // 为每个search term创建一个CompiledRegex
                for term in &and_terms {
                    let term_regex = if term.case_sensitive {
                        Regex::new(&regex::escape(&term.value))
                    } else {
                        Regex::new(&format!("(?i:{})", regex::escape(&term.value)))
                    }.map_err(|e| ExecutionError::InvalidPattern(e.to_string()))?;
                    
                    regexes.push(CompiledRegex {
                        regex: term_regex,
                        term_id: term.id.clone(),
                        priority: term.priority,
                    });
                }
                // 保存搜索项用于验证
                terms_list = and_terms.iter().map(|t| t.value.to_lowercase()).collect();
            }
            SearchStrategy::Or => {
                // 为每个search term创建一个CompiledRegex
                for term in &or_terms {
                    let term_regex = if term.case_sensitive {
                        Regex::new(&regex::escape(&term.value))
                    } else {
                        Regex::new(&format!("(?i:{})", regex::escape(&term.value)))
                    }.map_err(|e| ExecutionError::InvalidPattern(e.to_string()))?;
                    
                    regexes.push(CompiledRegex {
                        regex: term_regex,
                        term_id: term.id.clone(),
                        priority: term.priority,
                    });
                }
                terms_list = or_terms.iter().map(|t| t.value.clone()).collect();
            }
            SearchStrategy::Not => {
                // 为每个search term创建一个CompiledRegex
                for term in &not_terms {
                    let term_regex = if term.case_sensitive {
                        Regex::new(&regex::escape(&term.value))
                    } else {
                        Regex::new(&format!("(?i:{})", regex::escape(&term.value)))
                    }.map_err(|e| ExecutionError::InvalidPattern(e.to_string()))?;
                    
                    regexes.push(CompiledRegex {
                        regex: term_regex,
                        term_id: term.id.clone(),
                        priority: term.priority,
                    });
                }
                terms_list = not_terms.iter().map(|t| t.value.clone()).collect();
            }
        }

        Ok(ExecutionPlan {
            strategy,
            regexes,
            term_count: enabled_terms.len(),
            terms: terms_list,
        })
    }

    /**
     * 构建 AND 策略的正则表达式
     * 注意：Rust regex 不支持前向断言，所以使用简单匹配+后处理
     */
    #[allow(dead_code)]  // 保留供将来使用
    fn build_and_regex(&mut self, terms: &[&SearchTerm]) -> Result<Regex, ExecutionError> {
        if terms.is_empty() {
            return Err(ExecutionError::EmptyQuery);
        }

        if terms.len() == 1 {
            let term = terms[0];
            let pattern = if term.case_sensitive {
                regex::escape(&term.value)
            } else {
                format!("(?i:{})", regex::escape(&term.value))
            };
            return self
                .get_or_compile_regex(&pattern)
                .map(|r| r.clone())
                .map_err(|e| ExecutionError::InvalidPattern(e.to_string()));
        }

        // 多个项：使用 OR 匹配，然后在 matches_line 中验证所有项都存在
        let patterns: Vec<String> = terms
            .iter()
            .map(|term| {
                let escaped = regex::escape(&term.value);
                if term.case_sensitive {
                    escaped
                } else {
                    format!("(?i:{})", escaped)
                }
            })
            .collect();

        let pattern = format!("({})", patterns.join("|"));

        self.get_or_compile_regex(&pattern)
            .map(|r| r.clone())
            .map_err(|e| ExecutionError::InvalidPattern(e.to_string()))
    }

    /**
     * 构建 OR 策略的正则表达式
     */
    #[allow(dead_code)]  // 保留供将来使用
    fn build_or_regex(&mut self, terms: &[&SearchTerm]) -> Result<Regex, ExecutionError> {
        if terms.is_empty() {
            return Err(ExecutionError::EmptyQuery);
        }

        let patterns: Vec<String> = terms
            .iter()
            .map(|term| {
                let escaped = regex::escape(&term.value);
                if term.case_sensitive {
                    escaped
                } else {
                    format!("(?i:{})", escaped)
                }
            })
            .collect();

        let pattern = format!("({})", patterns.join("|"));

        self.get_or_compile_regex(&pattern)
            .map(|r| r.clone())
            .map_err(|e| ExecutionError::InvalidPattern(e.to_string()))
    }

    /**
     * 构建 NOT 策略的正则表达式
     * 注意：Rust regex 不支持负向断言，需要在 matches_line 中反向处理
     */
    #[allow(dead_code)]  // 保留供将来使用
    fn build_not_regex(&mut self, terms: &[&SearchTerm]) -> Result<Regex, ExecutionError> {
        if terms.is_empty() {
            return Err(ExecutionError::EmptyQuery);
        }

        let patterns: Vec<String> = terms
            .iter()
            .map(|term| {
                let escaped = regex::escape(&term.value);
                if term.case_sensitive {
                    escaped
                } else {
                    format!("(?i:{})", escaped)
                }
            })
            .collect();

        let pattern = format!("({})", patterns.join("|"));

        self.get_or_compile_regex(&pattern)
            .map(|r| r.clone())
            .map_err(|e| ExecutionError::InvalidPattern(e.to_string()))
    }

    /**
     * 获取或编译正则表达式（带缓存）
     */
    #[allow(dead_code)]  // 缓存功能保留供将来使用
    fn get_or_compile_regex(&mut self, pattern: &str) -> Result<&Regex, ExecutionError> {
        if !self.regex_cache.contains_key(pattern) {
            let regex = Regex::new(pattern)
                .map_err(|e| ExecutionError::InvalidPattern(e.to_string()))?;

            // 限制缓存大小
            if self.regex_cache.len() >= self.cache_size {
                self.regex_cache.clear();
            }

            self.regex_cache.insert(pattern.to_string(), regex);
        }

        Ok(self.regex_cache.get(pattern).unwrap())
    }

    /**
     * 测试单行是否匹配
     */
    pub fn matches_line(&self, plan: &ExecutionPlan, line: &str) -> bool {
        match plan.strategy {
            SearchStrategy::And => {
                // AND 逻辑：必须包含所有搜索项
                let line_lower = line.to_lowercase();
                for term in &plan.terms {
                    if !line_lower.contains(term) {
                        return false;
                    }
                }
                true
            }
            SearchStrategy::Or => {
                // OR 逻辑：匹配任意一个
                for compiled in &plan.regexes {
                    if compiled.regex.is_match(line) {
                        return true;
                    }
                }
                false
            }
            SearchStrategy::Not => {
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
     */
    #[allow(dead_code)]  // 保留供将来使用或测试调用
    pub fn filter_lines<'a>(
        &self,
        plan: &ExecutionPlan,
        lines: &'a [String],
    ) -> Vec<&'a String> {
        lines
            .iter()
            .filter(|line| self.matches_line(plan, line))
            .collect()
    }

    /**
     * 匹配并返回详情（新方法）
     */
    pub fn match_with_details(&self, plan: &ExecutionPlan, line: &str) -> Option<Vec<MatchDetail>> {
        if !self.matches_line(plan, line) {
            return None;
        }

        let mut details = Vec::new();
        
        for compiled in &plan.regexes {
            if let Some(mat) = compiled.regex.find(line) {
                // 直接使用term_id和terms列表查找值
                let term_value = plan.terms
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
 * 执行计划
 */
#[allow(dead_code)]  // term_count 字段在测试中使用
pub struct ExecutionPlan {
    pub strategy: SearchStrategy,
    pub regexes: Vec<CompiledRegex>,
    pub term_count: usize,
    pub terms: Vec<String>, // 保存原始搜索项，用于AND验证
}

impl ExecutionPlan {
    /**
     * 获取搜索项数量
     */
    #[allow(dead_code)]  // 保留供将来使用或API调用
    pub fn get_term_count(&self) -> usize {
        self.term_count
    }

    /**
     * 获取所有搜索项
     */
    #[allow(dead_code)]  // 保留供将来使用或API调用
    pub fn get_terms(&self) -> &[String] {
        &self.terms
    }

    /**
     * 按优先级排序正则表达式
     */
    #[allow(dead_code)]  // 保留供将来使用或API调用
    pub fn sort_by_priority(&mut self) {
        self.regexes.sort_by_key(|r| std::cmp::Reverse(r.priority));
    }
}

/**
 * 搜索策略
 */
pub enum SearchStrategy {
    And,
    Or,
    Not,
}

/**
 * 编译后的正则表达式
 */
#[allow(dead_code)]  // term_id 和 priority 字段在 match_with_details 中使用
pub struct CompiledRegex {
    pub regex: Regex,
    pub term_id: String,
    pub priority: u32,
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

/**
 * 执行错误
 */
#[derive(Debug)]
pub enum ExecutionError {
    EmptyQuery,
    NoEnabledTerms,
    EmptyTermValue(String),
    TermValueTooLong(String),
    InvalidRegex(String, String),
    InvalidPattern(String),
}

impl std::fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::EmptyQuery => write!(f, "Query is empty"),
            Self::NoEnabledTerms => write!(f, "No enabled terms"),
            Self::EmptyTermValue(id) => write!(f, "Term {} has empty value", id),
            Self::TermValueTooLong(id) => write!(f, "Term {} value is too long", id),
            Self::InvalidRegex(id, msg) => write!(f, "Term {} has invalid regex: {}", id, msg),
            Self::InvalidPattern(msg) => write!(f, "Invalid pattern: {}", msg),
        }
    }
}

impl std::error::Error for ExecutionError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_term(value: &str, enabled: bool) -> SearchTerm {
        SearchTerm {
            id: "test".to_string(),
            value: value.to_string(),
            operator: QueryOperator::And,
            source: TermSource::User,
            preset_group_id: None,
            is_regex: false,
            priority: 1,
            enabled,
            case_sensitive: false,
        }
    }

    #[test]
    fn test_validate_empty_query() {
        let executor = QueryExecutor::new(100);
        let query = SearchQuery {
            id: "test".to_string(),
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

        let result = executor.validate(&query);
        assert!(result.is_err());
    }

    #[test]
    fn test_build_and_regex_single_term() {
        let mut executor = QueryExecutor::new(100);
        let term = create_test_term("error", true);

        let regex = executor.build_and_regex(&[&term]).unwrap();
        assert!(regex.is_match("This is an error message"));
        assert!(!regex.is_match("This is ok"));
    }

    #[test]
    fn test_build_and_regex_multiple_terms() {
        let mut executor = QueryExecutor::new(100);
        let term1 = create_test_term("error", true);
        let term2 = create_test_term("timeout", true);

        // AND逻辑现在不使用前向断言，而是在matches_line中验证
        let regex = executor.build_and_regex(&[&term1, &term2]).unwrap();
        // regex 只是用来匹配任意一个关键词
        assert!(regex.is_match("error occurred due to timeout"));
        assert!(regex.is_match("timeout caused error"));
        assert!(regex.is_match("just an error"));
        assert!(regex.is_match("only timeout"));
    }

    #[test]
    fn test_build_or_regex() {
        let mut executor = QueryExecutor::new(100);
        let term1 = SearchTerm {
            id: "1".to_string(),
            value: "error".to_string(),
            operator: QueryOperator::Or,
            source: TermSource::User,
            preset_group_id: None,
            is_regex: false,
            priority: 1,
            enabled: true,
            case_sensitive: false,
        };
        let term2 = SearchTerm {
            id: "2".to_string(),
            value: "warning".to_string(),
            operator: QueryOperator::Or,
            source: TermSource::User,
            preset_group_id: None,
            is_regex: false,
            priority: 1,
            enabled: true,
            case_sensitive: false,
        };

        let regex = executor.build_or_regex(&[&term1, &term2]).unwrap();
        assert!(regex.is_match("error occurred"));
        assert!(regex.is_match("warning message"));
        assert!(!regex.is_match("just info"));
    }

    #[test]
    fn test_build_not_regex() {
        let mut executor = QueryExecutor::new(100);
        let term = SearchTerm {
            id: "1".to_string(),
            value: "debug".to_string(),
            operator: QueryOperator::Not,
            source: TermSource::User,
            preset_group_id: None,
            is_regex: false,
            priority: 1,
            enabled: true,
            case_sensitive: false,
        };

        // NOT逻辑现在在matches_line中反向验证
        let regex = executor.build_not_regex(&[&term]).unwrap();
        // regex 本身只是匹配包含debug的行
        assert!(regex.is_match("debug message"));
        assert!(!regex.is_match("error message"));
    }

    #[test]
    fn test_matches_line() {
        let mut executor = QueryExecutor::new(100);
        let query = SearchQuery {
            id: "test".to_string(),
            terms: vec![create_test_term("error", true)],
            global_operator: QueryOperator::And,
            filters: None,
            metadata: QueryMetadata {
                created_at: 0,
                last_modified: 0,
                execution_count: 0,
                label: None,
            },
        };

        let plan = executor.execute(&query).unwrap();

        assert!(executor.matches_line(&plan, "error occurred"));
        assert!(!executor.matches_line(&plan, "all good"));
    }

    #[test]
    fn test_filter_lines() {
        let mut executor = QueryExecutor::new(100);
        let query = SearchQuery {
            id: "test".to_string(),
            terms: vec![create_test_term("error", true)],
            global_operator: QueryOperator::And,
            filters: None,
            metadata: QueryMetadata {
                created_at: 0,
                last_modified: 0,
                execution_count: 0,
                label: None,
            },
        };

        let plan = executor.execute(&query).unwrap();
        let lines = vec![
            "error occurred".to_string(),
            "all good".to_string(),
            "another error".to_string(),
        ];

        let filtered = executor.filter_lines(&plan, &lines);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_and_logic_with_multiple_terms() {
        let mut executor = QueryExecutor::new(100);
        let term1 = create_test_term("error", true);
        let term2 = create_test_term("timeout", true);

        let query = SearchQuery {
            id: "test".to_string(),
            terms: vec![term1, term2],
            global_operator: QueryOperator::And,
            filters: None,
            metadata: QueryMetadata {
                created_at: 0,
                last_modified: 0,
                execution_count: 0,
                label: None,
            },
        };

        let plan = executor.execute(&query).unwrap();

        // AND逻辑：必须同时包含两个关键词
        assert!(executor.matches_line(&plan, "error occurred due to timeout"));
        assert!(executor.matches_line(&plan, "timeout caused error"));
        assert!(!executor.matches_line(&plan, "just an error"));
        assert!(!executor.matches_line(&plan, "only timeout"));
    }

    #[test]
    fn test_match_with_details() {
        let mut executor = QueryExecutor::new(100);
        
        let term1 = SearchTerm {
            id: "term1".to_string(),
            value: "error".to_string(),
            operator: QueryOperator::Or,
            source: TermSource::User,
            preset_group_id: None,
            is_regex: false,
            priority: 2,  // 高优先级
            enabled: true,
            case_sensitive: false,
        };
        
        let term2 = SearchTerm {
            id: "term2".to_string(),
            value: "warning".to_string(),
            operator: QueryOperator::Or,
            source: TermSource::User,
            preset_group_id: None,
            is_regex: false,
            priority: 1,  // 低优先级
            enabled: true,
            case_sensitive: false,
        };

        let query = SearchQuery {
            id: "test".to_string(),
            terms: vec![term1, term2],
            global_operator: QueryOperator::Or,
            filters: None,
            metadata: QueryMetadata {
                created_at: 0,
                last_modified: 0,
                execution_count: 0,
                label: None,
            },
        };

        let plan = executor.execute(&query).unwrap();
        
        // 测试匹配详情
        let line = "This is an error with warning";
        let details = executor.match_with_details(&plan, line);
        
        assert!(details.is_some());
        let details = details.unwrap();
        
        // 应该有两个匹配
        assert_eq!(details.len(), 2);
        
        // 验证按优先级排序（高优先级在前）
        assert_eq!(details[0].term_value, "error");
        assert_eq!(details[0].priority, 2);
        assert_eq!(details[1].term_value, "warning");
        assert_eq!(details[1].priority, 1);
        
        // 验证匹配位置
        assert!(details[0].match_position.is_some());
        assert!(details[1].match_position.is_some());
    }

    #[test]
    fn test_execution_plan_methods() {
        let mut executor = QueryExecutor::new(100);
        
        let term1 = SearchTerm {
            id: "term1".to_string(),
            value: "error".to_string(),
            operator: QueryOperator::Or,
            source: TermSource::User,
            preset_group_id: None,
            is_regex: false,
            priority: 1,
            enabled: true,
            case_sensitive: false,
        };
        
        let term2 = SearchTerm {
            id: "term2".to_string(),
            value: "warning".to_string(),
            operator: QueryOperator::Or,
            source: TermSource::User,
            preset_group_id: None,
            is_regex: false,
            priority: 2,
            enabled: true,
            case_sensitive: false,
        };

        let query = SearchQuery {
            id: "test".to_string(),
            terms: vec![term1, term2],
            global_operator: QueryOperator::Or,
            filters: None,
            metadata: QueryMetadata {
                created_at: 0,
                last_modified: 0,
                execution_count: 0,
                label: None,
            },
        };

        let mut plan = executor.execute(&query).unwrap();
        
        // 测试 get_term_count
        assert_eq!(plan.get_term_count(), 2);
        
        // 测试 get_terms
        let terms = plan.get_terms();
        assert_eq!(terms.len(), 2);
        assert!(terms.contains(&"error".to_string()));
        assert!(terms.contains(&"warning".to_string()));
        
        // 测试 sort_by_priority
        plan.sort_by_priority();
        // 验证排序后的顺序（高优先级在前）
        assert_eq!(plan.regexes[0].priority, 2);
        assert_eq!(plan.regexes[1].priority, 1);
    }

    #[test]
    fn test_match_detail_serialization() {
        // 测试 MatchDetail 的序列化
        let detail = MatchDetail {
            term_id: "test_term".to_string(),
            term_value: "error".to_string(),
            priority: 5,
            match_position: Some((10, 15)),
        };
        
        let json = serde_json::to_string(&detail).unwrap();
        assert!(json.contains("test_term"));
        assert!(json.contains("error"));
        assert!(json.contains("priority"));
        
        // 测试反序列化
        let deserialized: MatchDetail = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.term_id, "test_term");
        assert_eq!(deserialized.term_value, "error");
        assert_eq!(deserialized.priority, 5);
        assert_eq!(deserialized.match_position, Some((10, 15)));
    }
}
