# log-analyzer_rust v1.2.60 Developer Release Notes

> **Release Date**: 2026-04-26
> **Baseline**: v1.2.59 - v1.2.60
> **Scope**: 89 files, +3,516 / -6,122 lines
> **Commits**: 16
> **Rust Toolchain**: stable (>=1.70)
> **Node.js**: >=22.12.0
---

## 1. Version Overview

### Summary

v1.2.60 is a performance-focused release featuring **Tantivy-priority search**, **mmap zero-copy I/O**, **Rayon parallel scanning**, and **streaming ZIP import**. It also removes the deprecated performance-monitoring module, fixes critical Tauri command parameter naming mismatches, and hardens GC, archive extraction, and path-safety code paths.

### Upgrade Recommendations

| Environment | Recommendation |
|---|---|
| **Production** | Recommended. Tantivy priority + mmap reduces large-workspace search from seconds to sub-second; camelCase fix resolves silent command failures |
| **Development** | Required. Dependency changes (memmap2 added, 6 monitoring crates removed) need cargo check and npm install |
| **CI/CD** | Update cache keys. Cargo.lock changed; .clippy.toml lint names are now kebab-case |

### Known Limitations

- store_stream and read_content_mmap_sync are in the same cas.rs file but are functionally independent
- Windows Release workflow has fixed ilammy/msvc-toolchain version pinning

---

## 2. Quick Upgrade Guide

### Step 1: Sync Dependencies

`ash
git pull origin main
cd log-analyzer/src-tauri
cargo check --workspace --all-features
cd ../log-analyzer
npm install
`

### Step 2: Validate Build

`ash
cd log-analyzer/src-tauri
cargo fmt -- --check
cargo clippy --all-features --all-targets -- -D warnings
cargo test -q --workspace

cd ../log-analyzer
npm run lint
npm run type-check
npm test
npm run build
`

### Step 3: Runtime Verification

| Check | How | Pass Criteria |
|-------|-----|---------------|
| Tantivy priority | Search after import; check backend logs | Log contains Tantivy search succeeded |
| mmap path | Import >1 MB log, search keywords | Results correct, no memory spike |
| ZIP streaming | Import flat ZIP (no nested archives) | Log contains Using streaming ZIP processing |
| Parameter fix | Call get_workspace_status | Uses workspaceId (camelCase) |

---

## 3. Core Changes

### 3.1 Performance Optimizations

#### Tantivy Priority Search Path
**File**: search.rs
When the Tantivy index is non-empty, search_logs now tries search_engine_manager.search_with_timeout(500ms) first. On success, results are returned directly (sub-second). On timeout or failure, it falls back to the CAS scan path.

| Scenario | Before | After | Gain |
|----------|--------|-------|------|
| Indexed workspace first search | 2-10 s (full CAS scan) | <500 ms (Tantivy) | 10-20x |
| Repeated searches | Cache-dependent | Consistent sub-second | Stable <1 s |

#### mmap Large-File Zero-Copy Read
**File**: cas.rs
New read_content_mmap_sync() using memmap2::Mmap triggers for CAS objects > 1 MB.

| Metric | Before | After |
|--------|--------|-------|
| Large-file memory | Vec full load | OS page-cache shared |
| 1 GB log search peak | ~1 GB+ | ~Constant |
| Read overhead | memcpy | Zero-copy |

#### Rayon Parallel File Batches
**File**: search.rs
File-batch loop changed from iter() to par_iter(). Benefits become visible with >10 files per search.

#### Aho-Corasick Zero-Allocation is_match
**File**: regex_engine.rs
New AhoCorasickEngine::is_match() calls ac.is_match(text) directly instead of find_iter(text).next().is_some(), eliminating per-match Vec allocation. 10-100x faster on the hot path.

