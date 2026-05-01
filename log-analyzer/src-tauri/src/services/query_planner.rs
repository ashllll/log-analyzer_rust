use crate::services::regex_engine::{EngineType, RegexEngine};
use crate::services::traits::{PlanResult, QueryPlanning};
use la_core::error::{AppError, Result};
use la_core::models::search::*;
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
    fn regex_has_inline_case_flag(pattern: &str) -> bool {
        let bytes = pattern.as_bytes();
        let mut index = 0;

        while index + 1 < bytes.len() {
            if bytes[index] == b'(' && bytes[index + 1] == b'?' {
                let mut cursor = index + 2;
                let mut saw_flag = false;
                let mut has_case_flag = false;

                while cursor < bytes.len() {
                    let ch = bytes[cursor] as char;
                    if ch == ')' || ch == ':' {
                        if saw_flag && has_case_flag {
                            return true;
                        }
                        break;
                    }

                    if ch.is_ascii_alphabetic() || ch == '-' {
                        saw_flag = true;
                        if ch == 'i' {
                            has_case_flag = true;
                        }
                        cursor += 1;
                        continue;
                    }

                    break;
                }
            }

            index += 1;
        }

        false
    }

    /**
     * 创建新的计划构建器
     *
     * # 参数
     * * `max_capacity` - 引擎缓存最大容量
     */
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
    #[cfg(test)]
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

        let fast_or_engine = self.build_fast_or_engine(&enabled_terms, &strategy)?;

        Ok(ExecutionPlan::new(
            strategy,
            engines,
            enabled_terms.len(),
            terms_list,
            fast_or_engine,
        ))
    }

    fn build_fast_or_engine(
        &mut self,
        enabled_terms: &[&SearchTerm],
        strategy: &SearchStrategy,
    ) -> Result<Option<Arc<RegexEngine>>> {
        if *strategy != SearchStrategy::Or || enabled_terms.len() < 2 {
            return Ok(None);
        }

        if enabled_terms
            .iter()
            .any(|term| term.is_regex || term.value.is_empty())
        {
            return Ok(None);
        }

        let case_sensitive = enabled_terms[0].case_sensitive;
        if enabled_terms
            .iter()
            .any(|term| term.case_sensitive != case_sensitive)
        {
            return Ok(None);
        }

        let combined_pattern = enabled_terms
            .iter()
            .map(|term| term.value.as_str())
            .collect::<Vec<_>>()
            .join("|");

        let engine = if case_sensitive {
            RegexEngine::new(&combined_pattern, false)
        } else {
            RegexEngine::new_with_case(&combined_pattern, false, true)
        }
        .map_err(|e| AppError::validation_error(format!("Engine error: {}", e)))?;

        Ok(Some(Arc::new(engine)))
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
        let (pattern, use_ci) = if term.is_regex {
            let pattern = if term.case_sensitive || Self::regex_has_inline_case_flag(&term.value) {
                term.value.clone()
            } else {
                format!("(?i:{})", term.value)
            };
            (pattern, false)
        } else {
            let raw_pattern = &term.value;

            if raw_pattern.contains('|') {
                // 多模式：保留原始模式，由 AhoCorasick ascii_case_insensitive 处理
                (raw_pattern.clone(), !term.case_sensitive)
            } else {
                let escaped = regex::escape(raw_pattern);
                if term.case_sensitive {
                    (escaped, false)
                } else {
                    // Fix: do NOT wrap with (?i:) which makes is_simple_keyword fail.
                    // Instead pass case_insensitive=true to new_with_case so that
                    // MemchrEngine (for simple keywords) or AhoCorasickEngine can be used.
                    (escaped, true)
                }
            }
        };

        let cache_key = format!(
            "{}|{}|{}|{}",
            pattern, term.is_regex, term.case_sensitive, use_ci
        );

        if let Some(cached) = self.engine_cache.get(&cache_key) {
            return Ok(Arc::clone(&cached));
        }

        let engine = if use_ci {
            RegexEngine::new_with_case(&pattern, term.is_regex, true)
        } else {
            RegexEngine::new(&pattern, term.is_regex)
        }
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
    pub term_count: usize,
    pub terms: Vec<PlanTerm>,
    pub fast_or_engine: Option<Arc<RegexEngine>>,
    /// term_id 到 engine 的 HashMap 映射，用于 O(1) 查找
    engine_map: std::collections::HashMap<String, Arc<RegexEngine>>,
    execution_order: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlanTerm {
    pub id: String,
    pub value: String,
    pub case_sensitive: bool,
}

impl ExecutionPlan {
    /**
     * 创建新的执行计划
     */
    pub fn new(
        strategy: SearchStrategy,
        engines: Vec<CompiledEngine>,
        term_count: usize,
        terms: Vec<PlanTerm>,
        fast_or_engine: Option<Arc<RegexEngine>>,
    ) -> Self {
        // 构建 term_id 到 engine 的 HashMap，实现 O(1) 查找
        let engine_map: std::collections::HashMap<String, Arc<RegexEngine>> = engines
            .iter()
            .map(|e| (e.term_id.clone(), Arc::clone(&e.engine)))
            .collect();

        let execution_order = Self::build_execution_order(&engines, &terms);

        Self {
            strategy,
            engines,
            term_count,
            terms,
            fast_or_engine,
            engine_map,
            execution_order,
        }
    }

    fn build_execution_order(engines: &[CompiledEngine], terms: &[PlanTerm]) -> Vec<String> {
        let term_lengths = terms
            .iter()
            .map(|term| (term.id.as_str(), term.value.len()))
            .collect::<std::collections::HashMap<_, _>>();

        let mut ordered = engines
            .iter()
            .map(|engine| {
                let term_length = term_lengths
                    .get(engine.term_id.as_str())
                    .copied()
                    .unwrap_or(0);
                (
                    engine.term_id.clone(),
                    engine.priority,
                    Self::engine_cost_rank(engine.engine.as_ref()),
                    term_length,
                )
            })
            .collect::<Vec<_>>();

        ordered.sort_by(|left, right| {
            right
                .1
                .cmp(&left.1)
                .then_with(|| left.2.cmp(&right.2))
                .then_with(|| right.3.cmp(&left.3))
                .then_with(|| left.0.cmp(&right.0))
        });

        ordered.into_iter().map(|entry| entry.0).collect()
    }

    fn engine_cost_rank(engine: &RegexEngine) -> u8 {
        match engine.engine_type() {
            EngineType::Memchr => 0,      // SIMD 子串搜索，最快
            EngineType::AhoCorasick => 1, // 多模式自动机，次快
            EngineType::Automata => 2,
            EngineType::Standard => 3,
            EngineType::Fancy => 4, // 回溯引擎，最慢
        }
    }

    /**
     * 根据 term_id 获取对应的引擎
     * 使用 HashMap 实现 O(1) 查找
     */
    pub fn get_engine_for_term(&self, term_id: &str) -> Option<Arc<RegexEngine>> {
        self.engine_map.get(term_id).cloned()
    }

    pub fn execution_term_ids(&self) -> &[String] {
        &self.execution_order
    }

    /**
     * 获取搜索项数量
     */
    #[cfg(test)]
    pub fn get_term_count(&self) -> usize {
        self.term_count
    }

    /**
     * 获取所有搜索项
     */
    #[cfg(test)]
    pub fn get_terms(&self) -> &[PlanTerm] {
        &self.terms
    }

    /**
     * 按优先级排序引擎
     */
    pub fn sort_by_priority(&mut self) {
        self.engines.sort_by(|left, right| {
            right.priority.cmp(&left.priority).then_with(|| {
                Self::engine_cost_rank(left.engine.as_ref())
                    .cmp(&Self::engine_cost_rank(right.engine.as_ref()))
            })
        });
        self.execution_order = Self::build_execution_order(&self.engines, &self.terms);
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

/// Adapter struct for implementing QueryPlanning trait
///
/// This wrapper uses interior mutability to allow the trait's immutable
/// `plan` method to work with the planner's mutable build_plan method.
pub struct QueryPlannerAdapter {
    inner: parking_lot::Mutex<QueryPlanner>,
}

impl QueryPlannerAdapter {
    /// Create a new adapter with default capacity
    pub fn new() -> Self {
        Self {
            inner: parking_lot::Mutex::new(QueryPlanner::with_default_capacity()),
        }
    }

    /// Create a new adapter with specified cache capacity
    pub fn with_capacity(max_capacity: usize) -> Self {
        Self {
            inner: parking_lot::Mutex::new(QueryPlanner::new(max_capacity)),
        }
    }
}

impl Default for QueryPlannerAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl QueryPlanning for QueryPlannerAdapter {
    fn plan(&self, query: &SearchQuery) -> Result<PlanResult> {
        let mut planner = self.inner.lock();
        let plan = planner.build_plan(query)?;
        let steps = plan
            .terms
            .iter()
            .map(|t| format!("Search for '{}'", t.value))
            .collect();
        let cost = plan.term_count as u32 * 10; // Simple cost estimation
        Ok(PlanResult::new(steps, cost))
    }

    fn build_execution_plan(&self, query: &SearchQuery) -> Result<ExecutionPlan> {
        let mut planner = self.inner.lock();
        planner.build_plan(query)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use la_core::models::search::{QueryMetadata, TermSource};

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
        let engines = vec![
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
        ];
        let terms = vec![
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
        ];
        let plan = ExecutionPlan::new(SearchStrategy::And, engines, 2, terms, None);

        assert_eq!(plan.engines.len(), 2);
        assert_eq!(plan.engines[0].priority, 1);
        assert_eq!(plan.engines[1].priority, 3);
    }

    #[test]
    fn test_plan_sort_by_priority_engines() {
        let engines = vec![
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
        ];
        let mut plan = ExecutionPlan::new(SearchStrategy::And, engines, 3, vec![], None);

        plan.sort_by_priority();

        assert_eq!(plan.engines[0].priority, 3);
        assert_eq!(plan.engines[1].priority, 2);
        assert_eq!(plan.engines[2].priority, 1);
    }

    #[test]
    fn test_build_or_plan_creates_fast_or_engine_for_simple_terms() {
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

        assert!(plan.fast_or_engine.is_some());
        assert!(plan
            .fast_or_engine
            .as_ref()
            .unwrap()
            .is_match("warning: service degraded"));
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
    fn test_case_insensitive_regex_terms_match_case_variants() {
        let mut planner = QueryPlanner::new(100);
        let term = SearchTerm {
            id: "test".to_string(),
            value: "error.*timeout".to_string(),
            operator: QueryOperator::And,
            source: TermSource::Preset,
            preset_group_id: Some("group_1".to_string()),
            is_regex: true,
            priority: 1,
            enabled: true,
            case_sensitive: false,
        };

        let engine = planner.get_or_compile_engine(&term).unwrap();
        assert!(engine.is_match("ERROR before TIMEOUT"));
    }

    #[test]
    fn test_regex_terms_respect_inline_case_flags() {
        let mut planner = QueryPlanner::new(100);
        let term = SearchTerm {
            id: "test".to_string(),
            value: "(?-i:error).*timeout".to_string(),
            operator: QueryOperator::And,
            source: TermSource::Preset,
            preset_group_id: Some("group_1".to_string()),
            is_regex: true,
            priority: 1,
            enabled: true,
            case_sensitive: false,
        };

        let engine = planner.get_or_compile_engine(&term).unwrap();
        assert!(!engine.is_match("ERROR before timeout"));
        assert!(engine.is_match("error before timeout"));
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
        // 简单关键词（case-sensitive）应使用 MemchrEngine（SIMD 加速）
        let engine = RegexEngine::new("error", false).unwrap();
        assert!(
            matches!(engine, RegexEngine::Memchr(_)),
            "Simple keyword should use MemchrEngine"
        );
        assert!(engine.is_match("error occurred"));
        assert!(!engine.is_match("ERROR occurred"));
    }

    #[test]
    fn test_engine_selection_case_insensitive() {
        // case-insensitive 模式 (?i:...) 应回退到 StandardEngine
        let engine = RegexEngine::new("(?i:error)", false).unwrap();
        assert!(
            matches!(engine, RegexEngine::Standard(_)),
            "Case-insensitive pattern should use StandardEngine"
        );
        assert!(engine.is_match("ERROR occurred"));
    }

    #[test]
    fn test_engine_selection_regex() {
        let pattern = r"\d{4}-\d{2}-\d{2}";
        let engine = RegexEngine::new(pattern, true).unwrap();

        assert!(
            matches!(engine, RegexEngine::Automata(_)),
            "Complex regex should use AutomataEngine"
        );
        assert!(engine.is_match("2024-01-30"));
    }

    #[test]
    fn test_engine_selection_fancy_lookaround() {
        // lookaround 模式应使用 FancyEngine
        let engine = RegexEngine::new(r"(?<=foo)bar", true).unwrap();
        assert!(
            matches!(engine, RegexEngine::Fancy(_)),
            "Lookaround should use FancyEngine"
        );
        assert!(engine.is_match("foobar"));
        assert!(!engine.is_match("bar"));
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

        assert!(
            matches.len() >= 3,
            "Expected at least 3 matches, got {}",
            matches.len()
        );
    }
}
