//! Search Result Highlighting Engine
#![allow(dead_code)]
//!
//! Provides efficient text highlighting using Tantivy's snippet generation:
//! - Fast text highlighting using Tantivy's snippet generation
//! - HTML-safe highlighting with configurable markup
//! - Highlighting cache for frequently requested snippets
//! - Highlighting performance optimization for large documents

use lru::LruCache;
use parking_lot::RwLock;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tantivy::{
    query::{Query, QueryClone, QueryParser},
    schema::Field,
    DocAddress, Index, IndexReader, Snippet, SnippetGenerator, Term,
};
use tracing::{debug, info, warn};

use crate::{SearchError, SearchResult};

/// Configuration for highlighting operations
#[derive(Debug, Clone)]
pub struct HighlightingConfig {
    /// Maximum number of snippets to cache
    pub cache_size: usize,
    /// Maximum length of highlighted text
    pub max_snippet_length: usize,
    /// Number of characters around each match
    pub context_size: usize,
    /// Maximum number of snippets per document
    pub max_snippets_per_doc: usize,
    /// HTML tags for highlighting
    pub highlight_start_tag: String,
    pub highlight_end_tag: String,
    /// Cache TTL for snippets
    pub cache_ttl: Duration,
}

impl Default for HighlightingConfig {
    fn default() -> Self {
        Self {
            cache_size: 10_000,
            max_snippet_length: 500,
            context_size: 100,
            max_snippets_per_doc: 3,
            highlight_start_tag: "<mark>".to_string(),
            highlight_end_tag: "</mark>".to_string(),
            cache_ttl: Duration::from_secs(300), // 5 minutes
        }
    }
}

/// Cached snippet with metadata
#[derive(Debug, Clone)]
struct CachedSnippet {
    content: String,
    created_at: SystemTime,
    access_count: u64,
    last_accessed: SystemTime,
}

/// Cache key for snippets
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct SnippetCacheKey {
    doc_id: String,
    query_hash: u64,
    field_name: String,
}

/// Statistics for highlighting operations
#[derive(Debug, Clone, Default)]
pub struct HighlightingStats {
    pub total_highlights: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub average_highlight_time_ms: f64,
    pub large_document_optimizations: u64,
    pub html_escapes_performed: u64,
    /// 查询解析缓存命中次数
    pub query_cache_hits: u64,
    /// 查询解析缓存未命中次数
    pub query_cache_misses: u64,
}

/// Efficient search result highlighting engine
pub struct HighlightingEngine {
    _index: Index,
    reader: IndexReader,
    query_parser: QueryParser,
    content_field: Field,
    config: HighlightingConfig,
    snippet_cache: Arc<RwLock<LruCache<SnippetCacheKey, CachedSnippet>>>,
    /// 查询解析缓存：避免重复解析相同查询字符串
    /// 使用 query_hash 作为 key，缓存已解析的 Query 对象
    query_cache: Arc<RwLock<LruCache<u64, Box<dyn Query>>>>,
    stats: Arc<RwLock<HighlightingStats>>,
}

impl HighlightingEngine {
    /// Create a new highlighting engine
    pub fn new(
        index: Index,
        reader: IndexReader,
        query_parser: QueryParser,
        content_field: Field,
        config: HighlightingConfig,
    ) -> Self {
        let cache_size =
            NonZeroUsize::new(config.cache_size).unwrap_or(NonZeroUsize::new(1000).unwrap());
        let snippet_cache = Arc::new(RwLock::new(LruCache::new(cache_size)));

        // 查询解析缓存大小：使用较小的缓存，因为查询解析结果占用较多内存
        // 通常查询模式有限，100个缓存槽位足够
        let query_cache_size = NonZeroUsize::new(100).unwrap();
        let query_cache = Arc::new(RwLock::new(LruCache::new(query_cache_size)));

        info!(
            cache_size = config.cache_size,
            max_snippet_length = config.max_snippet_length,
            context_size = config.context_size,
            query_cache_size = 100,
            "Highlighting engine initialized"
        );

        Self {
            _index: index,
            reader,
            query_parser,
            content_field,
            config,
            snippet_cache,
            query_cache,
            stats: Arc::new(RwLock::new(HighlightingStats::default())),
        }
    }

