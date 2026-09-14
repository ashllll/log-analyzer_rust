//! Search Engine Manager
//!
//! Core search engine implementation using Tantivy with:
//! - Sub-200ms search response times
//! - Timeout-based search with cancellation
//! - Index management and configuration
//! - Query parsing and execution

use la_core::utils::TimestampParser;
use parking_lot::{Mutex, MutexGuard, RwLock};
use std::collections::HashSet;
use std::ops::Bound;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tantivy::{
    collector::{Count, TopDocs},
    query::{BooleanQuery, Occur, Query, QueryParser, RangeQuery, RegexQuery, TermQuery},
    schema::Value,
    DocAddress, Index, IndexReader, IndexWriter, ReloadPolicy, TantivyDocument, Term,
};
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

use crate::{
    boolean_query_processor::BooleanQueryProcessor,
    highlighting_engine::{HighlightingConfig, HighlightingEngine},
    schema::{LogSchema, INDEX_SCHEMA_VERSION},
    SearchError, SearchResult,
};
use la_core::models::config::SearchConfig as AppSearchConfig;
use la_core::models::LogEntry;

/// 索引目录中的 schema 版本标记文件
const SCHEMA_VERSION_FILE: &str = "schema_version";

/// 检查索引 schema 是否过期（无版本文件视为旧版索引，需要重建）。
fn index_schema_is_stale(index_path: &std::path::Path) -> bool {
    match std::fs::read_to_string(index_path.join(SCHEMA_VERSION_FILE)) {
        Ok(version) => version.trim().parse::<u32>().unwrap_or(0) != INDEX_SCHEMA_VERSION,
        Err(_) => true,
    }
}

/// 写入当前 schema 版本标记。
fn write_schema_version(index_path: &std::path::Path) -> SearchResult<()> {
    std::fs::write(
        index_path.join(SCHEMA_VERSION_FILE),
        INDEX_SCHEMA_VERSION.to_string(),
    )
    .map_err(|e| SearchError::IndexError(format!("Failed to write schema version: {e}")))?;
    Ok(())
}

fn glob_to_regex(glob: &str) -> String {
    let mut regex = String::with_capacity(glob.len() * 2);
    regex.push('^');
    for ch in glob.chars() {
        match ch {
            '*' => regex.push_str(".*"),
            '?' => regex.push('.'),
            '.' | '+' | '(' | ')' | '[' | ']' | '{' | '}' | '\\' | '|' | '^' | '$' => {
                regex.push('\\');
                regex.push(ch);
            }
            _ => regex.push(ch),
        }
    }
    regex.push('$');
    regex
}

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
#[allow(dead_code)]
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

/// Exclusive, rollback-safe transaction over documents in a search index.
///
/// The previously committed index remains visible to readers until `commit`.
/// Dropping an unfinished transaction rolls the writer back to its last commit.
pub struct SearchIndexRebuild<'a> {
    manager: &'a SearchEngineManager,
    writer: Option<MutexGuard<'a, IndexWriter>>,
    committed: bool,
}

impl SearchIndexRebuild<'_> {
    /// Remove a file within the same rollback-safe transaction as its replacement.
    pub fn remove_file(&mut self, virtual_path: &str) {
        let term = Term::from_field_text(self.manager.schema.file_path, virtual_path);
        self.writer
            .as_ref()
            .expect("active index transaction")
            .delete_term(term);
    }

    /// Add a batch without publishing partial results.
    pub fn add_documents(&mut self, entries: &[LogEntry]) -> SearchResult<()> {
        let writer = self
            .writer
            .as_ref()
            .expect("search index rebuild writer must exist before commit");

        for entry in entries {
            writer.add_document(self.manager.document_for_entry(entry))?;
        }

        Ok(())
    }

    /// Atomically publish the replacement index and refresh readers.
    pub fn commit(mut self) -> SearchResult<()> {
        self.writer
            .as_mut()
            .expect("search index rebuild writer must exist before commit")
            .commit()?;
        self.committed = true;
        drop(self.writer.take());

        if let Err(e) = self.manager.reader.reload() {
            warn!(
                error = %e,
                "Reader reload failed after index rebuild; readers may see stale data temporarily"
            );
        }

        Ok(())
    }
}

