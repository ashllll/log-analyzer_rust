//! High-Performance Search Engine Module
//!
//! This module provides a Tantivy-based search engine for log analysis with:
//! - Sub-200ms search response times for datasets under 100MB
//! - Streaming index builder for large datasets
//! - Query optimization and suggestion engine
//! - Advanced search features (bitmap filtering, regex, time-partitioned indexes)

pub mod advanced_features;
pub mod boolean_query_processor;
pub mod concurrent_search;
pub mod highlighting_engine;
pub mod index_optimizer;
pub mod manager;
pub mod query_optimizer;
pub mod schema;
pub mod streaming_builder;

#[cfg(test)]
pub mod property_tests;

// 公共 API 导出 - 这些类型供外部模块使用
#[allow(unused_imports)]
pub use advanced_features::{
    AutocompleteEngine, FilterEngine, RegexSearchEngine, TimePartitionedIndex,
};
pub use boolean_query_processor::BooleanQueryProcessor;
#[allow(unused_imports)]
pub use concurrent_search::{
    ConcurrentSearchConfig, ConcurrentSearchManager, ConcurrentSearchStats,
};
pub use highlighting_engine::{HighlightingConfig, HighlightingEngine, HighlightingStats};
pub use manager::SearchEngineManager;
#[allow(unused_imports)]
pub use query_optimizer::QueryOptimizer;
pub use schema::LogSchema;
#[allow(unused_imports)]
pub use streaming_builder::StreamingIndexBuilder;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum SearchError {
    #[error("Search timeout: {0}")]
    Timeout(String),

    #[error("Index error: {0}")]
    IndexError(String),

    #[error("Query error: {0}")]
    QueryError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Tantivy error: {0}")]
    TantivyError(#[from] tantivy::TantivyError),

    #[error("Regex error: {0}")]
    RegexError(#[from] regex::Error),
}

pub type SearchResult<T> = Result<T, SearchError>;
