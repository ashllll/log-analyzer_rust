//! LogSearcher adapter backed by the existing query planner/executor.
//!
//! P7: 消除 side-map (Mutex<HashMap<u64, ServiceExecutionPlan>>)。
//! 真实 plan 现在嵌入 domain ExecutionPlan 的 plan 字段中，
//! 通过 MatchPlan trait 调用——无全局缓存，无 Any 向下转型，编译期类型安全。

use std::collections::HashSet;

use parking_lot::Mutex;

use la_core::domain::filter::{Filter, LineMetadata};
use la_core::domain::{ExecutionPlan, LogSearcher};
use la_core::error::Result;
use la_core::models::{LogEntry, SearchFilters, SearchQuery};

use crate::services::query_planner::QueryPlanner;
use crate::services::search_filters::{CompiledSearchFilters, ParsedLineMetadata};

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
        filters: &SearchFilters,
        global_offset: usize,
    ) -> Vec<LogEntry> {
        // Compile filters via the concrete adapter, then use the Filter trait.
        let compiled: Box<dyn Filter> = match CompiledSearchFilters::compile(filters) {
            Ok(f) => Box::new(f),
            Err(_) => return Vec::new(),
        };

        if !compiled.matches_file(virtual_path, None) {
            return Vec::new();
        }

        let match_plan = match &plan.plan {
            Some(p) => p,
            None => return Vec::new(),
        };

        let has_time = compiled.has_time_filter();
        let mut entries = Vec::new();
        for (index, line) in content.lines().enumerate() {
            let metadata = ParsedLineMetadata::parse(line, has_time);
            if !compiled.matches_line(&LineMetadata {
                timestamp: metadata.timestamp.clone(),
                level: metadata.level,
                level_normalized: metadata.level_normalized,
                datetime: metadata.datetime,
                level_mask: metadata.level_mask,
            }) {
                continue;
            }

            if let Some(details) = match_plan.match_line(line) {
                let keywords = details
                    .iter()
                    .map(|detail| detail.term_value.clone())
                    .collect::<HashSet<_>>()
                    .into_iter()
                    .collect::<Vec<_>>();

                entries.push(LogEntry {
                    id: global_offset + entries.len(),
                    timestamp: metadata.timestamp.into(),
                    level: metadata.level.into(),
                    file: virtual_path.to_string().into(),
                    real_path: virtual_path.to_string().into(),
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
