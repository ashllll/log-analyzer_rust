# Bug Fixes Design Document

## Overview

This design document outlines the systematic approach to fixing critical bugs identified in the log analyzer application. The fixes address compilation errors, memory safety issues, concurrency problems, and resource management concerns across both the Rust backend and React frontend.

## Architecture

The bug fixes leverage battle-tested, industry-standard solutions with proven track records in production environments:

### Phase 1: Core Infrastructure (1-2 weeks)
1. **Industry-Standard Error Handling**: Replace custom AppError with `eyre` ecosystem (used by major Rust projects)
2. **Production Observability**: Replace println! with `tracing` (Rust official standard) + `sentry` (enterprise-grade)
3. **High-Performance Concurrency**: Replace unsafe locks with `parking_lot` (widely adopted) + `tokio::sync`

### Phase 2: State and Performance (2-3 weeks)
4. **Modern React State**: Replace complex patterns with `zustand` + `@tanstack/react-query` (industry standards)
5. **Enterprise Caching**: Replace manual LRU with `moka` (based on battle-tested Caffeine cache)
6. **Native Event Systems**: Use React's built-in events + `tokio::sync::broadcast` (no third-party event libs)

### Phase 3: Validation and Safety (1-2 weeks)
7. **Production Validation**: Use only `validator` framework (most mature Rust validation library)
8. **RAII Resource Management**: Implement with `scopeguard` (standard Rust RAII pattern)

### Phase 4: Architecture and Monitoring (1-2 weeks)
9. **Rust-Native Dependency Management**: Use constructor injection and builder patterns (Rust best practices)
10. **APM Integration**: Deep Sentry integration for performance monitoring
11. **Production Hardening**: Focus on error recovery and graceful degradation

## Components and Interfaces

### Backend Components (Mature Solutions)

#### Modern Error System (`eyre` + `miette`)
- **Purpose**: Replace custom AppError with industry-standard error handling
- **Key Libraries**: `eyre` for error context, `miette` for user-facing diagnostics, `color-eyre` for enhanced reporting
- **Features**:
  - Automatic error context propagation with `.context()` method
  - Colored terminal output with source code snippets
  - Integration with `tracing` for structured logging
  - Stack trace capture and error chaining
- **Migration Strategy**: Replace `Result<T, AppError>` with `eyre::Result<T>`

#### Advanced Concurrency System (`parking_lot` + `tokio::sync`)
- **Purpose**: Replace unsafe lock management with high-performance alternatives
- **Key Libraries**: `parking_lot` for sync locks, `tokio::sync` for async locks, `crossbeam` for lock-free structures
- **Features**:
  - Deadlock detection and timeout mechanisms with `try_lock_for()`
  - RwLock for improved read performance (multiple readers, single writer)
  - Lock-free data structures (`SegQueue`, `ArrayQueue`) where applicable
  - Fair locking algorithms to prevent starvation
- **Migration Strategy**: Replace `std::sync::Mutex` with `parking_lot::Mutex`, add timeout handling

#### High-Performance Cache (`moka`)
- **Purpose**: Replace manual LRU with enterprise-grade caching
- **Key Libraries**: `moka` for advanced caching features
- **Features**:
  - TTL (Time To Live) and TTI (Time To Idle) expiration policies
  - Async cache operations with `get_with()` for compute-on-miss
  - Intelligent invalidation strategies with `invalidate_entries_if()`
  - Built-in metrics (hit rate, eviction count) and monitoring
  - Concurrent access optimization with segmented locks
- **Migration Strategy**: Replace `lru::LruCache` with `moka::Cache`, add expiration policies

#### Production Validation (`validator`)
- **Purpose**: Replace manual validation with the most mature Rust validation framework
- **Key Libraries**: `validator` (most widely adopted), `sanitize-filename` for path safety
- **Features**:
  - Declarative validation rules with derive macros
  - Custom validation functions with comprehensive error reporting
  - Automatic error message generation with i18n support
  - Full integration with serde serialization
  - Extensive validation rule library (email, URL, length, range, regex, etc.)
- **Migration Strategy**: Add validation derives to structs, replace all manual validation logic

