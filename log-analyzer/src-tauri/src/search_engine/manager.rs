//! Search Engine Manager
//!
//! Core search engine implementation using Tantivy with:
//! - Sub-200ms search response times
//! - Timeout-based search with cancellation
//! - Index management and configuration
//! - Query parsing and execution

use parking_lot::{Mutex, RwLock};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tantivy::{
    collector::{Count, TopDocs},
    query::{Query, QueryParser, TermQuery},
    schema::Value,
    Index, IndexReader, IndexWriter, ReloadPolicy, TantivyDocument, Term,
};
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

use super::{
    BooleanQueryProcessor, HighlightingConfig, HighlightingEngine, LogSchema, SearchError,
    SearchResult,
};
use crate::models::config::SearchConfig as AppSearchConfig;
use crate::models::LogEntry;
use tantivy::DocAddress;

/// Search result entry with document address for highlighting
#[derive(Debug, Clone)]
pub struct SearchResultEntry {
    pub entry: LogEntry,
    pub doc_address: DocAddress,
}

/// Search results with metadata
#[derive(Debug, Clone)]
pub struct SearchResults {
    pub entries: Vec<LogEntry>,
    /// DocAddress for each entry, aligned with entries vector
    /// Used for highlighting functionality
    pub doc_addresses: Vec<DocAddress>,
    pub total_count: usize,
    pub query_time_ms: u64,
    pub was_timeout: bool,
}

impl SearchResults {
    /// Create empty search results
    pub fn empty() -> Self {
        Self {
            entries: Vec::new(),
            doc_addresses: Vec::new(),
            total_count: 0,
            query_time_ms: 0,
            was_timeout: false,
        }
    }

    /// Get entry with its document address at the given index
    pub fn get_entry_with_address(&self, index: usize) -> Option<(&LogEntry, DocAddress)> {
        self.entries
            .get(index)
            .and_then(|entry| self.doc_addresses.get(index).map(|addr| (entry, *addr)))
    }
}

/// Search results with highlighting metadata
#[derive(Debug, Clone)]
pub struct SearchResultsWithHighlighting {
    pub entries: Vec<LogEntry>,
    pub total_count: usize,
    pub query_time_ms: u64,
    pub highlight_time_ms: u64,
    pub was_timeout: bool,
}

/// Search configuration
#[derive(Debug, Clone)]
pub struct SearchConfig {
    pub default_timeout: Duration,
    pub max_results: usize,
    pub index_path: PathBuf,
    pub writer_heap_size: usize,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            default_timeout: Duration::from_millis(200), // 200ms as per requirements
            max_results: 50_000,
            index_path: PathBuf::from("./search_index"),
            writer_heap_size: 50_000_000, // 50MB
        }
    }
}

/// High-performance search engine manager using Tantivy
pub struct SearchEngineManager {
    pub(crate) index: Index,
    reader: IndexReader,
    writer: Arc<Mutex<IndexWriter>>,
    query_parser: QueryParser,
    schema: LogSchema,
    config: SearchConfig,
    stats: Arc<RwLock<SearchStats>>,
    boolean_processor: BooleanQueryProcessor,
    highlighting_engine: HighlightingEngine,
}

#[derive(Debug, Default)]
pub struct SearchStats {
    pub total_searches: u64,
    total_query_time_ms: u64,
    timeout_count: u64,
    cache_hits: u64,
}

