# Design Document

## Overview

增强型压缩包处理系统采用分层架构设计，核心组件包括：路径管理器（PathManager）、解压引擎（ExtractionEngine）、安全检测器（SecurityDetector）、进度追踪器（ProgressTracker）和配置管理器（PolicyManager）。系统使用迭代式深度优先遍历替代递归调用，通过显式栈管理避免栈溢出；采用Windows长路径支持（UNC前缀）和内容哈希缩短策略解决路径长度限制；实现多层次的压缩炸弹检测机制保障系统安全。

## Architecture

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     API Layer                                │
│  ┌──────────────────┐  ┌──────────────────┐                │
│  │ Sync Extraction  │  │ Async Extraction │                │
│  │     API          │  │      API         │                │
│  └────────┬─────────┘  └────────┬─────────┘                │
└───────────┼────────────────────┼──────────────────────────┘
            │                    │
┌───────────┼────────────────────┼──────────────────────────┐
│           │   Orchestration Layer                          │
│           ▼                    ▼                           │
│  ┌─────────────────────────────────────────┐              │
│  │      ExtractionOrchestrator             │              │
│  │  - Request deduplication                │              │
│  │  - Concurrency control                  │              │
│  │  - Cancellation handling                │              │
│  └──────────────┬──────────────────────────┘              │
└─────────────────┼─────────────────────────────────────────┘
                  │
┌─────────────────┼─────────────────────────────────────────┐
│                 │   Core Processing Layer                  │
│                 ▼                                          │
│  ┌──────────────────────────────────────────────────┐    │
│  │         ExtractionEngine                          │    │
│  │  - Iterative depth-first traversal               │    │
│  │  - Explicit stack management                     │    │
│  │  - Stream-based extraction                       │    │
│  └──┬────────┬────────┬────────┬────────┬──────────┘    │
│     │        │        │        │        │                │
│     ▼        ▼        ▼        ▼        ▼                │
│  ┌────┐  ┌────┐  ┌────┐  ┌────┐  ┌────────┐            │
│  │Path│  │Sec │  │Prog│  │Meta│  │Archive │            │
│  │Mgr │  │Det │  │Trac│  │DB  │  │Handler │            │
│  └────┘  └────┘  └────┘  └────┘  └────────┘            │
└──────────────────────────────────────────────────────────┘
```

### Component Responsibilities

1. **ExtractionOrchestrator**: 请求协调、并发控制、取消处理
2. **ExtractionEngine**: 核心解压逻辑、迭代遍历、流式处理
3. **PathManager**: 路径长度预测、缩短策略、映射管理
4. **SecurityDetector**: 压缩炸弹检测、异常模式识别
5. **ProgressTracker**: 进度事件发射、层级追踪
6. **MetadataDB**: SQLite持久化存储、路径映射查询
7. **ArchiveHandler**: 多格式支持（ZIP/RAR/TAR/GZ）

## Components and Interfaces

### 1. PathManager

**职责**: 管理路径长度、应用缩短策略、维护原始路径映射

```rust
pub struct PathManager {
    config: PathConfig,
    metadata_db: Arc<MetadataDB>,
    shortening_cache: DashMap<String, String>,
}

pub struct PathConfig {
    pub max_path_length: usize,        // OS-specific limit
    pub shortening_threshold: f32,      // 0.8 = 80% of max
    pub enable_long_paths: bool,        // Windows UNC prefix
    pub hash_algorithm: HashAlgorithm, // SHA256
    pub hash_length: usize,             // 16 chars
}

impl PathManager {
    /// Predict final path length before extraction
    pub fn predict_path_length(
        &self,
        base_path: &Path,
        archive_name: &str,
        internal_path: &str,
        depth: usize,
    ) -> usize;

    /// Apply path shortening if needed
    pub async fn resolve_extraction_path(
        &self,
        workspace_id: &str,
        full_path: &Path,
    ) -> Result<PathBuf>;

    /// Get original path from shortened path
    pub async fn resolve_original_path(
        &self,
        workspace_id: &str,
        short_path: &Path,
    ) -> Result<PathBuf>;