#### Structured Observability (`tracing` + `sentry`)
- **Purpose**: Replace println! with structured logging and error monitoring
- **Key Libraries**: `tracing` ecosystem, `sentry` for error tracking, `tracing-subscriber` for output formatting
- **Features**:
  - Structured JSON logging with contextual fields
  - Distributed tracing support with span correlation
  - Automatic error capture and reporting to Sentry
  - Performance monitoring and alerting
  - Log level filtering and dynamic configuration
- **Migration Strategy**: Replace all println!/eprintln! with tracing macros, add instrumentation

### Frontend Components (Modern React Patterns)

#### Modern State Management (`zustand` + `react-query`)
- **Purpose**: Replace complex Context+Reducer with modern state solutions
- **Key Libraries**: `zustand` for client state, `@tanstack/react-query` for server state, `immer` for immutable updates
- **Features**:
  - Automatic deduplication and caching of server requests
  - Optimistic updates and automatic rollback on failure
  - Background refetching and synchronization
  - DevTools integration for debugging
  - Type-safe state management with TypeScript
- **Migration Strategy**: Replace useReducer patterns with zustand stores, move server state to react-query

#### Native Event Management (React + Tokio)
- **Purpose**: Use built-in event systems instead of third-party libraries
- **Key Libraries**: React's built-in event system, `tokio::sync::broadcast` for backend events
- **Features**:
  - React's proven event system with automatic cleanup
  - Type-safe event definitions with TypeScript
  - Built-in memory leak prevention
  - Integration with React DevTools
  - Backend event broadcasting with tokio's battle-tested primitives
- **Migration Strategy**: Use React's useEffect cleanup, replace custom event systems with native patterns

#### Resource Management (React + Immer)
- **Purpose**: Use proven patterns for state updates and resource management
- **Key Libraries**: `immer` (industry standard), `react-error-boundary` (React team recommended)
- **Features**:
  - Immutable state updates with mutable syntax via Immer
  - React's built-in cleanup with useEffect
  - Error boundaries for component error isolation
  - Native React performance optimization patterns
  - Standard React lifecycle management
- **Migration Strategy**: Use React's built-in patterns, add Immer for complex state updates

#### Rust-Native Dependency Management (Constructor Injection + Builder Pattern)
- **Purpose**: Use proven Rust patterns instead of immature DI frameworks
- **Key Patterns**: Constructor injection, Builder pattern, Service registry, Modular design
- **Features**:
  - Compile-time type safety with zero runtime overhead
  - Simple and explicit dependency relationships
  - Easy testing with dependency injection through constructors
  - Configuration-driven service creation
  - Follows Rust ownership model naturally
- **Migration Strategy**: Replace complex service location with simple constructor injection and builder patterns

## Data Models

### Modern Error Types (eyre + miette)
```rust
// Replace custom AppError with eyre::Result
pub type AppResult<T> = eyre::Result<T>;

// Use miette for user-facing errors with rich diagnostics
use miette::{Diagnostic, Result, miette, SourceSpan};
use thiserror::Error;

#[derive(Error, Diagnostic, Debug)]
pub enum UserFacingError {
    #[error("Invalid workspace configuration")]
    #[diagnostic(
        code(app::workspace::invalid_config),
        help("Check that the workspace ID contains only alphanumeric characters and hyphens"),
        url("https://docs.example.com/workspace-naming")
    )]
    InvalidWorkspaceConfig {
        #[source_code]
        src: String,
        #[label("Invalid character here")]
        bad_char: SourceSpan,
    },
    
    #[error("Search operation failed: {reason}")]
    #[diagnostic(
        code(app::search::failed),
        help("Try reducing the search scope or check file permissions")
    )]
    SearchFailed {
        reason: String,
        workspace_id: String,
        query: String,
    },
}

// Context-aware error creation
fn validate_workspace_id(id: &str) -> AppResult<()> {
    if id.is_empty() {
        return Err(eyre!("Workspace ID cannot be empty"))
            .with_context(|| "Validating workspace configuration");
    }
    Ok(())
}
```