impl SearchEngineManager {
    /// Create a new search engine manager
    pub fn new(config: SearchConfig) -> SearchResult<Self> {
        let schema = LogSchema::build();

        // Create or open index
        // 检查是否存在有效的Tantivy索引（通过检查meta.json文件）
        let meta_path = config.index_path.join("meta.json");
        let index = if meta_path.exists() {
            // 索引已存在，打开它
            Index::open_in_dir(&config.index_path)?
        } else {
            // 创建新索引
            std::fs::create_dir_all(&config.index_path)?;
            Index::create_in_dir(&config.index_path, schema.schema.clone())?
        };

        // Configure tokenizers
        schema.configure_tokenizers(&index)?;

        // Create reader with auto-reload policy
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;

        // Create writer with configured heap size
        let writer = index.writer(config.writer_heap_size)?;

        // Create query parser for content field
        let query_parser = QueryParser::for_index(&index, vec![schema.content]);

        // Create boolean query processor
        let boolean_processor = BooleanQueryProcessor::new(
            index.clone(),
            reader.clone(),
            schema.content,
            query_parser.clone(),
        );

        // Create highlighting engine
        let highlighting_config = HighlightingConfig::default();
        let highlighting_engine = HighlightingEngine::new(
            index.clone(),
            reader.clone(),
            query_parser.clone(),
            schema.content,
            highlighting_config,
        );

        info!(
            index_path = %config.index_path.display(),
            heap_size = config.writer_heap_size,
            "Search engine initialized"
        );

        Ok(Self {
            index,
            reader,
            writer: Arc::new(Mutex::new(writer)),
            query_parser,
            schema,
            config,
            stats: Arc::new(RwLock::new(SearchStats::default())),
            boolean_processor,
            highlighting_engine,
        })
    }

    /// Create a new search engine manager using application configuration
    ///
    /// This method uses the unified config system for settings while keeping
    /// Tantivy-specific defaults for engine internals.
    ///
    /// # Arguments
    ///
    /// * `app_config` - Application search configuration
    /// * `index_path` - Path to store the search index
    /// * `writer_heap_size` - Heap size for Tantivy index writer (bytes)
    pub fn with_app_config(
        app_config: AppSearchConfig,
        index_path: PathBuf,
        writer_heap_size: usize,
    ) -> SearchResult<Self> {
        let engine_config = SearchConfig {
            default_timeout: Duration::from_secs(app_config.timeout_seconds),
            max_results: app_config.max_results,
            index_path,
            writer_heap_size,
        };
        Self::new(engine_config)
    }

    /// Search with timeout support
    pub async fn search_with_timeout(
        &self,
        query: &str,
        limit: Option<usize>,
        timeout_duration: Option<Duration>,
    ) -> SearchResult<SearchResults> {
        let start_time = Instant::now();
        let limit = limit.unwrap_or(self.config.max_results);
        let timeout_duration = timeout_duration.unwrap_or(self.config.default_timeout);

        debug!(query = %query, limit = limit, timeout_ms = timeout_duration.as_millis(), "Starting search");

        // Parse query
        let parsed_query = self.parse_query(query)?;

        // Execute search with timeout
        let search_future = self.execute_search(parsed_query, limit);

        match timeout(timeout_duration, search_future).await {
            Ok(result) => {
                let query_time = start_time.elapsed();
                self.update_stats(query_time, false);

                match result {
                    Ok(mut search_results) => {
                        search_results.query_time_ms = query_time.as_millis() as u64;
                        search_results.was_timeout = false;

                        info!(
                            query = %query,
                            results = search_results.entries.len(),
                            total = search_results.total_count,
                            time_ms = search_results.query_time_ms,
                            "Search completed successfully"
                        );

                        Ok(search_results)
                    }
                    Err(e) => {
                        error!(query = %query, error = %e, "Search execution failed");
                        Err(e)
                    }
                }
            }
            Err(_) => {
                let query_time = start_time.elapsed();
                self.update_stats(query_time, true);

                warn!(
                    query = %query,
                    timeout_ms = timeout_duration.as_millis(),
                    actual_ms = query_time.as_millis(),
                    "Search timed out"
                );

                Err(SearchError::Timeout(format!(
                    "Search timed out after {}ms",
                    timeout_duration.as_millis()
                )))
            }
        }
    }