    /// Apply Windows long path support
    fn apply_long_path_prefix(&self, path: &Path) -> PathBuf;

    /// Generate content-based hash for path component
    fn hash_path_component(&self, component: &str) -> String;
}
```

### 2. ExtractionEngine

**职责**: 迭代式解压、深度控制、流式处理

```rust
pub struct ExtractionEngine {
    path_manager: Arc<PathManager>,
    security_detector: Arc<SecurityDetector>,
    progress_tracker: Arc<ProgressTracker>,
    policy: ExtractionPolicy,
}

pub struct ExtractionContext {
    pub workspace_id: String,
    pub current_depth: usize,
    pub parent_archive: Option<PathBuf>,
    pub accumulated_size: u64,
    pub accumulated_files: usize,
    pub start_time: Instant,
}

pub struct ExtractionStack {
    items: Vec<ExtractionItem>,
}

pub struct ExtractionItem {
    pub archive_path: PathBuf,
    pub target_dir: PathBuf,
    pub depth: usize,
    pub parent_context: ExtractionContext,
}

impl ExtractionEngine {
    /// Main extraction entry point (iterative, not recursive)
    pub async fn extract_archive(
        &self,
        archive_path: &Path,
        target_dir: &Path,
        workspace_id: &str,
    ) -> Result<ExtractionResult>;

    /// Iterative depth-first traversal
    async fn extract_iterative(
        &self,
        initial_item: ExtractionItem,
    ) -> Result<ExtractionResult>;

    /// Process single archive file
    async fn process_archive_file(
        &self,
        item: &ExtractionItem,
        stack: &mut ExtractionStack,
    ) -> Result<Vec<PathBuf>>;

    /// Stream-based file extraction
    async fn extract_file_streaming(
        &self,
        reader: impl Read,
        target_path: &Path,
        expected_size: u64,
    ) -> Result<u64>;
}
```

### 3. SecurityDetector

**职责**: 压缩炸弹检测、异常模式识别、风险评分

```rust
pub struct SecurityDetector {
    policy: SecurityPolicy,
    metrics_collector: Arc<MetricsCollector>,
}

pub struct SecurityPolicy {
    pub max_compression_ratio: f64,      // 100.0
    pub max_cumulative_size: u64,        // 10GB
    pub max_workspace_size: u64,         // 50GB
    pub exponential_backoff_threshold: f64, // 1_000_000.0
}

pub struct CompressionMetrics {
    pub compressed_size: u64,
    pub uncompressed_size: u64,
    pub compression_ratio: f64,
    pub nesting_depth: usize,
    pub risk_score: f64,
}

impl SecurityDetector {
    /// Calculate compression ratio for a file
    pub fn calculate_compression_ratio(
        &self,
        compressed_size: u64,
        uncompressed_size: u64,
    ) -> f64;

    /// Calculate risk score using exponential backoff
    pub fn calculate_risk_score(
        &self,
        compression_ratio: f64,
        nesting_depth: usize,
    ) -> f64;

    /// Check if extraction should be halted
    pub fn should_halt_extraction(
        &self,
        metrics: &CompressionMetrics,
        context: &ExtractionContext,
    ) -> (bool, Option<SecurityViolation>);

    /// Detect suspicious patterns
    pub fn detect_suspicious_patterns(
        &self,
        archive_path: &Path,
        entries: &[ArchiveEntry],
    ) -> Vec<SecurityWarning>;
}
```

### 4. ProgressTracker

**职责**: 进度事件发射、层级追踪、性能指标收集

```rust
pub struct ProgressTracker {
    event_emitter: Arc<dyn EventEmitter>,
    metrics: Arc<RwLock<ProgressMetrics>>,
}

pub struct ProgressMetrics {
    pub files_processed: AtomicUsize,
    pub bytes_processed: AtomicU64,
    pub current_depth: AtomicUsize,
    pub max_depth_reached: AtomicUsize,
    pub errors_by_category: DashMap<ErrorCategory, usize>,
    pub path_shortenings_applied: AtomicUsize,
    pub suspicious_files_detected: AtomicUsize,
}

