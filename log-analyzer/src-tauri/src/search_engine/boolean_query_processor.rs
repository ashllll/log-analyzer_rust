//! Boolean Query Processor
#![allow(dead_code)]
//!
//! Optimized multi-keyword intersection algorithms using Tantivy's boolean query capabilities.
//! Provides:
//! - Term frequency analysis for optimal query term ordering
//! - Early termination strategies for large result sets
//! - Query plan optimization based on term selectivity

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tantivy::{
    collector::{Count, TopDocs},
    query::{BooleanQuery, Occur, Query, QueryParser, TermQuery},
    schema::Field,
    Index, IndexReader, Term,
};
use tracing::{debug, info};

use super::{SearchError, SearchResult};

/// Statistics for query term frequency and selectivity
#[derive(Debug, Clone)]
pub struct TermStats {
    pub frequency: u64,
    pub document_count: u64,
    pub selectivity: f64, // Lower is more selective
    pub last_used: std::time::SystemTime,
}

impl Default for TermStats {
    fn default() -> Self {
        Self {
            frequency: 0,
            document_count: 0,
            selectivity: 1.0,
            last_used: std::time::SystemTime::now(),
        }
    }
}

/// Query plan with optimized term ordering
#[derive(Debug, Clone)]
pub struct QueryPlan {
    pub terms: Vec<(String, Occur, f64)>, // term, occurrence type, selectivity
    pub estimated_cost: f64,
    pub should_use_early_termination: bool,
}

/// Boolean query processor with optimization capabilities
pub struct BooleanQueryProcessor {
    _index: Index,
    reader: IndexReader,
    content_field: Field,
    term_stats: Arc<RwLock<HashMap<String, TermStats>>>,
    query_parser: QueryParser,
}

impl BooleanQueryProcessor {
    /// Create a new boolean query processor
    pub fn new(
        index: Index,
        reader: IndexReader,
        content_field: Field,
        query_parser: QueryParser,
    ) -> Self {
        Self {
            _index: index,
            reader,
            content_field,
            term_stats: Arc::new(RwLock::new(HashMap::new())),
            query_parser,
        }
    }

    /// Process a multi-keyword query with optimization
    pub fn process_multi_keyword_query(
        &self,
        keywords: &[String],
        require_all: bool,
        limit: usize,
    ) -> SearchResult<(Vec<tantivy::DocAddress>, usize)> {
        if keywords.is_empty() {
            return Err(SearchError::QueryError("No keywords provided".to_string()));
        }

        debug!(
            keywords = ?keywords,
            require_all = require_all,
            limit = limit,
            "Processing multi-keyword query"
        );

        // Analyze terms and create optimized query plan
        let query_plan = self.create_query_plan(keywords, require_all)?;

        // Build optimized boolean query
        let boolean_query = self.build_optimized_boolean_query(&query_plan)?;

        // Execute query with early termination if beneficial
        self.execute_optimized_query(boolean_query, &query_plan, limit)
    }