    /// Parse query string into Tantivy query
    fn parse_query(&self, query_str: &str) -> SearchResult<Box<dyn Query>> {
        // Handle empty query
        if query_str.trim().is_empty() {
            return Err(SearchError::QueryError("Empty query".to_string()));
        }

        // Check if this is a multi-keyword query that would benefit from optimization
        let keywords: Vec<&str> = query_str.split_whitespace().collect();
        if keywords.len() > 1 {
            // Use boolean query processor for multi-keyword queries
            return self.boolean_processor.parse_and_optimize_query(query_str);
        }

        // Parse using query parser (handles phrase queries, boolean operators, etc.)
        match self.query_parser.parse_query(query_str) {
            Ok(query) => Ok(query),
            Err(e) => {
                // Fallback to simple term query if parsing fails
                warn!(query = %query_str, error = %e, "Query parsing failed, using simple term search");

                let term = Term::from_field_text(self.schema.content, query_str);
                Ok(Box::new(TermQuery::new(
                    term,
                    tantivy::schema::IndexRecordOption::Basic,
                )))
            }
        }
    }

    /// Execute search query
    async fn execute_search(
        &self,
        query: Box<dyn Query>,
        limit: usize,
    ) -> SearchResult<SearchResults> {
        let searcher = self.reader.searcher();

        // Get total count
        let count_collector = Count;
        let total_count = searcher.search(&*query, &count_collector)?;

        // Get top documents
        let top_docs_collector = TopDocs::with_limit(limit);
        let top_docs = searcher.search(&*query, &top_docs_collector)?;

        // Convert documents to LogEntry, capturing DocAddress for each
        let mut entries = Vec::with_capacity(top_docs.len());
        let mut doc_addresses = Vec::with_capacity(top_docs.len());

        for (_score, doc_address) in top_docs {
            let retrieved_doc = searcher.doc(doc_address)?;

            if let Some(log_entry) = self.document_to_log_entry(&retrieved_doc) {
                entries.push(log_entry);
                doc_addresses.push(doc_address); // Store DocAddress for highlighting
            }
        }

        Ok(SearchResults {
            entries,
            doc_addresses, // Include DocAddresses in results
            total_count,
            query_time_ms: 0, // Will be set by caller
            was_timeout: false,
        })
    }

    /// Convert Tantivy document to LogEntry
    fn document_to_log_entry(&self, doc: &TantivyDocument) -> Option<LogEntry> {
        let content = doc.get_first(self.schema.content)?.as_str()?.to_string();
        let timestamp_i64 = doc.get_first(self.schema.timestamp)?.as_i64()?;
        let timestamp = timestamp_i64.to_string(); // Convert back to string for LogEntry
        let level = doc.get_first(self.schema.level)?.as_str()?.to_string();
        let file_path = doc.get_first(self.schema.file_path)?.as_str()?.to_string();
        let real_path = doc.get_first(self.schema.real_path)?.as_str()?.to_string();
        let line_number = doc.get_first(self.schema.line_number)?.as_u64()? as usize;

        Some(LogEntry {
            id: 0, // Will be set by caller if needed
            timestamp,
            level,
            file: file_path,
            real_path,
            line: line_number,
            content,
            tags: vec![],
            match_details: None,
            matched_keywords: None,
        })
    }

    /// Add document to index
    pub fn add_document(&self, log_entry: &LogEntry) -> SearchResult<()> {
        let mut doc = TantivyDocument::default();

        doc.add_text(self.schema.content, &log_entry.content);
        // Parse timestamp string to i64
        let timestamp_i64 = log_entry.timestamp.parse::<i64>().unwrap_or(0);
        doc.add_i64(self.schema.timestamp, timestamp_i64);
        doc.add_text(self.schema.level, &log_entry.level);
        doc.add_text(self.schema.file_path, &log_entry.file);
        doc.add_text(self.schema.real_path, &log_entry.real_path);
        doc.add_u64(self.schema.line_number, log_entry.line as u64);

        let writer = self.writer.lock();
        writer.add_document(doc)?;

        Ok(())
    }

