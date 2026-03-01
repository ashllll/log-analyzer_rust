# Architecture Research

**Domain:** Flutter Desktop Application - Log Analyzer Frontend
**Researched:** 2026-02-28
**Confidence:** HIGH

## Standard Architecture

### System Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Flutter UI Layer                             │
├─────────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌──────────┐ │
│  │   Screens   │  │   Widgets   │  │   Dialogs   │  │  Router  │ │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘  └─────┬────┘ │
│         │                │                │               │        │
├─────────┴────────────────┴────────────────┴───────────────┴────────┤
│                    Presentation Layer (Riverpod)                    │
├─────────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────┐  ┌─────────────────────┐                  │
│  │   StateNotifiers    │  │      Providers      │                  │
│  │  (UI State Mgmt)    │  │  (Dependency Inject)│                  │
│  └──────────┬───────────┘  └──────────┬───────────┘                  │
├─────────────┴─────────────────────────┴─────────────────────────────┤
│                         Domain Layer                                 │
├─────────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                 │
│  │  Entities  │  │  Use Cases  │  │ Repo Interfaces│               │
│  └─────────────┘  └─────────────┘  └─────────────┘                 │
├─────────────────────────────────────────────────────────────────────┤
│                          Data Layer                                 │
├─────────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                 │
│  │ Repositories│  │  DataSources │  │    Models   │                 │
│  │  (Impl)     │  │ (FFI/HTTP)   │  │  (JSON)     │                 │
│  └─────────────┘  └─────────────┘  └─────────────┘                 │
├─────────────────────────────────────────────────────────────────────┤
│                    External Integrations                            │
├─────────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────┐  ┌──────────────────────────────┐ │
│  │   Rust Backend (FFI/HTTP)   │  │      Local Storage           │ │
│  │   - Search Engine           │  │      (path_provider)        │ │
│  │   - Archive Handlers        │  │                              │ │
│  │   - CAS Storage             │  │                              │ │
│  └─────────────────────────────┘  └──────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Responsibility | Typical Implementation |
|-----------|----------------|------------------------|
| **Screens** | Top-level page widgets, orchestrate sub-widgets | `ConsumerWidget` / `ConsumerStatefulWidget` |
| **Widgets** | Reusable UI components | Stateless/Stateful Widgets |
| **StateNotifiers** | UI state management, business logic delegation | `Notifier` / `AsyncNotifier` (Riverpod 2.x) |
| **Providers** | Dependency injection, instance management | `Riverpod` providers |
| **Use Cases** | Single-responsibility business operations | Pure Dart classes |
| **Entities** | Core business data structures | Immutable classes with `Equatable` |
| **Repository Interfaces** | Contracts for data access | Abstract classes |
| **Repository Implementations** | Concrete data access logic | Implements interfaces |
| **DataSources** | Low-level FFI/HTTP communication | `flutter_rust_bridge` / `dio` |

## Recommended Project Structure

```
log-analyzer_flutter/lib/
├── main.dart                           # App entry point
├── app.dart                           # MaterialApp configuration
│
├── core/                              # Core utilities & shared
│   ├── constants/                    # App constants
│   ├── theme/                        # Theme configuration
│   ├── router/                      # go_router configuration
│   ├── utils/                        # Utility functions
│   └── errors/                       # Error handling (Failure classes)
│
├── shared/                           # Shared across features
│   ├── models/                       # Shared data models (generated)
│   ├── providers/                    # Global providers
│   ├── services/                     # Services (API, FFI bridge)
│   │   ├── api_service.dart         # HTTP API client
│   │   ├── bridge_service.dart      # FFI bridge wrapper
│   │   └── event_stream_service.dart
│   └── widgets/                      # Shared UI components
│
└── features/                         # Feature modules
    ├── search/
    │   ├── domain/
    │   │   ├── entities/            # SearchResult, LogEntry entities
    │   │   ├── repositories/        # Repository interfaces
    │   │   └── usecases/            # SearchUseCase, FilterUseCase
    │   ├── data/
    │   │   ├── models/              # API response models
    │   │   ├── datasources/         # FFI/HTTP data sources
    │   │   └── repositories/        # Repository implementations
    │   └── presentation/
    │       ├── providers/           # SearchNotifier, SearchProvider
    │       ├── screens/             # SearchPage
    │       └── widgets/             # SearchBar, ResultList, FilterPanel
    │
    ├── workspace/
    │   ├── domain/
    │   ├── data/
    │   └── presentation/
    │
    ├── task/
    │   ├── domain/
    │   ├── data/
    │   └── presentation/
    │
    ├── settings/
    │   ├── domain/
    │   ├── data/
    │   └── presentation/
    │
    └── keyword/
        ├── domain/
        ├── data/
        └── presentation/
```

