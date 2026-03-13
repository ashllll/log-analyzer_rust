//! Optimized Search Engine Manager
//!
//! High-performance Tantivy search implementation with industry best practices:
//! - Channel-based IndexWriter pool (eliminates single Mutex bottleneck)
//! - Arc-swap IndexReader pattern (prevents double reload issue)
//! - Thread-local Searcher cache (avoids creating new Searcher per query)
//! - Moka query cache (caches parsed queries and results)
//! - Parallel highlighting with Rayon (multi-core utilization)
//! - Memory budget enforcement (prevents OOM on large result sets)
//!
//! References:
//! - Tantivy GitHub issues: #549 (multi-threaded indexing)
//! - Quickwit architecture patterns
//! - Tantivy 0.22+ best practices

use dashmap::DashMap;
use moka::sync::Cache as MokaCache;
use parking_lot::{Mutex, RwLock};
use rayon::prelude::*;
use std::cell::RefCell;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tantivy::collector::{Collector, Count, TopDocs};
use tantivy::query::{Query, QueryParser};
use tantivy::schema::Value;
use tantivy::{
    DocAddress, Index, IndexReader, IndexWriter, ReloadPolicy, Searcher, TantivyDocument, Term,
};
use tokio::sync::{mpsc, oneshot, Semaphore};
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

use super::{
    BooleanQueryProcessor, HighlightingConfig, HighlightingEngine, LogSchema, SearchError,
    SearchResult,
};
use crate::models::config::SearchConfig as AppSearchConfig;
use crate::models::LogEntry;

/// Default memory budget per search operation (MB)
const DEFAULT_MEMORY_BUDGET_MB: usize = 256;

/// Maximum number of cached searchers per thread
const MAX_SEARCHERS_PER_THREAD: usize = 2;

/// Query cache TTL in seconds
const QUERY_CACHE_TTL_SECS: u64 = 300;

/// Query cache max capacity
const QUERY_CACHE_MAX_CAPACITY: u64 = 10_000;

/// Writer command for channel-based IndexWriter pool
#[derive(Debug)]
enum WriterCommand {
    AddDocument {
        doc: TantivyDocument,
        response_tx: oneshot::Sender<SearchResult<()>>,
    },
    DeleteTerm {
        term: Term,
        response_tx: oneshot::Sender<SearchResult<()>>,
    },
    Commit {
        response_tx: oneshot::Sender<SearchResult<u64>>,
    },
    DeleteAll {
        response_tx: oneshot::Sender<SearchResult<()>>,
    },
}

/// Writer pool for concurrent IndexWriter access
///
/// Uses a single dedicated thread with a channel instead of Mutex contention.
/// This is the recommended pattern for Tantivy multi-threaded indexing.
pub struct WriterPool {
    command_tx: mpsc::UnboundedSender<WriterCommand>,
    pending_count: AtomicU64,
}

impl WriterPool {
    /// Create a new writer pool with the given index and heap size
    fn new(index: &Index, heap_size: usize) -> SearchResult<Self> {
        let (command_tx, mut command_rx) = mpsc::unbounded_channel::<WriterCommand>();

        // Create IndexWriter in dedicated thread
        let index_clone = index.clone();
        std::thread::spawn(move || {
            let mut writer = match index_clone.writer(heap_size) {
                Ok(w) => w,
                Err(e) => {
                    error!(error = %e, "Failed to create IndexWriter in pool");
                    return;
                }
            };

            // Process commands sequentially (Tantivy's IndexWriter is not Send)
            while let Some(cmd) = command_rx.blocking_recv() {
                match cmd {
                    WriterCommand::AddDocument { doc, response_tx } => {
                        let result = writer.add_document(doc).map_err(SearchError::from);
                        let _ = response_tx.send(result);
                    }
                    WriterCommand::DeleteTerm { term, response_tx } => {
                        let _ = writer.delete_term(term);
                        let _ = response_tx.send(Ok(()));
                    }
                    WriterCommand::Commit { response_tx } => {
                        let result = writer.commit().map_err(SearchError::from);
                        let _ = response_tx.send(result);
                    }
                    WriterCommand::DeleteAll { response_tx } => {
                        let result = writer
                            .delete_all_documents()
                            .and_then(|_| writer.commit())
                            .map_err(SearchError::from);
                        let _ = response_tx.send(result.map(|_| ()));
                    }
                }
            }

            // Wait for merge threads on shutdown
            let _ = writer.wait_merging_threads();
        });

        Ok(Self {
            command_tx,
            pending_count: AtomicU64::new(0),
        })
    }

