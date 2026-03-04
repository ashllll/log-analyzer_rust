# Project Research Summary

**Project:** log-analyzer_rust - Flutter Desktop Log Analyzer v1.1
**Domain:** Desktop Application - Log Analysis Tool
**Researched:** 2026-03-04
**Confidence:** HIGH

## Executive Summary

This research focuses on v1.1 milestone features: regex search, multi-keyword combined search (AND/OR/NOT), search history, and virtual file system. The key finding is that **the Rust backend already implements most required functionality** - the main work is Flutter frontend integration. The recommended approach is to leverage existing Riverpod patterns, extend the API service with new methods, and build UI components following Clean Architecture. The primary risks are ReDoS attacks via malicious regex, Boolean logic parsing errors, search history growth, and virtual file system performance with large datasets. These can be mitigated through frontend validation, backend timeout enforcement, LRU limits, and lazy loading.

## Key Findings

### Recommended Stack

**Core technologies:**
- **Flutter SDK >=3.8.0 <4.0.0** - Desktop UI framework with official Windows/macOS/Linux support
- **Riverpod 3.0** - State management (2026 recommended, compile-time safe, minimal boilerplate)
- **flutter_rust_bridge 2.x** - FFI bridge (already in use, type-safe async support)
- **Dio 5.4.0** - HTTP client (interceptors, auto JSON decode, request cancellation)
- **go_router 14.0.0** - Declarative routing with deep linking
- **freezed 3.2.3** - Immutable data classes with copyWith
- **flutter_fancy_tree_view2 1.6.3** - Virtual file tree UI (Sliver-based, lazy loading)
- **rich_text_controller 3.0.1** - Regex syntax highlighting

### Expected Features

**Must have (table stakes):**
- Regex expression search - Log analysis requires complex pattern matching (IP, date, stack traces)
- Multi-keyword combined search (AND/OR/NOT) - Core requirement for narrowing results
- Search history with quick access - Common workflow for repeated searches
- Virtual file tree navigation - Browse archive file structure within app

**Should have (competitive):**
- Search syntax highlighting - Real-time regex validation feedback
- Smart search suggestions - Auto-complete based on history
- File tree search filter - Quickly locate files in large trees

**Defer (v2+):**
- Cloud search history sync - Privacy concerns, keep local
- Complex tree animations - Performance overhead on desktop

### Architecture Approach

The research reveals a clear integration path: Rust backend already provides all necessary commands (`add_search_history`, `get_search_history`, `get_virtual_file_tree`, regex via `regex-automata`). Flutter needs API service extensions, new Riverpod providers, and UI components following Clean Architecture. The existing `SearchQuery` model already supports multiple terms with AND/OR/NOT operators.

**Major components:**
1. **API Service Extensions** - Add methods for search history and virtual file tree
2. **SearchHistoryProvider** - Riverpod AsyncNotifier for history CRUD operations
3. **VirtualFileTreeProvider** - Riverpod AsyncNotifier with lazy loading support
4. **SearchInputBar** - Enhanced search widget with regex toggle and keyword chips
5. **VirtualFileTreeView** - TreeView widget for workspace file navigation

### Critical Pitfalls

1. **ReDoS Attack Risk** - Malicious regex (e.g., `a+*`) can cause CPU 100% and freeze app. Prevention: Dart `RegExp` validation before sending to backend + Rust timeout enforcement.

2. **Boolean Logic Parsing Errors** - `error AND warning OR critical` may parse incorrectly. Prevention: Implement standard NOT>AND>OR priority or support parentheses.

3. **Search History Data Growth** - Unlimited history causes memory bloat. Prevention: LRU limit (100 entries), deduplication, 30-day expiration.

4. **Virtual File System Performance** - 1000+ files crashes UI with full rendering. Prevention: Lazy loading on expand, virtual scrolling, limit initial depth to 2-3 levels.

5. **File State Desync** - Virtual tree shows stale data vs actual filesystem. Prevention: Show last update timestamp, manual refresh button.

## Implications for Roadmap

Based on research, suggested phase structure:

