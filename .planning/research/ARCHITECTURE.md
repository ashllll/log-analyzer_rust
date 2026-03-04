# Architecture Research: Advanced Search & Virtual File System Integration

**Project:** log-analyzer_rust (Flutter Desktop App v1.1)
**Researched:** 2026-03-04
**Confidence:** HIGH

## Executive Summary

This document outlines how to integrate advanced search (regex, multi-keyword, search history) and virtual file system features into the existing Flutter application architecture. The research reveals that **the Rust backend already implements most required functionality** - the main work is on the Flutter frontend integration.

**Key Findings:**
- Rust backend has `search_history` commands ready to use
- Rust backend has `get_virtual_file_tree` command ready to use
- Existing `SearchQuery` model already supports multiple terms with AND/OR/NOT operators
- Flutter frontend has existing patterns to follow (ArchiveTreeView, Riverpod providers)

---

## Existing Architecture Analysis

### Current Stack

| Layer | Technology | Notes |
|-------|------------|-------|
| UI Framework | Flutter 3.8+ | Desktop target |
| State Management | Riverpod | Provider, StateNotifier, AsyncNotifier |
| FFI Bridge | flutter_rust_bridge | Auto-generated bindings |
| Backend | Rust + Tauri | Already implements core features |

### Current Component Structure

```
log-analyzer_flutter/lib/
├── shared/
│   ├── services/
│   │   ├── api_service.dart      # Main API wrapper
│   │   ├── bridge_service.dart   # FFI communication
│   │   └── generated/           # Auto-generated FFI
│   ├── providers/
│   │   ├── workspace_provider.dart
│   │   ├── task_provider.dart
│   │   └── keyword_provider.dart
│   └── models/
│       ├── search.dart           # SearchQuery, SearchTerm
│       └── common.dart           # Workspace, Filter types
├── features/
│   ├── search/                   # Search UI
│   ├── archive_browsing/        # Archive tree view
│   ├── workspace/               # Workspace management
│   └── realtime_monitoring/     # File monitoring
└── core/
    └── router/                   # go_router
```

### Backend API Availability (Ready to Use)

#### Search History Commands (`src-tauri/src/commands/search_history.rs`)

| Command | Function | Status |
|---------|----------|--------|
| `add_search_history` | Save search query with result count | Implemented |
| `get_search_history` | Retrieve history with optional workspace filter | Implemented |
| `clear_search_history` | Delete history entries | Implemented |

#### Virtual File System Commands (`src-tauri/src/commands/virtual_tree.rs`)

| Command | Function | Status |
|---------|----------|--------|
| `get_virtual_file_tree` | Get hierarchical file/folder structure | Implemented |
| `read_file_by_hash` | Read file content by CAS SHA-256 hash | Implemented |

#### Search Engine (Existing)

- **Regex Support:** `PatternMatcher` in Rust backend supports regex via `regex-automata` crate
- **Multi-keyword:** `SearchQuery` + `SearchTerm` already support AND/OR/NOT operators
- **Query Builder:** `SearchQueryBuilder` service exists in Flutter

---

## Integration Points

### 1. Advanced Search Integration

#### New API Methods Needed in `api_service.dart`

```dart
// Search History
Future<void> addSearchHistory({
  required String query,
  required String workspaceId,
  required int resultCount,
});

Future<List<SearchHistoryEntry>> getSearchHistory({
  String? workspaceId,
  int? limit,
});

Future<int> clearSearchHistory({String? workspaceId});
```

#### New Provider: `SearchHistoryProvider`

```dart
// Location: lib/shared/providers/search_history_provider.dart
@riverpod
class SearchHistory extends _$SearchHistory {
  @override
  Future<List<SearchHistoryEntry>> build(String? workspaceId) async {
    return apiService.getSearchHistory(workspaceId: workspaceId, limit: 50);
  }

  Future<void> addEntry(String query, String workspaceId, int resultCount) async {
    await apiService.addSearchHistory(
      query: query,
      workspaceId: workspaceId,
      resultCount: resultCount,
    );
    ref.invalidateSelf();
  }

  Future<void> clear({String? workspaceId}) async {
    await apiService.clearSearchHistory(workspaceId: workspaceId);
    ref.invalidateSelf();
  }
}
```

#### New Components for Advanced Search UI

| Component | Location | Purpose |
|-----------|----------|---------|
| `SearchInputBar` | `features/search/presentation/widgets/` | Enhanced search input with regex toggle |
| `SearchHistoryPanel` | `features/search/presentation/widgets/` | Sidebar showing recent searches |
| `KeywordChip` | `features/search/presentation/widgets/` | Individual keyword tag with operator |
| `SearchOperatorSelector` | `features/search/presentation/widgets/` | AND/OR/NOT dropdown |

