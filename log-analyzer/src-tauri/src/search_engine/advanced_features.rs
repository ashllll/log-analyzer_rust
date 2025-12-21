//! Advanced Search Features
//!
//! Implements high-performance search features:
//! - Bitmap indexing using RoaringBitmap for efficient filtering
//! - Regex search engine with compilation caching
//! - Time-partitioned indexes for efficient temporal queries
//! - Prefix tree-based autocomplete with <100ms response time

use lru::LruCache;
use parking_lot::{Mutex, RwLock};
use regex::Regex;
use roaring::RoaringBitmap;
use std::collections::{BTreeMap, HashMap};
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tracing::{debug, warn};

use super::{SearchError, SearchResult};
use crate::models::LogEntry;

/// Filter types for bitmap indexing
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Filter {
    Level(String),
    TimeRange { start: i64, end: i64 },
    FilePath(String),
    ContentContains(String),
}

/// Bitmap-based filter engine for efficient multi-filter operations
pub struct FilterEngine {
    level_bitmaps: Arc<RwLock<HashMap<String, RoaringBitmap>>>,
    time_range_bitmaps: Arc<RwLock<BTreeMap<TimeRange, RoaringBitmap>>>,
    file_bitmaps: Arc<RwLock<HashMap<String, RoaringBitmap>>>,
    document_count: Arc<RwLock<u32>>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct TimeRange {
    start: i64,
    end: i64,
}

impl FilterEngine {
    /// Create a new filter engine
    pub fn new() -> Self {
        Self {
            level_bitmaps: Arc::new(RwLock::new(HashMap::new())),
            time_range_bitmaps: Arc::new(RwLock::new(BTreeMap::new())),
            file_bitmaps: Arc::new(RwLock::new(HashMap::new())),
            document_count: Arc::new(RwLock::new(0)),
        }
    }

    /// Add a document to the bitmap indexes
    pub fn add_document(&self, doc_id: u32, log_entry: &LogEntry) {
        // Update level bitmap
        {
            let mut level_bitmaps = self.level_bitmaps.write();
            level_bitmaps
                .entry(log_entry.level.clone())
                .or_insert_with(RoaringBitmap::new)
                .insert(doc_id);
        }

        // Update file path bitmap
        {
            let mut file_bitmaps = self.file_bitmaps.write();
            file_bitmaps
                .entry(log_entry.file.clone())
                .or_insert_with(RoaringBitmap::new)
                .insert(doc_id);
        }

        // Update time range bitmaps (partition by hour)
        // Parse timestamp string to i64 for time partitioning
        let timestamp_i64 = log_entry.timestamp.parse::<i64>().unwrap_or(0);
        let hour_timestamp = (timestamp_i64 / 3600) * 3600;
        let time_range = TimeRange {
            start: hour_timestamp,
            end: hour_timestamp + 3600,
        };

        {
            let mut time_bitmaps = self.time_range_bitmaps.write();
            time_bitmaps
                .entry(time_range)
                .or_insert_with(RoaringBitmap::new)
                .insert(doc_id);
        }

        // Update document count
        {
            let mut count = self.document_count.write();
            *count = (*count).max(doc_id + 1);
        }
    }

    /// Apply multiple filters efficiently using bitmap intersection
    pub fn apply_filters(&self, filters: &[Filter]) -> RoaringBitmap {
        if filters.is_empty() {
            // Return all documents
            let count = *self.document_count.read();
            return RoaringBitmap::from_iter(0..count);
        }

        let mut result_bitmap = None;

        for filter in filters {
            let filter_bitmap = self.get_bitmap_for_filter(filter);

            match result_bitmap {
                None => result_bitmap = Some(filter_bitmap),
                Some(ref mut bitmap) => *bitmap &= filter_bitmap,
            }
        }

        result_bitmap.unwrap_or_else(RoaringBitmap::new)
    }

    /// Get bitmap for a specific filter
    fn get_bitmap_for_filter(&self, filter: &Filter) -> RoaringBitmap {
        match filter {
            Filter::Level(level) => {
                let level_bitmaps = self.level_bitmaps.read();
                level_bitmaps
                    .get(level)
                    .cloned()
                    .unwrap_or_else(RoaringBitmap::new)
            }
            Filter::TimeRange { start, end } => {
                let time_bitmaps = self.time_range_bitmaps.read();
                let mut result = RoaringBitmap::new();

                // Find all time ranges that overlap with the requested range
                for (time_range, bitmap) in time_bitmaps.iter() {
                    if time_range.start < *end && time_range.end > *start {
                        result |= bitmap;
                    }
                }

                result
            }
            Filter::FilePath(path) => {
                let file_bitmaps = self.file_bitmaps.read();
                file_bitmaps
                    .get(path)
                    .cloned()
                    .unwrap_or_else(RoaringBitmap::new)
            }
            Filter::ContentContains(_) => {
                // Content filtering would need to be handled by the main search engine
                // Return all documents for now
                let count = *self.document_count.read();
                RoaringBitmap::from_iter(0..count)
            }
        }
    }