pub struct ProgressEvent {
    pub workspace_id: String,
    pub current_file: String,
    pub files_processed: usize,
    pub bytes_processed: u64,
    pub current_depth: usize,
    pub estimated_remaining_time: Option<Duration>,
    pub hierarchical_path: Vec<String>, // Parent-child relationships
}

impl ProgressTracker {
    /// Emit progress event
    pub async fn emit_progress(
        &self,
        context: &ExtractionContext,
        current_file: &Path,
    ) -> Result<()>;

    /// Record error with categorization
    pub fn record_error(
        &self,
        category: ErrorCategory,
        error: &Error,
    );

    /// Generate final summary report
    pub fn generate_summary(&self) -> ExtractionSummary;

    /// Estimate remaining time based on current progress
    fn estimate_remaining_time(
        &self,
        context: &ExtractionContext,
    ) -> Option<Duration>;
}
```

### 5. MetadataDB

**职责**: SQLite持久化存储、路径映射查询、清理管理

```rust
pub struct MetadataDB {
    pool: SqlitePool,
}

// Schema
// CREATE TABLE path_mappings (
//     id INTEGER PRIMARY KEY AUTOINCREMENT,
//     workspace_id TEXT NOT NULL,
//     short_path TEXT NOT NULL,
//     original_path TEXT NOT NULL,
//     created_at INTEGER NOT NULL,
//     access_count INTEGER DEFAULT 0,
//     UNIQUE(workspace_id, short_path)
// );
// CREATE INDEX idx_workspace_short ON path_mappings(workspace_id, short_path);
// CREATE INDEX idx_workspace_original ON path_mappings(workspace_id, original_path);

impl MetadataDB {
    /// Store path mapping
    pub async fn store_mapping(
        &self,
        workspace_id: &str,
        short_path: &str,
        original_path: &str,
    ) -> Result<()>;

    /// Retrieve original path
    pub async fn get_original_path(
        &self,
        workspace_id: &str,
        short_path: &str,
    ) -> Result<Option<String>>;

    /// Retrieve shortened path
    pub async fn get_short_path(
        &self,
        workspace_id: &str,
        original_path: &str,
    ) -> Result<Option<String>>;

    /// Cleanup mappings for deleted workspace
    pub async fn cleanup_workspace(
        &self,
        workspace_id: &str,
    ) -> Result<usize>;

    /// Increment access counter
    pub async fn increment_access_count(
        &self,
        workspace_id: &str,
        short_path: &str,
    ) -> Result<()>;
}
```

### 6. PolicyManager

**职责**: 配置加载、验证、热更新

```rust
pub struct PolicyManager {
    config_path: PathBuf,
    current_policy: Arc<RwLock<ExtractionPolicy>>,
}

pub struct ExtractionPolicy {
    pub max_depth: usize,                // 10
    pub max_file_size: u64,              // 100MB
    pub max_total_size: u64,             // 10GB
    pub max_workspace_size: u64,         // 50GB
    pub compression_ratio_threshold: f64, // 100.0
    pub enable_long_paths: bool,         // true
    pub concurrent_extractions: usize,   // CPU cores / 2
    pub buffer_size: usize,              // 64KB
    pub temp_dir_ttl: Duration,          // 24 hours
    pub log_retention_days: usize,       // 90
}

impl PolicyManager {
    /// Load policy from TOML file
    pub async fn load_policy(&self) -> Result<ExtractionPolicy>;

    /// Validate policy constraints
    pub fn validate_policy(&self, policy: &ExtractionPolicy) -> Result<()>;

    /// Apply new policy (hot reload)
    pub async fn update_policy(&self, policy: ExtractionPolicy) -> Result<()>;

    /// Get current active policy
    pub fn get_policy(&self) -> ExtractionPolicy;
}
```

## Data Models

### Core Data Structures

```rust
/// Extraction result returned to caller
pub struct ExtractionResult {
    pub workspace_id: String,
    pub extracted_files: Vec<PathBuf>,
    pub metadata_mappings: HashMap<PathBuf, PathBuf>, // short -> original
    pub warnings: Vec<ExtractionWarning>,
    pub performance_metrics: PerformanceMetrics,
    pub security_events: Vec<SecurityEvent>,
}