### 2. Virtual File System Integration

#### New API Methods Needed

```dart
// Virtual File Tree
Future<List<VirtualTreeNode>> getVirtualFileTree(String workspaceId);

Future<FileContentResponse> readFileByHash({
  required String workspaceId,
  required String hash,
});
```

#### New Provider: `VirtualFileTreeProvider`

```dart
// Location: lib/features/virtual_file_system/providers/virtual_file_tree_provider.dart
@riverpod
class VirtualFileTree extends _$VirtualFileTree {
  @override
  Future<List<VfsNode>> build(String workspaceId) async {
    return apiService.getVirtualFileTree(workspaceId);
  }

  Future<void> refresh() async {
    ref.invalidateSelf();
  }
}
```

#### New Components for Virtual File System

| Component | Location | Purpose |
|-----------|----------|---------|
| `VirtualFileTreeView` | `features/virtual_file_system/presentation/widgets/` | Main tree view (replaces ArchiveTreeView) |
| `VfsNodeTile` | `features/virtual_file_system/presentation/widgets/` | Individual node row |
| `VfsBreadcrumb` | `features/virtual_file_system/presentation/widgets/` | Path breadcrumb navigation |
| `VfsPreviewPanel` | `features/virtual_file_system/presentation/widgets/` | File content preview |

---

## Data Flow Changes

### Advanced Search Data Flow

```
User Input
    │
    ▼
SearchInputBar (regex toggle, keyword chips)
    │
    ▼
SearchQueryBuilder (builds SearchQuery with multiple terms)
    │
    ▼
ApiService.searchLogs(SearchQuery)
    │
    ▼
Rust Backend (regex engine, pattern matcher)
    │
    ▼
SearchHistoryProvider.addEntry() ──New──
    │
    ▼
SearchResultsProvider (existing)
    │
    ▼
SearchPage (results display)
```

### Virtual File System Data Flow

```
Workspace Load
    │
    ▼
VirtualFileTreeProvider.getVirtualFileTree()
    │
    ▼
ApiService.getVirtualFileTree()
    │
    ▼
Rust Backend (queries metadata store)
    │
    ▼
VirtualTreeNode[] response
    │
    ▼
VirtualFileTreeView (lazy-load children)
    │
    ▼
User selects file
    │
    ▼
ApiService.readFileByHash()
    │
    ▼
VfsPreviewPanel (displays content)
```

---

## New vs Modified Components

### New Components (Create Fresh)

| Component | Type | Reason |
|-----------|------|--------|
| `search_history_provider.dart` | Provider | New functionality |
| `virtual_file_tree_provider.dart` | Provider | New functionality |
| `SearchHistoryPanel` | Widget | New UI feature |
| `SearchInputBar` | Widget | Enhanced search UI |
| `VirtualFileTreeView` | Widget | New navigation |
| `VfsPreviewPanel` | Widget | File preview |
| `VirtualFileTreePage` | Page | New main page |

### Modified Components (Extend Existing)

| Component | Changes |
|-----------|---------|
| `api_service.dart` | Add new methods for search history + VFS |
| `search.dart` | Add `SearchHistoryEntry` model |
| `common.dart` | Add `VfsNode` model |
| `app_router.dart` | Add routes for new pages |
| `SearchPage` | Integrate advanced search UI |

### Reusable Existing Components

| Component | Reuse Pattern |
|-----------|---------------|
| `ArchiveTreeView` | Reference for `VirtualFileTreeView` implementation |
| `ArchiveNode` | Extend or reference for `VfsNode` |
| `ArchiveBrowserProvider` | Reference for `VirtualFileTreeProvider` |
| `SearchQuery` / `SearchTerm` | Already supports multi-term + regex |

---

## Suggested Build Order

### Phase 1: Backend API Integration (Foundation)

1. **Add API methods to `api_service.dart`**
   - `getSearchHistory()`, `addSearchHistory()`, `clearSearchHistory()`
   - `getVirtualFileTree()`, `readFileByHash()`

2. **Add data models**
   - `SearchHistoryEntry` (in `search.dart` or new file)
   - `VfsNode` / `VirtualTreeNode` (in `common.dart` or new file)

**Dependencies:** None (pure Flutter changes)
**Risk:** Low

### Phase 2: State Management (Providers)

3. **Create `SearchHistoryProvider`**
   - AsyncNotifier pattern following existing providers
   - CRUD operations for history

4. **Create `VirtualFileTreeProvider`**
   - AsyncNotifier pattern
   - Lazy loading support

**Dependencies:** Phase 1 complete
**Risk:** Low

### Phase 3: Advanced Search UI

5. **Create `SearchInputBar` widget**
   - Regex toggle button
   - Keyword chip display
   - Operator selector (AND/OR/NOT)

