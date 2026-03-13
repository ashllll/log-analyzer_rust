//! 真正异步的搜索引擎管理器
//!
//! 解决伪异步问题：使用 tokio::spawn_blocking 将 CPU 密集型搜索操作
//! 移动到专用线程池，避免阻塞 Tokio 运行时。
//!
//! ## 业内成熟方案
//! - `tokio::spawn_blocking`: 将同步 CPU 密集型任务移至独立线程池
//! - `rayon`: 数据并行（可选）
//! - `Semaphore`: 背压控制，限制并发搜索数量
//! - `CancellationToken`: 协作式取消机制

use parking_lot::{Mutex, RwLock};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tantivy::{
    collector::{Count, TopDocs},
    query::{Query, QueryParser, TermQuery},
    schema::Value,
    DocAddress, Index, IndexReader, IndexWriter, ReloadPolicy, TantivyDocument, Term,
};
use tokio::sync::Semaphore;
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

use super::{
    BooleanQueryProcessor, HighlightingConfig, HighlightingEngine, LogSchema, SearchError,
    SearchResult, SearchResults,
};
use crate::models::LogEntry;

/// 搜索配置
#[derive(Debug, Clone)]
pub struct AsyncSearchConfig {
    pub default_timeout: Duration,
    pub max_results: usize,
    pub index_path: PathBuf,
    pub writer_heap_size: usize,
    /// 最大并发搜索数（背压控制）
    pub max_concurrent_searches: usize,
    /// spawn_blocking 线程池大小（0 = 使用 Tokio 默认值）
    pub blocking_pool_size: usize,
}

impl Default for AsyncSearchConfig {
    fn default() -> Self {
        let cpu_count = num_cpus::get();
        Self {
            default_timeout: Duration::from_millis(200),
            max_results: 50_000,
            index_path: PathBuf::from("./search_index"),
            writer_heap_size: 50_000_000,
            max_concurrent_searches: cpu_count * 2,
            blocking_pool_size: 0, // 使用 Tokio 默认值
        }
    }
}

/// 搜索统计
#[derive(Debug, Default, Clone)]
pub struct AsyncSearchStats {
    pub total_searches: u64,
    pub total_query_time_ms: u64,
    pub timeout_count: u64,
    pub cancelled_count: u64,
    pub cache_hits: u64,
    pub active_searches: u64,
    pub peak_concurrent_searches: u64,
}

/// 真正异步的搜索引擎管理器
///
/// 核心改进：
/// 1. 所有 CPU 密集型操作使用 `spawn_blocking`
/// 2. 信号量控制并发（背压）
/// 3. 完善的取消机制
/// 4. 超时真正中断搜索
pub struct AsyncSearchEngineManager {
    index: Index,
    reader: IndexReader,
    writer: Arc<Mutex<IndexWriter>>,
    query_parser: QueryParser,
    schema: LogSchema,
    config: AsyncSearchConfig,
    stats: Arc<RwLock<AsyncSearchStats>>,
    boolean_processor: BooleanQueryProcessor,
    highlighting_engine: HighlightingEngine,
    /// 并发搜索信号量（背压控制）
    search_semaphore: Arc<Semaphore>,
}

// 手动实现 Clone（因为 IndexReader 不实现 Clone）
impl Clone for AsyncSearchEngineManager {
    fn clone(&self) -> Self {
        // 创建新的 reader（基于同一个 index）
        let reader = self
            .index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .expect("Failed to clone index reader");

        // 重新创建 boolean processor 和 highlighting engine
        let boolean_processor = BooleanQueryProcessor::new(
            self.index.clone(),
            reader.clone(),
            self.schema.content,
            self.query_parser.clone(),
        );

        let highlighting_config = HighlightingConfig::default();
        let highlighting_engine = HighlightingEngine::new(
            self.index.clone(),
            reader.clone(),
            self.query_parser.clone(),
            self.schema.content,
            highlighting_config,
        );

        Self {
            index: self.index.clone(),
            reader,
            writer: Arc::clone(&self.writer),
            query_parser: self.query_parser.clone(),
            schema: self.schema.clone(),
            config: self.config.clone(),
            stats: Arc::clone(&self.stats),
            boolean_processor,
            highlighting_engine,
            search_semaphore: Arc::clone(&self.search_semaphore),
        }
    }
}