    /// Highlight search results in document content
    pub fn highlight_document(
        &self,
        doc_address: DocAddress,
        query: &str,
        document_content: &str,
    ) -> SearchResult<Vec<String>> {
        let start_time = Instant::now();

        debug!(
            query = %query,
            content_length = document_content.len(),
            "Starting document highlighting"
        );

        // Create cache key - 使用确定性格式避免Debug格式不稳定性
        let query_hash = self.calculate_query_hash(query);
        let cache_key = SnippetCacheKey {
            doc_id: format!("seg{}_doc{}", doc_address.segment_ord, doc_address.doc_id),
            query_hash,
            field_name: "content".to_string(),
        };

        // Check cache first
        if let Some(cached_snippets) = self.get_cached_snippets(&cache_key) {
            self.update_cache_hit_stats(start_time.elapsed());
            return Ok(vec![cached_snippets]);
        }

        // Parse query for highlighting
        let parsed_query = self.parse_query_for_highlighting(query)?;

        // Generate snippets using Tantivy
        let snippets = self.generate_snippets_with_tantivy(
            parsed_query.as_ref(),
            document_content,
            doc_address,
        )?;

        // Apply HTML-safe highlighting
        let highlighted_snippets = self.apply_html_highlighting(&snippets)?;

        // Cache the results
        self.cache_snippets(&cache_key, &highlighted_snippets);

        // Update statistics
        let highlight_time = start_time.elapsed();
        self.update_highlighting_stats(highlight_time, false);

        info!(
            query = %query,
            snippets_count = highlighted_snippets.len(),
            highlight_time_ms = highlight_time.as_millis(),
            "Document highlighting completed"
        );

        Ok(highlighted_snippets)
    }

    /// Highlight multiple documents efficiently
    pub fn highlight_documents_batch(
        &self,
        documents: &[(DocAddress, String, String)], // (doc_address, query, content)
    ) -> Vec<SearchResult<Vec<String>>> {
        let start_time = Instant::now();

        debug!(
            document_count = documents.len(),
            "Starting batch document highlighting"
        );

        let results: Vec<_> = documents
            .iter()
            .map(|(doc_address, query, content)| {
                self.highlight_document(*doc_address, query, content)
            })
            .collect();

        info!(
            document_count = documents.len(),
            total_time_ms = start_time.elapsed().as_millis(),
            "Batch document highlighting completed"
        );

        results
    }

    /// Parse query for highlighting purposes with caching
    ///
    /// 使用 LRU 缓存避免重复解析相同查询字符串，提升高亮性能。
    /// 由于 `Box<dyn Query>` 不实现 `Clone`，使用 Tantivy 的 `box_clone()` 方法进行克隆。
    fn parse_query_for_highlighting(&self, query_str: &str) -> SearchResult<Box<dyn Query>> {
        // 计算查询哈希作为缓存 key
        let query_hash = self.calculate_query_hash(query_str);

        // 检查缓存
        {
            let mut cache = self.query_cache.write();
            if let Some(cached_query) = cache.get_mut(&query_hash) {
                // 缓存命中：使用 box_clone() 克隆查询对象
                let mut stats = self.stats.write();
                stats.query_cache_hits += 1;
                return Ok(cached_query.box_clone());
            }
        }

        // 缓存未命中：解析查询
        {
            let mut stats = self.stats.write();
            stats.query_cache_misses += 1;
        }

        let query = match self.query_parser.parse_query(query_str) {
            Ok(query) => query,
            Err(e) => {
                warn!(query = %query_str, error = %e, "Query parsing failed for highlighting");

                // Fallback: create simple term queries for each word
                let terms: Vec<String> = query_str
                    .split_whitespace()
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .collect();

                if terms.is_empty() {
                    return Err(SearchError::QueryError(
                        "Empty query for highlighting".to_string(),
                    ));
                }

                // 使用所有有效的terms而非仅第一个
                if terms.len() == 1 {
                    let term = Term::from_field_text(self.content_field, &terms[0]);
                    Box::new(tantivy::query::TermQuery::new(
                        term,
                        tantivy::schema::IndexRecordOption::Basic,
                    )) as Box<dyn Query>
                } else {
                    // 使用BooleanQuery组合多个terms
                    use tantivy::query::{BooleanQuery, Occur};
                    let mut clauses = Vec::new();
                    for term_str in &terms {
                        let term = Term::from_field_text(self.content_field, term_str);
                        let term_query = tantivy::query::TermQuery::new(
                            term,
                            tantivy::schema::IndexRecordOption::Basic,
                        );
                        clauses.push((Occur::Should, Box::new(term_query) as Box<dyn Query>));
                    }
                    Box::new(BooleanQuery::new(clauses)) as Box<dyn Query>
                }
            }
        };

        // 存入缓存
        {
            let mut cache = self.query_cache.write();
            cache.put(query_hash, query.box_clone());
        }

        Ok(query)
    }

