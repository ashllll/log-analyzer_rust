use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use log_analyzer::models::log_entry::LogEntry;
use log_analyzer::models::search::SearchQuery;
use log_analyzer::services::pattern_matcher::PatternMatcher;
use std::sync::Arc;
use tokio::runtime::Runtime;

fn create_test_log_entries(count: usize) -> Vec<LogEntry> {
    (0..count)
        .map(|i| LogEntry {
            id: i,
            content: format!("Test log entry {} with some error message", i),
            file: format!("/test/path/file_{}.log", i),
            real_path: format!("/test/path/file_{}.log", i),
            line: i,
            timestamp: format!("2024-01-01T00:00:{:02}Z", i % 60),
            level: "INFO".to_string(),
            tags: vec![],
            match_details: None,
            matched_keywords: None,
        })
        .collect()
}

fn bench_pattern_matching(c: &mut Criterion) {
    let mut group = c.benchmark_group("pattern_matching");

    let pattern_matcher =
        PatternMatcher::new(vec!["error".to_string(), "warning".to_string()], false).unwrap();
    let log_entries = create_test_log_entries(1000);

    for pattern in &["error", "warning", "info", "debug"] {
        group.bench_with_input(
            BenchmarkId::new("single_pattern", pattern),
            pattern,
            |b, pattern| {
                b.iter(|| {
                    // Simple pattern matching for benchmark
                    let results: Vec<_> = log_entries
                        .iter()
                        .filter(|entry| entry.content.contains(black_box(pattern)))
                        .collect();
                    black_box(results)
                })
            },
        );
    }

    group.finish();
}

fn bench_search_query_execution(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("search_execution");

    let log_entries = create_test_log_entries(10000);

    for size in &[100, 1000, 10000] {
        let entries = &log_entries[..*size];

        group.bench_with_input(BenchmarkId::new("query_execution", size), size, |b, _| {
            b.to_async(&rt).iter(|| async {
                let search_term = "error";

                // Simulate query execution
                let results: Vec<_> = entries
                    .iter()
                    .filter(|entry| entry.content.contains(search_term))
                    .cloned()
                    .collect();

                black_box(results)
            })
        });
    }

    group.finish();
}

fn bench_concurrent_search(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("concurrent_search");

    let log_entries = Arc::new(create_test_log_entries(5000));

    for thread_count in &[1, 2, 4, 8] {
        group.bench_with_input(
            BenchmarkId::new("concurrent_execution", thread_count),
            thread_count,
            |b, &thread_count| {
                b.to_async(&rt).iter(|| async {
                    let entries = Arc::clone(&log_entries);
                    let mut handles = Vec::new();

                    for i in 0..thread_count {
                        let entries = Arc::clone(&entries);
                        let handle = tokio::spawn(async move {
                            let query = format!("error_{}", i);
                            entries
                                .iter()
                                .filter(|entry| entry.content.contains(&query))
                                .count()
                        });
                        handles.push(handle);
                    }

                    let results: Vec<_> = futures::future::join_all(handles)
                        .await
                        .into_iter()
                        .collect::<Result<Vec<_>, _>>()
                        .unwrap();

                    black_box(results)
                })
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_pattern_matching,
    bench_search_query_execution,
    bench_concurrent_search
);
criterion_main!(benches);
