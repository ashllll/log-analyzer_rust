//! High-Performance Search Engine Module (re-export from la-search crate)
//!
//! This module re-exports all types from the la-search crate.
//! The actual implementation lives in crates/la-search/.

// 重新导出 la-search crate 的所有公共类型
pub use la_search::{
    AutocompleteEngine, BooleanQueryProcessor, ConcurrentSearchConfig, ConcurrentSearchManager,
    ConcurrentSearchStats, DiskResultStore, FilterEngine, HighlightingConfig, HighlightingEngine,
    HighlightingStats, LogSchema, QueryOptimizer, RegexSearchEngine, SearchEngineManager,
    SearchError, SearchPageResult, SearchResult, StreamingIndexBuilder, TimePartitionedIndex,
    VirtualSearchManager, VirtualSearchStats,
};

// 重新导出子模块（保持向后兼容）
pub mod advanced_features {
    pub use la_search::advanced_features::{
        AutocompleteEngine, AutocompleteStats, AutocompleteSuggestion, Filter, FilterEngine,
        FilterStats, RegexEngineStats, RegexMatch, RegexSearchEngine, TimeIndexStats,
        TimePartitionedIndex,
    };
}

pub mod boolean_query_processor {
    pub use la_search::boolean_query_processor::{
        BooleanQueryProcessor, CancellableChildCollector, CancellableCollector, QueryPlan,
        TermStats,
    };
}

pub mod concurrent_search {
    pub use la_search::concurrent_search::{
        ConcurrentSearchConfig, ConcurrentSearchManager, ConcurrentSearchStats,
    };
}

pub mod disk_result_store {
    pub use la_search::disk_result_store::{DiskResultStore, SearchPageResult};
}

pub mod highlighting_engine {
    pub use la_search::highlighting_engine::{
        HighlightingConfig, HighlightingEngine, HighlightingStats,
    };
}

pub mod index_optimizer {
    pub use la_search::index_optimizer::*;
}

pub mod manager {
    pub use la_search::manager::{
        SearchConfig, SearchEngineManager, SearchResultEntry, SearchResults,
        SearchResultsWithHighlighting, SearchStats,
    };
}

pub mod query_optimizer {
    pub use la_search::query_optimizer::*;
}

pub mod schema {
    pub use la_search::schema::LogSchema;
}

pub mod streaming_builder {
    pub use la_search::streaming_builder::{
        IndexingProgress, IndexingStats, ProgressCallback, StreamingConfig, StreamingIndexBuilder,
    };
}

pub mod virtual_search_manager {
    pub use la_search::virtual_search_manager::{
        SearchSession, VirtualSearchManager, VirtualSearchStats,
    };
}

#[cfg(test)]
pub mod property_tests;