/// Warning during extraction
pub struct ExtractionWarning {
    pub category: WarningCategory,
    pub message: String,
    pub file_path: Option<PathBuf>,
    pub timestamp: SystemTime,
}

pub enum WarningCategory {
    PathShortened,
    DepthLimitReached,
    HighCompressionRatio,
    DuplicateFilename,
    UnicodeNormalization,
    InsufficientDiskSpace,
}

/// Security event for audit
pub struct SecurityEvent {
    pub event_type: SecurityEventType,
    pub severity: Severity,
    pub archive_path: PathBuf,
    pub details: serde_json::Value,
    pub timestamp: SystemTime,
}

pub enum SecurityEventType {
    ZipBombDetected,
    PathTraversalAttempt,
    ForbiddenExtension,
    ExcessiveCompressionRatio,
    DepthLimitExceeded,
}

/// Performance metrics
pub struct PerformanceMetrics {
    pub total_duration: Duration,
    pub files_extracted: usize,
    pub bytes_extracted: u64,
    pub max_depth_reached: usize,
    pub average_extraction_speed: f64, // MB/s
    pub peak_memory_usage: usize,
    pub disk_io_operations: usize,
}

/// Error with structured information
pub struct ExtractionError {
    pub error_code: ErrorCode,
    pub error_message: String,
    pub failed_file_path: Option<PathBuf>,
    pub suggested_remediation: String,
    pub context: HashMap<String, String>,
}

pub enum ErrorCode {
    PathTooLong,
    UnsupportedFormat,
    CorruptedArchive,
    PermissionDenied,
    ZipBombDetected,
    DepthLimitExceeded,
    DiskSpaceExhausted,
    CancellationRequested,
}
```

### Configuration Schema (TOML)

```toml
[extraction]
max_depth = 10
max_file_size = 104857600  # 100MB
max_total_size = 10737418240  # 10GB
max_workspace_size = 53687091200  # 50GB
concurrent_extractions = 4
buffer_size = 65536  # 64KB

[security]
compression_ratio_threshold = 100.0
exponential_backoff_threshold = 1000000.0
enable_zip_bomb_detection = true

[paths]
enable_long_paths = true
shortening_threshold = 0.8
hash_algorithm = "SHA256"
hash_length = 16

[performance]
temp_dir_ttl_hours = 24
log_retention_days = 90
enable_streaming = true

