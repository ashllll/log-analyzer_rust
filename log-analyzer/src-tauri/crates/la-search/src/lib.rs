// la-search: Tantivy 全文搜索 + Aho-Corasick 多模式匹配

pub mod boolean_query_processor;
pub mod disk_result_store;
pub mod highlighting_engine;
pub mod manager;
pub mod schema;
pub mod virtual_search_manager;

// 重新导出核心类型
pub use boolean_query_processor::BooleanQueryProcessor;
pub use disk_result_store::{DiskResultStore, SearchPageResult};
pub use highlighting_engine::{HighlightingConfig, HighlightingEngine, HighlightingStats};
pub use manager::{parse_log_timestamp_to_unix, SearchEngineManager};
pub use schema::LogSchema;
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
                let lower = msg.to_lowercase();
                lower.contains("permission denied") || lower.contains("temporarily")
            }
            SearchError::IoError(_) => true,
            SearchError::TantivyError(e) => {
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