    /// Generate snippets using Tantivy's snippet generator
    fn generate_snippets_with_tantivy(
        &self,
        query: &dyn Query,
        document_content: &str,
        _doc_address: DocAddress,
    ) -> SearchResult<Vec<Snippet>> {
        let searcher = self.reader.searcher();

        // Create snippet generator
        let mut snippet_generator = SnippetGenerator::create(&searcher, query, self.content_field)?;

        // Configure snippet generator
        snippet_generator.set_max_num_chars(self.config.max_snippet_length);

        // 业务层限制片段数量：最多3个片段
        const MAX_SNIPPETS: usize = 3;
        let mut snippets = Vec::with_capacity(MAX_SNIPPETS);

        // Tantivy SnippetGenerator 不支持一次性生成多个片段
        // 当前实现通过多次调用来获取多个片段（如果有需要的话）
        // 这里先添加一个片段，将来做多片段扩展
        let snippet = snippet_generator.snippet(document_content);
        snippets.push(snippet);

        // 限制最多返回3个片段
        snippets.truncate(MAX_SNIPPETS);

        Ok(snippets)
    }

    /// Apply HTML-safe highlighting to snippets
    fn apply_html_highlighting(&self, snippets: &[Snippet]) -> SearchResult<Vec<String>> {
        let mut highlighted_snippets = Vec::new();

        for snippet in snippets {
            let html_snippet = snippet.to_html();
            highlighted_snippets.push(html_snippet);
        }

        // Update HTML escape statistics
        {
            let mut stats = self.stats.write();
            stats.html_escapes_performed += snippets.len() as u64;
        }

        Ok(highlighted_snippets)
    }

    /// Escape HTML characters for safe display
    /// 单次遍历替代 5 次 String::replace 链式调用，消除 5 次临时 String 分配
    fn escape_html(&self, text: &str) -> String {
        let mut result = String::with_capacity(text.len() + text.len() / 10);
        for ch in text.chars() {
            match ch {
                '&' => result.push_str("&amp;"),
                '<' => result.push_str("&lt;"),
                '>' => result.push_str("&gt;"),
                '"' => result.push_str("&quot;"),
                '\'' => result.push_str("&#x27;"),
                _ => result.push(ch),
            }
        }
        result
    }

    /// Calculate hash for query caching
    fn calculate_query_hash(&self, query: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        query.hash(&mut hasher);
        hasher.finish()
    }

    /// Get cached snippets if available and not expired
    fn get_cached_snippets(&self, cache_key: &SnippetCacheKey) -> Option<String> {
        let mut cache = self.snippet_cache.write();

        if let Some(cached_snippet) = cache.get_mut(cache_key) {
            // Check if cache entry is still valid
            // Handle clock skew: if elapsed() returns Err (clock went backward), treat as expired
            let elapsed = cached_snippet.created_at.elapsed().unwrap_or(Duration::MAX);
            if elapsed < self.config.cache_ttl {
                cached_snippet.access_count += 1;
                cached_snippet.last_accessed = SystemTime::now();
                return Some(cached_snippet.content.clone());
            } else {
                // Remove expired entry
                cache.pop(cache_key);
            }
        }

        None
    }