### Advanced Concurrency Types (parking_lot + tokio)
```rust
use parking_lot::{Mutex, RwLock, MutexGuard};
use tokio::sync::{Mutex as AsyncMutex, RwLock as AsyncRwLock};
use crossbeam::queue::SegQueue;
use std::time::Duration;

pub struct SafeConcurrencyManager {
    // High-performance synchronous locks with timeout support
    sync_locks: HashMap<String, Arc<RwLock<WorkspaceData>>>,
    // Async locks for async contexts
    async_locks: HashMap<String, Arc<AsyncMutex<TaskData>>>,
    // Lock-free queues for high-throughput scenarios
    task_queue: SegQueue<Task>,
    cleanup_queue: SegQueue<CleanupItem>,
}

impl SafeConcurrencyManager {
    pub fn acquire_with_timeout<T>(
        &self, 
        lock: &Mutex<T>, 
        timeout: Duration
    ) -> Option<MutexGuard<T>> {
        lock.try_lock_for(timeout)
    }
    
    pub async fn acquire_async_lock<T>(
        &self,
        lock: &AsyncMutex<T>
    ) -> tokio::sync::MutexGuard<T> {
        lock.lock().await
    }
}
```

### High-Performance Cache Types (moka)
```rust
use moka::future::Cache;
use moka::policy::EvictionPolicy;
use std::time::Duration;

pub struct AdvancedCacheManager {
    // Replace LRU with moka's advanced caching
    search_cache: Cache<SearchCacheKey, Vec<LogEntry>>,
    workspace_cache: Cache<String, WorkspaceData>,
    file_metadata_cache: Cache<String, FileMetadata>,
}

impl AdvancedCacheManager {
    pub fn new() -> Self {
        let search_cache = Cache::builder()
            .max_capacity(1000)
            .time_to_live(Duration::from_secs(300))  // 5 minutes TTL
            .time_to_idle(Duration::from_secs(60))   // 1 minute idle timeout
            .eviction_policy(EvictionPolicy::lru())
            .build();
            
        let workspace_cache = Cache::builder()
            .max_capacity(100)
            .time_to_live(Duration::from_secs(3600)) // 1 hour TTL
            .build();
            
        Self { search_cache, workspace_cache, file_metadata_cache }
    }
    
    pub async fn get_or_compute_search<F, Fut>(
        &self, 
        key: SearchCacheKey, 
        compute: F
    ) -> Vec<LogEntry> 
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Vec<LogEntry>>,
    {
        self.search_cache.get_with(key, compute).await
    }
}
```

### Rust-Native Dependency Management Types
```rust
use std::sync::Arc;
use eyre::Result;

/// 应用服务容器 - 使用构造函数注入模式
pub struct AppServices {
    pub cache_manager: Arc<CacheManager>,
    pub search_service: Arc<SearchService>,
    pub validation_service: Arc<ValidationService>,
    pub workspace_service: Arc<WorkspaceService>,
}

impl AppServices {
    /// 使用 Builder 模式创建服务容器
    pub fn builder() -> AppServicesBuilder {
        AppServicesBuilder::new()
    }
    
    /// 直接创建（使用默认配置）
    pub fn new() -> Result<Self> {
        Self::builder().build()
    }
}

/// Builder 模式用于灵活的服务配置
pub struct AppServicesBuilder {
    cache_config: Option<CacheConfig>,
    validation_config: Option<ValidationConfig>,
    workspace_config: Option<WorkspaceConfig>,
}

impl AppServicesBuilder {
    pub fn new() -> Self {
        Self {
            cache_config: None,
            validation_config: None,
            workspace_config: None,
        }
    }
    
    pub fn with_cache_config(mut self, config: CacheConfig) -> Self {
        self.cache_config = Some(config);
        self
    }
    
    pub fn with_validation_config(mut self, config: ValidationConfig) -> Self {
        self.validation_config = Some(config);
        self
    }
    
    pub fn build(self) -> Result<AppServices> {
        // 按依赖顺序创建服务
        let cache_manager = Arc::new(CacheManager::with_config(
            self.cache_config.unwrap_or_default()
        ));
        
        let validation_service = Arc::new(ValidationService::with_config(
            self.validation_config.unwrap_or_default()
        ));
        
        let search_service = Arc::new(SearchService::new(
            Arc::clone(&cache_manager),
            Arc::clone(&validation_service),
        ));
        
        let workspace_service = Arc::new(WorkspaceService::new(
            Arc::clone(&cache_manager),
            Arc::clone(&validation_service),
        ));
        
        Ok(AppServices {
            cache_manager,
            search_service,
            validation_service,
            workspace_service,
        })
    }
}

/// 服务特征 - 定义服务的生命周期
pub trait Service {
    fn start(&self) -> Result<()>;
    fn stop(&self) -> Result<()>;
    fn health_check(&self) -> Result<ServiceHealth>;
}

/// 配置驱动的服务创建
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ServiceConfiguration {
    pub cache: CacheConfig,
    pub validation: ValidationConfig,
    pub workspace: WorkspaceConfig,
    pub monitoring: MonitoringConfig,
}

impl ServiceConfiguration {
    pub fn from_file(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }
    
    pub fn build_services(self) -> Result<AppServices> {
        AppServices::builder()
            .with_cache_config(self.cache)
            .with_validation_config(self.validation)
            .build()
    }
}

/// 服务健康状态
#[derive(Debug, Clone, serde::Serialize)]
pub struct ServiceHealth {
    pub is_healthy: bool,
    pub last_check: std::time::SystemTime,
    pub details: std::collections::HashMap<String, String>,
}
```

