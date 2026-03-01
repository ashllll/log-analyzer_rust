# Architecture

**Analysis Date:** 2026-02-28

## Pattern Overview

**Overall:** Layered Architecture with Domain-Driven Design (DDD) and CQRS patterns

**Key Characteristics:**
- **Layered Architecture**: Commands, Application, Domain, Infrastructure, Storage layers
- **Domain-Driven Design (DDD)**: Entities, Value Objects, Domain Services, Repositories
- **CQRS Pattern**: Separation of Commands and Queries in application layer
- **Actor Model**: Task management using Actor pattern with message passing
- **Strategy Pattern**: Archive handlers (ZIP/TAR/GZ/RAR/7Z) as pluggable strategies
- **Builder Pattern**: ProcessBuilder for parameter management in archive processing

## Layers

**Commands Layer:**
- Purpose: Tauri command handlers exposed to frontend
- Location: `log-analyzer/src-tauri/src/commands/`
- Contains: Command handlers (search, import, workspace, config, etc.)
- Depends on: Services, Models
- Used by: Frontend (via Tauri invoke)

**Application Layer:**
- Purpose: Use case orchestration, CQRS pattern implementation
- Location: `log-analyzer/src-tauri/src/application/`
- Contains: Commands, Queries, Handlers, Application Services
- Depends on: Domain, Infrastructure
- Used by: Commands Layer

**Domain Layer:**
- Purpose: Core business logic, entities, value objects
- Location: `log-analyzer/src-tauri/src/domain/`
- Contains: log_analysis, search, export, shared modules
- Depends on: None (pure business logic)
- Used by: Application Layer

**Services Layer:**
- Purpose: Core business services (PatternMatcher, QueryExecutor, FileWatcher)
- Location: `log-analyzer/src-tauri/src/services/`
- Contains: Pattern matching, query execution, file watching, statistics
- Depends on: Models, Storage
- Used by: Commands, Application

**Search Engine Layer:**
- Purpose: Tantivy-based full-text search engine
- Location: `log-analyzer/src-tauri/src/search_engine/`
- Contains: Index management, DFA engine, Roaring index, concurrent search
- Depends on: tantivy, roaring
- Used by: Services, Commands

**Archive Layer:**
- Purpose: Multi-format archive extraction (ZIP/TAR/GZ/RAR/7Z)
- Location: `log-analyzer/src-tauri/src/archive/`
- Contains: Handlers, extraction engine, actors, streaming
- Depends on: zip, tar, flate2, unrar, sevenz-rust
- Used by: Commands, Services

**Storage Layer:**
- Purpose: Content-Addressable Storage (CAS) and metadata management
- Location: `log-analyzer/src-tauri/src/storage/`
- Contains: CAS, metadata store, integrity verification, metrics
- Depends on: sqlx, sha2
- Used by: Services, Commands

**Infrastructure Layer:**
- Purpose: Technical implementations (config, persistence, messaging, external)
- Location: `log-analyzer/src-tauri/src/infrastructure/`
- Contains: Config management, JSON file storage, event bus, external services
- Depends on: config, toml
- Used by: All layers

**Models Layer:**
- Purpose: Data structures and state management
- Location: `log-analyzer/src-tauri/src/models/`
- Contains: Search, State, Config, Filters, LogEntry models
- Depends on: serde
- Used by: All layers

**Task Manager:**
- Purpose: Task lifecycle management using Actor model
- Location: `log-analyzer/src-tauri/src/task_manager/`
- Contains: TaskManager, TaskStatus, TaskInfo
- Depends on: tokio
- Used by: Commands

**Monitoring Layer:**
- Purpose: Observability and metrics collection
- Location: `log-analyzer/src-tauri/src/monitoring/`
- Contains: Metrics collection, tracing
- Depends on: prometheus, metrics, tracing
- Used by: All layers

**Security Layer:**
- Purpose: Security features (validation, sanitization)
- Location: `log-analyzer/src-tauri/src/security/`
- Contains: Security validators
- Depends on: sanitize-filename
- Used by: Archive, Commands