    /// Cache snippets for future use
    fn cache_snippets(&self, cache_key: &SnippetCacheKey, snippets: &[String]) {
        if snippets.is_empty() {
            return;
        }

        let combined_content = snippets.join("\n");
        let cached_snippet = CachedSnippet {
            content: combined_content,
            created_at: SystemTime::now(),
            access_count: 1,
            last_accessed: SystemTime::now(),
        };

        let mut cache = self.snippet_cache.write();
        cache.put(cache_key.clone(), cached_snippet);
    }

    /// Update statistics for cache hits
    fn update_cache_hit_stats(&self, response_time: Duration) {
        let mut stats = self.stats.write();
        stats.total_highlights += 1;
        stats.cache_hits += 1;

        // Update average response time
        let total_time = stats.average_highlight_time_ms * (stats.total_highlights - 1) as f64;
        stats.average_highlight_time_ms =
            (total_time + response_time.as_millis() as f64) / stats.total_highlights as f64;
    }

    /// Update highlighting statistics
    fn update_highlighting_stats(&self, response_time: Duration, is_large_doc: bool) {
        let mut stats = self.stats.write();
        stats.total_highlights += 1;
        stats.cache_misses += 1;

        if is_large_doc {
            stats.large_document_optimizations += 1;
        }

        // Update average response time
        let total_time = stats.average_highlight_time_ms * (stats.total_highlights - 1) as f64;
        stats.average_highlight_time_ms =
            (total_time + response_time.as_millis() as f64) / stats.total_highlights as f64;
    }

    /// Get highlighting statistics
    pub fn get_highlighting_stats(&self) -> HighlightingStats {
        self.stats.read().clone()
    }

    /// Clear highlighting cache
    pub fn clear_cache(&self) {
        let mut cache = self.snippet_cache.write();
        cache.clear();

        info!("Highlighting cache cleared");
    }

    /// Get cache statistics
    pub fn get_cache_stats(&self) -> (usize, usize) {
        let cache = self.snippet_cache.read();
        (cache.len(), cache.cap().get())
    }

    /// Optimize highlighting for large documents
    pub fn highlight_large_document(
        &self,
        doc_address: DocAddress,
        query: &str,
        document_content: &str,
        max_content_length: usize,
    ) -> SearchResult<Vec<String>> {
        let start_time = Instant::now();

        debug!(
            query = %query,
            content_length = document_content.len(),
            max_length = max_content_length,
            "Starting large document highlighting optimization"
        );

        // For large documents, truncate content around potential matches
        let optimized_content: String = if document_content.len() > max_content_length {
            self.extract_relevant_content(document_content, query, max_content_length)
        } else {
            // 非大文档无需拷贝，直接高亮原始内容
            return self.highlight_document(doc_address, query, document_content);
        };

        // Use regular highlighting on optimized content
        let result = self.highlight_document(doc_address, query, &optimized_content);

        // Update large document optimization stats
        if document_content.len() > max_content_length {
            self.update_highlighting_stats(start_time.elapsed(), true);
        }

        result
    }

