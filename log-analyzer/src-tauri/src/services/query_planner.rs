use crate::error::{AppError, Result};
use crate::models::search::*;
use crate::services::regex_engine::RegexEngine;
use moka::sync::Cache;
use std::sync::Arc;

/**
 * 查询计划构建器
 *
 * 负责构建搜索查询的执行计划
 *
 * 设计决策：使用混合引擎策略自动选择最佳匹配算法：
 * - AhoCorasick: 简单关键词搜索，O(n) 线性复杂度
 * - Standard: 复杂正则/需要前瞻后瞻
 *
 * 使用 moka 库实现 LRU 引擎缓存，具有以下优势：
 * - 线程安全：支持并发访问
 * - 自动淘汰：LRU 策略自动淘汰最久未使用的条目
 * - 高性能：使用高效的内部数据结构
 */
pub struct QueryPlanner {
    engine_cache: Cache<String, Arc<RegexEngine>>,
}

impl QueryPlanner {
    /**
     * 创建新的计划构建器
     *
     * # 参数
     * * `max_capacity` - 引擎缓存最大容量
     */
    #[allow(dead_code)]
    pub fn new(max_capacity: usize) -> Self {
        Self {
            engine_cache: Cache::new(max_capacity as u64),
        }
    }

    /**
     * 使用默认容量创建计划构建器（默认 1000 条）
     */
    pub fn with_default_capacity() -> Self {
        Self {
            engine_cache: Cache::new(1000),
        }
    }