### Structure Rationale

- **Feature-first organization:** Each feature (search, workspace, task, settings) contains its own Clean Architecture layers. This keeps related code together and makes feature removal straightforward.

- **Core folder:** Contains app-wide utilities that don't belong to any specific feature (theme, routing, constants).

- **Shared folder:** Holds cross-cutting concerns like API services, FFI bridge, and global state providers.

- **Domain/Data/Presentation split:** Follows Clean Architecture principles with strict dependency rules.

## Architectural Patterns

### Pattern 1: Clean Architecture with Riverpod

**What:** Four-layer architecture (Domain, Data, Presentation, DI) with strict dependency rules.

**When to use:** Medium to large applications requiring testability and maintainability.

**Trade-offs:**
- Pros: Clear separation, testable, scalable
- Cons: More boilerplate, potential overkill for small apps

**Example:**
```dart
// Domain Layer - Entity
class LogEntry extends Equatable {
  final String id;
  final String content;
  final DateTime timestamp;
  final String level;
}

// Domain Layer - Repository Interface
abstract class SearchRepository {
  Future<Either<Failure, List<SearchResult>>> search(SearchQuery query);
}

// Domain Layer - Use Case
class SearchLogsUseCase {
  final SearchRepository repository;

  Future<Either<Failure, List<SearchResult>>> call(SearchQuery query) {
    return repository.search(query);
  }
}

// Data Layer - Repository Implementation
class SearchRepositoryImpl implements SearchRepository {
  final SearchDataSource dataSource;

  @override
  Future<Either<Failure, List<SearchResult>>> search(SearchQuery query) async {
    try {
      final results = await dataSource.search(query);
      return Right(results);
    } catch (e) {
      return Left(SearchFailure(e.toString()));
    }
  }
}

// Presentation Layer - Notifier
class SearchNotifier extends Notifier<SearchState> {
  @override
  SearchState build() => const SearchState();

  Future<void> search(String query) async {
    state = state.copyWith(isLoading: true);

    final useCase = ref.read(searchLogsUseCaseProvider);
    final result = await useCase(SearchQuery(query));

    result.fold(
      (failure) => state = state.copyWith(error: failure.message),
      (results) => state = state.copyWith(results: results, isLoading: false),
    );
  }
}
```

### Pattern 2: Feature-First Organization

**What:** Each feature contains its own complete set of layers, rather than grouping all layers together.

**When to use:** Medium to large apps with multiple distinct functional areas.

**Trade-offs:**
- Pros: All feature code in one place, easy to add/remove features
- Cons: Potential code duplication for shared logic

**Example:**
```
features/
├── search/          # All search-related code
│   ├── domain/     # Entities, use cases for search
│   ├── data/      # Data sources, models for search
│   └── presentation/  # UI for search
├── workspace/      # All workspace-related code
│   ├── domain/
│   ├── data/
│   └── presentation/
```

### Pattern 3: Unidirectional Data Flow

**What:** Data flows in one direction: Data Layer -> Presentation Layer. User events flow back.

**When to use:** All Flutter apps (Flutter recommended pattern).

**Trade-offs:**
- Pros: Predictable state changes, easier debugging
- Cons: More initial setup

