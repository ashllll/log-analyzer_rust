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
pub mod virtual_search_manager;

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
pub use virtual_search_manager::{VirtualSearchManager, VirtualSearchStats};

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

impl SearchError {
    /// 判断错误是否可重试
    /// 可重试错误：临时性问题，如超时、IO错误
    /// 不可重试错误：永久性问题，如查询语法错误
    pub fn is_retryable(&self) -> bool {
        match self {
            SearchError::Timeout(_) => true,
            SearchError::IndexError(msg) => {
                // 索引损坏等不可重试，权限问题等可重试
                !msg.contains("permission denied")
                    && !msg.contains("corrupt")
                    && !msg.contains("damaged")
            }
            SearchError::IoError(_) => true,
            SearchError::TantivyError(e) => {
                // Tantivy内部错误，根据错误类型判断
                // 使用更精确的字符串匹配，减少误判（"IO" 过于宽泛，可能匹配无关内容）
                e.to_string().contains("timeout")
                    || e.to_string().contains("IO error")
                    || e.to_string().contains("Os {")
            }
            SearchError::QueryError(_) => false,
            SearchError::RegexError(_) => false,
        }
    }

    /// 判断是否为致命错误（不应继续执行）
    pub fn is_fatal(&self) -> bool {
        matches!(
            self,
            SearchError::QueryError(_) | SearchError::RegexError(_)
        )
    }
}

pub type SearchResult<T> = Result<T, SearchError>;