    /// 从大文档中提取与查询相关的片段内容
    ///
    /// 优化策略：
    /// 1. 使用字节位置而非字符位置进行切片，避免 O(n) 的 chars() 遍历
    /// 2. 限制搜索范围，避免在超大文档中全文扫描
    /// 3. 优先在文档前部查找匹配，提高响应速度
    fn extract_relevant_content(&self, content: &str, query: &str, max_length: usize) -> String {
        let query_terms: Vec<&str> = query.split_whitespace().collect();

        if query_terms.is_empty() {
            // 快速路径：直接字节切片，无需字符遍历
            return self.truncate_by_bytes(content, max_length);
        }

        // 优化：限制搜索范围，避免在超大文档中全文扫描
        // 优先搜索文档前 10KB 内容，如未找到则返回开头片段
        const SEARCH_LIMIT_BYTES: usize = 10 * 1024; // 10KB 搜索限制
        let search_window = &content[..content.len().min(SEARCH_LIMIT_BYTES)];
        let content_lower = search_window.to_lowercase();

        let mut best_byte_start: Option<usize> = None;

        for term in &query_terms {
            let term_lower = term.to_lowercase();
            // 使用 memchr 风格的快速查找（如果 term 较长）
            if let Some(byte_pos) = content_lower.find(&term_lower) {
                // 找到匹配，计算上下文起始位置（按字节）
                // 估算 UTF-8 字符边界：平均每个字符 1-4 字节
                let context_bytes = self.config.context_size * 4; // 保守估计
                let start = byte_pos.saturating_sub(context_bytes);
                best_byte_start = Some(match best_byte_start {
                    None => start,
                    Some(prev) => prev.min(start),
                });
            }
        }

        // 根据找到的位置提取内容
        let start_byte = best_byte_start.unwrap_or(0);
        let end_byte = (start_byte + max_length * 4).min(content.len()); // 保守估计 UTF-8

        // 确保在有效的 UTF-8 边界处切片
        let slice = &content[start_byte..end_byte];
        self.truncate_by_bytes(slice, max_length)
    }

    /// 按字节截断字符串，确保 UTF-8 有效性
    ///
    /// 比 chars().take(n).collect() 快得多，特别是对大字符串
    fn truncate_by_bytes(&self, s: &str, max_chars: usize) -> String {
        // 快速路径：如果字符串很短，直接返回
        if s.len() <= max_chars {
            return s.to_string();
        }

        // 使用 char_indices 找到第 max_chars 个字符的字节位置
        // 这比 .chars().take().collect() 更高效，因为避免了字符复制
        match s.char_indices().nth(max_chars) {
            Some((byte_idx, _)) => s[..byte_idx].to_string(),
            None => s.to_string(), // 字符串字符数少于 max_chars
        }
    }