6. **Create `SearchHistoryPanel` widget**
   - List of recent searches
   - Click to reuse
   - Delete option

7. **Integrate into `SearchPage`**
   - Replace existing simple input
   - Add history panel sidebar

**Dependencies:** Phase 2 complete
**Risk:** Medium (UI changes may need iteration)

### Phase 4: Virtual File System UI

8. **Create `VirtualFileTreeView` widget**
   - Similar to `ArchiveTreeView` but for workspace files
   - Expand/collapse directories
   - Lazy loading for large trees

9. **Create `VfsPreviewPanel` widget**
   - Display file content by hash
   - Handle text/binary detection

10. **Create `VirtualFileTreePage`**
    - Main page combining tree + preview
    - Split pane layout

11. **Add route to `app_router.dart`**

**Dependencies:** Phase 2 complete
**Risk:** Medium (tree view performance important)

### Phase 5: Integration & Polish

12. **Connect search to file tree**
    - Click search result -> highlight in tree

13. **Add keyboard navigation**
    - Arrow keys for tree navigation
    - Enter to select

14. **Performance optimization**
    - Virtual scrolling for large trees
    - Debounce search input

**Dependencies:** Phase 3 + 4 complete
**Risk:** Medium

---

## Architecture Diagram

```
+-----------------------------------------------------------------+
|                        Flutter Frontend                         |
+-----------------------------------------------------------------+
|  Pages                                                          |
|  +------------------------------------------------------------+|
|  | SearchPage (enhanced)                                     ||
|  |   +-- SearchInputBar <- NEW                               ||
|  |   +-- SearchHistoryPanel <- NEW                           ||
|  |   +-- SearchResultsPanel (existing)                      ||
|  +------------------------------------------------------------+|
|  | VirtualFileTreePage <- NEW                                ||
|  |   +-- VirtualFileTreeView <- NEW                          ||
|  |   +-- VfsPreviewPanel <- NEW                              ||
|  +------------------------------------------------------------+|
+-----------------------------------------------------------------+
|  Providers (Riverpod)                                          |
|  +-- SearchHistoryProvider <- NEW                              |
|  +-- VirtualFileTreeProvider <- NEW                            |
|  +-- SearchResultsProvider (existing)                          |
|  +-- WorkspaceProvider (existing)                              |
+-----------------------------------------------------------------+
|  Services                                                      |
|  +-- ApiService <- ADD METHODS                                |
|  +-- BridgeService (existing)                                 |
+-----------------------------------------------------------------+
|  Models                                                        |
|  +-- SearchHistoryEntry <- NEW                                 |
|  +-- VfsNode <- NEW                                           |
|  +-- SearchQuery / SearchTerm (existing)                      |
+-----------------------------------------------------------------+
                              |
                              | FFI (flutter_rust_bridge)
                              v
+-----------------------------------------------------------------+
|                        Rust Backend                             |
+-----------------------------------------------------------------+
|  Commands (existing, ready to use)                             |
|  +-- search_history: add/get/clear                             |
|  +-- virtual_tree: get_virtual_file_tree, read_file_by_hash   |
|  +-- search: search_logs (supports regex + multi-term)        |
+-----------------------------------------------------------------+
|  Services (existing)                                           |
|  +-- PatternMatcher (regex + Aho-Corasick)                    |
|  +-- MetadataStore (CAS + SQLite)                             |
|  +-- ContentAddressableStorage                                 |
+-----------------------------------------------------------------+
```

---

## Risks and Mitigations

| Risk | Severity | Mitigation |
|------|----------|------------|
| Large file tree performance | Medium | Lazy loading + virtual scrolling |
| Regex performance | Medium | Backend already optimizes; add client-side timeout |
| History storage growth | Low | Limit entries (default 50) + user clear option |
| FFI serialization errors | Low | Already tested pattern; follow existing naming |

---

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Backend API availability | HIGH | All required commands already implemented |
| Flutter architecture fit | HIGH | Follows existing Riverpod + Provider patterns |
| Component estimates | MEDIUM | Based on existing similar components |
| Build order | MEDIUM | Dependency chain seems correct, may need adjustment |

---

## Sources

- Rust backend search history: `src-tauri/src/commands/search_history.rs` - HIGH
- Rust backend VFS: `src-tauri/src/commands/virtual_tree.rs` - HIGH
- Flutter API service: `log-analyzer_flutter/lib/shared/services/api_service.dart` - HIGH
- Archive tree view: `log-analyzer_flutter/lib/features/archive_browsing/presentation/widgets/archive_tree_view.dart` - HIGH
- Search models: `log-analyzer_flutter/lib/shared/models/search.dart` - HIGH

---

*Architecture research for: Flutter Desktop Log Analyzer v1.1*
*Researched: 2026-03-04*