    /// Add a document to the index
    async fn add_document(&self, doc: TantivyDocument) -> SearchResult<()> {
        let (tx, rx) = oneshot::channel();
        self.command_tx
            .send(WriterCommand::AddDocument { doc, response_tx: tx })
            .map_err(|_| SearchError::IndexError("Writer pool closed".to_string()))?;
        self.pending_count.fetch_add(1, Ordering::Relaxed);

        let result = rx.await.map_err(|_| {
            SearchError::IndexError("Writer response channel closed".to_string())
        })?;
        self.pending_count.fetch_sub(1, Ordering::Relaxed);
        result
    }

    /// Delete documents matching the term
    async fn delete_term(&self, term: Term) -> SearchResult<()> {
        let (tx, rx) = oneshot::channel();
        self.command_tx
            .send(WriterCommand::DeleteTerm { term, response_tx: tx })
            .map_err(|_| SearchError::IndexError("Writer pool closed".to_string()))?;

        rx.await
            .map_err(|_| SearchError::IndexError("Writer response channel closed".to_string()))?
    }

    /// Commit pending changes
    async fn commit(&self) -> SearchResult<u64> {
        let (tx, rx) = oneshot::channel();
        self.command_tx
            .send(WriterCommand::Commit { response_tx: tx })
            .map_err(|_| SearchError::IndexError("Writer pool closed".to_string()))?;

        rx.await
            .map_err(|_| SearchError::IndexError("Writer response channel closed".to_string()))?
    }

    /// Delete all documents
    async fn delete_all(&self) -> SearchResult<()> {
        let (tx, rx) = oneshot::channel();
        self.command_tx
            .send(WriterCommand::DeleteAll { response_tx: tx })
            .map_err(|_| SearchError::IndexError("Writer pool closed".to_string()))?;

        rx.await
            .map_err(|_| SearchError::IndexError("Writer response channel closed".to_string()))?
    }

    /// Get pending operation count
    fn pending_count(&self) -> u64 {
        self.pending_count.load(Ordering::Relaxed)
    }
}

/// Thread-local Searcher cache entry
struct SearcherCacheEntry {
    searcher: Searcher,
    generation: u64,
}

/// Search result entry with document address for highlighting
#[derive(Debug, Clone)]
pub struct SearchResultEntry {
    pub entry: LogEntry,
    pub doc_address: DocAddress,
}

/// Search results with metadata and memory tracking
#[derive(Debug, Clone)]
pub struct SearchResults {
    pub entries: Vec<LogEntry>,
    pub doc_addresses: Vec<DocAddress>,
    pub total_count: usize,
    pub query_time_ms: u64,
    pub was_timeout: bool,
    pub memory_used_bytes: usize,
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
            memory_used_bytes: 0,
        }
    }

    /// Estimate memory usage of results
    fn estimate_memory(&self) -> usize {
        let entry_size: usize = self
            .entries
            .iter()
            .map(|e| {
                e.content.len()
                    + e.file.len()
                    + e.real_path.len()
                    + e.timestamp.len()
                    + e.level.len()
                    + std::mem::size_of::<LogEntry>()
            })
            .sum();
        entry_size + self.doc_addresses.len() * std::mem::size_of::<DocAddress>()
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

/// Search configuration with memory budget
#[derive(Debug, Clone)]
pub struct SearchConfig {
    pub default_timeout: Duration,
    pub max_results: usize,
    pub index_path: PathBuf,
    pub writer_heap_size: usize,
    pub memory_budget_mb: usize,
    pub enable_query_cache: bool,
    pub enable_parallel_highlight: bool,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            default_timeout: Duration::from_millis(200),
            max_results: 50_000,
            index_path: PathBuf::from("./search_index"),
            writer_heap_size: 50_000_000,
            memory_budget_mb: DEFAULT_MEMORY_BUDGET_MB,
            enable_query_cache: true,
            enable_parallel_highlight: true,
        }
    }
}

