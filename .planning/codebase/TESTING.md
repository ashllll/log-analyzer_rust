# Testing Patterns

**Analysis Date:** 2026-02-28

## Test Framework

**Rust Backend:**

**Unit/Integration Testing:**
- `rstest` 0.18 - Parametrized tests
- `proptest` 1.4 - Property-based testing
- Built-in `#[test]` attribute

**Async Testing:**
- `tokio-test` 0.4 - Async test runtime

**Benchmarking:**
- `criterion` 0.5 - Performance benchmarks
- Built-in `#[bench]` attribute

**Flutter/Dart:**
- Flutter test framework (`flutter_test`)
- `flutter_test` for widget testing

**Run Commands:**
```bash
# Run all Rust tests
cargo test --all-features

# Show test output
cargo test -- --nocapture

# Run specific module tests
cargo test pattern_matcher
cargo test query_validator

# Run with coverage
cargo install cargo-tarpaulin
cargo tarpaulin --out html

# Run Flutter tests
cd log-analyzer_flutter
flutter test
```

## Test File Organization

**Location:**
- Unit tests: Inline in source files (`#[cfg(test)] mod tests { ... }`)
- Integration tests: `log-analyzer/src-tauri/tests/` directory

**Naming:**
- Rust unit tests: `mod tests { #[test] fn test_function_name() { ... } }`
- Integration tests: `*_integration_tests.rs`, `*_property_tests.rs`

**Structure:**
```
log-analyzer/src-tauri/
├── src/
│   ├── services/
│   │   ├── pattern_matcher.rs    # Inline unit tests
│   │   └── query_validator.rs   # Inline unit tests
│   └── ...
├── tests/
│   ├── task_manager_integration_tests.rs
│   ├── error_recovery_tests.rs
│   └── archive_processing_integration.rs
└── benches/
    └── m1_benchmark.rs
```

## Test Structure

**Rust Unit Test Example (from `services/pattern_matcher.rs`):**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_matcher_empty_patterns() {
        let matcher = PatternMatcher::new(Vec::new(), false).unwrap();
        assert!(!matcher.matches_all("test text"));
        assert!(!matcher.matches_any("test text"));
    }

    #[test]
    fn test_pattern_matcher_single_pattern() {
        let matcher = PatternMatcher::new(vec!["error".to_string()], false).unwrap();

        // 包含error子串的应该匹配
        assert!(matcher.matches_all("error occurred"));
        assert!(matcher.matches_all("no error here"));

        // 不包含error子串的应该不匹配
        assert!(!matcher.matches_all("no here"));
    }
}
```

**Helper Functions:**
```rust
fn create_test_term(value: &str, enabled: bool) -> SearchTerm {
    SearchTerm {
        id: "test".to_string(),
        value: value.to_string(),
        operator: QueryOperator::And,
        source: TermSource::User,
        preset_group_id: None,
        is_regex: false,
        priority: 1,
        enabled,
        case_sensitive: false,
    }
}
```

## Rstest Patterns

**Parametrized Tests (from `tests/task_manager_integration_tests.rs`):**
```rust
use rstest::*;

#[rstest]
fn test_task_manager_config_creation() {
    let config = TaskManagerConfig::default();
    assert_eq!(config.completed_task_ttl, 300);
}

#[rstest]
fn test_task_status_enum() {
    let statuses = vec![
        TaskStatus::Running,
        TaskStatus::Completed,
        TaskStatus::Failed,
        TaskStatus::Stopped,
    ];

    for status in statuses {
        let serialized = serde_json::to_string(&status);
        assert!(serialized.is_ok());
    }
}
```

## Proptest Patterns

**Property-Based Testing (from `src/proptest_strategies.rs`):**
```rust
use proptest::prelude::*;

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
    /// Generate valid workspace IDs
    pub fn workspace_id() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9_-]{1,50}"
    }

    /// Generate search queries with various patterns
    pub fn search_query() -> impl Strategy<Value = SearchQuery> { ... }
}
```

**Property Test Example (from `search_engine/property_tests.rs`):**
```rust
proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Performance Optimization, Property 1: Search Response Time Guarantee
    #[test]
    fn property_search_response_time_guarantee(
        query in search_query_string(),
        log_entries in prop::collection::vec(search_log_entry(), 1..1000)
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let _ = rt.block_on(async {
            // Test implementation
            prop_assert!(elapsed <= Duration::from_millis(200),
                "Search took {}ms, expected ≤200ms", elapsed.as_millis());
        });
    }
}
```

## Mocking

**What to Mock:**
- File system operations (use `tempfile::TempDir`)
- External services (HTTP, database)
- Time-dependent operations (use `std::time::Duration`)

**Not to Mock:**
- Business logic (test directly)
- Internal library functions
- Simple data transformations

**Test Fixtures:**
```rust
fn create_test_manager() -> (SearchEngineManager, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let index_path = temp_dir.path().join("search_index");
    let config = SearchConfig {
        index_path,
        default_timeout: Duration::from_millis(500),
        ..Default::default()
    };
    let manager = SearchEngineManager::new(config).unwrap();
    (manager, temp_dir)
}
```

## Coverage

**Target:** 80%+ (as per project requirements)

**Coverage Tools:**
- `cargo-tarpaulin` for Rust coverage reporting

**View Coverage:**
```bash
cargo install cargo-tarpaulin
cargo tarpaulin --out html
```

**Key Modules Requiring Coverage:**
- `storage/` - CAS storage, integrity verification
- `archive/` - Archive processing (ZIP/RAR/GZ/TAR/7Z)
- `search_engine/` - Tantivy search, DFA engine, Roaring index
- `services/` - PatternMatcher, QueryExecutor, FileWatcher
- `task_manager/` - Actor model task management
- `domain/` - Domain models, business rules

## Test Types

**Unit Tests:**
- Focus: Single function/module behavior
- Location: Inline (`#[cfg(test)] mod tests`)
- Example: `pattern_matcher.rs`, `query_validator.rs`

**Integration Tests:**
- Focus: Multiple modules working together
- Location: `tests/` directory
- Example: `task_manager_integration_tests.rs`, `archive_processing_integration.rs`

**Property-Based Tests:**
- Focus: Invariant verification across random inputs
- Location: `*_property_tests.rs` files
- Example: `search_engine/property_tests.rs`, `concurrency_property_tests.rs`

**Benchmark Tests:**
- Focus: Performance verification
- Location: `benches/` directory
- Config: `harness = false` in `Cargo.toml`

```rust
// Benchmark example (from Cargo.toml)
[[bench]]
name = "m1_benchmark"
harness = false
```

## Common Patterns

**Async Testing:**
```rust
#[tokio::test]
async fn test_async_file_read() {
    let content = read_file_from_offset(&path, 0, Some(8192)).await;
    assert!(content.is_ok());
}
```

**Error Testing:**
```rust
#[test]
fn test_validate_empty_query() {
    let query = SearchQuery { terms: vec![], ... };
    let result = QueryValidator::validate(&query);
    assert!(result.is_err());

    let error = result.unwrap_err();
    if let AppError::Validation(msg) = &error {
        assert!(msg.contains("empty"));
    }
}
```

**Timeout Handling:**
```rust
use std::time::Duration;

#[tokio::test]
async fn test_timeout() {
    let result = operation_with_timeout(Duration::from_millis(100)).await;
    assert!(result.is_err());
}
```

---

*Testing analysis: 2026-02-28*
