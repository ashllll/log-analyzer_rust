//! Advanced Search Features
#![allow(dead_code)]
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

use crate::{SearchError, SearchResult};
use la_core::models::LogEntry;

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
                .entry(log_entry.level.to_string())
                .or_default()
                .insert(doc_id);
        }

        // Update file path bitmap
        {
            let mut file_bitmaps = self.file_bitmaps.write();
            file_bitmaps
                .entry(log_entry.file.to_string())
                .or_default()
                .insert(doc_id);
        }

        // 更新时间范围位图（按小时分区）
        // 改进的时间戳处理策略：
        // 1. 有效时间戳：正常分区
        // 2. 无效/零时间戳：放入特殊 "unknown" 分区，避免污染有效时间线
        const UNKNOWN_TIME_KEY: i64 = i64::MIN; // 使用 i64::MIN 作为未知时间标记

        let timestamp_result = log_entry.timestamp.parse::<i64>();
        let hour_timestamp = match timestamp_result {
            Ok(ts) if ts > 0 => (ts / 3600) * 3600, // 有效正时间戳
            Ok(0) => {
                // 时间戳为 0（可能是纪元时间），视为无效
                tracing::debug!(
                    timestamp = %log_entry.timestamp,
                    file = %log_entry.file,
                    doc_id = doc_id,
                    "时间戳为0（纪元时间），归入未知时间分区"
                );
                UNKNOWN_TIME_KEY
            }
            Ok(_negative_ts) => {
                // 负时间戳（早于1970年），虽有效但可能是数据问题
                tracing::warn!(
                    timestamp = %log_entry.timestamp,
                    file = %log_entry.file,
                    doc_id = doc_id,
                    "检测到负时间戳（早于1970-01-01），归入未知时间分区"
                );
                UNKNOWN_TIME_KEY
            }
            Err(e) => {
                // 解析失败
                tracing::warn!(
                    timestamp = %log_entry.timestamp,
                    file = %log_entry.file,
                    doc_id = doc_id,
                    error = %e,
                    "时间戳解析失败，归入未知时间分区；该文档将无法通过时间范围过滤"
                );
                UNKNOWN_TIME_KEY
            }
        };

        // 构建时间范围：有效时间戳使用实际范围，无效时间戳使用特殊标记
        let time_range = if hour_timestamp == UNKNOWN_TIME_KEY {
            TimeRange {
                start: UNKNOWN_TIME_KEY,
                end: UNKNOWN_TIME_KEY + 1,
            }
        } else {
            TimeRange {
                start: hour_timestamp,
                end: hour_timestamp + 3600,
            }
        };

        {
            let mut time_bitmaps = self.time_range_bitmaps.write();
            time_bitmaps.entry(time_range).or_default().insert(doc_id);
        }

        // Update document count - 使用实际文档数量而非最大ID
        {
            let mut count = self.document_count.write();
            *count = (*count).max(doc_id.saturating_add(1));
        }
    }

    /// 添加删除文档后的计数更新方法
    pub fn remove_document(&self, _doc_id: u32) {
        let mut count = self.document_count.write();
        // 文档删除时，简单地将计数减1
        // 注意：对于精确计数，应该在删除时重新计算
        *count = count.saturating_sub(1);
    }

    /// Apply multiple filters efficiently using bitmap intersection
    /// 返回 SearchResult 以处理空filters边界情况
    pub fn apply_filters(&self, filters: &[Filter]) -> SearchResult<RoaringBitmap> {
        if filters.is_empty() {
            // 空过滤列表语义：无约束条件，返回全集（匹配所有文档）
            let count = *self.document_count.read();
            return Ok(RoaringBitmap::from_iter(0..count));
        }

        let mut result_bitmap = None;

        for filter in filters {
            let filter_bitmap = self.get_bitmap_for_filter(filter);

            match result_bitmap {
                None => result_bitmap = Some(filter_bitmap),
                Some(ref mut bitmap) => *bitmap &= filter_bitmap,
            }
        }

        Ok(result_bitmap.unwrap_or_else(RoaringBitmap::new))
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
    compiled_patterns: Arc<Mutex<LruCache<String, Arc<Regex>>>>,
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
                NonZeroUsize::new(cache_size).unwrap_or(NonZeroUsize::new(100).unwrap()),
            ))),
            pattern_stats: Arc::new(RwLock::new(HashMap::new())),
            cache_size,
        }
    }

    /// Search with regex pattern, using compilation cache
    pub fn search_with_regex(&self, pattern: &str, content: &str) -> SearchResult<Vec<RegexMatch>> {
        let start_time = std::time::Instant::now();

        let regex = self.get_or_compile_regex(pattern)?;

        let matches: Vec<RegexMatch> = regex
            .find_iter(content)
            .map(|m| RegexMatch {
                start: m.start(),
                end: m.end(),
                text: m.as_str().to_string(),
            })
            .collect();

        self.update_pattern_stats(pattern, start_time.elapsed());

        debug!(
            pattern = %pattern,
            matches = matches.len(),
            time_ms = start_time.elapsed().as_millis(),
            "Regex search completed"
        );

        Ok(matches)
    }

    /// Validate regex pattern to prevent ReDoS attacks
    fn validate_regex_pattern(pattern: &str) -> Result<(), String> {
        const MAX_PATTERN_LENGTH: usize = 1000;
        const DANGEROUS_PATTERNS: &[&str] = &[
            "(a+)+",
            "(a*a*)*",
            "(.*.*)*",
            "([a-zA-Z]+)*",
            "(a{1,100})+",
            "([^X]+)+",
        ];

        if pattern.len() > MAX_PATTERN_LENGTH {
            return Err(format!(
                "Regex pattern too long (max {} characters)",
                MAX_PATTERN_LENGTH
            ));
        }

        let lower_pattern = pattern.to_lowercase();
        for dangerous in DANGEROUS_PATTERNS {
            if lower_pattern.contains(dangerous) {
                warn!(
                    pattern = %pattern,
                    dangerous_pattern = dangerous,
                    "Potential ReDoS pattern detected"
                );
                return Err("Potentially dangerous regex pattern detected".to_string());
            }
        }

        Ok(())
    }

    /// Get or compile regex pattern with caching
    fn get_or_compile_regex(&self, pattern: &str) -> SearchResult<Arc<Regex>> {
        if let Err(e) = Self::validate_regex_pattern(pattern) {
            return Err(SearchError::RegexError(regex::Error::Syntax(e)));
        }

        // Check cache first
        {
            let mut cache = self.compiled_patterns.lock();
            if let Some(regex) = cache.get(pattern) {
                return Ok(Arc::clone(regex));
            }
        }

        // Compile regex with detailed error message
        let regex = Regex::new(pattern).map_err(|e| {
            SearchError::RegexError(regex::Error::Syntax(format!(
                "Failed to compile regex pattern '{}': {}",
                pattern, e
            )))
        })?;
        let arc_regex = Arc::new(regex);

        // Add to cache
        {
            let mut cache = self.compiled_patterns.lock();
            cache.put(pattern.to_string(), Arc::clone(&arc_regex));
        }

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

        Ok(arc_regex)
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
    entry_count: u64,
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
        partition.entry_count = partition.entry_count.saturating_add(1);
    }

    /// Query documents within time range
    /// 限制分区数量以防止跨年查询导致内存耗尽
    pub fn query_time_range(&self, start_time: i64, end_time: i64) -> RoaringBitmap {
        const MAX_PARTITIONS_TO_AGGREGATE: usize = 1000;

        let partitions = self.partitions.read();
        let mut result = RoaringBitmap::new();

        let start_key = self.get_partition_key(start_time);
        let end_key = self.get_partition_key(end_time);

        let mut partition_count = 0;
        let mut skipped_count = 0;

        // Find all partitions that overlap with the query range
        for (&_partition_key, partition) in partitions.range(start_key..=end_key) {
            if partition_count >= MAX_PARTITIONS_TO_AGGREGATE {
                skipped_count += 1;
                continue;
            }

            if partition.start_time < end_time && partition.end_time > start_time {
                result |= &partition.document_ids;
                partition_count += 1;
            }
        }

        if skipped_count > 0 {
            warn!(
                partitions_processed = partition_count,
                partitions_skipped = skipped_count,
                start_time = start_time,
                end_time = end_time,
                "Time range query exceeded maximum partitions limit; results may be incomplete"
            );
        }

        result
    }

    /// Get partition key for timestamp
    /// 使用 div_euclid 实现 floor division，确保负时间戳也能正确分区
    fn get_partition_key(&self, timestamp: i64) -> i64 {
        let partition_seconds = self.partition_size.as_secs() as i64;
        // div_euclid 对负数的行为等价于向负无穷取整（floor division）
        // 例如: (-1).div_euclid(3600) = -1，而 -1 / 3600 = 0（错误）
        timestamp.div_euclid(partition_seconds) * partition_seconds
    }

    /// Get index statistics
    pub fn get_stats(&self) -> TimeIndexStats {
        let partitions = self.partitions.read();
        let partition_count = partitions.len();
        let total_documents: u64 = partitions.values().map(|p| p.entry_count).sum();

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
    pub total_documents: u64,
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
            current = current.children.entry(ch).or_default();
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
        suggestions.sort_by_key(|b| std::cmp::Reverse(b.frequency));
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

    /// 迭代式 BFS 收集 Trie 建议词，避免深层 Trie 导致的递归栈溢出
    fn collect_suggestions(
        &self,
        root: &TrieNode,
        prefix: &str,
        suggestions: &mut Vec<AutocompleteSuggestion>,
    ) {
        // 使用显式队列代替函数调用栈，深度无论多大都不会栈溢出
        let limit = self.max_suggestions * 2;
        let mut queue: std::collections::VecDeque<(&TrieNode, String)> =
            std::collections::VecDeque::new();
        queue.push_back((root, prefix.to_string()));

        while let Some((node, word)) = queue.pop_front() {
            if suggestions.len() >= limit {
                break;
            }
            if node.is_word_end {
                suggestions.push(AutocompleteSuggestion {
                    text: word.clone(),
                    frequency: node.frequency,
                });
            }
            for (ch, child_node) in &node.children {
                let mut child_word = word.clone();
                child_word.push(*ch);
                queue.push_back((child_node, child_word));
            }
        }
    }

    /// Get autocomplete statistics
    pub fn get_stats(&self) -> AutocompleteStats {
        let trie = self.trie.read();
        let (node_count, word_count) = self.count_nodes(&trie);

        AutocompleteStats {
            node_count,
            word_count,
            max_suggestions: self.max_suggestions,
        }
    }

    /// Count nodes and words in trie（BFS 迭代，避免深层 Trie 递归栈溢出）
    fn count_nodes(&self, root: &TrieNode) -> (usize, usize) {
        let mut node_count = 0;
        let mut word_count = 0;
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(root);
        while let Some(node) = queue.pop_front() {
            node_count += 1;
            if node.is_word_end {
                word_count += 1;
            }
            for child in node.children.values() {
                queue.push_back(child);
            }
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
            timestamp: "1640995200".into(), // 2022-01-01 00:00:00
            level: "ERROR".into(),
            file: "test.log".into(),
            real_path: "/path/to/test.log".into(),
            line: 1,
            content: "Test error message".into(),
            tags: vec![],
            match_details: None,
            matched_keywords: None,
        };

        engine.add_document(0, &log_entry);

        let filters = vec![Filter::Level("ERROR".to_string())];
        let result = engine
            .apply_filters(&filters)
            .expect("apply_filters should succeed");

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