**Example:**
```
User types in search bar
    ↓
SearchNotifier.search() called
    ↓
SearchLogsUseCase.execute(query)
    ↓
SearchRepository.search() via FFI/HTTP
    ↓
Rust backend processes search
    ↓
Result returned to SearchNotifier
    ↓
State updated, UI rebuilds
```

### Pattern 4: FFI/HTTP Bridge Pattern

**What:** Abstract Rust backend communication behind repository interface, allowing FFI or HTTP implementation.

**When to use:** When backend might be accessed via multiple protocols.

**Trade-offs:**
- Pros: Flexibility, easy to switch implementations
- Cons: Additional abstraction layer

**Example:**
```dart
// Abstract data source
abstract class SearchDataSource {
  Future<List<LogEntry>> search(String query);
}

// FFI implementation
class FFISearchDataSource implements SearchDataSource {
  @override
  Future<List<LogEntry>> search(String query) async {
    final result = await RustBridge.searchLogs(query);
    return result.map((e) => e.toLogEntry()).toList();
  }
}

// HTTP implementation
class HttpSearchDataSource implements SearchDataSource {
  final ApiService api;

  @override
  Future<List<LogEntry>> search(String query) async {
    final response = await api.post('/search', {'query': query});
    return response.data.map((e) => LogEntry.fromJson(e)).toList();
  }
}
```

## Data Flow

### Request Flow (Search Example)

```
[User Action: Enter search query]
    │
    ▼
[SearchPage] → calls ref.read(searchNotifierProvider.notifier).search(query)
    │
    ▼
[SearchNotifier] → state = state.copyWith(isLoading: true)
    │
    ▼
[SearchNotifier] → ref.read(searchLogsUseCaseProvider).call(query)
    │
    ▼
[SearchLogsUseCase] → repository.search(query)
    │
    ▼
[SearchRepositoryImpl] → dataSource.search(query)
    │
    ▼
[FFISearchDataSource] → await RustBridge.searchLogs(ffi_request)
    │
    ▼
[Rust Backend] → Tantivy/DFA search → returns results
    │
    ▼
[FFISearchDataSource] → converts to LogEntry entities
    │
    ▼
[SearchNotifier] → state = state.copyWith(results: results, isLoading: false)
    │
    ▼
[SearchPage] → Consumer rebuilds with new state
```

### State Management (Riverpod)

```
┌─────────────────────────────────────────────────────┐
│                   Riverpod Providers                 │
├─────────────────────────────────────────────────────┤
│                                                      │
│  ┌──────────────────┐    ┌──────────────────────┐  │
│  │ Infrastructure   │    │   Feature Providers  │  │
│  │ Providers        │    │                       │  │
│  │ - apiClient      │    │ - searchNotifier     │  │
│  │ - ffiBridge      │    │ - workspaceNotifier  │  │
│  │ - storage        │    │ - taskNotifier       │  │
│  └────────┬─────────┘    └──────────┬───────────┘  │
│           │                          │               │
│           └──────────┬───────────────┘               │
│                      ▼                               │
│           ┌──────────────────────┐                   │
│           │   Use Case Providers │                   │
│           │ - searchLogsUseCase  │                   │
│           │ - importFolderUseCase│                   │
│           └──────────┬───────────┘                   │
│                      ▼                               │
│           ┌──────────────────────┐                   │
│           │ Repository Providers │                   │
│           │ - searchRepository   │                   │
│           └──────────────────────┘                   │
└─────────────────────────────────────────────────────┘
```

### Key Data Flows

1. **Search Flow:** User enters query → Notifier → UseCase → Repository → FFI → Rust → Results → State update → UI rebuild

2. **Import Flow:** User selects folder → Notifier → UseCase → Repository → FFI → Rust (CAS storage, indexing) → Progress events → UI update

3. **Workspace Flow:** User manages workspace → Notifier → UseCase → Repository → FFI → Rust (SQLite operations) → State update

## Integration with Rust Backend

### FFI Integration Points