[audit]
enable_audit_logging = true
log_format = "json"
log_level = "info"
```

## Correctness Properties


*A property is a characteristic or behavior that should hold true across all valid executions of a system-essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*

### Long Filename Handling Properties

**Property 1: Long filename support**
*For any* filename with length between 256 and the OS maximum (32,767 on Windows, 255 bytes per component on Unix), the system should successfully process the file without truncation or error.
**Validates: Requirements 1.1**

**Property 2: Windows UNC prefix application**
*For any* path on Windows exceeding 260 characters, the system should automatically prepend the UNC prefix (\\?\) to enable long path support.
**Validates: Requirements 1.2**

**Property 3: Path shortening consistency**
*For any* filename exceeding the OS limit, applying the path shortening strategy twice should produce the same shortened path (idempotent hashing).
**Validates: Requirements 1.3**

**Property 4: Path mapping round-trip**
*For any* path that undergoes shortening, retrieving the original path from the shortened path should return the exact original path (bidirectional mapping integrity).
**Validates: Requirements 1.4**

**Property 5: Transparent path resolution**
*For any* shortened path stored in the system, resolving it for user display should return the original path without requiring explicit user action.
**Validates: Requirements 1.5**

### Nesting Depth Control Properties

**Property 6: Depth limit enforcement**
*For any* archive extraction operation, the maximum nesting depth should never exceed the configured limit (default 10 levels).
**Validates: Requirements 2.1**

**Property 7: Iterative traversal stack safety**
*For any* deeply nested archive structure (up to 20 levels), the extraction process should complete without stack overflow errors.
**Validates: Requirements 2.3**

**Property 8: Extraction context consistency**
*For any* file being extracted, the ExtractionContext should accurately reflect the current depth, parent archive, and accumulated metrics.
**Validates: Requirements 2.4**

**Property 9: Sibling processing independence**
*For any* archive tree where one branch reaches the depth limit, all sibling branches at the same level should continue processing normally.
**Validates: Requirements 2.5**

### Security and Zip Bomb Detection Properties

**Property 10: Compression ratio calculation**
*For any* extracted file, the calculated compression ratio should equal uncompressed_size / compressed_size with floating-point precision.
**Validates: Requirements 3.1**

**Property 11: Suspicious file flagging**
*For any* file with compression ratio exceeding the configured threshold (default 100:1), the system should flag it as suspicious.
**Validates: Requirements 3.2**

**Property 12: Cumulative size limit enforcement**
*For any* extraction operation, when the cumulative extracted size exceeds the configured limit, extraction should halt immediately.
**Validates: Requirements 3.3**

**Property 13: Exponential backoff scoring**
*For any* nested archive at depth d with compression ratio r, the risk score should equal r^d.
**Validates: Requirements 3.4**

**Property 14: Security metrics logging**
*For any* file flagged as suspicious, the system should log compression_ratio, nesting_depth, extracted_size, and extraction_time.
**Validates: Requirements 3.5**

### Path Length Management Properties

**Property 15: Path length prediction accuracy**
*For any* extraction path, the predicted length should match the actual length within ±5 characters (accounting for path separators).
**Validates: Requirements 4.1**

**Property 16: Automatic shortening trigger**
*For any* path whose predicted length exceeds 80% of the OS limit, path shortening should be automatically applied.
**Validates: Requirements 4.2**

**Property 17: Hierarchical shortening order**
*For any* path requiring shortening, the system should first shorten timestamp suffixes, then apply hash-based shortening to the longest components.
**Validates: Requirements 4.3**

**Property 18: Collision counter uniqueness**
*For any* set of archives that would extract to the same shortened path, each should receive a unique collision counter (_001, _002, etc.).
**Validates: Requirements 4.4**

**Property 19: Mapping persistence**
*For any* shortened path, the mapping should be retrievable from the SQLite database even after system restart.
**Validates: Requirements 4.5**

### Progress Reporting Properties

**Property 20: Progress event completeness**
*For any* extraction operation, all emitted progress events should contain: current_file, files_processed, bytes_processed, current_depth, and hierarchical_path.
**Validates: Requirements 5.1**

**Property 21: Hierarchical progress structure**
*For any* nested archive extraction, the progress events should maintain parent-child relationships visible in the hierarchical_path field.
**Validates: Requirements 5.2**

**Property 22: Error categorization and resilience**
*For any* error during extraction, the error should be categorized into one of the defined categories, and extraction should continue with remaining files.
**Validates: Requirements 5.3**

**Property 23: Resumption checkpoint consistency**
*For any* extraction that is paused and resumed, the system should continue from the last successfully extracted file without re-extracting previous files.
**Validates: Requirements 5.4**

**Property 24: Summary report completeness**
*For any* completed extraction, the summary report should contain: total_files, total_bytes, max_depth_reached, errors_by_category, path_shortenings_applied, and suspicious_files_detected.
**Validates: Requirements 5.5**

### Configuration Management Properties

**Property 25: Configuration loading correctness**
*For any* valid TOML configuration file, the system should load all policy values correctly without data loss or type conversion errors.
**Validates: Requirements 6.1**

**Property 26: Configuration validation enforcement**
*For any* configuration update, all constraints (e.g., max_depth in range 1-20, positive sizes) should be validated before application.
**Validates: Requirements 6.3**

**Property 27: Invalid configuration rejection**
*For any* configuration that fails validation, the system should reject it and continue using the previous valid configuration.
**Validates: Requirements 6.4**

**Property 28: Policy logging**
*For any* policy application or update, the system should log the complete active policy set at INFO level.
**Validates: Requirements 6.5**

### Edge Case Handling Properties

**Property 29: Unicode normalization consistency**
*For any* path containing Unicode characters, the system should normalize it to NFC form, and normalizing twice should produce the same result (idempotent).
**Validates: Requirements 7.1**

**Property 30: Duplicate filename uniqueness**
*For any* archive containing duplicate filenames (case-insensitive on Windows), the system should ensure all extracted files have unique names by appending numeric suffixes.
**Validates: Requirements 7.2**

**Property 31: Incomplete extraction detection**
*For any* extraction interrupted by process crash or power loss, restarting the system should detect the incomplete state and offer cleanup or resume options.
**Validates: Requirements 7.3**

**Property 32: Disk space pre-flight check**
*For any* extraction operation, the system should check available disk space before starting and fail fast if insufficient space is detected.
**Validates: Requirements 7.4**

**Property 33: Circular reference detection**
*For any* archive containing circular symlink references, the system should detect the cycle and skip the problematic entries without infinite loops.
**Validates: Requirements 7.5**

### Performance Properties

**Property 34: Concurrency limit enforcement**
*For any* number of concurrent extraction requests, the number of actively executing extractions should never exceed the configured limit (default: CPU cores / 2).
**Validates: Requirements 8.1**

**Property 35: Streaming memory bounds**
*For any* large archive extraction, the peak memory usage should not exceed buffer_size * concurrent_extractions + overhead (estimated 10MB).
**Validates: Requirements 8.2**

**Property 36: Directory creation batching**
*For any* extraction creating multiple directories, the number of filesystem syscalls should be less than the number of directories (due to batching).
**Validates: Requirements 8.3**

**Property 37: Temporary file cleanup**
*For any* extraction operation, all temporary files should be located in the dedicated temp directory and cleaned up within the configured TTL (default 24 hours).
**Validates: Requirements 8.4**

**Property 38: Resource release timing**
*For any* completed extraction, all file handles and memory buffers should be released within 5 seconds of completion.
**Validates: Requirements 8.5**

### Audit and Compliance Properties

**Property 39: Audit log completeness**
*For any* extraction operation, the audit log should contain: timestamp, user_id, workspace_id, archive_path, and extraction_policy_applied.
**Validates: Requirements 9.1**

**Property 40: Security event logging**
*For any* security event (zip bomb, path traversal, etc.), the system should log at WARN level with full context including event type, severity, and details.
**Validates: Requirements 9.2**

**Property 41: Completion logging**
*For any* extraction that completes or fails, the system should log: duration, files_extracted, bytes_extracted, and errors_encountered.
**Validates: Requirements 9.3**

**Property 42: Structured log format**
*For any* audit log entry, the log should be valid JSON with consistent field names across all log entries.
**Validates: Requirements 9.4**

**Property 43: Log rotation triggers**
*For any* log file, rotation should occur when either the file age exceeds 24 hours OR the file size exceeds 100MB.
**Validates: Requirements 9.5**

### API Integration Properties

**Property 44: Cancellation responsiveness**
*For any* in-progress extraction, invoking cancellation should stop the extraction within 2 seconds and perform graceful cleanup.
**Validates: Requirements 10.2**

**Property 45: Request deduplication**
*For any* set of concurrent extraction requests targeting the same archive, only one extraction should execute and all requests should receive the same result.
**Validates: Requirements 10.3**

**Property 46: Error structure completeness**
*For any* extraction failure, the returned error should contain: error_code, error_message, failed_file_path (if applicable), and suggested_remediation.
**Validates: Requirements 10.4**

**Property 47: Result structure completeness**
*For any* successful extraction, the ExtractionResult should contain: extracted_files, metadata_mappings, warnings, performance_metrics, and security_events.
**Validates: Requirements 10.5**

## Error Handling

### Error Categories and Recovery Strategies

```rust
pub enum ErrorCategory {
    PathTooLong,           // Recovery: Apply path shortening
    UnsupportedFormat,     // Recovery: Skip file, continue extraction
    CorruptedArchive,      // Recovery: Log error, skip archive
    PermissionDenied,      // Recovery: Log error, skip file
    ZipBombDetected,       // Recovery: Halt extraction, alert user
    DepthLimitExceeded,    // Recovery: Skip nested archive, continue siblings
    DiskSpaceExhausted,    // Recovery: Fail fast, cleanup partial extraction
    CancellationRequested, // Recovery: Graceful cleanup, return partial results
}
```

### Error Handling Principles

1. **Fail Fast for Critical Errors**: Zip bombs, disk space exhaustion → immediate halt
2. **Continue on Recoverable Errors**: Corrupted files, permission issues → log and skip
3. **Graceful Degradation**: Path too long → apply shortening and continue
4. **Detailed Error Context**: Always include file path, error code, suggested remediation
5. **Audit Trail**: All errors logged with categorization for analysis

### Retry Logic

```rust
pub struct RetryPolicy {
    pub max_attempts: usize,        // 3
    pub initial_delay: Duration,    // 100ms
    pub backoff_multiplier: f64,    // 2.0
    pub max_delay: Duration,        // 5s
    pub retryable_errors: Vec<ErrorCode>,
}