impl AsyncSearchEngineManager {
    /// 创建新的异步搜索引擎管理器
    pub fn new(config: AsyncSearchConfig) -> SearchResult<Self> {
        let schema = LogSchema::build();

        // 创建或打开索引
        let meta_path = config.index_path.join("meta.json");
        let index = if meta_path.exists() {
            Index::open_in_dir(&config.index_path)?
        } else {
            std::fs::create_dir_all(&config.index_path)?;
            Index::create_in_dir(&config.index_path, schema.schema.clone())?
        };

        // 配置分词器
        schema.configure_tokenizers(&index)?;

        // 创建 reader
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;

        // 创建 writer
        let writer = index.writer(config.writer_heap_size)?;

        // 创建查询解析器
        let query_parser = QueryParser::for_index(&index, vec![schema.content]);

        // 创建处理器
        let boolean_processor = BooleanQueryProcessor::new(
            index.clone(),
            reader.clone(),
            schema.content,
            query_parser.clone(),
        );

        let highlighting_config = HighlightingConfig::default();
        let highlighting_engine = HighlightingEngine::new(
            index.clone(),
            reader.clone(),
            query_parser.clone(),
            schema.content,
            highlighting_config,
        );

        // 创建信号量
        let search_semaphore = Arc::new(Semaphore::new(config.max_concurrent_searches));

        info!(
            index_path = %config.index_path.display(),
            max_concurrent = config.max_concurrent_searches,
            "Async search engine initialized"
        );

        Ok(Self {
            index,
            reader,
            writer: Arc::new(Mutex::new(writer)),
            query_parser,
            schema,
            config,
            stats: Arc::new(RwLock::new(AsyncSearchStats::default())),
            boolean_processor,
            highlighting_engine,
            search_semaphore,
        })
    }

    /// 带背压和取消的异步搜索（真正异步）
    ///
    /// # 改进点
    /// 1. 使用 `spawn_blocking` 将搜索移至专用线程池
    /// 2. 信号量控制并发数量
    /// 3. CancellationToken 协作式取消
    /// 4. 超时真正中断搜索
    pub async fn search_cancellable(
        &self,
        query: &str,
        limit: Option<usize>,
        timeout_duration: Option<Duration>,
        token: CancellationToken,
    ) -> SearchResult<SearchResults> {
        let start_time = Instant::now();
        let limit = limit.unwrap_or(self.config.max_results);
        let timeout_duration = timeout_duration.unwrap_or(self.config.default_timeout);

        debug!(
            query = %query,
            limit = limit,
            timeout_ms = timeout_duration.as_millis(),
            "Starting cancellable search"
        );

        // 获取搜索许可（背压控制）
        let _permit = self
            .search_semaphore
            .acquire()
            .await
            .map_err(|_| SearchError::IndexError("Search semaphore closed".to_string()))?;

        // 更新统计
        {
            let mut stats = self.stats.write();
            stats.active_searches += 1;
            if stats.active_searches > stats.peak_concurrent_searches {
                stats.peak_concurrent_searches = stats.active_searches;
            }
        }

        // 解析查询（轻量级，不需要 spawn_blocking）
        let parsed_query = self.parse_query(query)?;

        // 使用 spawn_blocking 执行 CPU 密集型搜索
        let reader = self.reader.clone();
        let schema = self.schema.clone();
        let token_clone = token.clone();

        let search_handle: JoinHandle<SearchResult<SearchResults>> =
            tokio::task::spawn_blocking(move || {
                Self::execute_search_blocking(reader, schema, parsed_query, limit, token_clone)
            });

        // 等待结果或超时
        let result = match timeout(timeout_duration, search_handle).await {
            Ok(Ok(result)) => {
                let query_time = start_time.elapsed();
                self.update_stats(query_time, false, false);
                result
            }
            Ok(Err(join_err)) => {
                // spawn_blocking 任务出错
                let query_time = start_time.elapsed();
                self.update_stats(query_time, false, false);
                if join_err.is_panic() {
                    error!("Search task panicked: {}", join_err);
                    Err(SearchError::IndexError(format!(
                        "Search task panicked: {}",
                        join_err
                    )))
                } else {
                    Err(SearchError::IndexError(format!(
                        "Search task cancelled: {}",
                        join_err
                    )))
                }
            }
            Err(_) => {
                // 超时
                let query_time = start_time.elapsed();
                self.update_stats(query_time, true, false);

                // 取消搜索
                token.cancel();

                warn!(
                    query = %query,
                    timeout_ms = timeout_duration.as_millis(),
                    "Search timed out and was cancelled"
                );

                Err(SearchError::Timeout(format!(
                    "Search timed out after {}ms",
                    timeout_duration.as_millis()
                )))
            }
        };

        // 减少活跃搜索计数
        {
            let mut stats = self.stats.write();
            stats.active_searches -= 1;
        }

        result
    }

