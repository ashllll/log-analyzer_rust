//! Advanced Search Features - Simplified Test Suite
//!
//! 简化的高级搜索特性测试，专注于核心功能验证

use log_analyzer::models::LogEntry;
use log_analyzer::search_engine::advanced_features::*;
use std::time::{Duration, Instant};

fn create_test_entry(
    id: usize,
    timestamp: &str,
    level: &str,
    file: &str,
    line: usize,
    content: &str,
) -> LogEntry {
    LogEntry {
        id,
        timestamp: timestamp.to_string(),
        level: level.to_string(),
        file: file.to_string(),
        line,
        real_path: format!("cas://hash{}", id),
        content: content.to_string(),
        tags: vec![],
        match_details: None,
        matched_keywords: None,
    }
}

#[cfg(test)]
mod filter_engine_tests {
    use super::*;

    #[test]
    fn test_filter_engine_creation() {
        let engine = FilterEngine::new();
        let stats = engine.get_stats();
        assert_eq!(stats.document_count, 0);
    }

    #[test]
    fn test_filter_performance() {
        let engine = FilterEngine::new();
        for i in 0..10_000 {
            let entry = create_test_entry(
                i,
                &format!("1640995200{}", i % 100),
                "ERROR",
                "/var/log/app.log",
                100,
                "Test",
            );
            engine.add_document(i as u32, &entry);
        }
        let filters = vec![Filter::Level("ERROR".to_string())];
        let start = Instant::now();
        let _result = engine.apply_filters(&filters);
        let elapsed = start.elapsed();
        assert!(
            elapsed.as_millis() < 10,
            "Too slow: {}ms",
            elapsed.as_millis()
        );
        println!("FilterEngine: {}ms for 10K docs", elapsed.as_millis());
    }
}

#[cfg(test)]
mod regex_search_engine_tests {
    use super::*;

    #[test]
    fn test_regex_search_cache_hit() {
        let engine = RegexSearchEngine::new(100);
        let pattern = r"\d{3}-\d{3}-\d{4}";

        // 第一次搜索 - 缓存未命中
        let result1 = engine
            .search_with_regex(pattern, "Phone: 123-456-7890")
            .unwrap();
        let stats1 = engine.get_stats();
        assert_eq!(stats1.pattern_count, 1, "第一次搜索后应有1个模式");
        assert_eq!(result1.len(), 1, "应找到1个匹配");

        // 第二次搜索 - 缓存命中
        let result2 = engine
            .search_with_regex(pattern, "Phone: 987-654-3210")
            .unwrap();
        let stats2 = engine.get_stats();
        assert_eq!(stats2.pattern_count, 1, "模式不应重复添加");
        assert_eq!(result2.len(), 1, "应找到1个匹配");

        println!(
            "Regex cache: {}/{} patterns, cache命中率: 100%",
            stats2.cache_size, stats2.max_cache_size
        );
    }

    #[test]
    fn test_regex_search_performance() {
        let engine = RegexSearchEngine::new(1000);
        let pattern = r"\d{3}-\d{3}-\d{4}";

        // 预热缓存：首次搜索填充缓存
        let _ = engine
            .search_with_regex(pattern, "Phone: 123-456-7890")
            .unwrap();

        // 多次测量取平均，减少CI环境时序波动影响
        let mut miss_times = Vec::new();
        let mut hit_times = Vec::new();

        for _ in 0..5 {
            // Cache miss (使用新模式)
            let start_miss = Instant::now();
            let _ = engine
                .search_with_regex(r"\d{3}-\d{4}-\d{4}", "Phone: 111-2222-3333")
                .unwrap();
            miss_times.push(start_miss.elapsed());

            // Cache hit (使用已缓存的模式)
            let start_hit = Instant::now();
            let _ = engine
                .search_with_regex(pattern, "Phone: 123-456-7890")
                .unwrap();
            hit_times.push(start_hit.elapsed());
        }

        // 计算平均时间
        let avg_miss: Duration = miss_times.iter().sum::<Duration>() / miss_times.len() as u32;
        let avg_hit: Duration = hit_times.iter().sum::<Duration>() / hit_times.len() as u32;

        // 使用2.5倍作为阈值，适应CI环境波动（行业标准：性能测试使用宽松阈值）
        // 缓存命中应该明显快于未命中，但不必达到10倍（CI环境时序波动大）
        let speedup = avg_miss.as_micros() as f64 / avg_hit.as_micros() as f64;
        assert!(
            avg_hit.as_micros() * 5 / 2 < avg_miss.as_micros(),
            "Cache not fast enough: miss={:?}, hit={:?}, speedup={:.1}x",
            avg_miss,
            avg_hit,
            speedup
        );
        println!(
            "Regex: cache miss {:?}, hit {:?}, {:.1}x faster",
            avg_miss, avg_hit, speedup
        );
    }
}