### Modern State Management Types (zustand + react-query)
```typescript
// Replace complex reducers with zustand
import { create } from 'zustand'
import { subscribeWithSelector, devtools } from 'zustand/middleware'
import { immer } from 'zustand/middleware/immer'

interface AppStore {
  // State
  workspaces: Workspace[]
  tasks: Task[]
  toasts: Toast[]
  
  // Actions with built-in deduplication
  addWorkspace: (workspace: Workspace) => void
  updateWorkspace: (id: string, updates: Partial<Workspace>) => void
  addTaskIfNotExists: (task: Task) => void
  updateTask: (id: string, updates: Partial<Task>) => void
  
  // Async actions with react-query integration
  loadWorkspaces: () => Promise<void>
  refreshWorkspace: (id: string) => Promise<void>
}

export const useAppStore = create<AppStore>()(
  devtools(
    subscribeWithSelector(
      immer((set, get) => ({
        workspaces: [],
        tasks: [],
        toasts: [],
        
        addTaskIfNotExists: (task) => set((state) => {
          const exists = state.tasks.some(t => t.id === task.id)
          if (!exists) {
            state.tasks.push(task)
          }
        }),
        
        updateWorkspace: (id, updates) => set((state) => {
          const index = state.workspaces.findIndex(w => w.id === id)
          if (index !== -1) {
            Object.assign(state.workspaces[index], updates)
          }
        }),
      }))
    ),
    { name: 'app-store' }
  )
)

// Event management with automatic cleanup
interface EventManager {
  subscribe<T>(componentId: string, event: string, handler: (data: T) => void): () => void
  cleanup(componentId: string): void
  emit<T>(event: string, data: T): void
}

// Validation types with structured validation
interface ValidatedWorkspaceConfig {
  id: string      // Validated with regex pattern
  name: string    // Length validated (1-100 chars)
  path: string    // Path safety validated
}

interface ValidatedSearchQuery {
  query: string        // Length validated (1-1000 chars)
  maxResults: number   // Range validated (1-100000)
  workspaceId: string  // Format validated
}
```

## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system-essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*

### Property 1: Compilation Success
*For any* valid Rust source code with proper imports, compilation should succeed without missing type errors
**Validates: Requirements 1.2, 1.5**

### Property 2: Error Type Consistency  
*For any* validation function call with invalid input, the function should return the correct Result type with appropriate AppError variant
**Validates: Requirements 1.2, 2.1**

### Property 3: Type Safety in Lock Management
*For any* lock acquisition operation, the system should use safe type conversion methods without unsafe casts
**Validates: Requirements 1.3, 3.2**

### Property 4: Error Propagation Consistency
*For any* file operation that encounters an error, the system should propagate the error using Result type consistently
**Validates: Requirements 2.2**

### Property 5: Lock Poisoning Handling
*For any* mutex lock operation that encounters poisoning, the system should handle it gracefully without panicking
**Validates: Requirements 2.3**