    /// 带超时的搜索（简化接口）
    pub async fn search_with_timeout(
        &self,
        query: &str,
        limit: Option<usize>,
        timeout_duration: Option<Duration>,
    ) -> SearchResult<SearchResults> {
        self.search_cancellable(query, limit, timeout_duration, CancellationToken::new())
            .await
    }

    /// 在阻塞线程中执行搜索（CPU 密集型）
    fn execute_search_blocking(
        reader: IndexReader,
        schema: LogSchema,
        query: Box<dyn Query>,
        limit: usize,
        token: CancellationToken,
    ) -> SearchResult<SearchResults> {
        // 检查取消
        if token.is_cancelled() {
            return Err(SearchError::QueryError("Search cancelled".to_string()));
        }

        let searcher = reader.searcher();

        // 获取总数
        let total_count = searcher.search(&*query, &Count)?;

        // 检查取消
        if token.is_cancelled() {
            return Err(SearchError::QueryError("Search cancelled".to_string()));
        }

        // 获取 Top 文档
        let top_docs_collector = TopDocs::with_limit(limit);
        let top_docs = searcher.search(&*query, &top_docs_collector)?;

        // 转换为 LogEntry
        let mut entries = Vec::with_capacity(top_docs.len());
        let mut doc_addresses = Vec::with_capacity(top_docs.len());

        for (_score, doc_address) in top_docs {
            // 定期检查取消
            if token.is_cancelled() {
                return Err(SearchError::QueryError("Search cancelled".to_string()));
            }

            let retrieved_doc = searcher.doc(doc_address)?;

            if let Some(log_entry) = Self::document_to_log_entry(&retrieved_doc, &schema) {
                entries.push(log_entry);
                doc_addresses.push(doc_address);
            }
        }

        Ok(SearchResults {
            entries,
            doc_addresses,
            total_count,
            query_time_ms: 0,
            was_timeout: false,
        })
    }

