# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**RuFlo V3** - A high-performance desktop log analyzer built with Rust + Tauri + Flutter.

- **Backend**: Rust with Tantivy full-text search engine and Aho-Corasick multi-pattern matching
- **Frontend**: Flutter (cross-platform desktop)
- **Storage**: CAS (Content-Addressable Storage) with SQLite metadata
- **Architecture**: DDD (Domain-Driven Design) with CQRS pattern

## Build & Run Commands

### Rust Backend (src-tauri)

```bash
cd log-analyzer/src-tauri

# Build
cargo build --release

# Run in development
cargo run

# Run tests
cargo test --all-features

# Run specific test
cargo test pattern_matcher

# Lint
cargo clippy --all-features -- -D warnings

# Format check
cargo fmt --check
```

### Flutter Frontend (log-analyzer_flutter)

```bash
cd log-analyzer_flutter

# Run in debug mode
flutter run

# Build for desktop
flutter build linux   # or: flutter build macos, flutter build windows

# Run tests
flutter test

# Analyze
flutter analyze
```

### Tauri Application

```bash
cd log-analyzer

# Development mode
npm run tauri dev

# Build production
npm run tauri build
```

## Directory Structure

```
log-analyzer_rust/
├── log-analyzer/              # Tauri project root
│   ├── src-tauri/            # Rust backend
│   │   ├── src/
│   │   │   ├── commands/     # Tauri command handlers
│   │   │   ├── services/     # Core services (PatternMatcher, QueryExecutor)
│   │   │   ├── search_engine/ # Tantivy search + Roaring index
│   │   │   ├── archive/      # ZIP/TAR/GZ/RAR/7Z handlers
│   │   │   ├── storage/      # CAS + SQLite
│   │   │   ├── domain/      # DDD domain layer
│   │   │   └── models/      # Data models
│   │   └── crates/          # Internal crates (log-lexer)
│   └── src/                  # (legacy or unused)
└── log-analyzer_flutter/     # Flutter frontend
    ├── lib/
    │   ├── features/        # Feature modules (search, archive, settings...)
    │   ├── shared/          # Shared providers, services, widgets
    │   └── core/            # Router, theme, constants
    └── pubspec.yaml
```

## Key Architecture Patterns

### Backend (Rust)

- **CQRS Pattern**: QueryExecutor uses Validator/Planner/Executor separation
- **Aho-Corasick Algorithm**: O(n+m) multi-pattern matching for search
- **Tantivy**: Full-text search engine with indexing
- **Roaring Bitmap**: Compressed result set storage
- **Actor Model**: TaskManager for async task lifecycle

### Frontend (Flutter)

- **Riverpod 3.0**: State management
- **FFI Bridge**: Direct Rust calls via flutter_rust_bridge
- **HTTP API**: Fallback communication (axum server in Rust)
- **Feature-first**: Organized by feature modules (search, archive, settings, etc.)

### Data Flow

1. **Search**: Flutter → FFI/HTTP → Commands → QueryExecutor → Tantivy/PatternMatcher → Results
2. **Import**: Flutter → Archive extraction → CAS storage → SQLite metadata → Index update
3. **Real-time**: FileWatcher → EventBus → Flutter updates

## Key Dependencies

### Rust (Cargo.toml)
- `tauri = "2.0"` - Desktop framework
- `tantivy = "0.22"` - Search engine
- `aho-corasick = "1.1"` - Pattern matching
- `tokio` + `rayon` - Async + parallel processing
- `sqlx` - SQLite database
- `flutter_rust_bridge` - FFI generation

### Flutter (pubspec.yaml)
- `flutter_rust_bridge` - FFI bindings
- `riverpod` + `flutter_riverpod` - State management
- `fl_chart` - Charts (log level distribution)
- `lucide_icons_flutter` - Icons

## Important Files

- `src-tauri/src/main.rs` - Application entry
- `src-tauri/src/lib.rs` - Library entry + module exports
- `src-tauri/src/commands/search.rs` - Search command handler
- `src-tauri/src/services/query_executor.rs` - Query execution
- `src-tauri/CLAUDE.md` - Detailed backend documentation

## Testing

- **Rust**: Unit tests inline in modules (`#[cfg(test)]`), property tests in `*_property_tests.rs`
- **Flutter**: Widget tests with `flutter_test`
- **Integration**: Tauri E2E tests

## Code Style

- Rust: `snake_case` functions, `CamelCase` types, `SCREAMING_SNAKE_CASE` constants
- Dart: Flutter conventions (camelCase, PascalCase classes)
- Both: Run format/lint before commit
