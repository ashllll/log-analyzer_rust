# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Log Analyzer is a high-performance desktop log analysis tool built with Rust + Flutter. It provides powerful search capabilities, multi-format archive support, and a modern UI.

**Architecture**: Flutter (frontend) + Rust (backend via flutter_rust_bridge FFI)

## Environment Requirements

- **Node.js**: 22.12.0+
- **Rust**: 1.70+ (MSVC toolchain on Windows)
- **Flutter**: 3.27+ / Dart 3.6+
- **System dependencies**: Tauri prerequisites (GTK3/GTK4 dev libs, Xcode CLT, or MSVC Build Tools)

## Development Commands

### Rust Backend (log-analyzer/src-tauri)

```bash
cd log-analyzer/src-tauri

# Run all tests
cargo test --all-features

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_function_name -- --exact
cargo test pattern_matcher  # module tests

# Run property tests
cargo test --all-features -- proptest

# Code quality
cargo fmt
cargo fmt -- --check
cargo clippy -- -D warnings
cargo clippy --all-features --all-targets -- -D warnings

# Performance benchmarks
cargo bench
```

### Flutter Frontend (log-analyzer_flutter)

```bash
cd log-analyzer_flutter

# Install dependencies
flutter pub get

# Run app
flutter run -d macos   # or -d windows / -d linux

# Code generation (freezed, riverpod, etc.)
flutter pub run build_runner build --delete-conflicting-outputs

# Code quality
flutter analyze

# Tests
flutter test

# Build release
flutter build macos --release
```

### FFI Bridge Code Generation

```bash
cd log-analyzer/src-tauri
flutter_rust_bridge_codegen generate
```

## Project Structure

```
log-analyzer_rust/
├── log-analyzer/               # Rust backend (Tauri + FFI)
│   └── src-tauri/
│       ├── src/
│       │   ├── application/    # Command handlers, services
│       │   ├── domain/         # Business logic (search, export, log_analysis)
│       │   ├── infrastructure/ # Config, persistence, external services
│       │   ├── archive/        # Multi-format archive processing
│       │   ├── search_engine/  # Tantivy search, Aho-Corasick matching
│       │   ├── storage/        # CAS storage, SQLite metadata
│       │   ├── services/       # Pattern matching, query execution
│       │   ├── commands/       # Tauri/FFI command interfaces
│       │   ├── ffi/            # flutter_rust_bridge integration
│       │   ├── security/       # Path validation, plugin sandboxing
│       │   ├── task_manager/   # Async task actor model
│       │   ├── monitoring/     # Metrics, tracing
│       │   └── error.rs        # Unified error handling (thiserror)
│       ├── crates/             # Local crates (log-lexer)
│       ├── tests/              # Integration tests
│       └── benches/            # Performance benchmarks
│
├── log-analyzer_flutter/       # Flutter frontend
│   ├── lib/
│   │   ├── core/               # Router, theme, constants
│   │   ├── features/           # Search, workspace, keyword, task modules
│   │   ├── shared/             # Models, providers (Riverpod), services
│   │   └── l10n/               # i18n (en/zh)
│   └── test/
│
└── docs/                       # Architecture and API documentation
```

## Key Architecture Patterns

### Backend Layers
- **Commands** (Tauri/FFI) → **Handlers** → **Services** → **Domain** → **Infrastructure**
- Use `thiserror` for all error types via `AppError`
- Async I/O via `tokio`, parallel processing via `rayon`
- CAS storage with SHA-256 content addressing for file deduplication

### Frontend Layers
- **Features**: Feature-based directory structure (search/, workspace/, etc.)
- **State**: Riverpod 3.0 providers for all state management
- **Services**: API service for FFI communication, EventBus for real-time updates

### Import Order (Rust)
1. Standard library (`std::`, `core::`)
2. External crates (`tokio::`, `serde::`, `tauri::`)
3. Internal modules (`crate::`)
4. Local modules (`super::`, `crate::models::`)

### Import Order (Dart/Flutter)
1. Flutter/SDK imports (`import 'package:flutter/...'`)
2. Third-party packages (`import 'package:riverpod/...'`)
3. Internal modules (`import 'package:log_analyzer_flutter/...'`)
4. Relative imports (`import '../widgets/...'`)

## Critical Coding Rules

### Field Naming Consistency (Iron Law)
Rust field names MUST match JSON/TypeScript field names exactly:
```rust
pub struct TaskInfo {
    pub task_id: String,     // NOT taskId
    pub created_at: DateTime<Utc>,
}
```

### Required Libraries by Use Case
| Need | Use | Never Use |
|------|-----|-----------|
| Timeout control | tokio-util CancellationToken | Manual setTimeout |
| State management | Riverpod (Flutter) | Custom state |
| Multi-pattern search | aho-corasick crate | Regex per line |
| Async retry | tokio-retry | Manual loop+sleep |
| Form validation | validator derive | Manual regex |
| Full-text search | Tantivy | Custom inverted index |
| Error handling | thiserror/eyre/miette | String/Box<dyn Error> |

### Rust Error Handling
- 100% eliminate `unwrap/expect` in production code
- Use `?` for error propagation with context
- All errors flow through `AppError` enum

## Testing Requirements

- **Rust**: 80%+ coverage, rstest + proptest + criterion
- **Flutter**: flutter_test framework
- Run `cargo test --all-features` before committing
- Run `cargo clippy --all-features -- -D warnings` before committing

## Pre-Commit Validation

Run these checks before pushing:
```bash
# Full CI validation
bash scripts/validate-ci.sh
```

This runs: ESLint, TypeScript check, frontend tests, frontend build, Rust fmt, Rust clippy, Rust tests.

## File Format Support

- **Archives**: ZIP, TAR, TAR.GZ, TGZ, GZ, RAR (via libunrar), 7Z (sevenz-rust)
- **Recursive extraction**: Up to 7 levels of nested archives
- **Safety limits**: 100MB per file, 1GB total, 1000 files max

## Security Features

- Path traversal prevention via recursive validation
- O_EXCL atomic writes to eliminate TOCTOU races
- Plugin directory whitelist + ABI version verification
- Circuit breaker for panic recovery and poisoned lock handling

## Project-Specific Rules

1. **Offline-first**: All features must work completely offline
2. **Language**: Chinese for documentation, comments, and communication
3. **Ask when uncertain**: Don't guess—ask for clarification
4. **No simple fixes**: Must use industry-standard solutions (see Required Libraries table)
5. **Full analysis required**: Analyze code structure comprehensively before making changes
6. **Task breakdown**: Split work into minimal executable steps