**FFI Layer:**
- Purpose: Flutter FFI bridge for cross-language calls
- Location: `log-analyzer/src-tauri/src/ffi/`
- Contains: FFI bindings
- Depends on: flutter_rust_bridge
- Used by: Flutter frontend

## Data Flow

**Search Flow:**

1. Frontend invokes `search_logs` command
2. Command layer validates and delegates to QueryExecutor
3. QueryExecutor uses Validator/Planner/Executor pattern:
   - QueryValidator: Validates search terms
   - QueryPlanner: Builds execution plan with optimizations
   - PatternMatcher: Aho-Corasick multi-pattern matching
4. SearchEngineManager builds Tantivy index if needed
5. Results returned through EventBus to frontend

**Import Flow:**

1. Frontend invokes `import_folder` command
2. Archive extraction (if compressed) via ArchiveManager
3. CAS storage stores content by SHA-256 hash
4. MetadataStore saves file metadata to SQLite
5. Search index built/updated via SearchEngineManager
6. TaskManager tracks progress and emits events

**Task Management Flow:**

1. TaskManagerActor receives CreateTask message
2. Creates TaskInfo with unique task_id
3. Emits task-update event to frontend
4. Handles UpdateTask messages for progress
5. Auto-cleanup of expired tasks

## Key Abstractions

**ArchiveHandler Trait:**
- Purpose: Unified interface for archive extraction
- Examples: `log-analyzer/src-tauri/src/archive/zip_handler.rs`, `tar_handler.rs`, `gz_handler.rs`, `rar_handler.rs`, `sevenz_handler.rs`
- Pattern: Strategy Pattern with async_trait

**QueryExecutor:**
- Purpose: Coordinates query validation, planning, and execution
- Examples: `log-analyzer/src-tauri/src/services/query_executor.rs`
- Pattern: Facade Pattern with Validator/Planner/Executor

**PatternMatcher:**
- Purpose: Aho-Corasick multi-pattern matching
- Examples: `log-analyzer/src-tauri/src/services/pattern_matcher.rs`
- Pattern: Strategy Pattern with compiled automaton

**ContentAddressableStorage:**
- Purpose: SHA-256 based file storage avoiding path limits
- Examples: `log-analyzer/src-tauri/src/storage/cas.rs`
- Pattern: Repository Pattern

**TaskManager:**
- Purpose: Actor-based task lifecycle management
- Examples: `log-analyzer/src-tauri/src/task_manager/mod.rs`
- Pattern: Actor Model with message passing

## Entry Points

**Main Entry:**
- Location: `log-analyzer/src-tauri/src/main.rs`
- Triggers: Application startup
- Responsibilities: Initialize logging, configure Tauri, register commands, setup TaskManager, start HTTP API server, initialize FFI context

**Library Entry:**
- Location: `log-analyzer/src-tauri/src/lib.rs`
- Triggers: Library initialization
- Responsibilities: Export all modules, define public API

**HTTP API Server:**
- Location: `log-analyzer/src-tauri/src/commands/http_api/`
- Triggers: Flutter HTTP calls
- Responsibilities: REST API for Flutter communication

## Error Handling

**Strategy:** thiserror-based AppError enum with context

**Patterns:**
- `AppError::Search` - Search operation errors
- `AppError::Archive` - Archive extraction errors
- `AppError::Storage` - Storage/CAS errors
- `AppError::Validation` - Input validation errors
- Context chain using `wrap_err` from eyre
- Frontend-friendly error messages via miette

## Cross-Cutting Concerns

**Logging:** tracing + tracing-subscriber with JSON output and file rotation

**Validation:** validator crate with derive macros + custom validation in services

**Authentication:** Not applicable (desktop app with local storage)

**Configuration:** config crate with TOML file support + environment variable override

**Metrics:** prometheus + metrics crates for observability

---

*Architecture analysis: 2026-02-28*
