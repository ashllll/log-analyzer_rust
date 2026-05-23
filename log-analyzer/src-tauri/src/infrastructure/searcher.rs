//! LogSearcher adapter backed by the existing query planner/executor.

use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

use parking_lot::Mutex;

use la_core::domain::{ExecutionPlan as DomainExecutionPlan, LogSearcher};
use la_core::error::Result;
use la_core::models::{LogEntry, SearchFilters, SearchQuery};

use crate::commands::search::filters::{CompiledSearchFilters, ParsedLineMetadata};
use crate::services::{ExecutionPlan as ServiceExecutionPlan, QueryPlanBuilder};

/// Domain LogSearcher implementation using the production regex/query engine.
pub struct QueryEngineLogSearcher {
    builder: Mutex<QueryPlanBuilder>,
    plans: Mutex<HashMap<u64, ServiceExecutionPlan>>,
}

impl QueryEngineLogSearcher {
    pub fn new(regex_cache_size: usize) -> Self {
        Self {
            builder: Mutex::new(QueryPlanBuilder::new(regex_cache_size.max(1))),
            plans: Mutex::new(HashMap::new()),
        }
    }

    fn plan_id(query: &SearchQuery) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        query.hash(&mut hasher);
        hasher.finish()
    }
}

impl LogSearcher for QueryEngineLogSearcher {
    fn build_plan(&self, query: &SearchQuery) -> Result<DomainExecutionPlan> {
        let id = Self::plan_id(query);
        let service_plan = self.builder.lock().build(query)?;
        let plan = DomainExecutionPlan {
            id,
            engine_count: service_plan.engines.len(),
            steps: service_plan.execution_term_ids().to_vec(),
        };
        self.plans.lock().insert(id, service_plan);
        Ok(plan)
    }

    fn match_content(
        &self,
        content: &str,
        virtual_path: &str,
        plan: &DomainExecutionPlan,
        filters: &SearchFilters,
        global_offset: usize,
    ) -> Vec<LogEntry> {
        let compiled_filters = match CompiledSearchFilters::compile(filters) {
            Ok(filters) => filters,
            Err(_) => return Vec::new(),
        };
        if !compiled_filters.matches_file(virtual_path, None) {
            return Vec::new();
        }

        let service_plan = {
            let plans = self.plans.lock();
            plans.get(&plan.id).cloned()
        };
        let Some(service_plan) = service_plan else {
            return Vec::new();
        };
        let builder = self.builder.lock();

        let mut entries = Vec::new();
        for (index, line) in content.lines().enumerate() {
            let metadata = ParsedLineMetadata::parse(line, compiled_filters.has_time_filter());
            if !compiled_filters.matches_parsed_line_metadata(&metadata) {
                continue;
            }

            if let Some(details) = builder.match_with_details(&service_plan, line) {
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
                    match_details: Some(details),
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
