use proptest::prelude::*;
use proptest::test_runner::Config;

/// Standard proptest configuration for all property-based tests
/// Configured for 1000 iterations as specified in requirements
pub fn proptest_config() -> Config {
    Config {
        cases: 1000,
        max_shrink_iters: 10000,
        ..Config::default()
    }
}

/// Custom strategies for domain-specific types
pub mod strategies {
    use super::*;
    use log_analyzer::models::log_entry::LogEntry;
    use log_analyzer::models::search::{
        QueryMetadata, QueryOperator, SearchQuery, SearchTerm, TermSource,
    };

    /// Generate valid workspace IDs (alphanumeric + hyphens/underscores)
    pub fn workspace_id() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9_-]{1,50}"
    }

    /// Generate valid file paths (no path traversal)
    pub fn safe_file_path() -> impl Strategy<Value = String> {
        prop::collection::vec("[a-zA-Z0-9_-]{1,20}", 1..5)
            .prop_map(|parts| format!("/{}", parts.join("/")))
    }

    /// Generate log entries with realistic content
    pub fn log_entry() -> impl Strategy<Value = LogEntry> {
        (
            1usize..10000,
            "[a-zA-Z0-9 .,!?-]{10,200}",
            safe_file_path(),
            safe_file_path(),
            1usize..10000,
            "[a-zA-Z0-9 .,!?-]{10,200}",
            "[A-Z]{3,5}",
        )
            .prop_map(
                |(id, content, file, real_path, line, timestamp, level)| LogEntry {
                    id,
                    content,
                    file,
                    real_path,
                    line,
                    timestamp,
                    level,
                    tags: vec![],
                    match_details: None,
                    matched_keywords: None,
                },
            )
    }

    /// Generate search queries with various patterns
    pub fn search_query() -> impl Strategy<Value = SearchQuery> {
        (
            "[a-zA-Z0-9_-]{1,20}",
            prop::collection::vec(search_term(), 1..5),
        )
            .prop_map(|(id, terms)| SearchQuery {
                id,
                terms,
                global_operator: QueryOperator::And,
                filters: None,
                metadata: QueryMetadata {
                    created_at: 0,
                    last_modified: 0,
                    execution_count: 0,
                    label: None,
                },
            })
    }

    /// Generate search terms
    pub fn search_term() -> impl Strategy<Value = SearchTerm> {
        (
            "[a-zA-Z0-9_-]{1,20}",
            "[a-zA-Z0-9 .,!?-]{1,100}",
            any::<bool>(),
            1u32..100,
        )
            .prop_map(|(id, value, is_regex, priority)| SearchTerm {
                id,
                value,
                operator: QueryOperator::And,
                source: TermSource::User,
                preset_group_id: None,
                is_regex,
                priority,
                enabled: true,
                case_sensitive: false,
            })
    }

    /// Generate potentially malicious paths for security testing
    pub fn malicious_path() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("../../../etc/passwd".to_string()),
            Just("..\\..\\..\\windows\\system32\\config\\sam".to_string()),
            Just("/dev/null".to_string()),
            Just("CON".to_string()),
            Just("file\0with\0nulls".to_string()),
            "[.]{1,10}/[a-zA-Z0-9/]{1,50}",
        ]
    }

    /// Generate Unicode strings for internationalization testing
    pub fn unicode_string() -> impl Strategy<Value = String> {
        prop_oneof![
            "[a-zA-Z0-9 ]{1,50}",
            "[\u{4e00}-\u{9fff}]{1,20}", // Chinese characters
            "[\u{0600}-\u{06ff}]{1,20}", // Arabic characters
            "[\u{0400}-\u{04ff}]{1,20}", // Cyrillic characters
        ]
    }
}

/// Helper functions for test setup
pub mod helpers {
    use std::path::PathBuf;
    use tempfile::TempDir;

    /// Create a temporary directory for testing
    pub fn create_temp_workspace() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let path = temp_dir.path().to_path_buf();
        (temp_dir, path)
    }

    /// Create test log files with specified content
    pub fn create_test_log_file(dir: &std::path::Path, name: &str, content: &str) -> PathBuf {
        let file_path = dir.join(name);
        std::fs::write(&file_path, content).expect("Failed to write test file");
        file_path
    }

    /// Generate realistic log content for testing
    pub fn generate_log_content(lines: usize) -> String {
        (0..lines)
            .map(|i| format!("2024-01-01 12:00:{:02} [INFO] Test log entry {}", i % 60, i))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Performance test utilities
pub mod performance {
    use std::time::{Duration, Instant};

    /// Measure execution time of a function
    pub fn measure_time<F, R>(f: F) -> (R, Duration)
    where
        F: FnOnce() -> R,
    {
        let start = Instant::now();
        let result = f();
        let duration = start.elapsed();
        (result, duration)
    }

    /// Assert that an operation completes within a time limit
    pub fn assert_performance<F, R>(f: F, max_duration: Duration, operation_name: &str) -> R
    where
        F: FnOnce() -> R,
    {
        let (result, duration) = measure_time(f);
        assert!(
            duration <= max_duration,
            "{} took {:?}, expected <= {:?}",
            operation_name,
            duration,
            max_duration
        );
        result
    }
}
