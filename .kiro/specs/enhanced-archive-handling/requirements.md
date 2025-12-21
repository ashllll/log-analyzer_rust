# Requirements Document

## Introduction

本需求文档定义了增强型压缩包处理系统的功能需求，旨在解决当前系统在处理超长文件名（256+字符）和深层嵌套压缩包（5层以上）时的限制。该系统将采用业界成熟的解决方案，包括路径缩短策略、嵌套深度控制、压缩炸弹防护等关键技术。

## Glossary

- **System**: 增强型压缩包处理系统（Enhanced Archive Handling System）
- **Archive**: 压缩文件，包括 ZIP、RAR、TAR、GZ 等格式
- **Nesting Level**: 压缩包嵌套层级，指压缩包内包含压缩包的深度
- **Path Shortening**: 路径缩短策略，使用哈希或短ID替代长路径组件
- **Zip Bomb**: 压缩炸弹，恶意构造的压缩包，解压后占用大量磁盘空间或内存
- **Extraction Context**: 解压上下文，包含当前解压深度、累计大小等状态信息
- **Long Path Support**: 长路径支持，Windows上使用UNC路径前缀（\\?\）突破260字符限制
- **Compression Ratio**: 压缩比，解压后大小与压缩包大小的比值
- **Decompression Bomb Detection**: 解压炸弹检测，基于压缩比和嵌套深度的异常检测

## Requirements

### Requirement 1

**User Story:** As a system administrator, I want to process archives with filenames exceeding 255 characters, so that I can handle log files from various systems without filename truncation.

#### Acceptance Criteria

1. WHEN the System encounters a filename longer than 255 characters THEN the System SHALL support filenames up to the operating system maximum (Windows: 32,767 characters with long path support, Unix: 255 bytes per component)
2. WHEN processing long filenames on Windows THEN the System SHALL automatically enable long path support using UNC prefix (\\?\) for paths exceeding 260 characters
3. WHEN a filename exceeds the operating system limit THEN the System SHALL apply path shortening strategy using content-based hashing (SHA-256 truncated to 16 characters) while preserving file extension
4. WHERE path shortening is applied THEN the System SHALL maintain a bidirectional mapping between original and shortened paths in a persistent metadata store
5. WHEN retrieving files with shortened paths THEN the System SHALL transparently resolve shortened names to original names for user display

### Requirement 2

**User Story:** As a log analyst, I want to safely extract deeply nested archives (5+ levels), so that I can analyze complex log archive structures without system crashes or resource exhaustion.

#### Acceptance Criteria

1. WHEN the System begins archive extraction THEN the System SHALL enforce a configurable maximum nesting depth (default: 10 levels, range: 1-20)
2. WHEN the nesting depth exceeds the configured maximum THEN the System SHALL halt extraction of that branch and log a warning with the archive path
3. WHEN processing nested archives THEN the System SHALL use iterative depth-first traversal with explicit stack management instead of recursive calls to prevent stack overflow
4. WHEN tracking extraction depth THEN the System SHALL maintain an ExtractionContext structure containing current_depth, parent_archive_path, and accumulated_metrics
5. WHERE extraction depth limit is reached THEN the System SHALL continue processing sibling archives at the same level without terminating the entire extraction operation

### Requirement 3

**User Story:** As a security engineer, I want the system to detect and prevent zip bomb attacks, so that malicious archives cannot exhaust system resources.

#### Acceptance Criteria

1. WHEN the System extracts any archive THEN the System SHALL calculate the compression ratio (uncompressed_size / compressed_size) for each file
2. WHEN the compression ratio exceeds a configurable threshold (default: 100:1) THEN the System SHALL flag the file as suspicious and require explicit user confirmation to continue
3. WHEN the cumulative extracted size exceeds a configurable limit (default: 10GB per archive, 50GB per workspace) THEN the System SHALL halt extraction and report a potential zip bomb
4. WHEN detecting nested archives with high compression ratios THEN the System SHALL apply exponential backoff scoring (score = ratio^depth) and reject archives with score > 1,000,000
5. WHERE suspicious patterns are detected THEN the System SHALL log detailed metrics (compression_ratio, nesting_depth, extracted_size, extraction_time) for security audit

### Requirement 4

**User Story:** As a system operator, I want intelligent path length management, so that extraction succeeds even when full paths would exceed OS limits.

#### Acceptance Criteria

1. WHEN constructing extraction paths THEN the System SHALL predict the final path length before creating directories
2. WHEN the predicted path length exceeds 80% of the OS limit (Windows: 208 chars, Unix: 3276 chars) THEN the System SHALL automatically apply path shortening strategy
3. WHEN shortening paths THEN the System SHALL use a hierarchical approach: first shorten timestamp suffixes, then apply hash-based shortening to longest path components
4. WHERE multiple archives extract to the same shortened path THEN the System SHALL append a collision counter (e.g., _001, _002) to ensure uniqueness
5. WHEN path shortening is applied THEN the System SHALL store the mapping in a SQLite database with schema: (workspace_id, short_path, original_path, created_at, access_count)

### Requirement 5

