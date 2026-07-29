//! LogSearcher adapter backed by the existing query planner/executor.
//!
//! P7: 消除 side-map (Mutex<HashMap<u64, ServiceExecutionPlan>>)。
//! 真实 plan 现在嵌入 domain ExecutionPlan 的 plan 字段中，
//! 通过 MatchPlan trait 调用——无全局缓存，无 Any 向下转型，编译期类型安全。

use std::sync::Arc;

use parking_lot::Mutex;

use la_core::domain::filter::{CompiledSearchFilter, LineMetadata};
use la_core::domain::{ExecutionPlan, LogSearcher};
use la_core::error::Result;
use la_core::models::{LogEntry, SearchQuery};

use crate::services::query_planner::QueryPlanner;
use crate::services::search_filters::ParsedLineMetadata;

/// Domain LogSearcher implementation using the production regex/query engine.
pub struct QueryEngineLogSearcher {
    planner: Mutex<QueryPlanner>,
}

impl QueryEngineLogSearcher {
    pub fn new(regex_cache_size: usize) -> Self {
        Self {
            planner: Mutex::new(QueryPlanner::new(regex_cache_size.max(1))),
        }
    }
}

impl LogSearcher for QueryEngineLogSearcher {
    fn build_plan(&self, query: &SearchQuery) -> Result<ExecutionPlan> {
        let service_plan = self.planner.lock().build(query)?;
        let engine_count = service_plan.engines.len();
        let steps = service_plan.execution_term_ids().to_vec();
        let plan = ExecutionPlan {
            id: 0,
            engine_count,
            steps,
            plan: Some(std::sync::Arc::new(service_plan)),
        };
        Ok(plan)
    }

    fn match_content(
        &self,
        content: &str,
        virtual_path: &str,
        plan: &ExecutionPlan,
        compiled_filter: &dyn CompiledSearchFilter,
        global_offset: usize,
    ) -> Vec<LogEntry> {
        if !compiled_filter.matches_file(virtual_path, None) {
            return Vec::new();
        }

        let match_plan = match &plan.plan {
            Some(p) => p,
            None => return Vec::new(),
        };

        let has_time = compiled_filter.has_time_filter();
        let mut entries = Vec::new();
        // 缓存 virtual_path 为 Arc<str>，避免每行分配两次 to_string()
        let virtual_path_arc: Arc<str> = Arc::from(virtual_path);
        for (index, line) in content.lines().enumerate() {
            let metadata = ParsedLineMetadata::parse(line, has_time);
            if !compiled_filter.matches_line(&LineMetadata {
                timestamp: metadata.timestamp.clone(),
                level: metadata.level,
                level_normalized: metadata.level_normalized,
                datetime: metadata.datetime,
                level_mask: metadata.level_mask,
            }) {
                continue;
            }

            if let Some(details) = match_plan.match_line(line) {
                // 使用排序+去重替代 HashSet，减少堆分配
                let mut keywords: Vec<String> = details
                    .iter()
                    .map(|detail| detail.term_value.clone())
                    .collect();
                keywords.sort_unstable();
                keywords.dedup();

                entries.push(LogEntry {
                    id: global_offset + entries.len(),
                    timestamp: metadata.timestamp.into(),
                    level: metadata.level.into(),
                    file: Arc::clone(&virtual_path_arc),
                    real_path: Arc::clone(&virtual_path_arc),
                    line: index + 1,
                    content: line.to_string().into(),
                    tags: vec![],
                    match_details: if details.is_empty() {
                        None
                    } else {
                        Some(details)
                    },
                    matched_keywords: if keywords.is_empty() {
                        None
                    } else {
                        Some(keywords)
                    },
                });
            }
        }

        entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use la_core::domain::MatchPlan;
    use la_core::models::match_detail::MatchDetail;

    struct MatchEveryLine;

    impl MatchPlan for MatchEveryLine {
        fn match_line(&self, line: &str) -> Option<Vec<MatchDetail>> {
            Some(vec![MatchDetail {
                term_id: "term-1".into(),
                term_value: line.into(),
                priority: 1,
                match_position: Some((0, line.len())),
            }])
        }
    }

    struct ErrorOnlyFilter;

    impl CompiledSearchFilter for ErrorOnlyFilter {
        fn matches_file(&self, _virtual_path: &str, _real_path: Option<&str>) -> bool {
            true
        }

        fn matches_line(&self, metadata: &LineMetadata) -> bool {
            metadata.level_normalized == "error"
        }

        fn has_time_filter(&self) -> bool {
            false
        }
    }

    #[test]
    fn match_content_uses_precompiled_filter() {
        let searcher = QueryEngineLogSearcher::new(1);
        let plan = ExecutionPlan {
            id: 1,
            engine_count: 1,
            steps: vec!["test".into()],
            plan: Some(Arc::new(MatchEveryLine)),
        };

        let entries = searcher.match_content(
            "INFO ready\nERROR failed",
            "service.log",
            &plan,
            &ErrorOnlyFilter,
            0,
        );

        assert_eq!(entries.len(), 1);
        assert_eq!(&*entries[0].level, "error");
        assert_eq!(&*entries[0].content, "ERROR failed");
    }
}