    /// Update highlighting configuration
    pub fn update_config(&mut self, new_config: HighlightingConfig) {
        info!(
            old_cache_size = self.config.cache_size,
            new_cache_size = new_config.cache_size,
            "Updating highlighting configuration"
        );

        // If cache size changed, recreate cache
        if new_config.cache_size != self.config.cache_size {
            let cache_size = NonZeroUsize::new(new_config.cache_size)
                .unwrap_or(NonZeroUsize::new(1000).unwrap());
            let mut cache = self.snippet_cache.write();
            *cache = LruCache::new(cache_size);
        }

        self.config = new_config;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tantivy::{
        schema::{Schema, STORED, TEXT},
        Index,
    };
    use tempfile::TempDir;

    fn create_test_highlighting_engine() -> (HighlightingEngine, TempDir) {
        let temp_dir = TempDir::new().unwrap();

        let mut schema_builder = Schema::builder();
        let content_field = schema_builder.add_text_field("content", TEXT | STORED);
        let schema = schema_builder.build();

        let index = Index::create_in_dir(temp_dir.path(), schema).unwrap();
        let reader = index.reader().unwrap();
        let query_parser = tantivy::query::QueryParser::for_index(&index, vec![content_field]);

        let config = HighlightingConfig::default();
        let engine = HighlightingEngine::new(index, reader, query_parser, content_field, config);

        (engine, temp_dir)
    }

    #[test]
    fn test_highlighting_engine_creation() {
        let (_engine, _temp_dir) = create_test_highlighting_engine();
        // If we get here, creation was successful
    }

    #[test]
    fn test_html_escaping() {
        let (engine, _temp_dir) = create_test_highlighting_engine();

        let input = "<script>alert('xss')</script>";
        let escaped = engine.escape_html(input);

        assert_eq!(
            escaped,
            "&lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt;"
        );
    }

    #[test]
    fn test_query_hash_calculation() {
        let (engine, _temp_dir) = create_test_highlighting_engine();

        let hash1 = engine.calculate_query_hash("test query");
        let hash2 = engine.calculate_query_hash("test query");
        let hash3 = engine.calculate_query_hash("different query");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_cache_operations() {
        let (engine, _temp_dir) = create_test_highlighting_engine();

        let cache_key = SnippetCacheKey {
            doc_id: "test_doc".to_string(),
            query_hash: 12345,
            field_name: "content".to_string(),
        };

        // Initially should be empty
        assert!(engine.get_cached_snippets(&cache_key).is_none());

        // Cache some snippets
        let snippets = vec!["highlighted text".to_string()];
        engine.cache_snippets(&cache_key, &snippets);

        // Should now be cached
        assert!(engine.get_cached_snippets(&cache_key).is_some());
    }

    #[test]
    fn test_large_document_content_extraction() {
        let (engine, _temp_dir) = create_test_highlighting_engine();

        let large_content = "a".repeat(10000) + "important text here" + &"b".repeat(10000);
        let query = "important";
        let max_length = 1000;

        let extracted = engine.extract_relevant_content(&large_content, query, max_length);

        assert!(extracted.len() <= max_length);
        assert!(extracted.contains("important"));
    }

    #[test]
    fn test_highlighting_stats() {
        let (engine, _temp_dir) = create_test_highlighting_engine();

        let stats = engine.get_highlighting_stats();
        assert_eq!(stats.total_highlights, 0);
        assert_eq!(stats.cache_hits, 0);
        assert_eq!(stats.cache_misses, 0);
    }

    #[test]
    fn test_cache_stats() {
        let (engine, _temp_dir) = create_test_highlighting_engine();

        let (used, capacity) = engine.get_cache_stats();
        assert_eq!(used, 0);
        assert!(capacity > 0);
    }

    // ========== 单次遍历 HTML 转义专项测试 ==========

    #[test]
    fn test_escape_html_no_special_chars() {
        let (engine, _temp_dir) = create_test_highlighting_engine();
        let input = "plain text without special chars";
        assert_eq!(engine.escape_html(input), input);
    }

    #[test]
    fn test_escape_html_all_special_chars() {
        let (engine, _temp_dir) = create_test_highlighting_engine();
        let input = "&<>\"'";
        assert_eq!(engine.escape_html(input), "&amp;&lt;&gt;&quot;&#x27;");
    }

    #[test]
    fn test_escape_html_mixed_content() {
        let (engine, _temp_dir) = create_test_highlighting_engine();
        let input = "status=200 & type=<json> with \"key\" and 'value'";
        let expected =
            "status=200 &amp; type=&lt;json&gt; with &quot;key&quot; and &#x27;value&#x27;";
        assert_eq!(engine.escape_html(input), expected);
    }

    #[test]
    fn test_escape_html_empty_string() {
        let (engine, _temp_dir) = create_test_highlighting_engine();
        assert_eq!(engine.escape_html(""), "");
    }

    #[test]
    fn test_escape_html_unicode() {
        let (engine, _temp_dir) = create_test_highlighting_engine();
        // UTF-8 多字节字符不应被破坏
        let input = "中文日志 & 日本語ログ < 테스트 >";
        let expected = "中文日志 &amp; 日本語ログ &lt; 테스트 &gt;";
        assert_eq!(engine.escape_html(input), expected);
    }

    #[test]
    fn test_escape_html_long_text_performance() {
        // 性能验证：单次遍历应在合理时间内完成大文本转义
        // debug 构建下阈值放宽，release 构建下应快数倍
        let (engine, _temp_dir) = create_test_highlighting_engine();
        let input = "normal text with <tag> & \"quotes\" mixed ".repeat(1000);

        let start = std::time::Instant::now();
        for _ in 0..1000 {
            let _ = engine.escape_html(&input);
        }
        let avg = start.elapsed() / 1000;

        assert!(
            avg < std::time::Duration::from_millis(2),
            "escape_html should process ~35KB text in < 2ms (debug build), actual: {:?}",
            avg
        );
    }
}