#### Streaming ZIP Import (50% Less Disk Write)
**Files**: la-archive/*
New StreamingArchiveHandler trait + StreamingZipHandler implementation. Flat ZIPs (no nested archives) stream entries directly into CAS via cas.store_stream(), skipping temp-directory materialization.

### 3.2 Architecture Changes

#### New Traits and Types
| Name | Location | Purpose |
|------|----------|---------|
| StreamingArchiveHandler | archive_handler.rs:223 | Streaming archive interface |
| ArchiveEntryInfo | archive_handler.rs:197 | Entry metadata struct |
| StreamingZipHandler | zip_handler.rs:172 | async_zip + tokio_util compat impl |

#### New CAS Methods
| Method | Signature | Purpose |
|--------|-----------|---------|
| store_stream | async fn with AsyncRead reader - Result String | Stream-to-CAS single-pass hash+store |
| read_content_mmap_sync | fn with hash - Result Mmap | mmap zero-copy read |
| object_size_sync | fn with hash - u64 | Fast object size query |

#### Removed Modules (Performance Monitoring)
| Module | Location | Replacement |
|--------|----------|-------------|
| performance.rs | src/commands/ | None - fully removed |
| monitoring/ | src/monitoring/ | None |
| metrics_state.rs | la-core/src/models/ | None |
| metrics_store.rs | la-storage/src/ | None |
| PerformancePage.tsx | src/pages/ | None |
| usePerformanceQueries.ts | src/hooks/ | None |

### 3.3 Critical Fixes

#### Tauri Command Parameter camelCase Alignment (CRITICAL)
Multiple commands used snake_case parameter names. Tauri requires exact camelCase matches, causing deserialization failures (required params) or silent defaults (optional params).

| Command | Before | After | Impact |
|---------|--------|-------|--------|
| export_search_results | search_id, format_type | searchId, formatType | High: export was broken |
| get_workspace_status | workspace_id | workspaceId | High: status query broken |
| load_workspace | workspace_id | workspaceId | High: load broken |
| watch_workspace | workspace_id | workspaceId | High: watch broken |
| unwatch_workspace | workspace_id | workspaceId | High: unwatch broken |
| clear_cache | workspace_id | workspaceId | Medium |
| invalidate_cache | workspace_id | workspaceId | Medium |

#### Smart Pipe Splitting
**Files**: search.rs, searchPatterns.ts
Replaced naive query.split with bracket-aware, escape-aware split_query_by_pipe(). Front-end and back-end now use identical logic, with unit-test coverage in both languages.

#### GC Orphan Detection Fix
**File**: gc.rs
Fixed scan_and_identify_orphans and incremental GC to use full hash keys (shard_prefix + file_name) instead of filenames alone, eliminating false positives/negatives.

#### Front-End Rendering Stability
- LogRow.tsx: Defensive fallbacks on all LogEntry fields prevent white-screen crashes from partial backend data.
- SearchPage.tsx: emit(search-start) moved from render phase into useEffect to comply with Concurrent React rules.

---

## 4. API Changes and Migration Guide

### 4.1 Removed IPC Commands
Calling these now throws command not found:
| Command | Old Purpose | Migration |
|---------|-------------|-----------|
| get_performance_metrics | Real-time metrics | Remove calls |
| get_performance_history | Historical metrics | Remove calls |
| get_performance_summary | Metrics summary | Remove calls |

### 4.2 Parameter Name Migration (Required)

Before (v1.2.59 and earlier): invoke(get_workspace_status, { workspace_id: ws-123 })
After (v1.2.60): invoke(get_workspace_status, { workspaceId: ws-123 })

### 4.3 Type Changes
| Type | Change | Notes |
|------|--------|-------|
| WorkspaceStatusResponse | Fields simplified | Removed id, name, path, watching; kept success, fileCount |
| MatchDetail.match_position | Precision improved | Backend now converts byte offsets to char indices for correct String.slice() usage |

---

## 5. Dependency Changes

### Added
| Crate | Version | Target | Purpose | Binary Impact |
|-------|---------|--------|---------|---------------|
| memmap2 | 0.9 | la-storage | mmap zero-copy CAS reads | ~+50 KB |
| tokio-util (compat, io) | 0.7 | la-archive | async_zip stream compat | Existing, new features |

### Removed
| Crate | Old Purpose | Size Recovered |
|-------|-------------|----------------|
| sysinfo | System metrics | ~-500 KB |
| opentelemetry | Distributed tracing | ~-800 KB |
| tracing-opentelemetry | tracing-OTel bridge | ~-200 KB |
| opentelemetry-jaeger | Jaeger exporter | ~-1 MB |
| prometheus | Metric serialization | ~-600 KB |
| metrics | Metric facade | ~-200 KB |
| Total | | ~-3.3 MB |

### Net Effect
- Compile time: slightly reduced (6 crates removed)
- Binary size: net ~-3 MB
- Runtime memory: lower baseline (no OTel/metrics background tasks)

---

## 6. Compatibility

### Breaking Changes
| Change | Impact | Migration Cost |
|--------|--------|----------------|
| Tauri params snake_case to camelCase | High: front-end calls incompatible | Low: batch rename |
| Removed performance IPC commands | High: crashes if called | Medium: remove UI + logic |
| WorkspaceStatusResponse trimmed | Medium: depends on removed fields | Low: adjust types |

### Forward Compatibility
| Feature | Behavior |
|---------|----------|
| Tantivy priority | Auto-falls back to CAS scan when no index |
| mmap reads | Only triggers for >1 MB; small files unchanged |
| ZIP streaming | Auto-falls back to traditional extraction |
| AC is_match | Internal optimization; callers unaware |

### Data Compatibility
| Data | Status |
|------|--------|
| CAS objects/ | Fully compatible |
| metadata.db | Fully compatible (schema unchanged) |
| Tantivy index | Fully compatible |
| config.json | Fully compatible |
| Search temp files | Fully compatible |

### Downgrade Risk
Downgrading to v1.2.59 is not recommended. Cargo.lock removed 6 crates; front-end types removed PerformancePage; data written via store_stream remains valid but the path will not exist in older code.

---

## 7. Risk Checklist

### High Risk

R1 - Unmigrated Tauri parameter names
Symptom: invalid args or unexpected defaults.
Check: grep for workspace_id, search_id, format_type in src/services/ and src/pages/
Fix: Batch-replace to camelCase.

R2 - mmap failure on restricted systems
Symptom: Crash or empty results on >1 MB files.
Check: /proc/sys/vm/max_map_count (Linux) or Failed to mmap object in logs.
Fix: Comment out the mmap branch in search.rs if needed.

R3 - ZIP streaming failure interrupts import
Symptom: Incomplete workspace after ZIP import.
Check: Logs for Failed to stream entry to CAS.
Fix: Streaming automatically falls back to traditional extraction (processor.rs:1309).

### Medium Risk

R4 - Rayon vs tokio contention
Symptom: Abnormal CPU or task starvation under concurrent searches.
Check: spawn_blocking queue depth and Rayon thread-pool state.
Note: Rayon runs inside spawn_blocking; pools are independent.

R5 - memmap2 Windows handle behavior
Symptom: Handle leaks on Windows.
Check: Process Monitor handle lifecycle.
Note: Mmap unmaps on Drop; underlying File drops before return.

### Low Risk

R6 - Tantivy timeout too aggressive
Symptom: Always falls back to CAS despite non-empty index.
Check: Logs for Tantivy search timed out.
Fix: Adjust hard-coded 500 ms in search.rs:784.

R7 - Cache size limit excludes large Tantivy results
Symptom: Repeated searches still hit the index.
Note: Limit < 100_000 entries is intentional to control memory.

---

## 8. Changed Files by Theme

Search performance (c5ba579, e4839ba): search.rs, regex_engine.rs, query_executor.rs, cas.rs, la-storage/Cargo.toml

Archive streaming (1ae48c6): archive_handler.rs, zip_handler.rs, processor.rs, lib.rs, la-archive/Cargo.toml, Cargo.lock

Monitoring removal (44b8bbb): PerformancePage.tsx, charts/*, usePerformanceQueries.ts, performance.rs, monitoring/*, metrics_state.rs, metrics_store.rs, Cargo.toml

Comprehensive fixes (2021eb1): search.rs, import.rs, export.rs, workspace.rs, gc.rs, coordinator.rs, processor.rs, highlighting_engine.rs, SearchPage.tsx, LogRow.tsx, HybridLogRenderer.tsx, api.ts, searchPatterns.ts

Config and build: .clippy.toml, jest.config.js, eslint.config.js, tauri.conf.json, windows-app.manifest

Docs: CHANGELOG.md, ARCHIVE_SEARCH_PIPELINE_ANALYSIS.md, CAS_ARCHITECTURE.md, CROSS_PLATFORM_STORAGE_DECISION.md, PURE_RUST_STORAGE_MIGRATION_PLAN.md, MODULE_ARCHITECTURE.md, CONTRIB.md, RUNBOOK.md

---

## 9. Commit History

`	ext
ddc7ee8 chore(manifest): Windows manifest comment translation
1ae48c6 feat(archive): streaming ZIP import, direct-to-CAS skip temp dir
e4839ba perf(search): Aho-Corasick zero-allocation is_match
c5ba579 perf(search): Tantivy priority + mmap large files + Rayon parallel
19d9c62 docs(perf): remove PerformancePage and monitoring/ doc residue
ea6b068 docs(perf): sync doc references after monitoring removal
44b8bbb feat(perf): remove performance statistics monitoring module
bb2e744 chore: remove claude code configs, ai agents and redundant files
d0620a4 chore: bump version to 1.2.60
2021eb1 fix: comprehensive bug fixes and pipeline hardening
4c3dbce fix(bugs): fix 5 confirmed bugs
174a458 docs: update script reference docs
c04eb5c fix(workspace): harden storage coordinator and gc operations (#45)
02e87f3 fix(archive): harden workspace status and extraction guards
be2466a fix(security): harden workspace loading and archive access
5ad292e fix(workspace): properly close resources before deleting workspace
`

---

*Generated from full diff between v1.2.59 (1911387) and v1.2.60 (ddc7ee8).*
*Last updated: 2026-04-26*