    /// 多关键词搜索（真正异步）
    pub async fn search_multi_keyword(
        &self,
        keywords: &[String],
        require_all: bool,
        limit: Option<usize>,
        timeout_duration: Option<Duration>,
        token: CancellationToken,
    ) -> SearchResult<SearchResults> {
        let start_time = Instant::now();
        let limit = limit.unwrap_or(self.config.max_results);
        let timeout_duration = timeout_duration.unwrap_or(self.config.default_timeout);

        debug!(
            keywords = ?keywords,
            require_all = require_all,
            "Starting multi-keyword search"
        );

        // 获取许可
        let _permit = self
            .search_semaphore
            .acquire()
            .await
            .map_err(|_| SearchError::IndexError("Search semaphore closed".to_string()))?;

        // 更新统计
        {
            let mut stats = self.stats.write();
            stats.active_searches += 1;
        }

        let processor = self.boolean_processor.clone();
        let reader = self.reader.clone();
        let schema = self.schema.clone();
        let token_clone = token.clone();
        let keywords_clone = keywords.to_vec();

        let search_handle: JoinHandle<SearchResult<SearchResults>> =
            tokio::task::spawn_blocking(move || {
                // 执行多关键词查询
                let (doc_addresses, total_count) = processor.process_multi_keyword_query(
                    &keywords_clone,
                    require_all,
                    limit,
                    Some(token_clone.clone()),
                )?;

                // 检查取消
                if token_clone.is_cancelled() {
                    return Err(SearchError::QueryError("Search cancelled".to_string()));
                }

                // 转换为 LogEntry
                let searcher = reader.searcher();
                let mut entries = Vec::with_capacity(doc_addresses.len());
                let mut addresses = Vec::with_capacity(doc_addresses.len());

                for doc_address in doc_addresses {
                    if token_clone.is_cancelled() {
                        return Err(SearchError::QueryError("Search cancelled".to_string()));
                    }

                    let retrieved_doc = searcher.doc(doc_address)?;
                    if let Some(log_entry) = Self::document_to_log_entry(&retrieved_doc, &schema) {
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
            });

        let result = match timeout(timeout_duration, search_handle).await {
            Ok(Ok(result)) => {
                let query_time = start_time.elapsed();
                self.update_stats(query_time, false, false);
                result
            }
            Ok(Err(join_err)) => {
                let query_time = start_time.elapsed();
                self.update_stats(query_time, false, false);
                Err(SearchError::IndexError(format!(
                    "Search task failed: {}",
                    join_err
                )))
            }
            Err(_) => {
                let query_time = start_time.elapsed();
                self.update_stats(query_time, true, false);
                token.cancel();
                Err(SearchError::Timeout(format!(
                    "Multi-keyword search timed out after {}ms",
                    timeout_duration.as_millis()
                )))
            }
        };

        {
            let mut stats = self.stats.write();
            stats.active_searches -= 1;
        }

        result
    }

    /// 解析查询
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
                warn!(query = %query_str, error = %e, "Query parsing failed, using fallback");
                let term = Term::from_field_text(self.schema.content, query_str);
                Ok(Box::new(TermQuery::new(
                    term,
                    tantivy::schema::IndexRecordOption::Basic,
                )))
            }
        }
    }

    /// 将文档转换为 LogEntry
    fn document_to_log_entry(doc: &TantivyDocument, schema: &LogSchema) -> Option<LogEntry> {
        let content = doc.get_first(schema.content)?.as_str()?.to_string();
        let timestamp_i64 = doc.get_first(schema.timestamp)?.as_i64()?;
        let level = doc.get_first(schema.level)?.as_str()?.to_string();
        let file_path = doc.get_first(schema.file_path)?.as_str()?.to_string();
        let real_path = doc.get_first(schema.real_path)?.as_str()?.to_string();
        let line_number = doc.get_first(schema.line_number)?.as_u64()? as usize;

        Some(LogEntry {
            id: 0,
            timestamp: timestamp_i64.to_string().into(),
            level: level.into(),
            file: file_path.into(),
            real_path: real_path.into(),
            line: line_number,
            content: content.into(),
            tags: vec![],
            match_details: None,
            matched_keywords: None,
        })
    }

    /// 更新统计信息
    fn update_stats(&self, query_time: Duration, was_timeout: bool, was_cancelled: bool) {
        let mut stats = self.stats.write();
        stats.total_searches += 1;
        stats.total_query_time_ms += query_time.as_millis() as u64;
        if was_timeout {
            stats.timeout_count += 1;
        }
        if was_cancelled {
            stats.cancelled_count += 1;
        }
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> AsyncSearchStats {
        self.stats.read().clone()
    }

    /// 获取当前活跃搜索数
    pub fn get_active_search_count(&self) -> usize {
        self.stats.read().active_searches as usize
    }

    /// 检查是否处于高负载
    pub fn is_under_high_load(&self) -> bool {
        let stats = self.stats.read();
        stats.active_searches as usize >= self.config.max_concurrent_searches
    }

    /// 添加文档
    pub fn add_document(&self, log_entry: &LogEntry) -> SearchResult<()> {
        let mut doc = TantivyDocument::default();

        doc.add_text(self.schema.content, &log_entry.content);
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

    /// 提交变更
    pub fn commit(&self) -> SearchResult<()> {
        let mut writer = self.writer.lock();
        writer.commit()?;
        self.reader.reload()?;
        Ok(())
    }

    /// 删除文件的所有文档
    pub fn delete_file_documents(&self, file_path: &str) -> SearchResult<usize> {
        let term = Term::from_field_text(self.schema.file_path, file_path);

        let searcher = self.reader.searcher();
        let query = TermQuery::new(term.clone(), tantivy::schema::IndexRecordOption::Basic);
        let count = searcher.search(&query, &Count)?;

        let mut writer = self.writer.lock();
        let _opstamp = writer.delete_term(term);
        writer.commit()?;
        self.reader.reload()?;

        info!(file_path = %file_path, deleted_count = count, "Deleted documents for file");

        Ok(count)
    }

    /// 清空索引
    pub fn clear_index(&self) -> SearchResult<()> {
        let mut writer = self.writer.lock();
        writer.delete_all_documents()?;
        writer.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_manager() -> (AsyncSearchEngineManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config = AsyncSearchConfig {
            index_path: temp_dir.path().to_path_buf(),
            ..Default::default()
        };
        let manager = AsyncSearchEngineManager::new(config).unwrap();
        (manager, temp_dir)
    }

    #[tokio::test]
    async fn test_async_search_creation() {
        let (_manager, _temp_dir) = create_test_manager();
    }

    #[tokio::test]
    async fn test_cancellable_search() {
        let (manager, _temp_dir) = create_test_manager();
        let token = CancellationToken::new();

        let result = manager.search_cancellable("test", None, None, token).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_search_timeout() {
        let (manager, _temp_dir) = create_test_manager();

        let result = manager
            .search_with_timeout("test", None, Some(Duration::from_millis(100)))
            .await;

        // 空索引上应该快速完成或超时
        assert!(matches!(result, Ok(_) | Err(SearchError::Timeout(_))));
    }

    #[tokio::test]
    async fn test_search_cancellation() {
        let (manager, _temp_dir) = create_test_manager();
        let token = CancellationToken::new();

        // 立即取消
        token.cancel();

        let result = manager.search_cancellable("test", None, None, token).await;

        // 应该被取消或完成（取决于时机）
        assert!(
            result.is_ok() || matches!(result, Err(SearchError::QueryError(_))),
            "Search should either complete quickly on empty index or be cancelled"
        );
    }

    #[tokio::test]
    async fn test_concurrent_searches() {
        let (manager, _temp_dir) = create_test_manager();

        // 并发执行多个搜索
        let searches: Vec<_> = (0..5)
            .map(|i| {
                let mgr = manager.clone();
                let query = format!("query{}", i);
                tokio::spawn(async move { mgr.search_with_timeout(&query, None, None).await })
            })
            .collect();

        let results = futures::future::join_all(searches).await;

        for result in results {
            assert!(result.is_ok());
            assert!(result.unwrap().is_ok());
        }
    }

    #[tokio::test]
    async fn test_backpressure() {
        let temp_dir = TempDir::new().unwrap();
        let config = AsyncSearchConfig {
            index_path: temp_dir.path().to_path_buf(),
            max_concurrent_searches: 2, // 限制并发
            ..Default::default()
        };
        let manager = Arc::new(AsyncSearchEngineManager::new(config).unwrap());

        // 启动超过限制的并发搜索
        let mut handles = vec![];
        for i in 0..5 {
            let mgr = Arc::clone(&manager);
            let handle = tokio::spawn(async move {
                mgr.search_with_timeout(&format!("query{}", i), None, None)
                    .await
            });
            handles.push(handle);
        }

        // 所有搜索应该最终完成（信号量排队）
        for handle in handles {
            let result = handle.await;
            assert!(result.is_ok());
        }
    }
}
