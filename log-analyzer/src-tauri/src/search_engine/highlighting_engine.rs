//! Search Result Highlighting Engine
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
    query::{Query, QueryParser},
    schema::Field,
    DocAddress, Index, IndexReader, Snippet, SnippetGenerator, Term,
};
use tracing::{debug, info, warn};

use super::{SearchError, SearchResult};

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
}

/// Efficient search result highlighting engine
pub struct HighlightingEngine {
    _index: Index,
    reader: IndexReader,
    query_parser: QueryParser,
    content_field: Field,
    config: HighlightingConfig,
    snippet_cache: Arc<RwLock<LruCache<SnippetCacheKey, CachedSnippet>>>,
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

        info!(
            cache_size = config.cache_size,
            max_snippet_length = config.max_snippet_length,
            context_size = config.context_size,
            "Highlighting engine initialized"
        );

        Self {
            _index: index,
            reader,
            query_parser,
            content_field,
            config,
            snippet_cache,
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

        // Create cache key
        let query_hash = self.calculate_query_hash(query);
        let cache_key = SnippetCacheKey {
            doc_id: format!("{:?}", doc_address), // Simple doc ID representation
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
        let snippets =
            self.generate_snippets_with_tantivy(&parsed_query, document_content, doc_address)?;

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

    /// Parse query for highlighting purposes
    fn parse_query_for_highlighting(&self, query_str: &str) -> SearchResult<Box<dyn Query>> {
        match self.query_parser.parse_query(query_str) {
            Ok(query) => Ok(query),
            Err(e) => {
                warn!(query = %query_str, error = %e, "Query parsing failed for highlighting");

                // Fallback: create simple term queries for each word
                let terms: Vec<String> = query_str
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect();

                if terms.is_empty() {
                    return Err(SearchError::QueryError(
                        "Empty query for highlighting".to_string(),
                    ));
                }

                // For simplicity, use the first term
                let term = Term::from_field_text(self.content_field, &terms[0]);
                Ok(Box::new(tantivy::query::TermQuery::new(
                    term,
                    tantivy::schema::IndexRecordOption::Basic,
                )))
            }
        }
    }

    /// Generate snippets using Tantivy's snippet generator
    fn generate_snippets_with_tantivy(
        &self,
        query: &Box<dyn Query>,
        document_content: &str,
        _doc_address: DocAddress,
    ) -> SearchResult<Vec<Snippet>> {
        let searcher = self.reader.searcher();

        // Create snippet generator
        let mut snippet_generator = SnippetGenerator::create(&searcher, query, self.content_field)?;

        // Configure snippet generator
        snippet_generator.set_max_num_chars(self.config.max_snippet_length);

        // For now, create a simple snippet from the document content
        // In a real implementation, we'd use the actual document from the index
        let snippet = snippet_generator.snippet(document_content);

        Ok(vec![snippet])
    }

    /// Apply HTML-safe highlighting to snippets
    fn apply_html_highlighting(&self, snippets: &[Snippet]) -> SearchResult<Vec<String>> {
        let mut highlighted_snippets = Vec::new();

        for snippet in snippets {
            // Use Tantivy's built-in HTML generation, then escape it properly
            let html_snippet = snippet.to_html();
            let escaped_snippet = self.escape_html(&html_snippet);
            highlighted_snippets.push(escaped_snippet);
        }

        // Update HTML escape statistics
        {
            let mut stats = self.stats.write();
            stats.html_escapes_performed += snippets.len() as u64;
        }

        Ok(highlighted_snippets)
    }

    /// Escape HTML characters for safe display
    fn escape_html(&self, text: &str) -> String {
        text.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#x27;")
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
            if cached_snippet.created_at.elapsed().unwrap_or(Duration::MAX) < self.config.cache_ttl
            {
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
        let optimized_content = if document_content.len() > max_content_length {
            self.extract_relevant_content(document_content, query, max_content_length)
        } else {
            document_content.to_string()
        };

        // Use regular highlighting on optimized content
        let result = self.highlight_document(doc_address, query, &optimized_content);

        // Update large document optimization stats
        if document_content.len() > max_content_length {
            self.update_highlighting_stats(start_time.elapsed(), true);
        }

        result
    }

    /// Extract relevant content around potential matches for large documents
    fn extract_relevant_content(&self, content: &str, query: &str, max_length: usize) -> String {
        let query_terms: Vec<&str> = query.split_whitespace().collect();

        if query_terms.is_empty() {
            return content.chars().take(max_length).collect();
        }

        // Find first occurrence of any query term
        let mut best_start = 0;
        for term in &query_terms {
            if let Some(pos) = content.to_lowercase().find(&term.to_lowercase()) {
                // Start extraction some characters before the match
                let start = pos.saturating_sub(self.config.context_size);
                if start < best_start || best_start == 0 {
                    best_start = start;
                }
                break;
            }
        }

        // Extract content around the match
        let end = (best_start + max_length).min(content.len());
        content
            .chars()
            .skip(best_start)
            .take(end - best_start)
            .collect()
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
}
