use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use log_analyzer::models::validated::{ValidatedSearchQuery, ValidatedWorkspaceConfig};
use log_analyzer::utils::validation::validate_path_safety;
use std::collections::HashMap;
use validator::Validate;

fn create_test_workspace_configs(count: usize) -> Vec<ValidatedWorkspaceConfig> {
    (0..count)
        .map(|i| ValidatedWorkspaceConfig {
            workspace_id: format!("workspace_{}", i),
            name: format!("Test Workspace {}", i),
            description: Some(format!("Test workspace description {}", i)),
            path: format!("/test/workspace/path_{}", i),
            max_file_size: 1024 * 1024, // 1MB
            max_file_count: 1000,
            enable_watch: true,
            tags: vec![format!("tag_{}", i)],
            metadata: HashMap::new(),
            contact_email: Some(format!("test{}@example.com", i)),
            project_url: Some(format!("https://example.com/project_{}", i)),
        })
        .collect()
}

fn create_test_search_queries(count: usize) -> Vec<ValidatedSearchQuery> {
    (0..count)
        .map(|i| ValidatedSearchQuery {
            query: format!("test query {}", i),
            workspace_id: format!("workspace_{}", i),
            max_results: Some(1000),
            case_sensitive: false,
            use_regex: false,
            file_pattern: Some("*.log".to_string()),
            time_start: None,
            time_end: None,
            log_levels: vec!["INFO".to_string(), "ERROR".to_string()],
            priority: Some(5),
            timeout_seconds: Some(30),
        })
        .collect()
}

fn bench_workspace_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("workspace_validation");

    let configs = create_test_workspace_configs(100);

    group.bench_function("validate_workspace_config", |b| {
        b.iter(|| {
            for config in &configs {
                let result = config.validate();
                black_box(result);
            }
        })
    });

    // Benchmark individual validation components
    let test_paths = vec![
        "/valid/path/to/workspace",
        "/another/valid/path",
        "../invalid/traversal/path",
        "/path/with/unicode/测试",
        "/very/long/path/that/might/cause/performance/issues/in/validation/logic/test",
    ];

    for (i, path) in test_paths.iter().enumerate() {
        group.bench_with_input(BenchmarkId::new("path_validation", i), path, |b, path| {
            b.iter(|| {
                let result = validate_path_safety(black_box(path));
                black_box(result)
            })
        });
    }

    group.finish();
}

fn bench_search_query_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_validation");

    let queries = create_test_search_queries(100);

    group.bench_function("validate_search_query", |b| {
        b.iter(|| {
            for query in &queries {
                let result = query.validate();
                black_box(result);
            }
        })
    });

    // Benchmark query complexity validation
    let complex_queries = vec![
        "simple query",
        "query with multiple words and special characters !@#$%",
        "very long query that contains many words and might trigger performance issues in validation logic because it exceeds normal length expectations",
        "query with unicode characters 测试查询 العربية русский",
        "query with regex patterns [a-zA-Z0-9]+ \\d{3,5} .*error.*",
    ];

    for (i, query) in complex_queries.iter().enumerate() {
        group.bench_with_input(
            BenchmarkId::new("query_complexity", i),
            query,
            |b, query| {
                b.iter(|| {
                    let test_query = ValidatedSearchQuery {
                        query: query.to_string(),
                        workspace_id: "test".to_string(),
                        max_results: Some(1000),
                        case_sensitive: false,
                        use_regex: false,
                        file_pattern: Some("*.log".to_string()),
                        time_start: None,
                        time_end: None,
                        log_levels: vec!["INFO".to_string()],
                        priority: Some(5),
                        timeout_seconds: Some(30),
                    };
                    let result = test_query.validate();
                    black_box(result)
                })
            },
        );
    }

    group.finish();
}

fn bench_path_sanitization(c: &mut Criterion) {
    let mut group = c.benchmark_group("path_sanitization");

    let test_paths = vec![
        "normal_filename.txt",
        "file with spaces.log",
        "file_with_unicode_测试.txt",
        "../../../etc/passwd",
        "file<>:\"|?*.txt",
        "very_long_filename_that_might_cause_performance_issues_in_sanitization_logic.txt",
    ];

    for (i, path) in test_paths.iter().enumerate() {
        group.bench_with_input(BenchmarkId::new("sanitize_path", i), path, |b, path| {
            b.iter(|| {
                // Use sanitize_filename crate directly
                let result = sanitize_filename::sanitize(black_box(path));
                black_box(result)
            })
        });
    }

    group.finish();
}

fn bench_batch_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_validation");

    for batch_size in &[10, 100, 1000] {
        let configs = create_test_workspace_configs(*batch_size);
        let queries = create_test_search_queries(*batch_size);

        group.bench_with_input(
            BenchmarkId::new("workspace_batch", batch_size),
            &configs,
            |b, configs| {
                b.iter(|| {
                    let results: Vec<_> = configs.iter().map(|config| config.validate()).collect();
                    black_box(results)
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("query_batch", batch_size),
            &queries,
            |b, queries| {
                b.iter(|| {
                    let results: Vec<_> = queries.iter().map(|query| query.validate()).collect();
                    black_box(results)
                })
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_workspace_validation,
    bench_search_query_validation,
    bench_path_sanitization,
    bench_batch_validation
);
criterion_main!(benches);
