// 搜索性能基准测试
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use log_analyzer::services::pattern_matcher::PatternMatcher;

/// 基准测试：单模式匹配
fn bench_single_pattern(c: &mut Criterion) {
    let text = "2024-01-01 12:00:00 INFO Application started
2024-01-01 12:00:01 ERROR Database connection failed
2024-01-01 12:00:02 WARN Retrying connection
2024-01-01 12:00:03 INFO Connection established";

    let patterns = vec!["ERROR".to_string()];
    let matcher = PatternMatcher::new(patterns, true).expect("Failed to create matcher");

    c.bench_function("single_pattern_match", |b| {
        b.iter(|| {
            black_box(matcher.find_matches(black_box(text)));
        });
    });
}

/// 基准测试：多模式匹配（不同数量）
fn bench_multiple_patterns(c: &mut Criterion) {
    let text = "2024-01-01 12:00:00 INFO Application started
2024-01-01 12:00:01 ERROR Database connection failed
2024-01-01 12:00:02 WARN Retrying connection
2024-01-01 12:00:03 INFO Connection established
2024-01-01 12:00:04 DEBUG Query executed successfully";

    let pattern_counts = [1, 5, 10, 50, 100];

    let mut group = c.benchmark_group("pattern_matching");

    for count in pattern_counts {
        let patterns: Vec<String> = (0..count)
            .map(|i| format!("PATTERN{}", i))
            .collect();

        let matcher = PatternMatcher::new(patterns, true).expect("Failed to create matcher");

        group.throughput(Throughput::Elements(count as u64));

        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, _| {
            b.iter(|| {
                black_box(matcher.find_matches(black_box(text)));
            });
        });
    }

    group.finish();
}

/// 基准测试：大小写敏感 vs 不敏感
fn bench_case_sensitivity(c: &mut Criterion) {
    let text = "2024-01-01 12:00:00 INFO Application started
2024-01-01 12:00:01 ERROR Database connection failed
2024-01-01 12:00:02 WARN Retrying connection";

    let patterns = vec!["error".to_string()];

    let mut group = c.benchmark_group("case_sensitivity");

    // 大小写不敏感
    let matcher_insensitive = PatternMatcher::new(patterns.clone(), true).expect("Failed to create matcher");
    group.bench_function("case_insensitive", |b| {
        b.iter(|| {
            black_box(matcher_insensitive.find_matches(black_box(text)));
        });
    });

    // 大小写敏感
    let matcher_sensitive = PatternMatcher::new(patterns, false).expect("Failed to create matcher");
    group.bench_function("case_sensitive", |b| {
        b.iter(|| {
            black_box(matcher_sensitive.find_matches(black_box(text)));
        });
    });

    group.finish();
}

/// 基准测试：不同文本长度
fn bench_text_length(c: &mut Criterion) {
    let base_line = "2024-01-01 12:00:00 INFO Sample log message with keyword ERROR\n";
    let patterns = vec!["ERROR".to_string()];
    let matcher = PatternMatcher::new(patterns, true).expect("Failed to create matcher");

    let mut group = c.benchmark_group("text_length");

    for line_count in [10, 100, 1000, 10000].iter() {
        let text = base_line.repeat(*line_count);

        group.throughput(Throughput::Bytes(text.len() as u64));

        group.bench_with_input(BenchmarkId::from_parameter(line_count), line_count, |b, _| {
            b.iter(|| {
                black_box(matcher.find_matches(black_box(&text)));
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_single_pattern,
    bench_multiple_patterns,
    bench_case_sensitivity,
    bench_text_length
);
criterion_main!(benches);