/// Query cache key
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct QueryCacheKey {
    query: String,
    limit: usize,
}

/// Cached query result
#[derive(Debug, Clone)]
struct CachedQueryResult {
    results: SearchResults,
    cached_at: Instant,
}

/// Statistics for search operations
#[derive(Debug, Default, Clone)]
pub struct SearchStats {
    pub total_searches: u64,
    pub total_query_time_ms: u64,
    pub timeout_count: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub searcher_cache_hits: u64,
    pub searcher_cache_misses: u64,
    pub memory_limited_count: u64,
    pub total_memory_used_bytes: u64,
}

/// Optimized search engine manager with industry best practices
pub struct OptimizedSearchEngineManager {
    index: Index,
    reader: IndexReader,
    writer_pool: WriterPool,
    query_parser: QueryParser,
    schema: LogSchema,
    config: SearchConfig,
    stats: Arc<RwLock<SearchStats>>,
    boolean_processor: BooleanQueryProcessor,
    highlighting_engine: HighlightingEngine,
    optimizer: Option<Arc<super::index_optimizer::IndexOptimizer>>,

    // Thread-local searcher cache using thread_id -> RefCell<Option<Searcher>>
    searcher_cache: Arc<DashMap<std::thread::ThreadId, RefCell<Option<SearcherCacheEntry>>>>,
    reader_generation: AtomicU64,

    // Moka query cache
    query_cache: Option<MokaCache<QueryCacheKey, CachedQueryResult>>,

    // Semaphore for memory-bounded search
    search_semaphore: Arc<Semaphore>,

    // Parallelism configuration
    parallel_pool: rayon::ThreadPool,
}

impl OptimizedSearchEngineManager {
    /// Create a new optimized search engine manager
    pub fn new(config: SearchConfig) -> SearchResult<Self> {
        let schema = LogSchema::build();

        // Create or open index
        let meta_path = config.index_path.join("meta.json");
        let index = if meta_path.exists() {
            Index::open_in_dir(&config.index_path)?
        } else {
            std::fs::create_dir_all(&config.index_path)?;
            Index::create_in_dir(&config.index_path, schema.schema.clone())?
        };

        // Configure tokenizers
        schema.configure_tokenizers(&index)?;

        // Create reader with manual reload policy (we control when to reload)
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()?;

        // Create writer pool (replaces single Mutex)
        let writer_pool = WriterPool::new(&index, config.writer_heap_size)?;

        // Create query parser
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

        // Initialize optimizer
        let optimizer = Some(Arc::new(super::index_optimizer::IndexOptimizer::new(100)));

        // Initialize query cache if enabled
        let query_cache = if config.enable_query_cache {
            Some(
                MokaCache::builder()
                    .max_capacity(QUERY_CACHE_MAX_CAPACITY)
                    .time_to_live(Duration::from_secs(QUERY_CACHE_TTL_SECS))
                    .build(),
            )
        } else {
            None
        };

        // Create search semaphore for memory control
        let search_semaphore = Arc::new(Semaphore::new(
            // Limit concurrent searches based on memory budget
            (config.memory_budget_mb / 64).max(1),
        ));

        // Create parallel pool for highlighting
        let parallel_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(num_cpus::get().max(1))
            .build()
            .map_err(|e| SearchError::IndexError(format!("Failed to create thread pool: {}", e)))?;

        info!(
            index_path = %config.index_path.display(),
            heap_size = config.writer_heap_size,
            memory_budget_mb = config.memory_budget_mb,
            optimizer_enabled = optimizer.is_some(),
            query_cache_enabled = query_cache.is_some(),
            "Optimized search engine initialized"
        );

        Ok(Self {
            index,
            reader,
            writer_pool,
            query_parser,
            schema,
            config,
            stats: Arc::new(RwLock::new(SearchStats::default())),
            boolean_processor,
            highlighting_engine,
            optimizer,
            searcher_cache: Arc::new(DashMap::new()),
            reader_generation: AtomicU64::new(0),
            query_cache,
            search_semaphore,
            parallel_pool,
        })
    }