### Property 6: Archive Error Detail
*For any* archive extraction failure, the error message should contain detailed information including file paths
**Validates: Requirements 2.4**

### Property 7: Search Error Communication
*For any* search operation error, the system should emit appropriate error events to the frontend
**Validates: Requirements 2.5**

### Property 8: Deadlock Prevention
*For any* multiple lock acquisition scenario, locks should be acquired in consistent order to prevent deadlocks
**Validates: Requirements 3.1**

### Property 9: Thread-Safe Cache Access
*For any* concurrent search cache access, operations should be thread-safe without race conditions
**Validates: Requirements 3.3**

### Property 10: Workspace State Protection
*For any* concurrent workspace state modification, the system should protect against race conditions
**Validates: Requirements 3.4**

### Property 11: Safe Cleanup Coordination
*For any* cleanup operation during active operations, the system should coordinate safely without conflicts
**Validates: Requirements 3.5**

### Property 12: Task Deduplication
*For any* duplicate task event from backend, the frontend should prevent creating duplicate tasks
**Validates: Requirements 4.1**

### Property 13: Workspace Status Consistency
*For any* workspace operation completion, the workspace status should be updated consistently
**Validates: Requirements 4.2**

### Property 14: Configuration Save Debouncing
*For any* rapid configuration changes, save operations should be properly debounced to prevent excessive writes
**Validates: Requirements 4.3**

### Property 15: Event Listener Cleanup
*For any* component unmounting, event listeners should be properly cleaned up to prevent memory leaks
**Validates: Requirements 4.4**

### Property 16: Toast Lifecycle Management
*For any* toast notification display, the lifecycle should be managed correctly with proper cleanup
**Validates: Requirements 4.5**

### Property 17: Temporary Directory Cleanup
*For any* temporary directory creation, cleanup should occur on application exit
**Validates: Requirements 5.1**

### Property 18: File Watcher Lifecycle
*For any* file watcher start operation, proper stop mechanisms should be available and functional
**Validates: Requirements 5.2**

### Property 19: Search Cancellation
*For any* search operation cancellation, ongoing file processing should be aborted properly
**Validates: Requirements 5.3**

### Property 20: Workspace Deletion Cleanup
*For any* workspace deletion, all associated resources should be cleaned up in correct order
**Validates: Requirements 5.4**

### Property 21: Application Shutdown Cleanup
*For any* application shutdown, comprehensive cleanup of all resources should be performed
**Validates: Requirements 5.5**

### Property 22: Path Traversal Protection
*For any* path parameter input, the system should validate against path traversal attacks
**Validates: Requirements 6.1**

### Property 23: Workspace ID Safety
*For any* workspace ID submission, only safe characters should be accepted
**Validates: Requirements 6.2**

### Property 24: Query Limits Enforcement
*For any* search query processing, length and complexity limits should be enforced
**Validates: Requirements 6.3**

### Property 25: Unicode Path Handling
*For any* file path processing, Unicode normalization should be handled correctly
**Validates: Requirements 6.4**

### Property 26: Archive Limits Enforcement
*For any* archive file extraction, size and count limits should be enforced
**Validates: Requirements 6.5**

### Property 27: Backend Error Logging
*For any* backend error occurrence, detailed error information with context should be logged
**Validates: Requirements 7.1**

### Property 28: Frontend Error Messages
*For any* frontend operation failure, meaningful error messages should be provided to users
**Validates: Requirements 7.2**

### Property 29: Performance Information Logging
*For any* performance issue detection, timing and resource usage information should be logged
**Validates: Requirements 7.3**

### Property 30: Cache Metrics Tracking
*For any* cache operation, hit rates and performance metrics should be tracked
**Validates: Requirements 7.4**

### Property 31: Cleanup Operation Logging
*For any* cleanup operation execution, success or failure of each step should be logged
**Validates: Requirements 7.5**

## Error Handling

The error handling strategy leverages mature ecosystems:

### Backend Error Handling (eyre + miette + tracing)

1. **eyre for Internal Errors**: Use `eyre::Result` for all internal error handling
   - Automatic context propagation with `.context()` and `.with_context()`
   - Error chaining and stack trace capture
   - Integration with `tracing` for structured logging

