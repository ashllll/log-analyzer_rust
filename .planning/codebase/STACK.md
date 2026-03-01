# Technology Stack

**Analysis Date:** 2026-02-28

## Languages

**Primary:**
- Rust 1.70+ - Core backend logic, search engine, archive handling
- Dart 3.8+ - Flutter frontend UI

**Secondary:**
- TypeScript/JavaScript - Build tooling and frontend helpers

## Runtime

**Environment:**
- Tauri 2.0.0 - Desktop application runtime
- Flutter SDK >=3.8.0 <4.0.0
- Node.js 22.12.0+ (development/build)

**Package Manager:**
- Cargo (Rust) - Lockfile: `Cargo.lock`
- Flutter pub (Dart) - Lockfile: `pubspec.lock`
- npm (JavaScript) - Lockfile: `package-lock.json`

## Frameworks

**Core:**
- Tauri 2.0.0 - Cross-platform desktop application framework
- Flutter 3.8+ - UI framework with Riverpod state management

**Search/Indexing:**
- Tantivy 0.22 - Full-text search engine (Rust-native Lucene)
- Aho-Corasick 1.1 - Multi-pattern string matching algorithm
- regex-automata 0.4 - High-performance DFA regex engine
- Roaring 0.10 - Efficient bitmap index for search results

**Database:**
- SQLite with sqlx 0.7 - Async SQL toolkit with FTS5 full-text search

**Async/Concurrency:**
- tokio 1.x - Asynchronous runtime for Rust
- parking_lot 0.12 - High-performance locking
- crossbeam 0.8 - Lock-free data structures
- DashMap 5.5 - Concurrent hash map

**Caching:**
- Moka 0.12 - Enterprise-grade caching (future + sync)
- LRU 0.12 - LRU cache for search results

**Compression/Archive:**
- zip 0.6 - ZIP format support
- tar 0.4 / tokio-tar 0.3 - TAR format support
- flate2 1.0 - GZIP compression/decompression
- unrar 0.5 - RAR format (libunrar C bindings)
- sevenz-rust 0.5 - Pure Rust 7z support
- async_zip 0.0.17 - Async ZIP processing

**Error Handling & Logging:**
- thiserror 1.0 - Error handling derive macros
- eyre 0.6 / color-eyre 0.6 - Modern error handling
- miette 5.0 - User-friendly error diagnostics
- tracing 0.1 - Structured logging and tracing
- tracing-subscriber 0.3 - Log subscription (env-filter, json)
- sentry 0.32 - Error monitoring

**Observability:**
- prometheus 0.13 - Metrics collection
- metrics 0.22 - Application metrics
- OpenTelemetry (optional) - Distributed tracing

**Flutter FFI:**
- flutter_rust_bridge 2.11.1 - Rust-Dart FFI bridge

**HTTP Server:**
- axum 0.7 - HTTP server (for Flutter communication)
- tower 0.4 / tower-http 0.5 - HTTP middleware (CORS)

**File System:**
- notify 6.1 - File system watching
- dunce 1.0 - Windows path normalization (UNC paths)
- walkdir 2.4 - Directory traversal

**Testing:**
- Rust: rstest 0.18, proptest 1.4, criterion 0.5, tokio-test 0.4
- Flutter: flutter_test (built-in)

**Plugin System:**
- libloading 0.8 - Dynamic library loading

**Rate Limiting:**
- governor 0.6 - Rate limiter

## Key Dependencies

**Critical:**
- `tauri` 2.0.0 - Desktop framework
- `flutter_rust_bridge` =2.11.1 - FFI bridge
- `tantivy` 0.22 - Full-text search
- `sqlx` 0.7 - Database (SQLite + FTS5)
- `tokio` 1.x - Async runtime
- `aho-corasick` 1.1 - Multi-pattern matching

**Internal Crates:**
- `log-lexer` - Log lexer (trait definitions)
- `log-lexer-derive` - Procedural macros for log lexer

## Configuration

**Environment:**
- `.env` files for local development (never committed with secrets)
- TOML configuration files for runtime settings
- Application data stored in platform-specific directories:
  - Windows: `%APPDATA%/com.joeash.log-analyzer/workspaces/`
  - macOS: `~/Library/Application Support/com.joeash.log-analyzer/workspaces/`
  - Linux: `~/.local/share/com.joeash.log-analyzer/workspaces/`

**Build:**
- `Cargo.toml` - Rust dependencies and build config
- `tauri.conf.json` - Tauri build configuration
- `pubspec.yaml` - Flutter dependencies
- `package.json` - npm dependencies
- `frb_codegen.yaml` - Flutter Rust Bridge code generation config
- `analysis_options.yaml` - Flutter analysis settings

**Features (Cargo.toml):**
- `default` - Default (no FFI)
- `ffi` - Enable Flutter FFI bridge
- `rar` - Enable RAR decompression support
- `telemetry` - Enable OpenTelemetry

## Platform Requirements

**Development:**
- Node.js 22.12.0+
- npm 10.0+
- Rust 1.70+ (MSVC toolchain on Windows)
- Flutter SDK >=3.8.0
- Tauri prerequisites (GTK3/4 on Linux, Xcode on macOS, MSVC Build Tools on Windows)

**Production:**
- Desktop application (Windows/macOS/Linux)
- SQLite database (embedded)
- File system access for workspace storage

---

*Stack analysis: 2026-02-28*