### Phase 1: Backend API Integration
**Rationale:** Foundation for all features - existing Rust commands need Flutter exposure
**Delivers:** API service methods, data models (SearchHistoryEntry, VfsNode)
**Addresses:** All features require API integration
**Avoids:** Integration errors by establishing patterns early
**Stack:** Dio, freezed, flutter_rust_bridge

### Phase 2: State Management (Providers)
**Rationale:** Providers depend on API service; required before UI development
**Delivers:** SearchHistoryProvider, VirtualFileTreeProvider (Riverpod)
**Uses:** AsyncNotifier pattern following existing providers
**Implements:** CRUD operations, lazy loading logic

### Phase 3: Advanced Search UI
**Rationale:** Core v1.1 feature - regex and multi-keyword are primary differentiators
**Delivers:** SearchInputBar with regex toggle, SearchHistoryPanel, KeywordChip components
**Addresses:** Regex search, multi-keyword AND/OR/NOT, search history
**Avoids:** Pitfall 1 (ReDoS) with frontend validation, Pitfall 2 (Boolean logic) with preview
**Research Flag:** Standard Riverpod patterns - skip detailed research

### Phase 4: Virtual File System UI
**Rationale:** New navigation paradigm, separate from search for clarity
**Delivers:** VirtualFileTreeView, VfsPreviewPanel, VirtualFileTreePage
**Addresses:** Virtual file tree navigation, directory browsing
**Avoids:** Pitfall 4 (performance) with lazy loading + virtual scrolling
**Research Flag:** TreeView performance needs validation with 1000+ files

### Phase 5: Integration & Polish
**Rationale:** Connect features for seamless experience
**Delivers:** Search result to tree linking, keyboard navigation, performance optimization
**Avoids:** Pitfall 5 (file state desync) with refresh indicators

### Phase Ordering Rationale

- **Foundation first:** Phase 1-2 establish the data layer before UI
- **Search before tree:** Advanced search is the core v1.1 value proposition
- **Separation of concerns:** Search and VFS are independent features
- **Performance-aware:** Lazy loading patterns established early to prevent Pitfall 4
- **Validation late:** Complex features deferred to integration phase

### Research Flags

Phases likely needing deeper research during planning:
- **Phase 4 (Virtual File System):** Performance with 1000+ files needs testing
- **Phase 5 (Integration):** Cross-feature interaction may reveal edge cases

Phases with standard patterns (skip research-phase):
- **Phase 1-2:** Well-documented Riverpod + API patterns, existing codebase references
- **Phase 3:** Standard search UI components, documented in research

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | All technologies verified with pub.dev and project compatibility |
| Features | HIGH | Backend capabilities confirmed from Rust source analysis |
| Architecture | HIGH | Existing patterns in codebase provide clear integration path |
| Pitfalls | MEDIUM | Some findings based on code analysis + experience, web search had errors |

**Overall confidence:** HIGH

The research is backed by actual Rust backend code analysis, existing Flutter patterns in the project, and pub.dev documentation. The main uncertainty is around performance characteristics that should be validated during Phase 4 implementation.

### Gaps to Address

- **Boolean Logic Priority:** Not fully verified in backend - needs unit test validation during implementation
- **VFS Lazy Loading API:** Not confirmed if backend supports incremental loading - may need API enhancement
- **Regex Complexity Detection:** No existing pattern in codebase - needs research during Phase 3

## Sources

### Primary (HIGH confidence)
- Rust backend source: `commands/search_history.rs`, `commands/virtual_tree.rs` - confirmed API exists
- Rust search engine: `search_engine/boolean_query_processor.rs`, `search_engine/advanced_features.rs` - confirmed regex + multi-keyword support
- Flutter project: `log-analyzer_flutter/lib/shared/services/api_service.dart` - confirmed integration patterns
- pub.dev: Riverpod 3.0, flutter_fancy_tree_view2, rich_text_controller - verified versions

### Secondary (MEDIUM confidence)
- Flutter Clean Architecture recommendations - applied to feature structure
- Performance considerations for virtual scrolling - based on Flutter documentation

### Tertiary (LOW confidence)
- Pitfall web validation - some search tools returned errors, some findings based on experience inference
- Verify all pitfall mitigations during implementation phase

---

*Research completed: 2026-03-04*
*Ready for roadmap: yes*
