// la-search: Tantivy 全文搜索 + Aho-Corasick 多模式匹配
//
// 注意：以下模块为预留能力，当前主搜索链路（CAS 文件扫描 + RegexEngine）未启用。
// 修改前请确认 AGENTS.md 中的搜索主链路说明。
#[doc(hidden)]
pub mod advanced_features;
#[doc(hidden)]
pub mod boolean_query_processor;
#[doc(hidden)]
pub mod concurrent_search;
pub mod disk_result_store;
#[doc(hidden)]
pub mod highlighting_engine;
#[doc(hidden)]
pub mod index_optimizer;
pub mod manager;
#[doc(hidden)]
pub mod query_optimizer;
pub mod schema;
#[doc(hidden)]
pub mod streaming_builder;
pub mod virtual_search_manager;

#[cfg(test)]
pub mod property_tests;

// 重新导出核心类型
pub use advanced_features::{
    AutocompleteEngine, FilterEngine, RegexSearchEngine, TimePartitionedIndex,
};
pub use boolean_query_processor::BooleanQueryProcessor;
#[allow(unused_imports)]
pub use concurrent_search::{
    ConcurrentSearchConfig, ConcurrentSearchManager, ConcurrentSearchStats,
};
pub use disk_result_store::{DiskResultStore, SearchPageResult};
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
    /// 可重试错误：临时性问题，如超时、IO错误、权限临时不足
    /// 不可重试错误：永久性问题，如查询语法错误、索引损坏
    pub fn is_retryable(&self) -> bool {
        match self {
            SearchError::Timeout(_) => true,
            SearchError::IndexError(msg) => {
                // M2 Fix: Corrected logic — permission denied is retryable (transient),
                // while corrupt/damaged indices are not
                let lower = msg.to_lowercase();
                if lower.contains("permission denied") || lower.contains("temporarily") {
                    true
                } else if lower.contains("corrupt") || lower.contains("damaged") {
                    false
                } else {
                    // Unknown index error — assume not retryable for safety
                    false
                }
            }
            SearchError::IoError(_) => true,
            SearchError::TantivyError(e) => {
                // M2 Fix: Check for inner IO errors via std::error::Error::source()
                // before falling back to string matching for timeout detection
                let mut source: Option<&(dyn std::error::Error + 'static)> =
                    std::error::Error::source(e);
                while let Some(s) = source {
                    if s.is::<std::io::Error>() {
                        return true;
                    }
                    source = std::error::Error::source(s);
                }

                let msg = e.to_string().to_lowercase();
                msg.contains("timeout") || msg.contains("io error") || msg.contains("timed out")
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
