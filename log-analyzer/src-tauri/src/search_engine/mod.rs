//! High-Performance Search Engine Module
//!
//! This module provides a Tantivy-based search engine for log analysis with:
//! - Sub-200ms search response times for datasets under 100MB
//! - Streaming index builder for large datasets
//! - Query optimization and suggestion engine
//! - Advanced search features (bitmap filtering, regex, time-partitioned indexes)
//! - Roaring Bitmap compression for search results (10M+ hits < 5MB)
//! - DFA regex engine for high-performance pattern matching
//!
//! ## 优化模块
//!
//! - `optimized_manager`: 生产级优化搜索引擎管理器，包含：
//!   - Channel-based Writer Pool (消除 IndexWriter 锁竞争)
//!   - Arc-swap IndexReader (防止双重重载)
//!   - Thread-local Searcher Cache (避免重复创建)
//!   - Moka Query Cache (查询结果缓存)
//!   - Rayon Parallel Highlighting (并行高亮)
//!   - Memory Budget Enforcement (内存预算限制)
//!
//! 详细文档请参考: `TANTIVY_OPTIMIZATION_GUIDE.md`

pub mod advanced_features;
pub mod async_manager;
pub mod boolean_query_processor;
pub mod cancellable_search;
pub mod concurrent_search;
pub mod dfa_engine;
pub mod highlighting_engine;
pub mod index_optimizer;
pub mod manager;
pub mod optimized_examples;
pub mod optimized_manager;
pub mod query_optimizer;
pub mod roaring_index;
pub mod schema;
pub mod streaming_builder;

#[cfg(test)]
pub mod property_tests;

// 公共 API 导出 - 这些类型供外部模块使用
#[allow(unused_imports)]
pub use advanced_features::{
    AutocompleteEngine, FilterEngine, RegexSearchEngine, TimePartitionedIndex,
};
pub use async_manager::{
    AsyncSearchConfig, AsyncSearchEngineManager, AsyncSearchStats,
};
pub use boolean_query_processor::BooleanQueryProcessor;
pub use cancellable_search::{
    CancellableBatchIterator, CancellableSearch, EnhancedCancellableCollector,
    SearchCancellationController,
};
#[allow(unused_imports)]
pub use concurrent_search::{
    ConcurrentSearchConfig, ConcurrentSearchManager, ConcurrentSearchStats,
};
pub use dfa_engine::{
    DfaError, DfaRegexEngine, DfaSearchResult, SearchProgress, SearchStats as DfaSearchStats, SearchStatus,
};
pub use highlighting_engine::{HighlightingConfig, HighlightingEngine, HighlightingStats};
pub use manager::SearchEngineManager;
pub use optimized_manager::{
    OptimizedSearchEngineManager, SearchConfig as OptimizedSearchConfig, SearchResultEntry,
    SearchResults, SearchResultsWithHighlighting, SearchStats, WriterPool,
};
#[allow(unused_imports)]
pub use query_optimizer::QueryOptimizer;
pub use roaring_index::SearchIndex;
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

    /// **NEW**: 搜索被取消
    #[error("Search was cancelled")]
    Cancelled,

    /// **NEW**: 并发控制错误
    #[error("Concurrency error: {0}")]
    Concurrency(String),

    /// **NEW**: 内部错误
    #[error("Internal error: {0}")]
    Internal(String),
}

pub type SearchResult<T> = Result<T, SearchError>;
