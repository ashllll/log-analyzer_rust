# External Integrations

**Analysis Date:** 2026-02-28

## APIs & External Services

**HTTP Server (Rust Backend):**
- axum 0.7 - Built-in HTTP API server for Flutter communication
  - Port: Configurable (default 8080)
  - CORS: Enabled via tower-http
  - Routes: `/api/workspaces`, `/api/search`, `/api/tasks`, etc.
  - Flutter integration: Via HTTP client (dio) or FFI bridge

**Communication Options:**
1. **FFI Bridge** (Primary): `flutter_rust_bridge` 2.x
   - Direct Rust function calls from Dart
   - Used when FFI is available

2. **HTTP API** (Fallback): axum HTTP server
   - RESTful endpoints for workspace, search, tasks
   - Client: dio 5.4.0 on Flutter side

3. **WebSocket** (Future): tokio-tungstenite 0.21
   - Real-time state synchronization (planned)

## Data Storage

**Database:**
- SQLite with FTS5 full-text search
  - Connection: Embedded SQLite file
  - Location: Application data directory (`workspaces/<workspace_id>/metadata.db`)
  - Client: sqlx 0.7 with runtime-tokio-native-tls
  - Features: FTS5 for full-text search, content-addressable storage (CAS)

**File Storage:**
- Local filesystem (Content-Addressable Storage - CAS)
  - SHA-256 content hashing for deduplication
  - Files stored in: `workspaces/<workspace_id>/store/`
  - Metadata stored in SQLite database
  - No external cloud storage integration

**Caching:**
- In-memory caching only
  - Moka (enterprise-grade, future + sync)
  - LRU cache for search results
  - No Redis or external cache service

## Authentication & Identity

**Auth Provider:**
- None - Local desktop application
  - No user authentication required
  - Workspace-based isolation (multi-workspace support)

## Monitoring & Observability

**Error Tracking:**
- Sentry (optional)
  - Rust: sentry 0.32 (with tracing integration)
  - Flutter: sentry_flutter 8.0.0
  - DSN: Configured via environment variable `SENTRY_DSN`
  - Features: Error monitoring, stack traces, release tracking

**Logs:**
- tracing (Rust) - Structured logging
  - Output: Console + file (via tracing-appender)
  - Formats: Human-readable, JSON
  - Filter: Via `RUST_LOG` environment variable
  - Integration: Sentry breadcrumbs

**Metrics:**
- Prometheus 0.13 - Metrics collection (optional)
- metrics 0.22 - Application-level metrics
- OpenTelemetry (optional) - Distributed tracing

## CI/CD & Deployment

**Hosting:**
- Desktop application (self-hosted)
  - Windows: MSI/EXE installer
  - macOS: DMG/App Linux: AppImage/ bundle
  -DEB

**CI Pipeline:**
- GitHub Actions (assumed from repository structure)
-: cargo Validation fmt, cargo clippy, cargo test

## Environment Configuration

**Required env vars:**
- `SENTRY_DSN` - Sentry error tracking (optional)
- `RUST_LOG` - Logging level filter (e.g., `info`, `debug`)
- `RUST_BACKTRACE` - Backtrace enabled (1)

**Secrets location:**
- `.env` files (development only, never committed)
- Platform-specific secure storage (Windows Credential Manager, macOS Keychain)
- Tauri secure storage plugin (future)

## Webhooks & Callbacks

**Incoming:**
- None - Desktop application, no incoming webhooks

**Outgoing:**
- File system events: notify 6.1 for file watching
  - Automatic incremental index updates
  - Real-time file change detection

## Architecture Notes

**Communication Flow:**
```
Flutter UI (Dart)
    |
    +-- FFI Bridge --> Rust Backend (direct function calls)
    |
    +-- HTTP Client (dio) --> axum HTTP Server --> Rust Backend
    |
    v
SQLite + File System (Content-Addressable Storage)
```

**Key Integrations:**
1. Flutter <-> Rust: flutter_rust_bridge (FFI)
2. Flutter <-> HTTP API: dio HTTP client
3. Rust <-> Search: Tantivy + Aho-Corasick
4. Rust <-> Database: sqlx (SQLite + FTS5)
5. Rust <-> Archives: zip/tar/flate2/unrar/sevenz-rust

---

*Integration audit: 2026-02-28*