| Rust Module | Flutter Integration | Data Flow |
|------------|-------------------|-----------|
| Search Engine (Tantivy) | `SearchRepository` | Query → Results |
| Archive Handlers | `ArchiveRepository` | File → Extracted Content |
| File Watcher | `WorkspaceNotifier` | Events → State Update |
| CAS Storage | `StorageRepository` | File metadata |
| Task Manager | `TaskNotifier` | Progress events |

### Communication Patterns

1. **FFI (Primary):** Use `flutter_rust_bridge` for direct Rust function calls
   - Better performance for frequent operations
   - Type-safe bindings generated automatically

2. **HTTP API (Fallback):** Use `dio` for REST API calls
   - Useful when FFI is unavailable
   - Better for debugging

## Scaling Considerations

| Scale | Architecture Adjustments |
|-------|--------------------------|
| 0-1K users | Single instance, basic Clean Architecture sufficient |
| 1K-100K users | Optimize FFI calls, add result caching, lazy loading |
| 100K+ users | Consider background isolates, incremental loading, pagination |

### Scaling Priorities

1. **First bottleneck:** Large log file loading → Implement virtual scrolling, pagination
2. **Second bottleneck:** Search responsiveness → Leverage Rust backend, add result caching
3. **Third bottleneck:** UI responsiveness → Use isolates for heavy computation

## Anti-Patterns

### Anti-Pattern 1: Business Logic in Widgets

**What people do:** Putting search logic, data transformation directly in `build()` methods.

**Why it's wrong:** Hard to test, mixed concerns, poor maintainability.

**Do this instead:** Use Notifiers/UseCases to handle business logic, widgets only render state.

### Anti-Pattern 2: Direct FFI Calls in UI

**What people do:** Calling `RustBridge.searchLogs()` directly from widget event handlers.

**Why it's wrong:** Tight coupling, hard to swap implementations, poor testability.

**Do this instead:** Use repository pattern, inject via providers, test against mock implementations.

### Anti-Pattern 3: Mutable State

**What people do:** Using `StatefulWidget` with direct state mutation.

**Why it's wrong:** Unpredictable state, harder to debug, doesn't scale.

**Do this instead:** Use immutable state with `copyWith`, update via Riverpod Notifiers.

### Anti-Pattern 4: Feature by Screen

**What people do:** Creating features named `search_page`, `settings_page`, etc.

**Why it's wrong:** Features should be defined by functionality (what user *does*), not UI (what user *sees*).

**Do this instead:** Name features by domain: `search`, `workspace`, `settings`, `task_management`.

## Build Order (Dependencies)

Based on Clean Architecture and Flutter best practices:

### Phase 1: Foundation
1. **Core utilities** - Theme, router, constants
2. **Shared services** - API client, FFI bridge wrapper
3. **Error handling** - Failure classes, error mapping

### Phase 2: Domain Layer
4. **Entities** - Define core business objects
5. **Repository interfaces** - Define contracts
6. **Use cases** - Implement business logic

### Phase 3: Data Layer
7. **Models** - API/FFI response models
8. **Data sources** - FFI/HTTP implementations
9. **Repository implementations** - Bridge domain and data

### Phase 4: Presentation Layer
10. **Providers** - Wire dependencies
11. **Notifiers** - State management
12. **Widgets** - Reusable UI components
13. **Screens** - Top-level pages

### Rationale
- Domain layer has no external dependencies → build first
- Data layer depends on Domain → build after Domain
- Presentation layer depends on Domain abstraction → build last
- This ensures testability and proper dependency inversion

## Sources

- [Flutter Architecture Recommendations](https://docs.flutter.dev/app-architecture/recommendations) - HIGH
- [Flutter Riverpod Clean Architecture Template](https://ssoad.github.io/flutter_riverpod_clean_architecture/) - HIGH
- [Flutter Project Structure: Feature-first vs Layer-first](https://codewithandrea.com/articles/flutter-project-structure/) - HIGH
- [Layer Structure | DeepWiki](https://deepwiki.com/ssoad/flutter_riverpod_clean_architecture/3.2-layer-structure) - HIGH

---

*Architecture research for: Flutter Desktop Log Analyzer*
*Researched: 2026-02-28*