    /// Get statistics about the filter engine
    pub fn get_stats(&self) -> FilterStats {
        let level_count = self.level_bitmaps.read().len();
        let time_range_count = self.time_range_bitmaps.read().len();
        let file_count = self.file_bitmaps.read().len();
        let document_count = *self.document_count.read();

        FilterStats {
            level_count,
            time_range_count,
            file_count,
            document_count,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FilterStats {
    pub level_count: usize,
    pub time_range_count: usize,
    pub file_count: usize,
    pub document_count: u32,
}

/// Regex search engine with compilation caching
pub struct RegexSearchEngine {
    compiled_patterns: Arc<Mutex<LruCache<String, Regex>>>,
    pattern_stats: Arc<RwLock<HashMap<String, PatternStats>>>,
    cache_size: usize,
}

#[derive(Debug, Clone)]
struct PatternStats {
    compilation_count: u64,
    execution_count: u64,
    total_execution_time_ms: u64,
    last_used: SystemTime,
}

impl RegexSearchEngine {
    /// Create a new regex search engine
    pub fn new(cache_size: usize) -> Self {
        Self {
            compiled_patterns: Arc::new(Mutex::new(LruCache::new(
                NonZeroUsize::new(cache_size).unwrap(),
            ))),
            pattern_stats: Arc::new(RwLock::new(HashMap::new())),
            cache_size,
        }
    }

    /// Search with regex pattern, using compilation cache
    pub fn search_with_regex(&self, pattern: &str, content: &str) -> SearchResult<Vec<RegexMatch>> {
        let start_time = std::time::Instant::now();

        // Get or compile regex
        let regex = self.get_or_compile_regex(pattern)?;

        // Execute search
        let matches: Vec<RegexMatch> = regex
            .find_iter(content)
            .map(|m| RegexMatch {
                start: m.start(),
                end: m.end(),
                text: m.as_str().to_string(),
            })
            .collect();

        // Update statistics
        self.update_pattern_stats(pattern, start_time.elapsed());

        debug!(
            pattern = %pattern,
            matches = matches.len(),
            time_ms = start_time.elapsed().as_millis(),
            "Regex search completed"
        );

        Ok(matches)
    }

    /// Get or compile regex pattern with caching
    fn get_or_compile_regex(&self, pattern: &str) -> SearchResult<Regex> {
        // Check cache first
        {
            let mut cache = self.compiled_patterns.lock();
            if let Some(regex) = cache.get(pattern) {
                return Ok(regex.clone());
            }
        }

        // Compile new regex
        let regex = Regex::new(pattern).map_err(SearchError::RegexError)?;

        // Add to cache
        {
            let mut cache = self.compiled_patterns.lock();
            cache.put(pattern.to_string(), regex.clone());
        }

        // Update compilation stats
        {
            let mut stats = self.pattern_stats.write();
            let pattern_stats = stats
                .entry(pattern.to_string())
                .or_insert_with(|| PatternStats {
                    compilation_count: 0,
                    execution_count: 0,
                    total_execution_time_ms: 0,
                    last_used: SystemTime::now(),
                });
            pattern_stats.compilation_count += 1;
        }

        Ok(regex)
    }

    /// Update pattern execution statistics
    fn update_pattern_stats(&self, pattern: &str, execution_time: Duration) {
        let mut stats = self.pattern_stats.write();
        let pattern_stats = stats
            .entry(pattern.to_string())
            .or_insert_with(|| PatternStats {
                compilation_count: 0,
                execution_count: 0,
                total_execution_time_ms: 0,
                last_used: SystemTime::now(),
            });

        pattern_stats.execution_count += 1;
        pattern_stats.total_execution_time_ms += execution_time.as_millis() as u64;
        pattern_stats.last_used = SystemTime::now();
    }

    /// Get regex engine statistics
    pub fn get_stats(&self) -> RegexEngineStats {
        let cache_size = self.compiled_patterns.lock().len();
        let pattern_count = self.pattern_stats.read().len();

        RegexEngineStats {
            cache_size,
            max_cache_size: self.cache_size,
            pattern_count,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RegexMatch {
    pub start: usize,
    pub end: usize,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct RegexEngineStats {
    pub cache_size: usize,
    pub max_cache_size: usize,
    pub pattern_count: usize,
}

/// Time-partitioned index for efficient temporal queries
pub struct TimePartitionedIndex {
    partitions: Arc<RwLock<BTreeMap<i64, TimePartition>>>,
    partition_size: Duration,
}

#[derive(Debug, Clone)]
struct TimePartition {
    start_time: i64,
    end_time: i64,
    document_ids: RoaringBitmap,
    entry_count: u32,
}

impl TimePartitionedIndex {
    /// Create a new time-partitioned index
    pub fn new(partition_size: Duration) -> Self {
        Self {
            partitions: Arc::new(RwLock::new(BTreeMap::new())),
            partition_size,
        }
    }

    /// Add document to time-partitioned index
    pub fn add_document(&self, doc_id: u32, timestamp: i64) {
        let partition_key = self.get_partition_key(timestamp);

        let mut partitions = self.partitions.write();
        let partition = partitions
            .entry(partition_key)
            .or_insert_with(|| TimePartition {
                start_time: partition_key,
                end_time: partition_key + self.partition_size.as_secs() as i64,
                document_ids: RoaringBitmap::new(),
                entry_count: 0,
            });

        partition.document_ids.insert(doc_id);
        partition.entry_count += 1;
    }

    /// Query documents within time range
    pub fn query_time_range(&self, start_time: i64, end_time: i64) -> RoaringBitmap {
        let partitions = self.partitions.read();
        let mut result = RoaringBitmap::new();

        let start_key = self.get_partition_key(start_time);
        let end_key = self.get_partition_key(end_time);

        // Find all partitions that overlap with the query range
        for (&_partition_key, partition) in partitions.range(start_key..=end_key) {
            if partition.start_time < end_time && partition.end_time > start_time {
                result |= &partition.document_ids;
            }
        }

        result
    }

    /// Get partition key for timestamp
    fn get_partition_key(&self, timestamp: i64) -> i64 {
        let partition_seconds = self.partition_size.as_secs() as i64;
        (timestamp / partition_seconds) * partition_seconds
    }

    /// Get index statistics
    pub fn get_stats(&self) -> TimeIndexStats {
        let partitions = self.partitions.read();
        let partition_count = partitions.len();
        let total_documents: u32 = partitions.values().map(|p| p.entry_count).sum();

        TimeIndexStats {
            partition_count,
            total_documents,
            partition_size_seconds: self.partition_size.as_secs(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TimeIndexStats {
    pub partition_count: usize,
    pub total_documents: u32,
    pub partition_size_seconds: u64,
}

/// Prefix tree-based autocomplete engine
pub struct AutocompleteEngine {
    trie: Arc<RwLock<TrieNode>>,
    max_suggestions: usize,
}

#[derive(Debug, Default)]
struct TrieNode {
    children: HashMap<char, TrieNode>,
    is_word_end: bool,
    frequency: u32,
    word: Option<String>,
}

impl AutocompleteEngine {
    /// Create a new autocomplete engine
    pub fn new(max_suggestions: usize) -> Self {
        Self {
            trie: Arc::new(RwLock::new(TrieNode::default())),
            max_suggestions,
        }
    }

    /// Add word to autocomplete index
    pub fn add_word(&self, word: &str, frequency: u32) {
        let mut trie = self.trie.write();
        let mut current = &mut *trie;

        for ch in word.chars() {
            current = current.children.entry(ch).or_insert_with(TrieNode::default);
        }

        current.is_word_end = true;
        current.frequency = frequency;
        current.word = Some(word.to_string());
    }

    /// Get autocomplete suggestions for prefix
    pub fn get_suggestions(&self, prefix: &str) -> SearchResult<Vec<AutocompleteSuggestion>> {
        let start_time = std::time::Instant::now();

        let trie = self.trie.read();
        let mut current = &*trie;

        // Navigate to prefix node
        for ch in prefix.chars() {
            match current.children.get(&ch) {
                Some(node) => current = node,
                None => return Ok(vec![]), // Prefix not found
            }
        }

        // Collect suggestions from this point
        let mut suggestions = Vec::new();
        self.collect_suggestions(current, prefix, &mut suggestions);

        // Sort by frequency (descending) and limit results
        suggestions.sort_by(|a, b| b.frequency.cmp(&a.frequency));
        suggestions.truncate(self.max_suggestions);

        let elapsed = start_time.elapsed();

        debug!(
            prefix = %prefix,
            suggestions = suggestions.len(),
            time_ms = elapsed.as_millis(),
            "Autocomplete suggestions generated"
        );

        // Ensure response time is under 100ms as per requirements
        if elapsed > Duration::from_millis(100) {
            warn!(
                prefix = %prefix,
                time_ms = elapsed.as_millis(),
                "Autocomplete response time exceeded 100ms threshold"
            );
        }

        Ok(suggestions)
    }

    /// Recursively collect suggestions from trie
    fn collect_suggestions(
        &self,
        node: &TrieNode,
        current_word: &str,
        suggestions: &mut Vec<AutocompleteSuggestion>,
    ) {
        if suggestions.len() >= self.max_suggestions * 2 {
            return; // Stop collecting to maintain performance
        }

        if node.is_word_end {
            suggestions.push(AutocompleteSuggestion {
                text: current_word.to_string(),
                frequency: node.frequency,
            });
        }

        for (ch, child_node) in &node.children {
            let mut new_word = current_word.to_string();
            new_word.push(*ch);
            self.collect_suggestions(child_node, &new_word, suggestions);
        }
    }

    /// Get autocomplete statistics
    pub fn get_stats(&self) -> AutocompleteStats {
        let trie = self.trie.read();
        let (node_count, word_count) = self.count_nodes(&*trie);

        AutocompleteStats {
            node_count,
            word_count,
            max_suggestions: self.max_suggestions,
        }
    }

    /// Count nodes and words in trie
    fn count_nodes(&self, node: &TrieNode) -> (usize, usize) {
        let mut node_count = 1;
        let mut word_count = if node.is_word_end { 1 } else { 0 };

        for child in node.children.values() {
            let (child_nodes, child_words) = self.count_nodes(child);
            node_count += child_nodes;
            word_count += child_words;
        }

        (node_count, word_count)
    }
}

#[derive(Debug, Clone)]
pub struct AutocompleteSuggestion {
    pub text: String,
    pub frequency: u32,
}

#[derive(Debug, Clone)]
pub struct AutocompleteStats {
    pub node_count: usize,
    pub word_count: usize,
    pub max_suggestions: usize,
}

impl Default for FilterEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for RegexSearchEngine {
    fn default() -> Self {
        Self::new(1000) // Default cache size of 1000 patterns
    }
}

impl Default for TimePartitionedIndex {
    fn default() -> Self {
        Self::new(Duration::from_secs(3600)) // Default 1-hour partitions
    }
}

impl Default for AutocompleteEngine {
    fn default() -> Self {
        Self::new(10) // Default 10 suggestions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_engine() {
        let engine = FilterEngine::new();

        let log_entry = LogEntry {
            id: 0,
            timestamp: "1640995200".to_string(), // 2022-01-01 00:00:00
            level: "ERROR".to_string(),
            file: "test.log".to_string(),
            real_path: "/path/to/test.log".to_string(),
            line: 1,
            content: "Test error message".to_string(),
            tags: vec![],
            match_details: None,
            matched_keywords: None,
        };

        engine.add_document(0, &log_entry);

        let filters = vec![Filter::Level("ERROR".to_string())];
        let result = engine.apply_filters(&filters);

        assert!(result.contains(0));
    }

    #[test]
    fn test_regex_engine() {
        let engine = RegexSearchEngine::new(100);

        let result = engine.search_with_regex(r"\d+", "Error 404 occurred at line 123");
        assert!(result.is_ok());

        let matches = result.unwrap();
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].text, "404");
        assert_eq!(matches[1].text, "123");
    }

    #[test]
    fn test_time_partitioned_index() {
        let index = TimePartitionedIndex::new(Duration::from_secs(3600));

        index.add_document(0, 1640995200); // 2022-01-01 00:00:00
        index.add_document(1, 1640998800); // 2022-01-01 01:00:00

        // 查询范围应该包含两个时间戳（使用包含边界的范围）
        // 注意：end_time 是排他的，所以需要 +1 来包含 1640998800
        let result = index.query_time_range(1640995200, 1640998801);
        assert!(
            result.contains(0),
            "Should contain document 0 with timestamp 1640995200"
        );
        assert!(
            result.contains(1),
            "Should contain document 1 with timestamp 1640998800"
        );
    }

    #[test]
    fn test_autocomplete_engine() {
        let engine = AutocompleteEngine::new(5);

        engine.add_word("error", 100);
        engine.add_word("exception", 50);
        engine.add_word("warning", 25);

        let suggestions = engine.get_suggestions("e").unwrap();
        assert_eq!(suggestions.len(), 2);
        assert_eq!(suggestions[0].text, "error");
        assert_eq!(suggestions[1].text, "exception");
    }
}