    /// Commit pending changes to index
    pub fn commit(&self) -> SearchResult<()> {
        let mut writer = self.writer.lock();
        writer.commit()?;
        Ok(())
    }

    /// Search with multiple keywords using optimized intersection algorithms
    pub async fn search_multi_keyword(
        &self,
        keywords: &[String],
        require_all: bool,
        limit: Option<usize>,
        timeout_duration: Option<Duration>,
    ) -> SearchResult<SearchResults> {
        let start_time = Instant::now();
        let limit = limit.unwrap_or(self.config.max_results);
        let timeout_duration = timeout_duration.unwrap_or(self.config.default_timeout);

        debug!(
            keywords = ?keywords,
            require_all = require_all,
            limit = limit,
            timeout_ms = timeout_duration.as_millis(),
            "Starting multi-keyword search"
        );

        // Use boolean query processor for optimized multi-keyword search
        let search_future = async {
            let (doc_addresses, total_count) =
                self.boolean_processor
                    .process_multi_keyword_query(keywords, require_all, limit)?;

            // Convert document addresses to LogEntry, preserving DocAddress for highlighting
            let searcher = self.reader.searcher();
            let mut entries = Vec::with_capacity(doc_addresses.len());
            let mut addresses = Vec::with_capacity(doc_addresses.len());

            for doc_address in doc_addresses {
                let retrieved_doc = searcher.doc(doc_address)?;
                if let Some(log_entry) = self.document_to_log_entry(&retrieved_doc) {
                    entries.push(log_entry);
                    addresses.push(doc_address); // Store DocAddress for highlighting
                }
            }

            Ok(SearchResults {
                entries,
                doc_addresses: addresses, // Include DocAddresses in results
                total_count,
                query_time_ms: 0, // Will be set by caller
                was_timeout: false,
            })
        };

        // Execute with timeout
        match timeout(timeout_duration, search_future).await {
            Ok(result) => {
                let query_time = start_time.elapsed();
                self.update_stats(query_time, false);

                match result {
                    Ok(mut search_results) => {
                        search_results.query_time_ms = query_time.as_millis() as u64;
                        search_results.was_timeout = false;

                        // Update term usage statistics
                        for keyword in keywords {
                            self.boolean_processor.update_term_usage(keyword);
                        }

                        info!(
                            keywords = ?keywords,
                            results = search_results.entries.len(),
                            total = search_results.total_count,
                            time_ms = search_results.query_time_ms,
                            "Multi-keyword search completed successfully"
                        );

                        Ok(search_results)
                    }
                    Err(e) => {
                        error!(keywords = ?keywords, error = %e, "Multi-keyword search execution failed");
                        Err(e)
                    }
                }
            }
            Err(_) => {
                let query_time = start_time.elapsed();
                self.update_stats(query_time, true);

                warn!(
                    keywords = ?keywords,
                    timeout_ms = timeout_duration.as_millis(),
                    actual_ms = query_time.as_millis(),
                    "Multi-keyword search timed out"
                );

                Err(SearchError::Timeout(format!(
                    "Multi-keyword search timed out after {}ms",
                    timeout_duration.as_millis()
                )))
            }
        }
    }