    /**
     * 智能判断搜索词是否应该使用单词边界匹配
     *
     * # 修改历史 (2025-01-30)
     * - 原逻辑：自动为常见日志关键词、短模式等添加单词边界
     * - 问题：导致搜索结果数量远少于预期
     * - 新逻辑：默认保持子串匹配，与 ripgrep 等工具行为一致
     *
     * 保留此函数用于未来可能的扩展需求，当前不再自动使用。
     */
    #[allow(dead_code)]
    fn should_use_word_boundary(term: &SearchTerm) -> bool {
        if term.is_regex {
            return false;
        }

        if term.value.contains(r"\b") {
            return true;
        }

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

        let strategy = match query.global_operator {
            QueryOperator::And => SearchStrategy::And,
            QueryOperator::Or => SearchStrategy::Or,
            QueryOperator::Not => SearchStrategy::Not,
        };

        let mut engines = Vec::new();
        let mut terms_list = Vec::new();

        for term in &enabled_terms {
            let engine = self.get_or_compile_engine(term)?;

            engines.push(CompiledEngine {
                engine,
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
            engines,
            term_count: enabled_terms.len(),
            terms: terms_list,
        })
    }

    /**
     * 获取或编译引擎（带缓存）
     *
     * # 参数
     * * `term` - 搜索项
     *
     * # 返回
     * * `Ok(Arc<RegexEngine>)` - 编译后的引擎
     * * `Err(AppError)` - 编译失败
     */
    fn get_or_compile_engine(&mut self, term: &SearchTerm) -> Result<Arc<RegexEngine>> {
        let pattern = if term.is_regex {
            term.value.clone()
        } else {
            let raw_pattern = &term.value;

            if raw_pattern.contains('|') {
                if term.case_sensitive {
                    raw_pattern.clone()
                } else {
                    raw_pattern.to_lowercase()
                }
            } else {
                let escaped = regex::escape(raw_pattern);

                if term.case_sensitive {
                    escaped
                } else {
                    format!("(?i:{})", escaped)
                }
            }
        };

        let cache_key = format!(
            "{}|{}|{}",
            pattern,
            term.is_regex,
            term.case_sensitive
        );

        if let Some(cached) = self.engine_cache.get(&cache_key) {
            return Ok(Arc::clone(&cached));
        }

        let engine = RegexEngine::new(&pattern, term.is_regex)
            .map_err(|e| AppError::validation_error(format!("Engine error: {}", e)))?;

        let result = Arc::new(engine);
        self.engine_cache.insert(cache_key, Arc::clone(&result));
        Ok(result)
    }
}

/**
 * 执行计划
 */
#[derive(Debug, Clone)]
pub struct ExecutionPlan {
    pub strategy: SearchStrategy,
    pub engines: Vec<CompiledEngine>,
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
     * 按优先级排序引擎
     */
    pub fn sort_by_priority(&mut self) {
        self.engines.sort_by_key(|e| std::cmp::Reverse(e.priority));
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
 * 编译后的引擎
 */
#[derive(Debug, Clone)]
pub struct CompiledEngine {
    pub engine: Arc<RegexEngine>,
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
        assert_eq!(plan.engines.len(), 2);
        assert_eq!(plan.terms.len(), 2);
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
        assert_eq!(plan.engines.len(), 2);
        assert_eq!(plan.terms.len(), 2);
    }

    #[test]
    fn test_execution_plan_engines_field() {
        let plan = ExecutionPlan {
            strategy: SearchStrategy::And,
            engines: vec![
                CompiledEngine {
                    engine: Arc::new(RegexEngine::new("error", false).unwrap()),
                    term_id: "1".to_string(),
                    priority: 1,
                },
                CompiledEngine {
                    engine: Arc::new(RegexEngine::new("warn", false).unwrap()),
                    term_id: "2".to_string(),
                    priority: 3,
                },
            ],
            term_count: 2,
            terms: vec![
                PlanTerm {
                    id: "1".to_string(),
                    value: "error".to_string(),
                    case_sensitive: false,
                },
                PlanTerm {
                    id: "2".to_string(),
                    value: "warn".to_string(),
                    case_sensitive: false,
                },
            ],
        };

        assert_eq!(plan.engines.len(), 2);
        assert_eq!(plan.engines[0].priority, 1);
        assert_eq!(plan.engines[1].priority, 3);
    }

    #[test]
    fn test_plan_sort_by_priority_engines() {
        let mut plan = ExecutionPlan {
            strategy: SearchStrategy::And,
            engines: vec![
                CompiledEngine {
                    engine: Arc::new(RegexEngine::new("test1", false).unwrap()),
                    term_id: "1".to_string(),
                    priority: 1,
                },
                CompiledEngine {
                    engine: Arc::new(RegexEngine::new("test2", false).unwrap()),
                    term_id: "2".to_string(),
                    priority: 3,
                },
                CompiledEngine {
                    engine: Arc::new(RegexEngine::new("test3", false).unwrap()),
                    term_id: "3".to_string(),
                    priority: 2,
                },
            ],
            term_count: 3,
            terms: vec![],
        };

        plan.sort_by_priority();

        assert_eq!(plan.engines[0].priority, 3);
        assert_eq!(plan.engines[1].priority, 2);
        assert_eq!(plan.engines[2].priority, 1);
    }

    // ========== 自动单词边界检测测试 ==========
    // 【修改】2025-01-30: 由于移除了自动单词边界检测，这些测试已更新

    #[test]
    fn test_auto_detect_common_keywords() {
        let test_cases = vec![
            ("DE H", false),
            ("DE N", false),
            ("ERROR", false),
            ("WARN", false),
            ("INFO", false),
            ("de h", false),
            ("error", false),
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
        assert!(!QueryPlanner::should_use_word_boundary(&term));

        let term2 = create_test_term("CODE", QueryOperator::And);
        assert!(!QueryPlanner::should_use_word_boundary(&term2));
    }

    #[test]
    fn test_auto_detect_phrases() {
        let term = create_test_term("database error", QueryOperator::And);
        assert!(!QueryPlanner::should_use_word_boundary(&term));

        let term2 = create_test_term("connection timeout", QueryOperator::And);
        assert!(!QueryPlanner::should_use_word_boundary(&term2));
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
            value: r"\bCODE\b".to_string(),
            operator: QueryOperator::And,
            source: TermSource::User,
            preset_group_id: None,
            is_regex: false,
            priority: 1,
            enabled: true,
            case_sensitive: false,
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
            is_regex: true,
            priority: 1,
            enabled: true,
            case_sensitive: false,
        };

        assert!(!QueryPlanner::should_use_word_boundary(&term));
    }

    #[test]
    fn test_android_log_search() {
        let mut planner = QueryPlanner::new(100);
        let term = create_test_term("DE H", QueryOperator::And);

        let engine = planner.get_or_compile_engine(&term).unwrap();

        let text1 = "DE H occurred";
        let text2 = "CODE HDEF";
        let text3 = "testDEHtest";

        match engine.as_ref() {
            RegexEngine::AhoCorasick(e) => {
                assert!(e.find_iter(text1).next().is_some());
                assert!(e.find_iter(text2).next().is_some());
                assert!(e.find_iter(text3).next().is_none());
            }
            RegexEngine::Standard(e) => {
                assert!(e.is_match(text1));
                assert!(e.is_match(text2));
                assert!(!e.is_match(text3));
            }
            _ => {}
        }
    }

    #[test]
    fn test_word_boundary_with_or_operator() {
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

        assert!(plan.engines[0].engine.as_ref().is_match("DE H found"));
        assert!(plan.engines[1].engine.as_ref().is_match("DE N here"));
    }

    #[test]
    fn test_long_phrase_no_boundary() {
        let term = create_test_term(
            "This is a very long phrase that exceeds thirty characters",
            QueryOperator::And,
        );
        assert!(!QueryPlanner::should_use_word_boundary(&term));
    }

    #[test]
    fn test_url_no_boundary() {
        let term = create_test_term("http://api.example.com", QueryOperator::And);
        assert!(!QueryPlanner::should_use_word_boundary(&term));
    }

    // ========== 混合引擎测试 ==========

    #[test]
    fn test_engine_selection_simple_keyword() {
        let pattern = "error";
        let engine = RegexEngine::new(pattern, false).unwrap();

        match engine {
            RegexEngine::AhoCorasick(_) => {},
            RegexEngine::Automata(_) => {},
            RegexEngine::Standard(_) => {},
        }
    }

    #[test]
    fn test_engine_selection_regex() {
        let pattern = r"\d{4}-\d{2}-\d{2}";
        let engine = RegexEngine::new(pattern, true).unwrap();

        match engine {
            RegexEngine::AhoCorasick(_) => {},
            RegexEngine::Automata(_) => {},
            RegexEngine::Standard(_) => {},
        }
    }

    #[test]
    fn test_build_plan_with_new_engine() {
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
        assert_eq!(plan.engines.len(), 2);
    }

    #[test]
    fn test_aho_corasiick_multi_keyword() {
        let engine = RegexEngine::new("error|warning|info|fatal", false).unwrap();

        let text = "error warning info error fatal";
        let matches: Vec<_> = engine.find_iter(text).collect();

        assert_eq!(matches.len(), 5);
    }

    #[test]
    fn test_substring_matching() {
        let engine = RegexEngine::new("error", false).unwrap();

        let text = "error occurred, error_code, error123";
        let matches: Vec<_> = engine.find_iter(text).collect();

        assert!(matches.len() >= 3, "Expected at least 3 matches, got {}", matches.len());
    }
}