2. **miette for User-Facing Errors**: Use `miette::Diagnostic` for errors shown to users
   - Rich error messages with source code snippets
   - Helpful suggestions and documentation links
   - Colored terminal output for better readability

3. **tracing for Error Logging**: Structured error logging with context
   - Automatic span correlation for distributed tracing
   - Error severity levels (error!, warn!, info!)
   - JSON output for log aggregation systems

4. **sentry for Error Monitoring**: Production error tracking
   - Automatic error capture and reporting
   - Performance monitoring and alerting
   - User feedback collection

### Frontend Error Handling (react-error-boundary + react-query)

1. **Error Boundaries**: Catch and handle React component errors
   - Graceful degradation with fallback UI
   - Error reporting to backend/Sentry
   - User-friendly error messages

2. **react-query Error Handling**: Automatic retry and error states
   - Configurable retry logic with exponential backoff
   - Error state management in queries and mutations
   - Optimistic updates with automatic rollback

3. **Toast Notifications**: User-friendly error communication
   - Categorized error messages (error, warning, info)
   - Automatic dismissal with configurable timeout
   - Action buttons for error recovery

4. **Validation Errors**: Structured validation feedback
   - Field-level error messages
   - Real-time validation feedback
   - Accessibility-compliant error announcements

## Testing Strategy

### Modern Testing Ecosystem

#### Backend Testing (Rust)

1. **Unit Testing with `rstest`**: Enhanced unit testing framework
   - Parameterized tests with `#[rstest]`
   - Fixture management for complex test setups
   - Async test support with `#[tokio::test]`

2. **Property-Based Testing with `proptest`**: More advanced than quickcheck
   - Custom strategy generation for domain-specific types
   - Shrinking for minimal failing examples
   - Regression test generation

3. **Integration Testing with `testcontainers`**: Real environment testing
   - Docker-based test environments
   - Database and external service testing
   - Cleanup automation

4. **Benchmarking with `criterion`**: Performance regression detection
   - Statistical analysis of performance changes
   - HTML report generation
   - Continuous benchmarking integration

#### Frontend Testing (TypeScript/React)

1. **Component Testing with `@testing-library/react`**: User-centric testing
   - Accessibility-focused test queries
   - User interaction simulation
   - Async behavior testing

2. **State Management Testing**: Zustand and React Query testing
   - Store behavior verification
   - Query/mutation testing with mock service worker
   - State persistence testing

3. **E2E Testing with `playwright`**: Full application testing
   - Cross-browser testing automation
   - Visual regression testing
   - Performance monitoring

4. **Property Testing with `fast-check`**: Frontend property testing
   - UI state property verification
   - Form validation property testing
   - Event handling property testing

### Test Configuration and Standards

**Backend Test Configuration**:
```rust
// Cargo.toml test configuration
[dev-dependencies]
rstest = "0.18"
proptest = "1.4"
criterion = "0.5"
testcontainers = "0.15"
tokio-test = "0.4"

// Property test configuration
proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]
    
    #[test]
    fn test_workspace_validation(id in "[a-zA-Z0-9\\-_]{1,50}") {
        // Property test implementation
    }
}
```

**Frontend Test Configuration**:
```typescript
// jest.config.js
module.exports = {
  testEnvironment: 'jsdom',
  setupFilesAfterEnv: ['<rootDir>/src/test/setup.ts'],
  transform: {
    '^.+\\.(ts|tsx)$': 'ts-jest',
  },
  moduleNameMapping: {
    '^@/(.*)$': '<rootDir>/src/$1',
  },
}

// Property test configuration
import fc from 'fast-check'

test('workspace state properties', () => {
  fc.assert(fc.property(
    fc.array(workspaceArbitrary),
    (workspaces) => {
      // Property test implementation
    }
  ), { numRuns: 1000 })
})
```

**Test Tagging and Documentation**:
- Each test references specific correctness properties
- Performance benchmarks for critical paths
- Integration tests for external dependencies
- Accessibility tests for UI components

The testing strategy ensures comprehensive coverage through multiple testing approaches, each suited to different aspects of the application.