    /// Search with highlighting support
    pub async fn search_with_highlighting(
        &self,
        query: &str,
        limit: Option<usize>,
        timeout_duration: Option<Duration>,
    ) -> SearchResult<SearchResultsWithHighlighting> {
        let start_time = Instant::now();

        // First, perform the regular search
        let search_results = self
            .search_with_timeout(query, limit, timeout_duration)
            .await?;

        // Then, add highlighting to the results using actual DocAddress
        let mut highlighted_entries = Vec::new();

        for (i, entry) in search_results.entries.iter().enumerate() {
            // Get the actual DocAddress for this entry
            let doc_address = match search_results.doc_addresses.get(i) {
                Some(&addr) => addr,
                None => {
                    warn!(
                        index = i,
                        "Missing DocAddress for entry, using fallback highlighting"
                    );
                    // Fallback: highlight the content directly without Tantivy
                    highlighted_entries.push(entry.clone());
                    continue;
                }
            };

            match self
                .highlighting_engine
                .highlight_document(doc_address, query, &entry.content)
            {
                Ok(highlights) => {
                    let mut highlighted_entry = entry.clone();
                    highlighted_entry.content = highlights.join(" ... ");
                    highlighted_entries.push(highlighted_entry);
                }
                Err(e) => {
                    warn!(error = %e, "Failed to highlight entry, using original content");
                    highlighted_entries.push(entry.clone());
                }
            }
        }

        let total_time = start_time.elapsed();

        info!(
            query = %query,
            results = highlighted_entries.len(),
            highlight_time_ms = total_time.as_millis(),
            "Search with highlighting completed"
        );

        Ok(SearchResultsWithHighlighting {
            entries: highlighted_entries,
            total_count: search_results.total_count,
            query_time_ms: search_results.query_time_ms,
            highlight_time_ms: total_time.as_millis() as u64,
            was_timeout: search_results.was_timeout,
        })
    }

    /// Get search suggestions for autocomplete
    pub fn get_search_suggestions(&self, prefix: &str, limit: usize) -> SearchResult<Vec<String>> {
        // For now, return empty suggestions - will be implemented in autocomplete engine
        debug!(prefix = %prefix, limit = limit, "Search suggestions requested");
        Ok(vec![])
    }

    /// Update search statistics
    fn update_stats(&self, query_time: Duration, was_timeout: bool) {
        let mut stats = self.stats.write();
        stats.total_searches += 1;
        stats.total_query_time_ms += query_time.as_millis() as u64;
        if was_timeout {
            stats.timeout_count += 1;
        }
    }

    /// Get search statistics
    pub fn get_stats(&self) -> SearchStats {
        self.stats.read().clone()
    }

    /// Get highlighting statistics
    pub fn get_highlighting_stats(&self) -> super::HighlightingStats {
        self.highlighting_engine.get_highlighting_stats()
    }

    /// Clear the index
    pub fn clear_index(&self) -> SearchResult<()> {
        let mut writer = self.writer.lock();
        writer.delete_all_documents()?;
        writer.commit()?;
        Ok(())
    }
}

impl Clone for SearchStats {
    fn clone(&self) -> Self {
        Self {
            total_searches: self.total_searches,
            total_query_time_ms: self.total_query_time_ms,
            timeout_count: self.timeout_count,
            cache_hits: self.cache_hits,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// 创建测试用的搜索引擎管理器
    /// 使用 Tantivy 的 RAMDirectory 或正确初始化的磁盘索引
    fn create_test_manager() -> (SearchEngineManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config = SearchConfig {
            index_path: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        // 确保目录存在但为空，让 SearchEngineManager::new 创建新索引
        // 而不是尝试打开不存在的索引
        let manager = SearchEngineManager::new(config).unwrap();
        (manager, temp_dir)
    }

    #[tokio::test]
    async fn test_search_engine_creation() {
        let (_manager, _temp_dir) = create_test_manager();
        // If we get here, creation was successful
    }

    #[tokio::test]
    async fn test_empty_search() {
        let (manager, _temp_dir) = create_test_manager();

        let result = manager.search_with_timeout("", None, None).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SearchError::QueryError(_)));
    }

    #[tokio::test]
    async fn test_search_timeout() {
        let (manager, _temp_dir) = create_test_manager();

        // Search with very short timeout should succeed on empty index
        let result = manager
            .search_with_timeout("test", None, Some(Duration::from_millis(100)))
            .await;

        // Should either succeed quickly or timeout - both are valid outcomes
        match result {
            Ok(results) => {
                // Search completed quickly on empty index
                assert_eq!(results.entries.len(), 0);
            }
            Err(SearchError::Timeout(_)) => {
                // Search timed out as expected
            }
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }
}