    /// Create an optimized query plan based on term statistics
    fn create_query_plan(&self, keywords: &[String], require_all: bool) -> SearchResult<QueryPlan> {
        let mut term_selectivities = Vec::new();

        // Calculate selectivity for each term
        for keyword in keywords {
            let selectivity = self.calculate_term_selectivity(keyword)?;
            let occur = if require_all {
                Occur::Must
            } else {
                Occur::Should
            };
            term_selectivities.push((keyword.clone(), occur, selectivity));
        }

        // Sort by selectivity (most selective first for better performance)
        term_selectivities
            .sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));

        // Calculate estimated cost and determine if early termination is beneficial
        let estimated_cost = self.estimate_query_cost(&term_selectivities);
        let should_use_early_termination = estimated_cost > 1000.0 || keywords.len() > 3;

        debug!(
            terms = ?term_selectivities,
            estimated_cost = estimated_cost,
            early_termination = should_use_early_termination,
            "Created query plan"
        );

        Ok(QueryPlan {
            terms: term_selectivities,
            estimated_cost,
            should_use_early_termination,
        })
    }

    /// Calculate selectivity for a term (lower = more selective)
    fn calculate_term_selectivity(&self, term: &str) -> SearchResult<f64> {
        let searcher = self.reader.searcher();

        // Check cache first
        {
            let stats = self.term_stats.read();
            if let Some(cached_stats) = stats.get(term) {
                // Use cached selectivity if recent (within 5 minutes)
                if cached_stats
                    .last_used
                    .elapsed()
                    .unwrap_or(std::time::Duration::MAX)
                    < std::time::Duration::from_secs(300)
                {
                    return Ok(cached_stats.selectivity);
                }
            }
        }

        // Calculate fresh selectivity
        let term_obj = Term::from_field_text(self.content_field, term);
        let term_query = TermQuery::new(term_obj, tantivy::schema::IndexRecordOption::Basic);

        let count_collector = Count;
        let doc_count = searcher.search(&term_query, &count_collector)?;
        let total_docs = searcher.num_docs();

        let selectivity = if total_docs > 0 {
            doc_count as f64 / total_docs as f64
        } else {
            1.0 // Assume high selectivity for empty index
        };

        // Update cache
        {
            let mut stats = self.term_stats.write();
            stats.insert(
                term.to_string(),
                TermStats {
                    frequency: 1, // Will be updated with actual usage
                    document_count: doc_count as u64,
                    selectivity,
                    last_used: std::time::SystemTime::now(),
                },
            );
        }

        debug!(term = %term, doc_count = doc_count, total_docs = total_docs, selectivity = selectivity, "Calculated term selectivity");

        Ok(selectivity)
    }

    /// Estimate the computational cost of a query
    fn estimate_query_cost(&self, terms: &[(String, Occur, f64)]) -> f64 {
        // Simple cost model: sum of selectivities weighted by occurrence type
        terms
            .iter()
            .map(|(_, occur, selectivity)| {
                match occur {
                    Occur::Must => selectivity * 1.0,   // Must terms are most expensive
                    Occur::Should => selectivity * 0.7, // Should terms are less expensive
                    Occur::MustNot => selectivity * 0.3, // MustNot terms are cheapest
                }
            })
            .sum()
    }

    /// Build an optimized boolean query from the query plan
    fn build_optimized_boolean_query(&self, plan: &QueryPlan) -> SearchResult<BooleanQuery> {
        let mut clauses = Vec::new();

        for (term, occur, _selectivity) in &plan.terms {
            let term_obj = Term::from_field_text(self.content_field, term);
            let term_query = TermQuery::new(term_obj, tantivy::schema::IndexRecordOption::Basic);
            clauses.push((*occur, Box::new(term_query) as Box<dyn Query>));
        }

        Ok(BooleanQuery::new(clauses))
    }

    /// Execute optimized query with potential early termination
    fn execute_optimized_query(
        &self,
        query: BooleanQuery,
        plan: &QueryPlan,
        limit: usize,
    ) -> SearchResult<(Vec<tantivy::DocAddress>, usize)> {
        let searcher = self.reader.searcher();

        // Get total count first
        let count_collector = Count;
        let total_count = searcher.search(&query, &count_collector)?;

        // Adjust limit based on early termination strategy
        let effective_limit = if plan.should_use_early_termination {
            // Use smaller limit for expensive queries to enable early termination
            std::cmp::min(limit, 1000)
        } else {
            limit
        };

        // Execute query with top docs collector
        let top_docs_collector = TopDocs::with_limit(effective_limit);
        let top_docs = searcher.search(&query, &top_docs_collector)?;

        let doc_addresses: Vec<tantivy::DocAddress> = top_docs
            .into_iter()
            .map(|(_score, doc_address)| doc_address)
            .collect();

        info!(
            returned_docs = doc_addresses.len(),
            total_count = total_count,
            early_termination = plan.should_use_early_termination,
            "Query executed successfully"
        );

        Ok((doc_addresses, total_count))
    }

    /// Update term usage statistics for future optimization
    pub fn update_term_usage(&self, term: &str) {
        let mut stats = self.term_stats.write();
        if let Some(term_stats) = stats.get_mut(term) {
            term_stats.frequency += 1;
            term_stats.last_used = std::time::SystemTime::now();
        }
    }

    /// Get statistics for a specific term
    pub fn get_term_stats(&self, term: &str) -> Option<TermStats> {
        self.term_stats.read().get(term).cloned()
    }

    /// Clear term statistics cache
    pub fn clear_stats_cache(&self) {
        self.term_stats.write().clear();
    }

    /// Get all term statistics for debugging
    pub fn get_all_term_stats(&self) -> HashMap<String, TermStats> {
        self.term_stats.read().clone()
    }

    /// Parse and optimize a query string with multiple keywords
    pub fn parse_and_optimize_query(&self, query_str: &str) -> SearchResult<Box<dyn Query>> {
        // 首先检查空查询
        if query_str.trim().is_empty() {
            return Err(SearchError::QueryError("Empty query".to_string()));
        }

        // Try to parse as boolean query first
        match self.query_parser.parse_query(query_str) {
            Ok(query) => Ok(query),
            Err(_) => {
                // Fallback: split into keywords and create optimized boolean query
                let keywords: Vec<String> = query_str
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect();

                if keywords.is_empty() {
                    return Err(SearchError::QueryError("Empty query".to_string()));
                }

                // Create query plan and build optimized query
                let query_plan = self.create_query_plan(&keywords, false)?; // Use OR by default
                let boolean_query = self.build_optimized_boolean_query(&query_plan)?;

                Ok(Box::new(boolean_query))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tantivy::{
        schema::{Schema, STORED, TEXT},
        Index,
    };
    use tempfile::TempDir;

    fn create_test_processor() -> (BooleanQueryProcessor, TempDir) {
        let temp_dir = TempDir::new().unwrap();

        let mut schema_builder = Schema::builder();
        let content_field = schema_builder.add_text_field("content", TEXT | STORED);
        let schema = schema_builder.build();

        let index = Index::create_in_dir(temp_dir.path(), schema).unwrap();
        let reader = index.reader().unwrap();
        let query_parser = tantivy::query::QueryParser::for_index(&index, vec![content_field]);

        let processor = BooleanQueryProcessor::new(index, reader, content_field, query_parser);
        (processor, temp_dir)
    }

    #[test]
    fn test_empty_keywords() {
        let (processor, _temp_dir) = create_test_processor();

        let result = processor.process_multi_keyword_query(&[], true, 10);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SearchError::QueryError(_)));
    }

    #[test]
    fn test_single_keyword() {
        let (processor, _temp_dir) = create_test_processor();

        let keywords = vec!["test".to_string()];
        let result = processor.process_multi_keyword_query(&keywords, true, 10);

        // Should succeed even on empty index
        assert!(result.is_ok());
        let (docs, count) = result.unwrap();
        assert_eq!(docs.len(), 0);
        assert_eq!(count, 0);
    }

    #[test]
    fn test_multiple_keywords() {
        let (processor, _temp_dir) = create_test_processor();

        let keywords = vec![
            "error".to_string(),
            "database".to_string(),
            "connection".to_string(),
        ];
        let result = processor.process_multi_keyword_query(&keywords, false, 10);

        // Should succeed even on empty index
        assert!(result.is_ok());
        let (docs, count) = result.unwrap();
        assert_eq!(docs.len(), 0);
        assert_eq!(count, 0);
    }

    #[test]
    fn test_query_plan_creation() {
        let (processor, _temp_dir) = create_test_processor();

        let keywords = vec!["error".to_string(), "warning".to_string()];
        let plan = processor.create_query_plan(&keywords, true).unwrap();

        assert_eq!(plan.terms.len(), 2);
        assert!(plan.estimated_cost >= 0.0);

        // Terms should be sorted by selectivity
        if plan.terms.len() > 1 {
            assert!(plan.terms[0].2 <= plan.terms[1].2);
        }
    }

    #[test]
    fn test_term_stats_caching() {
        let (processor, _temp_dir) = create_test_processor();

        // Calculate selectivity twice - second should be cached
        let selectivity1 = processor.calculate_term_selectivity("test").unwrap();
        let selectivity2 = processor.calculate_term_selectivity("test").unwrap();

        assert_eq!(selectivity1, selectivity2);

        // Check that stats were cached
        let stats = processor.get_term_stats("test");
        assert!(stats.is_some());
        assert_eq!(stats.unwrap().selectivity, selectivity1);
    }

    #[test]
    fn test_parse_and_optimize_query() {
        let (processor, _temp_dir) = create_test_processor();

        // Test simple query
        let result = processor.parse_and_optimize_query("error database");
        assert!(result.is_ok());

        // Test empty query
        let result = processor.parse_and_optimize_query("");
        assert!(result.is_err());

        // Test single word
        let result = processor.parse_and_optimize_query("error");
        assert!(result.is_ok());
    }
}