impl Drop for SearchIndexRebuild<'_> {
    fn drop(&mut self) {
        if self.committed {
            return;
        }

        if let Some(writer) = self.writer.as_mut() {
            if let Err(e) = writer.rollback() {
                error!(
                    error = %e,
                    "Failed to roll back unfinished search index rebuild"
                );
            }
        }
    }
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
            if index_schema_is_stale(&config.index_path) {
                // P2 修复：schema 版本不匹配（升级后字段/类型变化）时，
                // 旧的 Tantivy 索引无法安全打开。CAS 是内容源，索引可
                // 完整重建——删除旧索引目录并重建，而不是让工作区
                // 加载失败。
                warn!(
                    index_path = %config.index_path.display(),
                    "Search index schema is stale, rebuilding index directory"
                );
                std::fs::remove_dir_all(&config.index_path).map_err(|e| {
                    SearchError::IndexError(format!("Failed to remove stale index: {e}"))
                })?;
                std::fs::create_dir_all(&config.index_path).map_err(|e| {
                    SearchError::IndexError(format!("Failed to create index dir: {e}"))
                })?;
                write_schema_version(&config.index_path)?;
                Index::create_in_dir(&config.index_path, schema.schema.clone())?
            } else {
                // 索引已存在且版本匹配，打开它
                Index::open_in_dir(&config.index_path)?
            }
        } else {
            // 创建新索引
            std::fs::create_dir_all(&config.index_path)?;
            write_schema_version(&config.index_path)?;
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
        token: Option<CancellationToken>,
    ) -> SearchResult<SearchResults> {
        let start_time = Instant::now();
        let limit = limit.unwrap_or(self.config.max_results);
        let timeout_duration = timeout_duration.unwrap_or(self.config.default_timeout);
        let token = token.unwrap_or_else(|| {
            debug!("未传入 CancellationToken，创建本地 token（无外部取消能力，依赖超时机制兜底）");
            CancellationToken::new()
        });

        debug!(query = %query, limit = limit, timeout_ms = timeout_duration.as_millis(), "Starting search");

        // Parse query
        let parsed_query = self.parse_query(query)?;

        // Execute search with timeout and cancellation
        let search_future = self.execute_search(parsed_query, limit, Some(token.clone()));

        match timeout(timeout_duration, search_future).await {
            Ok(result) => {
                let query_time = start_time.elapsed();
                self.update_stats(query_time, false);
                // ...

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

                // Cancel the underlying search
                token.cancel();

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

    /// 使用预构建的 Tantivy Query 执行搜索（支持完整过滤条件）
    pub async fn search_with_query(
        &self,
        query: Box<dyn Query>,
        limit: Option<usize>,
        timeout_duration: Option<Duration>,
        token: Option<CancellationToken>,
    ) -> SearchResult<SearchResults> {
        let start_time = Instant::now();
        let limit = limit.unwrap_or(self.config.max_results);
        let timeout_duration = timeout_duration.unwrap_or(self.config.default_timeout);
        let token = token.unwrap_or_default();

        debug!(
            limit = limit,
            timeout_ms = timeout_duration.as_millis(),
            "Starting filtered Tantivy search"
        );

        let search_future = self.execute_search(query, limit, Some(token.clone()));

        match timeout(timeout_duration, search_future).await {
            Ok(result) => {
                let query_time = start_time.elapsed();
                self.update_stats(query_time, false);
                match result {
                    Ok(mut search_results) => {
                        search_results.query_time_ms = query_time.as_millis() as u64;
                        search_results.was_timeout = false;
                        info!(
                            results = search_results.entries.len(),
                            total = search_results.total_count,
                            time_ms = search_results.query_time_ms,
                            "Filtered Tantivy search completed"
                        );
                        Ok(search_results)
                    }
                    Err(e) => Err(e),
                }
            }
            Err(_) => {
                self.update_stats(start_time.elapsed(), true);
                token.cancel();
                warn!("Filtered Tantivy search timed out");
                Err(SearchError::Timeout(format!(
                    "Search timed out after {}ms",
                    timeout_duration.as_millis()
                )))
            }
        }
    }

    /// 构建包含内容查询和过滤条件的完整 Tantivy BooleanQuery
    ///
    /// # Arguments
    /// - `content_query_str` — 用户输入的原始查询字符串（如 "error | timeout"）
    /// - `time_range` — 可选的时间范围过滤 (start_timestamp, end_timestamp)，Unix 秒
    /// - `levels` — 可选的日志级别集合，如 {"ERROR", "WARN"}
    /// - `file_pattern` — 可选的文件路径 GLOB 模式，如 "logs/app*.log"
    pub fn build_tantivy_query(
        &self,
        content_query_str: &str,
        time_range: Option<(i64, i64)>,
        levels: Option<&HashSet<String>>,
        file_pattern: Option<&str>,
    ) -> SearchResult<Box<dyn Query>> {
        use tantivy::schema::IndexRecordOption;

        // 1. 解析内容查询
        let content_query = self.parse_query(content_query_str)?;

        let mut clauses: Vec<(Occur, Box<dyn Query>)> = vec![(Occur::Must, content_query)];

        // 2. 时间范围过滤 → RangeQuery on schema.timestamp (i64 fast field)
        if let Some((start, end)) = time_range {
            let start_term = Term::from_field_i64(self.schema.timestamp, start);
            let end_term = Term::from_field_i64(self.schema.timestamp, end);
            let range_query =
                RangeQuery::new(Bound::Included(start_term), Bound::Included(end_term));
            clauses.push((Occur::Must, Box::new(range_query)));
        }

        // 3. 日志级别过滤 → 多个 TermQuery 用 BooleanQuery::or 组合
        if let Some(levels) = levels {
            if !levels.is_empty() {
                let level_clauses: Vec<(Occur, Box<dyn Query>)> = levels
                    .iter()
                    .map(|lvl| {
                        let term = Term::from_field_text(self.schema.level, lvl);
                        (
                            Occur::Should,
                            Box::new(TermQuery::new(term, IndexRecordOption::Basic))
                                as Box<dyn Query>,
                        )
                    })
                    .collect();
                if !level_clauses.is_empty() {
                    clauses.push((Occur::Must, Box::new(BooleanQuery::new(level_clauses))));
                }
            }
        }

        // 4. 文件路径过滤 → RegexQuery on schema.file_path
        if let Some(pattern) = file_pattern {
            if !pattern.is_empty() {
                // 将 GLOB 模式转换为正则：* → .*，? → .
                let regex_pattern = glob_to_regex(pattern);
                let regex_query = RegexQuery::from_pattern(&regex_pattern, self.schema.file_path)
                    .map_err(|e| {
                    SearchError::QueryError(format!("Invalid file pattern: {e}"))
                })?;
                clauses.push((Occur::Must, Box::new(regex_query)));
            }
        }

        Ok(Box::new(BooleanQuery::new(clauses)))
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

    /// Execute search query.
    ///
    /// 所有阻塞的 Tantivy 调用均在 `tokio::task::spawn_blocking` 线程池中执行，
    /// 使得外层的 `tokio::time::timeout` 能够真正中断搜索（原来 async fn 内无
    /// `.await` 点，tokio 超时无法在 poll 间隙取消）。
    async fn execute_search(
        &self,
        query: Box<dyn Query>,
        limit: usize,
        token: Option<CancellationToken>,
    ) -> SearchResult<SearchResults> {
        let token = token.unwrap_or_else(|| {
            debug!("execute_search 未收到 CancellationToken，创建本地 token（无外部取消能力）");
            CancellationToken::new()
        });
        let reader = self.reader.clone();
        let schema = self.schema.clone();
        let token_clone = token.clone();

        let handle = tokio::task::spawn_blocking(move || -> SearchResult<SearchResults> {
            let searcher = reader.searcher();

            // Get total count with cancellation support
            let count_collector = crate::boolean_query_processor::CancellableCollector::new(
                Count,
                token_clone.clone(),
            );
            let total_count = match searcher.search(&*query, &count_collector) {
                Ok(count) => count,
                Err(e) => {
                    if token_clone.is_cancelled() {
                        return Err(SearchError::QueryError("Search cancelled".to_string()));
                    }
                    return Err(SearchError::IndexError(e.to_string()));
                }
            };

            if token_clone.is_cancelled() {
                return Err(SearchError::QueryError("Search cancelled".to_string()));
            }

            // Get top documents with cancellation support
            let top_docs_collector = TopDocs::with_limit(limit);
            let cancellable_top_docs = crate::boolean_query_processor::CancellableCollector::new(
                top_docs_collector,
                token_clone.clone(),
            );

            let top_docs = match searcher.search(&*query, &cancellable_top_docs) {
                Ok(docs) => docs,
                Err(e) => {
                    if token_clone.is_cancelled() {
                        return Err(SearchError::QueryError("Search cancelled".to_string()));
                    }
                    return Err(SearchError::IndexError(e.to_string()));
                }
            };

            // Convert documents to LogEntry, capturing DocAddress for each
            let mut entries = Vec::with_capacity(top_docs.len());
            let mut doc_addresses = Vec::with_capacity(top_docs.len());

            for (_score, doc_address) in top_docs {
                let retrieved_doc = match searcher.doc(doc_address) {
                    Ok(doc) => doc,
                    Err(e) => return Err(SearchError::IndexError(e.to_string())),
                };

                if let Some(log_entry) = document_to_log_entry_inner(&schema, &retrieved_doc) {
                    entries.push(log_entry);
                    doc_addresses.push(doc_address);
                }
            }

            Ok(SearchResults {
                entries,
                doc_addresses,
                total_count,
                query_time_ms: 0, // Will be set by caller
                was_timeout: false,
            })
        });

        handle
            .await
            .map_err(|e| SearchError::QueryError(format!("Search task panicked: {e}")))?
    }

    /// Convert Tantivy document to LogEntry（实例方法包装，供非 spawn_blocking 路径使用）
    #[allow(dead_code)]
    fn document_to_log_entry(&self, doc: &TantivyDocument) -> Option<LogEntry> {
        document_to_log_entry_inner(&self.schema, doc)
    }
}

/// 独立函数版的 document_to_log_entry，可在 spawn_blocking 闭包中使用（无 &self）。
fn document_to_log_entry_inner(schema: &LogSchema, doc: &TantivyDocument) -> Option<LogEntry> {
    document_to_log_entry_impl(schema, doc)
}

fn document_to_log_entry_impl(schema: &LogSchema, doc: &TantivyDocument) -> Option<LogEntry> {
    // 辅助宏：从 Tantivy 文档提取字段，消除重复的双层 match 嵌套
    // 对于 as_str() 字段，直接返回 Arc<str>
    macro_rules! extract_str {
        ($doc:expr, $schema:expr, $field:ident) => {
            match $doc.get_first($schema.$field) {
                Some(v) => match v.as_str() {
                    Some(s) => Arc::from(s),
                    None => {
                        tracing::warn!(
                            field = stringify!($field),
                            "Document has non-string field, skipping"
                        );
                        return None;
                    }
                },
                None => {
                    tracing::warn!(
                        field = stringify!($field),
                        "Document missing required field, skipping"
                    );
                    return None;
                }
            }
        };
    }

    let content = extract_str!(doc, schema, content);
    let level = extract_str!(doc, schema, level);
    let file_path = extract_str!(doc, schema, file_path);
    let real_path = extract_str!(doc, schema, real_path);

    // timestamp: i64 → Arc<str> (特殊转换)
    let timestamp: Arc<str> = match doc.get_first(schema.timestamp) {
        Some(v) => match v.as_i64() {
            Some(n) => Arc::from(n.to_string()),
            None => {
                tracing::warn!("Document has non-i64 timestamp field, skipping");
                return None;
            }
        },
        None => {
            tracing::warn!("Document missing required timestamp field, skipping");
            return None;
        }
    };

    // line_number: u64 → usize (特殊转换)
    let line_number = match doc.get_first(schema.line_number) {
        Some(v) => match v.as_u64() {
            Some(n) => n as usize,
            None => {
                tracing::warn!("Document has non-u64 line_number field, skipping");
                return None;
            }
        },
        None => {
            tracing::warn!("Document missing required line_number field, skipping");
            return None;
        }
    };

    Some(LogEntry {
        id: 0,
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

impl SearchEngineManager {
    fn document_for_entry(&self, entry: &LogEntry) -> TantivyDocument {
        let mut doc = TantivyDocument::default();

        doc.add_text(self.schema.content, &entry.content);
        let timestamp_i64 =
            TimestampParser::parse_unix_timestamp(&entry.timestamp).unwrap_or_else(|| {
                tracing::warn!(
                    timestamp = %entry.timestamp,
                    file = %entry.file,
                    "时间戳解析失败，使用 0 (1970-01-01) 作为占位值；不影响其他日志时序"
                );
                0i64
            });
        doc.add_i64(self.schema.timestamp, timestamp_i64);
        doc.add_text(self.schema.level, &entry.level);
        doc.add_text(self.schema.file_path, &entry.file);
        doc.add_text(self.schema.real_path, &entry.real_path);
        doc.add_u64(self.schema.line_number, entry.line as u64);

        doc
    }

    /// Add document to index
    pub fn add_document(&self, log_entry: &LogEntry) -> SearchResult<()> {
        let writer = self.writer.lock();
        writer.add_document(self.document_for_entry(log_entry))?;

        Ok(())
    }

    /// L1 Fix: Add multiple documents to index in a single writer lock cycle.
    ///
    /// This is significantly more efficient than calling `add_document` in a loop
    /// because it acquires the index writer lock once for the entire batch.
    /// Tantivy's IndexWriter is single-writer; reducing lock acquisitions
    /// improves throughput for bulk operations like file watcher indexing.
    pub fn add_documents(&self, entries: &[LogEntry]) -> SearchResult<()> {
        if entries.is_empty() {
            return Ok(());
        }

        let writer = self.writer.lock();
        for entry in entries {
            writer.add_document(self.document_for_entry(entry))?;
        }
        // Writer lock is released here when `writer` goes out of scope

        Ok(())
    }

    /// Begin an exclusive replacement of all indexed documents.
    ///
    /// Readers continue to see the last committed index until the returned
    /// rebuild is committed. Dropping it before commit restores the writer to
    /// the last committed state.
    pub fn begin_rebuild(&self) -> SearchResult<SearchIndexRebuild<'_>> {
        let writer = self.writer.lock();
        writer.delete_all_documents()?;

        Ok(SearchIndexRebuild {
            manager: self,
            writer: Some(writer),
            committed: false,
        })
    }

    /// Begin an exclusive append transaction.
    ///
    /// Readers continue to see the last committed index until the returned
    /// transaction is committed. Dropping it before commit rolls back every
    /// document appended through the transaction.
    pub fn begin_update(&self) -> SearchResult<SearchIndexRebuild<'_>> {
        Ok(SearchIndexRebuild {
            manager: self,
            writer: Some(self.writer.lock()),
            committed: false,
        })
    }

    /// Commit pending changes to index
    pub fn commit(&self) -> SearchResult<()> {
        // 先在独立作用域内提交并释放 writer 锁，
        // 再 reload reader，减少锁持有时间，提升并发写入吞吐
        {
            let mut writer = self.writer.lock();
            writer.commit()?;
        }
        // Reload reader to make committed changes visible
        // 如果reload失败，记录警告但仍然返回成功，因为数据已持久化
        if let Err(e) = self.reader.reload() {
            warn!(
                error = %e,
                "Reader reload failed after commit; readers may see stale data temporarily"
            );
        }
        Ok(())
    }

    /// Search with multiple keywords using optimized intersection algorithms
    pub async fn search_multi_keyword(
        &self,
        keywords: &[String],
        require_all: bool,
        limit: Option<usize>,
        timeout_duration: Option<Duration>,
        token: Option<CancellationToken>,
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

        let keywords_owned = keywords.to_vec();
        let boolean_processor = self.boolean_processor.clone();
        let reader = self.reader.clone();
        let schema = self.schema.clone();
        // 保留 token 副本：闭包与超时分支各持一份，超时后仍能取消
        // 后台阻塞任务（P2 修复：此前 move 进闭包后超时分支无法 cancel，
        // spawn_blocking 会继续跑完整索引）。
        let token_inner = token.clone();

        let search_result = timeout(
            timeout_duration,
            tokio::task::spawn_blocking(move || {
                let (doc_addresses, total_count) = boolean_processor.process_multi_keyword_query(
                    &keywords_owned,
                    require_all,
                    limit,
                    token_inner,
                )?;

                let searcher = reader.searcher();
                let mut entries = Vec::with_capacity(doc_addresses.len());
                let mut addresses = Vec::with_capacity(doc_addresses.len());

                for doc_address in doc_addresses {
                    let retrieved_doc = searcher.doc(doc_address)?;
                    if let Some(log_entry) = document_to_log_entry_inner(&schema, &retrieved_doc) {
                        entries.push(log_entry);
                        addresses.push(doc_address);
                    }
                }

                Ok(SearchResults {
                    entries,
                    doc_addresses: addresses,
                    total_count,
                    query_time_ms: 0,
                    was_timeout: false,
                })
            }),
        )
        .await;

        let query_time = start_time.elapsed();

        let search_results = match search_result {
            Ok(Ok(Ok(results))) => results,
            Ok(Ok(Err(e))) => {
                self.update_stats(query_time, false);
                error!(keywords = ?keywords, error = %e, "Multi-keyword search execution failed");
                return Err(e);
            }
            Ok(Err(join_err)) => {
                self.update_stats(query_time, true);
                error!(keywords = ?keywords, error = %join_err, "spawn_blocking task failed");
                return Err(SearchError::IndexError(format!(
                    "Search task failed: {join_err}"
                )));
            }
            Err(_) => {
                self.update_stats(query_time, true);

                // P2 修复：超时后取消后台任务，让 collector 尽快退出，
                // 而不是继续扫描完整索引。
                if let Some(t) = &token {
                    t.cancel();
                }

                warn!(
                    keywords = ?keywords,
                    timeout_ms = timeout_duration.as_millis(),
                    actual_ms = query_time.as_millis(),
                    "Multi-keyword search timed out"
                );

                return Err(SearchError::Timeout(format!(
                    "Multi-keyword search timed out after {}ms",
                    timeout_duration.as_millis()
                )));
            }
        };

        let mut search_results = search_results;

        self.update_stats(query_time, false);
        search_results.query_time_ms = query_time.as_millis() as u64;
        search_results.was_timeout = false;

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

    /// Search with highlighting support
    pub async fn search_with_highlighting(
        &self,
        query: &str,
        limit: Option<usize>,
        timeout_duration: Option<Duration>,
        token: Option<CancellationToken>,
    ) -> SearchResult<SearchResultsWithHighlighting> {
        let start_time = Instant::now();

        // First, perform the regular search
        let search_results = self
            .search_with_timeout(query, limit, timeout_duration, token)
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
                    highlighted_entry.content = highlights.join(" ... ").into();
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
        stats.total_query_time_ms = stats
            .total_query_time_ms
            .saturating_add(query_time.as_millis() as u64);
        if was_timeout {
            stats.timeout_count += 1;
        }
    }

    /// Get search statistics
    pub fn get_stats(&self) -> SearchStats {
        self.stats.read().clone()
    }

    /// Get highlighting statistics
    pub fn get_highlighting_stats(&self) -> crate::highlighting_engine::HighlightingStats {
        self.highlighting_engine.get_highlighting_stats()
    }

    /// Clear the index
    /// 注意：这是一个不可逆操作，delete_all是原子性的，但建议在调用前确认
    pub fn clear_index(&self) -> SearchResult<()> {
        let mut writer = self.writer.lock();
        info!("Clearing index - deleting all documents");
        writer.delete_all_documents()?;
        writer.commit()?;
        info!("Index cleared successfully");
        Ok(())
    }

    /// Get the time range of all logs in the index
    ///
    /// Returns the minimum and maximum timestamps from the index.
    /// Uses fast fields for efficient aggregation without loading all documents.
    ///
    /// # Returns
    ///
    /// - `Ok((min_timestamp, max_timestamp, total_count))` - Time range and total count
    /// - `Err(SearchError)` - If search fails
    ///
    /// # Example
    ///
    /// ```ignore
    /// let (min_ts, max_ts, count) = manager.get_time_range()?;
    /// info!("Time range: {} to {}, total: {} logs", min_ts, max_ts, count);
    /// ```
    pub fn get_time_range(&self) -> SearchResult<(i64, i64, usize)> {
        let searcher = self.reader.searcher();
        let total_count = searcher.num_docs() as usize;

        if total_count == 0 {
            return Ok((0, 0, 0));
        }

        // 利用 Fast Field 列统计：O(n_segments) 而非 O(n_docs)
        // Column<i64>.min_value() / max_value() 直接读取段级压缩统计，无需遍历文档
        // 注：已包含软删除但未合并文档的时间戳，偏宽属可接受估算误差
        let mut min_timestamp = i64::MAX;
        let mut max_timestamp = i64::MIN;

        for segment_reader in searcher.segment_readers() {
            let fast_fields = segment_reader.fast_fields();
            match fast_fields.i64("timestamp") {
                Ok(col) => {
                    min_timestamp = min_timestamp.min(col.min_value());
                    max_timestamp = max_timestamp.max(col.max_value());
                }
                Err(e) => {
                    // 段为空或字段未索引，跳过
                    tracing::warn!(
                        error = %e,
                        "get_time_range: 无法读取 timestamp fast field，跳过此段"
                    );
                }
            }
        }

        if min_timestamp == i64::MAX {
            min_timestamp = 0;
        }
        if max_timestamp == i64::MIN {
            max_timestamp = 0;
        }

        Ok((min_timestamp, max_timestamp, total_count))
    }

    /// Delete all documents for a specific file
    ///
    /// This is used when a file is deleted or removed from the workspace.
    /// It uses TermQuery to match and delete all documents with the given file_path.
    ///
    /// # Arguments
    ///
    /// * `file_path` - The file path to delete documents for
    ///
    /// # Returns
    ///
    /// - `Ok(deleted_count)` - Number of documents deleted
    /// - `Err(SearchError)` - If deletion or commit fails
    ///
    /// # Example
    ///
    /// ```ignore
    /// let count = manager.delete_file_documents("/path/to/file.log")?;
    /// info!("Deleted {} documents for file", count);
    /// ```
    pub fn delete_file_documents(&self, file_path: &str) -> SearchResult<usize> {
        use tantivy::query::TermQuery;
        use tantivy::Term;

        let term = Term::from_field_text(self.schema.file_path, file_path);

        // First, get the count of documents to be deleted
        let searcher = self.reader.searcher();
        let query = TermQuery::new(term.clone(), tantivy::schema::IndexRecordOption::Basic);
        let count = searcher.search(&query, &Count)?;

        // Delete and commit in isolated scope to minimize lock holding time
        {
            let mut writer = self.writer.lock();
            let _opstamp = writer.delete_term(term);
            writer.commit()?;
        }

        // Reload reader in separate scope
        if let Err(e) = self.reader.reload() {
            warn!(
                error = %e,
                "Reader reload failed after delete; readers may see stale data temporarily"
            );
        }

        info!(
            file_path = %file_path,
            deleted_count = count,
            "Deleted documents for file"
        );

        Ok(count)
    }

    /// 优雅关闭搜索引擎，确保 IndexWriter 在进程退出前完成 commit。
    ///
    /// `IndexWriter::commit()` 是同步阻塞调用，不能在 Drop 中直接执行，
    /// 因此暴露此显式 close 方法，由 lib.rs shutdown hook 调用。
    pub async fn close(&self) {
        let writer = self.writer.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut w = writer.lock();
            match w.commit() {
                Ok(_) => {
                    tracing::info!("SearchEngineManager: IndexWriter commit on close succeeded");
                }
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        "SearchEngineManager: IndexWriter commit on close failed"
                    );
                }
            }
        })
        .await;

        if let Err(e) = result {
            tracing::error!(error = %e, "SearchEngineManager: close task panicked");
        }
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

    fn test_entry(timestamp: &str, content: &str) -> LogEntry {
        LogEntry {
            id: 1,
            timestamp: timestamp.into(),
            level: "INFO".into(),
            file: "logs/app.log".into(),
            real_path: "cas://test".into(),
            line: 1,
            content: content.into(),
            tags: vec![],
            match_details: None,
            matched_keywords: None,
        }
    }

    #[tokio::test]
    async fn test_search_engine_creation() {
        let (_manager, _temp_dir) = create_test_manager();
        // If we get here, creation was successful
    }

    #[tokio::test]
    async fn test_empty_search() {
        let (manager, _temp_dir) = create_test_manager();

        let result = manager.search_with_timeout("", None, None, None).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SearchError::QueryError(_)));
    }

    #[tokio::test]
    async fn test_search_timeout() {
        let (manager, _temp_dir) = create_test_manager();

        // Search with very short timeout should succeed on empty index
        let result = manager
            .search_with_timeout("test", None, Some(Duration::from_millis(100)), None)
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
            Err(e) => {
                // Unexpected error type - this should not happen in normal circumstances
                panic!("Unexpected error during empty index search: {e}");
            }
        }
    }

    #[test]
    fn test_parse_log_timestamp_to_unix_accepts_common_formats() {
        let iso = TimestampParser::parse_unix_timestamp("2024-01-01T00:00:00").unwrap();
        let common = TimestampParser::parse_unix_timestamp("2024-01-01 00:00:00").unwrap();
        let millis = TimestampParser::parse_unix_timestamp("1704067200000").unwrap();
        let rfc3339 = TimestampParser::parse_unix_timestamp("2024-01-01T00:00:00Z").unwrap();

        assert_eq!(iso, 1_704_067_200);
        assert_eq!(common, 1_704_067_200);
        assert_eq!(millis, 1_704_067_200);
        assert_eq!(rfc3339, 1_704_067_200);
    }

    #[tokio::test]
    async fn test_get_time_range_uses_parsed_human_readable_timestamps() {
        let (manager, _temp_dir) = create_test_manager();

        let entry1 = la_core::models::LogEntry {
            id: 1,
            timestamp: "2024-01-01 00:00:00".into(),
            level: "INFO".into(),
            file: "logs/app.log".into(),
            real_path: "cas://a".into(),
            line: 1,
            content: "First log entry".into(),
            tags: vec![],
            match_details: None,
            matched_keywords: None,
        };
        let entry2 = la_core::models::LogEntry {
            id: 2,
            timestamp: "2024-01-01 00:05:00".into(),
            level: "ERROR".into(),
            file: "logs/app.log".into(),
            real_path: "cas://a".into(),
            line: 2,
            content: "Second log entry".into(),
            tags: vec![],
            match_details: None,
            matched_keywords: None,
        };

        manager.add_document(&entry1).unwrap();
        manager.add_document(&entry2).unwrap();
        manager.commit().unwrap();

        let (min_ts, max_ts, total) = manager.get_time_range().unwrap();
        assert_eq!(min_ts, 1_704_067_200);
        assert_eq!(max_ts, 1_704_067_500);
        assert_eq!(total, 2);
    }

    #[test]
    fn test_rebuild_keeps_previous_index_visible_until_commit() {
        let (manager, _temp_dir) = create_test_manager();
        manager
            .add_document(&test_entry("2024-01-01 00:00:00", "old"))
            .unwrap();
        manager.commit().unwrap();

        let mut rebuild = manager.begin_rebuild().unwrap();
        rebuild
            .add_documents(&[test_entry("2025-01-01 00:00:00", "new")])
            .unwrap();

        assert_eq!(
            manager.get_time_range().unwrap(),
            (1_704_067_200, 1_704_067_200, 1)
        );

        rebuild.commit().unwrap();
        assert_eq!(
            manager.get_time_range().unwrap(),
            (1_735_689_600, 1_735_689_600, 1)
        );
    }

    #[test]
    fn test_dropped_rebuild_rolls_back_to_previous_index() {
        let (manager, _temp_dir) = create_test_manager();
        manager
            .add_document(&test_entry("2024-01-01 00:00:00", "old"))
            .unwrap();
        manager.commit().unwrap();

        {
            let mut rebuild = manager.begin_rebuild().unwrap();
            rebuild
                .add_documents(&[test_entry("2025-01-01 00:00:00", "new")])
                .unwrap();
        }

        assert_eq!(
            manager.get_time_range().unwrap(),
            (1_704_067_200, 1_704_067_200, 1)
        );
        manager.commit().unwrap();
        assert_eq!(
            manager.get_time_range().unwrap(),
            (1_704_067_200, 1_704_067_200, 1)
        );
    }

    #[test]
    fn test_dropped_update_rolls_back_appended_documents() {
        let (manager, _temp_dir) = create_test_manager();
        manager
            .add_document(&test_entry("2024-01-01 00:00:00", "old"))
            .unwrap();
        manager.commit().unwrap();

        {
            let mut update = manager.begin_update().unwrap();
            update
                .add_documents(&[test_entry("2025-01-01 00:00:00", "new")])
                .unwrap();
        }

        manager.commit().unwrap();
        assert_eq!(
            manager.get_time_range().unwrap(),
            (1_704_067_200, 1_704_067_200, 1)
        );
    }

    #[test]
    fn file_replacement_and_truncation_preserve_other_files_and_rollback() {
        let (manager, _temp_dir) = create_test_manager();
        let mut old = test_entry("2024-01-01 00:00:00", "old");
        old.file = "logs/app.log".into();
        let mut other = old.clone();
        other.file = "logs/other.log".into();
        manager.add_documents(&[old.clone(), other]).unwrap();
        manager.commit().unwrap();
        {
            let mut update = manager.begin_update().unwrap();
            update.remove_file(&old.file);
        }
        manager.commit().unwrap();
        assert_eq!(manager.get_time_range().unwrap().2, 2);
        let mut update = manager.begin_update().unwrap();
        update.remove_file(&old.file);
        update.commit().unwrap();
        assert_eq!(manager.get_time_range().unwrap().2, 1);
    }

    #[test]
    fn test_committed_update_appends_documents() {
        let (manager, _temp_dir) = create_test_manager();
        manager
            .add_document(&test_entry("2024-01-01 00:00:00", "old"))
            .unwrap();
        manager.commit().unwrap();

        let mut update = manager.begin_update().unwrap();
        update
            .add_documents(&[test_entry("2025-01-01 00:00:00", "new")])
            .unwrap();
        update.commit().unwrap();

        assert_eq!(
            manager.get_time_range().unwrap(),
            (1_704_067_200, 1_735_689_600, 2)
        );
    }

    /// Test delete_file_documents functionality
    #[tokio::test]
    async fn test_delete_file_documents() {
        let (manager, _temp_dir) = create_test_manager();

        let file_path = "/test/path/file.log";

        // Add some documents for the file
        let entry1 = la_core::models::LogEntry {
            id: 1,
            timestamp: "2024-01-01T00:00:00".into(),
            level: "INFO".into(),
            file: file_path.into(),
            real_path: "/real/path/file.log".into(),
            line: 1,
            content: "First log entry".into(),
            tags: vec![],
            match_details: None,
            matched_keywords: None,
        };

        let entry2 = la_core::models::LogEntry {
            id: 2,
            timestamp: "2024-01-01T00:00:01".into(),
            level: "ERROR".into(),
            file: file_path.into(),
            real_path: "/real/path/file.log".into(),
            line: 2,
            content: "Second log entry".into(),
            tags: vec![],
            match_details: None,
            matched_keywords: None,
        };

        let entry3 = la_core::models::LogEntry {
            id: 3,
            timestamp: "2024-01-01T00:00:02".into(),
            level: "DEBUG".into(),
            file: "/other/path/other.log".into(),
            real_path: "/real/other.log".into(),
            line: 1,
            content: "Entry from other file".into(),
            tags: vec![],
            match_details: None,
            matched_keywords: None,
        };

        // Add documents to index
        manager.add_document(&entry1).unwrap();
        manager.add_document(&entry2).unwrap();
        manager.add_document(&entry3).unwrap();
        manager.commit().unwrap();

        // Verify documents are in the index
        let results = manager
            .search_with_timeout("log entry", None, None, None)
            .await
            .unwrap();
        assert_eq!(
            results.entries.len(),
            3,
            "Should have 3 documents initially"
        );

        // Delete documents for the first file
        let deleted_count = manager.delete_file_documents(file_path).unwrap();
        assert_eq!(deleted_count, 2, "Should delete 2 documents");

        // Verify only the third document remains
        let results = manager
            .search_with_timeout("log entry", None, None, None)
            .await
            .unwrap();
        assert_eq!(results.entries.len(), 1, "Should have 1 document remaining");
        assert_eq!(&*results.entries[0].file, "/other/path/other.log");
    }

    /// Test delete_file_documents for non-existent file
    #[tokio::test]
    async fn test_delete_nonexistent_file() {
        let (manager, _temp_dir) = create_test_manager();

        // Try to delete documents for a file that doesn't exist in the index
        let deleted_count = manager
            .delete_file_documents("/nonexistent/file.log")
            .unwrap();
        assert_eq!(
            deleted_count, 0,
            "Should delete 0 documents for non-existent file"
        );
    }
    // P2 regression: schema version migration

    #[test]
    fn stale_or_missing_schema_version_requires_rebuild() {
        let temp_dir = tempfile::tempdir().unwrap();

        assert!(index_schema_is_stale(temp_dir.path()));

        write_schema_version(temp_dir.path()).unwrap();
        assert!(!index_schema_is_stale(temp_dir.path()));

        std::fs::write(temp_dir.path().join("schema_version"), "999").unwrap();
        assert!(index_schema_is_stale(temp_dir.path()));

        std::fs::write(temp_dir.path().join("schema_version"), "not-a-number").unwrap();
        assert!(index_schema_is_stale(temp_dir.path()));
    }
}
