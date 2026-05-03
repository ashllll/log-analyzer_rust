//! High-Performance Search Engine Module (re-export from la-search crate)
//!
//! This module re-exports all types from the la-search crate.
//! The actual implementation lives in crates/la-search/.

// 重新导出 la-search crate 的所有公共类型
pub use la_search::{
    DiskResultStore, LogSchema, SearchEngineManager, SearchError, SearchPageResult, SearchResult,
    VirtualSearchManager, VirtualSearchStats,
};

pub mod disk_result_store {
    pub use la_search::disk_result_store::{DiskResultStore, SearchPageResult};
}

pub mod manager {
    pub use la_search::manager::{
        SearchConfig, SearchEngineManager, SearchResultEntry, SearchResults,
        SearchResultsWithHighlighting, SearchStats,
    };
}

pub mod schema {
    pub use la_search::schema::LogSchema;
}

pub mod virtual_search_manager {
    pub use la_search::virtual_search_manager::{
        SearchSession, VirtualSearchManager, VirtualSearchStats,
    };
}