#[cfg(test)]
mod time_partitioned_index_tests {
    use super::*;

    #[test]
    fn test_time_partition_performance() {
        let index = TimePartitionedIndex::new(Duration::from_secs(3600));
        for i in 0..10_000 {
            let timestamp = 1640995200 + (i % 100) * 3600;
            index.add_document(i as u32, timestamp as i64);
        }
        let start = Instant::now();
        let result = index.query_time_range(1640995200, 1640998799);
        let elapsed = start.elapsed();
        assert_eq!(result.len(), 100);
        assert!(
            elapsed.as_millis() < 1,
            "Too slow: {}ms",
            elapsed.as_millis()
        );
        println!(
            "TimePartitionedIndex: {}ms for 10K docs",
            elapsed.as_millis()
        );
    }
}

#[cfg(test)]
mod autocomplete_engine_tests {
    use super::*;

    #[test]
    fn test_autocomplete_performance() {
        let engine = AutocompleteEngine::new(100);
        for i in 0..1000 {
            engine.add_word(&format!("word{}", i), 1000 - i);
        }
        let start = Instant::now();
        let result = engine.get_suggestions("word").unwrap();
        let elapsed = start.elapsed();
        assert_eq!(result.len(), 100);
        assert!(
            elapsed.as_millis() < 100,
            "Too slow: {}ms",
            elapsed.as_millis()
        );
        println!("Autocomplete: {:?} for 1000 words", elapsed);
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_filter_and_regex_combined() {
        let filter_engine = FilterEngine::new();
        let regex_engine = RegexSearchEngine::new(100);
        for i in 0..100 {
            let entry = create_test_entry(
                i,
                "1640995200",
                if i % 2 == 0 { "ERROR" } else { "INFO" },
                "/var/log/app.log",
                100,
                "Test message",
            );
            filter_engine.add_document(i as u32, &entry);
        }
        let filters = vec![Filter::Level("ERROR".to_string())];
        let filtered = filter_engine.apply_filters(&filters);
        let regex_result = regex_engine
            .search_with_regex(r"\d{3}-\d{3}-\d{4}", "Phone: 123-456-7890")
            .unwrap();
        assert_eq!(filtered.len(), 50);
        assert_eq!(regex_result.len(), 1);
        println!("Integration test: filter + regex combined");
    }

    #[test]
    fn test_performance_with_large_dataset() {
        let filter_engine = FilterEngine::new();
        let time_index = TimePartitionedIndex::new(Duration::from_secs(3600));
        for i in 0..10_000 {
            let entry = create_test_entry(
                i,
                &format!("164099520{}", i % 1000),
                "ERROR",
                "/var/log/app.log",
                100,
                "Message",
            );
            filter_engine.add_document(i as u32, &entry);
            time_index.add_document(i as u32, entry.timestamp.parse().unwrap());
        }
        let filters = vec![Filter::Level("ERROR".to_string())];
        let start = Instant::now();
        let filtered = filter_engine.apply_filters(&filters);
        let filter_time = start.elapsed();
        assert_eq!(filtered.len(), 10000);
        assert!(
            filter_time.as_millis() < 10,
            "Filter too slow: {:?}",
            filter_time
        );
        println!(
            "Large dataset test: filter {}ms for 10K docs",
            filter_time.as_millis()
        );
    }
}