// Retryable errors: PermissionDenied (transient), CorruptedArchive (may be I/O issue)
// Non-retryable: ZipBombDetected, DepthLimitExceeded, CancellationRequested
```

## Testing Strategy

### Unit Testing

**Focus Areas:**
- Path shortening algorithm (hash generation, collision handling)
- Compression ratio calculation (edge cases: zero-byte files, highly compressed)
- Risk score calculation (exponential backoff formula)
- Configuration validation (boundary values, invalid inputs)
- Unicode normalization (various Unicode forms)

**Example Unit Tests:**
```rust
#[test]
fn test_path_shortening_idempotent() {
    let manager = PathManager::new(PathConfig::default());
    let long_path = "a".repeat(300);
    let short1 = manager.hash_path_component(&long_path);
    let short2 = manager.hash_path_component(&long_path);
    assert_eq!(short1, short2);
}

#[test]
fn test_compression_ratio_zero_compressed_size() {
    let detector = SecurityDetector::new(SecurityPolicy::default());
    let ratio = detector.calculate_compression_ratio(0, 1000);
    assert!(ratio.is_infinite());
}
```

### Property-Based Testing

**Testing Framework**: Use `proptest` crate for Rust property-based testing

**Key Properties to Test:**
1. Path shortening round-trip (Property 4)
2. Depth limit enforcement (Property 6)
3. Compression ratio calculation (Property 10)
4. Exponential backoff scoring (Property 13)
5. Unicode normalization idempotence (Property 29)
6. Concurrency limit enforcement (Property 34)

**Example Property Test:**
```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn prop_path_mapping_round_trip(
        original_path in "[a-zA-Z0-9_-]{256,500}"
    ) {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let manager = PathManager::new(PathConfig::default());
            let db = MetadataDB::new(":memory:").await.unwrap();
            
            // Shorten path
            let short_path = manager.resolve_extraction_path(
                "test_workspace",
                Path::new(&original_path)
            ).await.unwrap();
            
            // Store mapping
            db.store_mapping(
                "test_workspace",
                short_path.to_str().unwrap(),
                &original_path
            ).await.unwrap();
            
            // Retrieve original
            let retrieved = db.get_original_path(
                "test_workspace",
                short_path.to_str().unwrap()
            ).await.unwrap();
            
            prop_assert_eq!(retrieved, Some(original_path));
        });
    }
}
```

### Integration Testing

**Test Scenarios:**
1. **Deep Nesting Test**: Create 15-level nested archives, verify depth limit enforcement
2. **Zip Bomb Test**: Create archive with 1000:1 compression ratio, verify detection
3. **Long Path Test**: Create archive with 500-character filenames, verify extraction success
4. **Concurrent Extraction Test**: Submit 20 concurrent requests, verify concurrency limit
5. **Interruption Recovery Test**: Kill process mid-extraction, verify resume capability
6. **Disk Space Test**: Fill disk to near capacity, verify pre-flight check

**Example Integration Test:**
```rust
#[tokio::test]
async fn test_deep_nesting_enforcement() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create 15-level nested archive
    let archive = create_nested_archive(&temp_dir, 15);
    
    let engine = ExtractionEngine::new(
        ExtractionPolicy {
            max_depth: 10,
            ..Default::default()
        }
    );
    
    let result = engine.extract_archive(
        &archive,
        &temp_dir.path().join("output"),
        "test_workspace"
    ).await.unwrap();
    
    // Verify max depth reached is 10
    assert_eq!(result.performance_metrics.max_depth_reached, 10);
    
    // Verify warning was logged
    assert!(result.warnings.iter().any(|w| 
        matches!(w.category, WarningCategory::DepthLimitReached)
    ));
}
```

### Performance Testing

**Benchmarks:**
1. **Extraction Speed**: Measure MB/s for various archive sizes (1MB, 100MB, 1GB)
2. **Memory Usage**: Monitor peak memory during large archive extraction
3. **Concurrency Scaling**: Test throughput with 1, 2, 4, 8 concurrent extractions
4. **Path Shortening Overhead**: Measure latency added by shortening logic
5. **Database Query Performance**: Benchmark path mapping lookups (1K, 10K, 100K entries)

**Performance Targets:**
- Extraction speed: > 50 MB/s for uncompressed archives
- Memory usage: < 100MB per concurrent extraction
- Path shortening overhead: < 1ms per path
- Database lookup: < 1ms for 100K entries
- Concurrency scaling: Linear up to CPU core count

### Security Testing

**Attack Scenarios:**
1. **Zip Bomb**: 42.zip (42KB → 4.5PB), verify detection and halt
2. **Path Traversal**: Archives with ../../../etc/passwd, verify rejection
3. **Symlink Attack**: Circular symlinks, verify cycle detection
4. **Resource Exhaustion**: 1 million tiny files, verify file count limit
5. **Filename Injection**: Filenames with null bytes, control characters, verify sanitization

## Implementation Notes

### Technology Stack

- **Language**: Rust 1.70+
- **Async Runtime**: Tokio 1.x
- **Database**: SQLite with `sqlx` crate
- **Compression Libraries**: 
  - `zip` crate for ZIP format
  - `unrar` binary for RAR format
  - `tar` + `flate2` for TAR/GZ formats
- **Hashing**: `sha2` crate for SHA-256
- **Configuration**: `toml` crate for TOML parsing
- **Logging**: `tracing` crate for structured logging
- **Property Testing**: `proptest` crate

### Platform-Specific Considerations

**Windows:**
- Enable long path support via UNC prefix (\\?\)
- Handle case-insensitive filesystem
- Use Windows-specific path validation (reserved names, invalid characters)

**Unix/Linux:**
- Support 255-byte filename limit per component
- Handle case-sensitive filesystem
- Support POSIX permissions and symlinks

**macOS:**
- Handle HFS+ Unicode normalization (NFD vs NFC)
- Support extended attributes
- Handle case-insensitive but case-preserving filesystem

### Migration Path

**Phase 1: Core Infrastructure**
- Implement PathManager with shortening logic
- Implement MetadataDB with SQLite schema
- Implement PolicyManager with TOML loading

**Phase 2: Extraction Engine**
- Implement iterative ExtractionEngine
- Implement SecurityDetector with zip bomb detection
- Implement ProgressTracker with event emission

**Phase 3: Integration**
- Integrate with existing ArchiveManager
- Update import commands to use new engine
- Add configuration UI for policy management

**Phase 4: Testing & Validation**
- Comprehensive property-based tests
- Integration tests with real-world archives
- Performance benchmarking and optimization

### Backward Compatibility

- Existing archives extracted with old system remain accessible
- New system can read old path mappings (if any)
- Configuration migration tool for existing deployments
- Gradual rollout with feature flag control