**User Story:** As a developer, I want comprehensive extraction progress reporting, so that I can monitor long-running extraction operations and diagnose issues.

#### Acceptance Criteria

1. WHEN extraction begins THEN the System SHALL emit progress events containing: current_file, files_processed, bytes_processed, current_depth, estimated_remaining_time
2. WHEN processing nested archives THEN the System SHALL report hierarchical progress with parent-child relationships visible in the progress structure
3. WHEN errors occur during extraction THEN the System SHALL categorize errors (path_too_long, unsupported_format, corrupted_archive, permission_denied, zip_bomb_detected) and continue processing remaining files
4. WHERE extraction is paused or cancelled THEN the System SHALL support resumption from the last successfully extracted file using checkpoint metadata
5. WHEN extraction completes THEN the System SHALL generate a detailed summary report including: total_files, total_bytes, max_depth_reached, errors_by_category, path_shortenings_applied, suspicious_files_detected

### Requirement 6

**User Story:** As a system architect, I want configurable extraction policies, so that different environments can apply appropriate security and performance trade-offs.

#### Acceptance Criteria

1. WHEN the System initializes THEN the System SHALL load extraction policies from a configuration file (TOML format) with validation against a JSON schema
2. WHERE no configuration is provided THEN the System SHALL use secure defaults: max_depth=10, max_file_size=100MB, max_total_size=10GB, compression_ratio_threshold=100, enable_long_paths=true
3. WHEN configuration is updated THEN the System SHALL validate all constraints (e.g., max_depth must be 1-20, max_file_size must be positive) before applying changes
4. WHERE configuration validation fails THEN the System SHALL reject the configuration, log detailed validation errors, and continue using the previous valid configuration
5. WHEN policies are applied THEN the System SHALL log the active policy set at INFO level for audit purposes

### Requirement 7

**User Story:** As a quality assurance engineer, I want the system to handle edge cases gracefully, so that extraction remains robust under adverse conditions.

#### Acceptance Criteria

1. WHEN encountering Unicode normalization issues (NFC vs NFD) THEN the System SHALL normalize all paths to NFC form before processing
2. WHEN processing archives with duplicate filenames (case-insensitive on Windows) THEN the System SHALL append a numeric suffix to ensure uniqueness
3. WHEN extraction is interrupted (process crash, power loss) THEN the System SHALL detect incomplete extractions on restart and offer cleanup or resume options
4. WHERE disk space is insufficient THEN the System SHALL detect the condition before extraction, estimate required space, and fail fast with a clear error message
5. WHEN processing archives with circular references (symlinks pointing to parent directories) THEN the System SHALL detect cycles and skip the problematic entries with a warning

### Requirement 8

**User Story:** As a performance engineer, I want efficient resource utilization during extraction, so that the system can handle large-scale operations without degradation.

#### Acceptance Criteria

1. WHEN extracting multiple archives concurrently THEN the System SHALL limit concurrent extractions to a configurable number (default: CPU cores / 2) to prevent resource exhaustion
2. WHEN processing large archives THEN the System SHALL use streaming extraction with a configurable buffer size (default: 64KB) to minimize memory footprint
3. WHEN creating extraction directories THEN the System SHALL batch directory creation operations to reduce filesystem syscalls
4. WHERE temporary files are created THEN the System SHALL use a dedicated temporary directory with automatic cleanup on process exit or after configurable TTL (default: 24 hours)
5. WHEN extraction completes THEN the System SHALL release all file handles and memory buffers within 5 seconds to prevent resource leaks

### Requirement 9

**User Story:** As a compliance officer, I want detailed audit logs of extraction operations, so that I can track data access and security incidents.

#### Acceptance Criteria

1. WHEN any extraction operation begins THEN the System SHALL log: timestamp, user_id, workspace_id, archive_path, extraction_policy_applied
2. WHEN security events occur (zip bomb detected, path traversal attempt, forbidden extension) THEN the System SHALL log at WARN level with full context
3. WHEN extraction completes or fails THEN the System SHALL log: duration, files_extracted, bytes_extracted, errors_encountered, security_flags_raised
4. WHERE audit logs are written THEN the System SHALL use structured logging (JSON format) with consistent field names for automated analysis
5. WHEN log rotation is needed THEN the System SHALL rotate logs daily or when size exceeds 100MB, retaining logs for a configurable period (default: 90 days)

### Requirement 10

**User Story:** As an integration developer, I want a clean API for archive extraction, so that I can easily integrate the enhanced extraction capabilities into existing workflows.

#### Acceptance Criteria

1. WHEN calling the extraction API THEN the System SHALL provide both synchronous and asynchronous interfaces with consistent error handling
2. WHEN extraction is in progress THEN the System SHALL support cancellation via a cancellation token with graceful cleanup
3. WHEN multiple extraction requests target the same archive THEN the System SHALL deduplicate requests and share the extraction result
4. WHERE extraction fails THEN the System SHALL provide structured error information including: error_code, error_message, failed_file_path, suggested_remediation
5. WHEN extraction succeeds THEN the System SHALL return a comprehensive ExtractionResult structure containing: extracted_files, metadata_mappings, warnings, performance_metrics
