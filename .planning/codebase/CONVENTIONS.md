# Coding Conventions

**Analysis Date:** 2026-02-28

## Naming Patterns

**Files:**
- Rust: `snake_case.rs` (e.g., `pattern_matcher.rs`, `query_validator.rs`)
- Dart/Flutter: `snake_case.dart` (e.g., `search_query_builder.dart`)

**Modules (Rust):**
- `snake_case` for directories (e.g., `search_engine/`, `archive/`)
- `mod.rs` for module roots

**Functions:**
- `snake_case` (e.g., `matches_all()`, `validate_query()`)
- Private methods prefixed with underscore for clarity: `fn _internal_method()`

**Variables:**
- `snake_case` (e.g., `case_insensitive`, `max_results`)
- Constants: `SCREAMING_SNAKE_CASE` (e.g., `MAX_FILE_SIZE`)

**Types:**
- `CamelCase` for structs, enums, traits (e.g., `PatternMatcher`, `AppError`, `TaskStatus`)
- Type aliases: `snake_case` (e.g., `pub type Result<T> = crate::error::Result<T>;`)

**Generics:**
- `CamelCase` (e.g., `T`, `Result<T>`, `Vec<T>`)

## Code Style

**Formatting:**
- Tool: `cargo fmt` (Rust), Flutter analyzer (Dart)
- Configuration: `.rustfmt.toml` at project root
- Line length: Default (typically 100 characters)
- Use `cargo fmt` before commits

**Linting:**
- Rust: `cargo clippy --all-features --all-targets -- -D warnings`
- Dart: `flutter analyze` with `analysis_options.yaml`
- Strict mode in Flutter: `missing_required_param: error`, `missing_return: error`

**Indentation:**
- 4 spaces for Rust
- 2 spaces for Dart/Flutter

## Import Organization

**Rust (module-relative):**
```rust
// Standard library
use std::collections::HashMap;
use std::path::PathBuf;

// External crates
use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};
use tokio::fs;
use serde::{Deserialize, Serialize};

// Internal modules
use crate::error::{AppError, Result};
use crate::models::search::*;
```

**Order:**
1. Standard library
2. External crates (alphabetical)
3. Internal modules (`crate::`, `super::`, `self::`)

**Path Aliases:**
- `crate::` for crate root
- `super::` for parent module
- `self::` for current module

## Error Handling

**Primary Pattern: thiserror + miette**

Located in `log-analyzer/src-tauri/src/error.rs`:

```rust
use thiserror::Error;
use miette::Diagnostic;

/// 应用错误类型 - 使用 miette 提供用户友好的错误诊断
#[derive(Error, Debug, Diagnostic)]
pub enum AppError {
    #[error("Search error: {_message}")]
    #[diagnostic(
        code(app::search_error),
        help("Try simplifying your search query or checking the workspace status")
    )]
    Search {
        _message: String,
        #[source]
        _source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("Validation error: {0}")]
    #[diagnostic(
        code(app::validation_error),
        help("Check that your input meets the required format and constraints")
    )]
    Validation(String),
}
```

**Error Construction Helpers:**
```rust
impl AppError {
    pub fn search_error(message: impl Into<String>) -> Self { ... }
    pub fn validation_error(message: impl Into<String>) -> Self { ... }
    pub fn archive_error(message: impl Into<String>, path: Option<PathBuf>) -> Self { ... }
    pub fn with_context(self, context: impl Into<String>) -> Self { ... }
}
```

**Result Type:**
```rust
pub type Result<T> = std::result::Result<T, AppError>;
```

**Propagation:**
- Use `?` operator instead of `unwrap()`/`expect()` in production code
- Use `map_err()` for context enrichment

## Logging

**Framework:** `tracing` (structured logging)

**Levels:**
- `tracing::debug!()` - Detailed information for debugging
- `tracing::info!()` - General operational information
- `tracing::warn!()` - Warning conditions
- `tracing::error!()` - Error conditions

**Structured Logging Example:**
```rust
tracing::info!(
    source = %source_path.display(),
    encoding = %detection.encoding_name,
    needs_transcoding = detection.needs_transcoding,
    "Starting file transcoding"
);
```

**Location:** Throughout codebase (e.g., `log-analyzer/src-tauri/src/utils/transcoding_pipe.rs`)

## Comments

**Rust Doc Comments:**
- Public API: Use `///` for documentation
- Include: Purpose, parameters, return values, examples

```rust
/**
 * 模式匹配器 - 使用Aho-Corasick算法进行高效多模式匹配
 *
 * 该匹配器执行子串匹配，不是单词边界匹配。
 */
pub struct PatternMatcher { ... }
```

**Module Documentation:**
- Use `//!` at module level for crate/module docs

**When to Comment:**
- Complex algorithms and business logic
- Non-obvious decisions or workarounds
- TODO/FIXME with explanation
- Public API interfaces

**Flutter/Dart:**
- Use `///` for public API documentation
- Comments in Chinese (as per project language setting)

## Function Design

**Size:** Keep functions focused and small (< 50 lines preferred)

**Parameters:**
- Use struct for > 3 parameters (builder pattern if needed)
- Document complex parameter requirements
- Use `&str` for string slices, `String` for owned strings

**Return Values:**
- Return `Result<T>` for fallible operations
- Return `Option<T>` for optional values
- Avoid bare error values

**Async:**
- Use `async fn` with tokio runtime
- Document blocking vs non-blocking behavior

## Module Design

**Public API:**
- Re-export from parent module's `mod.rs`
- Use `pub use` for clean public interfaces

**Barrel Files:**
- Use `mod.rs` as barrel file for module exports

**Module Hierarchy:**
```
src/
├── lib.rs           # Crate root, public exports
├── error.rs        # Error types
├── commands/       # Tauri commands
├── services/       # Business logic
├── models/         # Data models
├── archive/        # Archive handling
└── ...
```

## Flutter/Dart Conventions

**File Naming:**
- `snake_case.dart`
- Test files: `*_test.dart`

**Class/Type Naming:**
- `PascalCase` for classes, enums, extensions
- `camelCase` for variables, functions, methods

**Flutter-Specific:**
- Use `const` constructors where possible
- `prefer_const_constructors` rule enabled
- Widgets: Functional components preferred over class-based

**Linter Rules (from `analysis_options.yaml`):**
- `prefer_single_quotes`
- `use_key_in_widget_constructors`
- `always_declare_return_types`
- `avoid_dynamic_calls`

---

*Convention analysis: 2026-02-28*