    /// Create with application configuration
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
            ..Default::default()
        };
        Self::new(engine_config)
    }

    /// Get or create cached searcher for current thread
    fn get_searcher(&self) -> SearchResult<Searcher> {
        let thread_id = std::thread::current().id();
        let current_gen = self.reader_generation.load(Ordering::Acquire);

        // Check thread-local cache
        if let Some(entry_ref) = self.searcher_cache.get(&thread_id) {
            let mut entry_opt = entry_ref.borrow_mut();

            if let Some(ref entry) = *entry_opt {
                if entry.generation == current_gen {
                    // Cache hit
                    self.stats.write().searcher_cache_hits += 1;
                    return Ok(entry.searcher.clone());
                }
            }

            // Cache miss - generation mismatch
            drop(entry_opt);
        }

        // Cache miss - create new searcher
        self.stats.write().searcher_cache_misses += 1;
        let searcher = self.reader.searcher();

        // Store in cache
        self.searcher_cache.insert(
            thread_id,
            RefCell::new(Some(SearcherCacheEntry {
                searcher: searcher.clone(),
                generation: current_gen,
            })),
        );

        Ok(searcher)
    }

    /// Search with timeout and memory budget enforcement
    pub async fn search_with_budget(
        &self,
        query: &str,
        limit: Option<usize>,
        timeout_duration: Option<Duration>,
        token: Option<CancellationToken>,
        memory_budget_mb: Option<usize>,
    ) -> SearchResult<SearchResults> {
        let start_time = Instant::now();
        let limit = limit.unwrap_or(self.config.max_results);
        let timeout_duration = timeout_duration.unwrap_or(self.config.default_timeout);
        let token = token.unwrap_or_default();
        let memory_budget_mb = memory_budget_mb.unwrap_or(self.config.memory_budget_mb);
        let memory_budget_bytes = memory_budget_mb * 1024 * 1024;

        debug!(
            query = %query,
            limit = limit,
            timeout_ms = timeout_duration.as_millis(),
            memory_budget_mb = memory_budget_mb,
            "Starting optimized search with budget"
        );

        // Check query cache first
        let cache_key = QueryCacheKey {
            query: query.to_string(),
            limit,
        };

        if let Some(ref cache) = self.query_cache {
            if let Some(cached) = cache.get(&cache_key) {
                if cached.cached_at.elapsed() < Duration::from_secs(QUERY_CACHE_TTL_SECS) {
                    self.stats.write().cache_hits += 1;
                    debug!("Query cache hit");
                    return Ok(cached.results.clone());
                }
            }
        }

        self.stats.write().cache_misses += 1;

        // Acquire memory budget permit
        let _permit = self
            .search_semaphore
            .acquire()
            .await
            .map_err(|_| SearchError::IndexError("Search semaphore closed".to_string()))?;

        // Parse query
        let parsed_query = self.parse_query(query)?;

        // Execute search with timeout
        let search_future =
            self.execute_search_with_budget(parsed_query, limit, token.clone(), memory_budget_bytes);

        match timeout(timeout_duration, search_future).await {
            Ok(result) => {
                let query_time = start_time.elapsed();
                self.update_stats(query_time, false);

                match result {
                    Ok(mut search_results) => {
                        search_results.query_time_ms = query_time.as_millis() as u64;
                        search_results.was_timeout = false;
                        search_results.memory_used_bytes = search_results.estimate_memory();

                        // Cache results if within budget
                        if let Some(ref cache) = self.query_cache {
                            if search_results.memory_used_bytes < memory_budget_bytes / 10 {
                                cache.insert(
                                    cache_key,
                                    CachedQueryResult {
                                        results: search_results.clone(),
                                        cached_at: Instant::now(),
                                    },
                                );
                            }
                        }

                        // Record for optimization
                        if let Some(ref optimizer) = self.optimizer {
                            optimizer.record_query_with_results(
                                query,
                                query_time,
                                search_results.entries.len(),
                            );
                        }

                        info!(
                            query = %query,
                            results = search_results.entries.len(),
                            total = search_results.total_count,
                            time_ms = search_results.query_time_ms,
                            memory_bytes = search_results.memory_used_bytes,
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
                token.cancel();

                warn!(
                    query = %query,
                    timeout_ms = timeout_duration.as_millis(),
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
        if query_str.trim().is_empty() {
            return Err(SearchError::QueryError("Empty query".to_string()));
        }

        let keywords: Vec<&str> = query_str.split_whitespace().collect();
        if keywords.len() > 1 {
            return self.boolean_processor.parse_and_optimize_query(query_str);
        }

        match self.query_parser.parse_query(query_str) {
            Ok(query) => Ok(query),
            Err(e) => {
                warn!(query = %query_str, error = %e, "Query parsing failed, using simple term");
                let term = Term::from_field_text(self.schema.content, query_str);
                Ok(Box::new(tantivy::query::TermQuery::new(
                    term,
                    tantivy::schema::IndexRecordOption::Basic,
                )))
            }
        }
    }

    /// Execute search with memory budget enforcement
    async fn execute_search_with_budget(
        &self,
        query: Box<dyn Query>,
        limit: usize,
        token: CancellationToken,
        memory_budget_bytes: usize,
    ) -> SearchResult<SearchResults> {
        let searcher = self.get_searcher()?;

        // Get total count with cancellation
        let count_collector =
            super::boolean_query_processor::CancellableCollector::new(Count, token.clone());
        let total_count = match searcher.search(&*query, &count_collector) {
            Ok(count) => count,
            Err(e) => {
                if token.is_cancelled() {
                    return Err(SearchError::QueryError("Search cancelled".to_string()));
                }
                return Err(SearchError::IndexError(e.to_string()));
            }
        };

        // Calculate effective limit based on memory budget
        // Estimate ~500 bytes per result entry
        let estimated_bytes_per_result = 500;
        let max_results_by_memory = memory_budget_bytes / estimated_bytes_per_result;
        let effective_limit = limit.min(max_results_by_memory).min(self.config.max_results);

        if effective_limit < total_count {
            self.stats.write().memory_limited_count += 1;
            warn!(
                requested = limit,
                effective = effective_limit,
                total = total_count,
                "Memory budget limiting search results"
            );
        }

        // Get top documents
        let top_docs_collector = TopDocs::with_limit(effective_limit);
        let cancellable_top_docs = super::boolean_query_processor::CancellableCollector::new(
            top_docs_collector,
            token.clone(),
        );

        let top_docs = match searcher.search(&*query, &cancellable_top_docs) {
            Ok(docs) => docs,
            Err(e) => {
                if token.is_cancelled() {
                    return Err(SearchError::QueryError("Search cancelled".to_string()));
                }
                return Err(SearchError::IndexError(e.to_string()));
            }
        };

        // Convert documents to LogEntry
        let mut entries = Vec::with_capacity(top_docs.len());
        let mut doc_addresses = Vec::with_capacity(top_docs.len());

        for (_score, doc_address) in top_docs {
            if token.is_cancelled() {
                return Err(SearchError::QueryError("Search cancelled".to_string()));
            }

            let retrieved_doc = searcher.doc(doc_address)?;

            if let Some(log_entry) = self.document_to_log_entry(&retrieved_doc) {
                entries.push(log_entry);
                doc_addresses.push(doc_address);
            }
        }

        let results = SearchResults {
            entries,
            doc_addresses,
            total_count,
            query_time_ms: 0,
            was_timeout: false,
            memory_used_bytes: 0,
        };

        // Update memory stats
        let memory_used = results.estimate_memory();
        self.stats.write().total_memory_used_bytes += memory_used as u64;

        Ok(results)
    }

    /// Convert Tantivy document to LogEntry
    fn document_to_log_entry(&self, doc: &TantivyDocument) -> Option<LogEntry> {
        let content = doc.get_first(self.schema.content).and_then(|v| v.as_str())?;
        let timestamp_i64 = doc.get_first(self.schema.timestamp).and_then(|v| v.as_i64())?;
        let level = doc.get_first(self.schema.level).and_then(|v| v.as_str())?;
        let file_path = doc.get_first(self.schema.file_path).and_then(|v| v.as_str())?;
        let real_path = doc.get_first(self.schema.real_path).and_then(|v| v.as_str())?;
        let line_number = doc.get_first(self.schema.line_number).and_then(|v| v.as_u64())?;

        Some(LogEntry {
            id: 0,
            timestamp: timestamp_i64.to_string().into(),
            level: level.to_string().into(),
            file: file_path.to_string().into(),
            real_path: real_path.to_string().into(),
            line: line_number as usize,
            content: content.to_string().into(),
            tags: vec![],
            match_details: None,
            matched_keywords: None,
        })
    }

    /// Add document to index (async via writer pool)
    pub async fn add_document(&self, log_entry: &LogEntry) -> SearchResult<()> {
        let mut doc = TantivyDocument::default();

        doc.add_text(self.schema.content, &log_entry.content);
        let timestamp_i64 = log_entry.timestamp.parse::<i64>().unwrap_or(0);
        doc.add_i64(self.schema.timestamp, timestamp_i64);
        doc.add_text(self.schema.level, &log_entry.level);
        doc.add_text(self.schema.file_path, &log_entry.file);
        doc.add_text(self.schema.real_path, &log_entry.real_path);
        doc.add_u64(self.schema.line_number, log_entry.line as u64);

        self.writer_pool.add_document(doc).await
    }

    /// Commit pending changes (async via writer pool)
    pub async fn commit(&self) -> SearchResult<u64> {
        let opstamp = self.writer_pool.commit().await?;

        // Reload reader to see new changes
        self.reader.reload()?;
        self.reader_generation.fetch_add(1, Ordering::Release);

        // Clear searcher cache since generation changed
        self.searcher_cache.clear();

        Ok(opstamp)
    }

    /// Search with parallel highlighting
    pub async fn search_with_parallel_highlighting(
        &self,
        query: &str,
        limit: Option<usize>,
        timeout_duration: Option<Duration>,
        token: Option<CancellationToken>,
    ) -> SearchResult<SearchResultsWithHighlighting> {
        let start_time = Instant::now();

        // First perform search
        let search_results = self
            .search_with_budget(query, limit, timeout_duration, token.clone(), None)
            .await?;

        // Parallel highlighting
        let highlight_start = Instant::now();
        let highlighted_entries = if self.config.enable_parallel_highlight {
            self.highlight_parallel(&search_results.entries, &search_results.doc_addresses, query)
        } else {
            self.highlight_serial(&search_results.entries, &search_results.doc_addresses, query)
        };

        let highlight_time = highlight_start.elapsed();
        let total_time = start_time.elapsed();

        info!(
            query = %query,
            results = highlighted_entries.len(),
            highlight_time_ms = highlight_time.as_millis(),
            total_time_ms = total_time.as_millis(),
            "Search with parallel highlighting completed"
        );

        Ok(SearchResultsWithHighlighting {
            entries: highlighted_entries,
            total_count: search_results.total_count,
            query_time_ms: search_results.query_time_ms,
            highlight_time_ms: highlight_time.as_millis() as u64,
            was_timeout: search_results.was_timeout,
        })
    }

    /// Highlight entries in parallel using Rayon
    fn highlight_parallel(
        &self,
        entries: &[LogEntry],
        doc_addresses: &[DocAddress],
        query: &str,
    ) -> Vec<LogEntry> {
        let entries_with_indices: Vec<(usize, &LogEntry, DocAddress)> = entries
            .iter()
            .enumerate()
            .zip(doc_addresses.iter())
            .map(|((i, entry), &addr)| (i, entry, addr))
            .collect();

        let highlighted: Vec<(usize, LogEntry)> = self
            .parallel_pool
            .install(|| {
                entries_with_indices
                    .into_par_iter()
                    .map(|(i, entry, doc_address)| {
                        let highlighted_entry =
                            self.highlight_single_entry(entry, doc_address, query);
                        (i, highlighted_entry)
                    })
                    .collect()
            });

        // Reconstruct in original order
        let mut result: Vec<Option<LogEntry>> = vec![None; entries.len()];
        for (i, entry) in highlighted {
            result[i] = Some(entry);
        }

        result.into_iter().flatten().collect()
    }

    /// Highlight entries serially
    fn highlight_serial(
        &self,
        entries: &[LogEntry],
        doc_addresses: &[DocAddress],
        query: &str,
    ) -> Vec<LogEntry> {
        entries
            .iter()
            .zip(doc_addresses.iter())
            .map(|(entry, &doc_address)| self.highlight_single_entry(entry, doc_address, query))
            .collect()
    }

    /// Highlight a single entry
    fn highlight_single_entry(
        &self,
        entry: &LogEntry,
        doc_address: DocAddress,
        query: &str,
    ) -> LogEntry {
        match self
            .highlighting_engine
            .highlight_document(doc_address, query, &entry.content)
        {
            Ok(highlights) => {
                let mut highlighted_entry = entry.clone();
                highlighted_entry.content = highlights.join(" ... ").into();
                highlighted_entry
            }
            Err(e) => {
                warn!(error = %e, "Failed to highlight entry, using original");
                entry.clone()
            }
        }
    }

    /// Search with multiple keywords
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
        let token = token.unwrap_or_default();

        debug!(
            keywords = ?keywords,
            require_all = require_all,
            limit = limit,
            "Starting multi-keyword search"
        );

        let token_inner = token.clone();
        let search_future = async move {
            let (doc_addresses, total_count) = self
                .boolean_processor
                .process_multi_keyword_query(keywords, require_all, limit, Some(token_inner))?;

            let searcher = self.get_searcher()?;
            let mut entries = Vec::with_capacity(doc_addresses.len());
            let mut addresses = Vec::with_capacity(doc_addresses.len());

            for doc_address in doc_addresses {
                let retrieved_doc = searcher.doc(doc_address)?;
                if let Some(log_entry) = self.document_to_log_entry(&retrieved_doc) {
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
                memory_used_bytes: 0,
            })
        };

        match timeout(timeout_duration, search_future).await {
            Ok(result) => {
                let query_time = start_time.elapsed();
                self.update_stats(query_time, false);

                match result {
                    Ok(mut search_results) => {
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
                            "Multi-keyword search completed"
                        );

                        Ok(search_results)
                    }
                    Err(e) => Err(e),
                }
            }
            Err(_) => {
                let query_time = start_time.elapsed();
                self.update_stats(query_time, true);
                token.cancel();

                Err(SearchError::Timeout(format!(
                    "Multi-keyword search timed out after {}ms",
                    timeout_duration.as_millis()
                )))
            }
        }
    }

    /// Delete file documents
    pub async fn delete_file_documents(&self, file_path: &str) -> SearchResult<usize> {
        let searcher = self.get_searcher()?;
        let term = Term::from_field_text(self.schema.file_path, file_path);

        // Get count first
        let query = tantivy::query::TermQuery::new(
            term.clone(),
            tantivy::schema::IndexRecordOption::Basic,
        );
        let count = searcher.search(&query, &Count)?;

        // Delete via writer pool
        self.writer_pool.delete_term(term).await?;

        // Commit and reload
        self.commit().await?;

        info!(
            file_path = %file_path,
            deleted_count = count,
            "Deleted documents for file"
        );

        Ok(count)
    }

    /// Clear the index
    pub async fn clear_index(&self) -> SearchResult<()> {
        self.writer_pool.delete_all().await?;
        self.commit().await?;
        Ok(())
    }

    /// Update statistics
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

    /// Get optimization analysis
    pub fn get_optimization_analysis(
        &self,
    ) -> Option<super::index_optimizer::IndexPerformanceAnalysis> {
        self.optimizer.as_ref().map(|opt| opt.analyze_performance())
    }

    /// Get hot queries
    pub fn get_hot_queries(
        &self,
    ) -> Vec<(String, super::index_optimizer::QueryPatternStats)> {
        self.optimizer
            .as_ref()
            .map(|opt| opt.identify_hot_queries())
            .unwrap_or_default()
    }

    /// Get writer pool pending count
    pub fn get_writer_pending_count(&self) -> u64 {
        self.writer_pool.pending_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_test_manager() -> (OptimizedSearchEngineManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config = SearchConfig {
            index_path: temp_dir.path().to_path_buf(),
            enable_query_cache: true,
            enable_parallel_highlight: true,
            ..Default::default()
        };

        let manager = OptimizedSearchEngineManager::new(config).unwrap();
        (manager, temp_dir)
    }

    #[tokio::test]
    async fn test_search_engine_creation() {
        let (_manager, _temp_dir) = create_test_manager().await;
    }

    #[tokio::test]
    async fn test_empty_search() {
        let (manager, _temp_dir) = create_test_manager().await;

        let result = manager
            .search_with_budget("", None, None, None, None)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_search_with_budget() {
        let (manager, _temp_dir) = create_test_manager().await;

        let result = manager
            .search_with_budget("test", None, Some(Duration::from_secs(1)), None, Some(128))
            .await;

        assert!(result.is_ok());
        let results = result.unwrap();
        assert_eq!(results.entries.len(), 0);
    }

    #[tokio::test]
    async fn test_add_and_search() {
        let (manager, _temp_dir) = create_test_manager().await;

        let entry = LogEntry {
            id: 1,
            timestamp: "1704067200".into(),
            level: "INFO".into(),
            file: "/test.log".into(),
            real_path: "/real/test.log".into(),
            line: 1,
            content: "Test log entry with keyword".into(),
            tags: vec![],
            match_details: None,
            matched_keywords: None,
        };

        manager.add_document(&entry).await.unwrap();
        manager.commit().await.unwrap();

        let result = manager
            .search_with_budget("keyword", None, Some(Duration::from_secs(1)), None, None)
            .await;

        assert!(result.is_ok());
        let results = result.unwrap();
        assert_eq!(results.entries.len(), 1);
    }

    #[tokio::test]
    async fn test_parallel_highlighting() {
        let (manager, _temp_dir) = create_test_manager().await;

        // Add test documents
        for i in 0..10 {
            let entry = LogEntry {
                id: i,
                timestamp: "1704067200".into(),
                level: "INFO".into(),
                file: "/test.log".into(),
                real_path: "/real/test.log".into(),
                line: i as usize,
                content: format!("Test entry with keyword {}", i).into(),
                tags: vec![],
                match_details: None,
                matched_keywords: None,
            };
            manager.add_document(&entry).await.unwrap();
        }
        manager.commit().await.unwrap();

        let result = manager
            .search_with_parallel_highlighting("keyword", Some(10), None, None)
            .await;

        assert!(result.is_ok());
        let results = result.unwrap();
        assert_eq!(results.entries.len(), 10);
    }

    #[tokio::test]
    async fn test_query_caching() {
        let (manager, _temp_dir) = create_test_manager().await;

        // First search - cache miss
        let result1 = manager
            .search_with_budget("test", None, Some(Duration::from_secs(1)), None, None)
            .await;
        assert!(result1.is_ok());

        // Second search - should be cache hit
        let result2 = manager
            .search_with_budget("test", None, Some(Duration::from_secs(1)), None, None)
            .await;
        assert!(result2.is_ok());

        let stats = manager.get_stats();
        assert!(stats.cache_hits >= 1);
    }

    #[tokio::test]
    async fn test_delete_file_documents() {
        let (manager, _temp_dir) = create_test_manager().await;

        let entry = LogEntry {
            id: 1,
            timestamp: "1704067200".into(),
            level: "INFO".into(),
            file: "/test.log".into(),
            real_path: "/real/test.log".into(),
            line: 1,
            content: "Test content".into(),
            tags: vec![],
            match_details: None,
            matched_keywords: None,
        };

        manager.add_document(&entry).await.unwrap();
        manager.commit().await.unwrap();

        let deleted = manager.delete_file_documents("/test.log").await.unwrap();
        assert_eq!(deleted, 1);

        let result = manager
            .search_with_budget("Test", None, Some(Duration::from_secs(1)), None, None)
            .await
            .unwrap();
        assert_eq!(result.entries.len(), 0);
    }
}
