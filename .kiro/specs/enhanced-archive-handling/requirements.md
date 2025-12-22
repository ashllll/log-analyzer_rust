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

1. WHEN the System encounters a filename longer than 255 characters THEN the System SHALL support filenames up to the operating system maximum (Windows: 32,767 characters with long path support enabled, Unix: 255 bytes per path component)
2. WHEN processing long filenames on Windows exceeding 260 characters THEN the System SHALL automatically prepend the UNC prefix (\\?\) to enable long path support
3. WHEN a filename exceeds the operating system limit THEN the System SHALL apply path shortening using SHA-256 hash truncated to 16 hexadecimal characters while preserving the original file extension
4. WHERE path shortening is applied THEN the System SHALL store a bidirectional mapping between the original path and shortened path in a persistent SQLite database
5. WHEN retrieving files with shortened paths THEN the System SHALL resolve the shortened path to the original path and display the original path to the user

### Requirement 2

**User Story:** As a log analyst, I want to safely extract deeply nested archives (5+ levels), so that I can analyze complex log archive structures without system crashes or resource exhaustion.

#### Acceptance Criteria

1. WHEN the System begins archive extraction THEN the System SHALL enforce a configurable maximum nesting depth with default value of 10 levels and valid range of 1 to 20 levels
2. WHEN the nesting depth exceeds the configured maximum THEN the System SHALL halt extraction of that specific branch and emit a warning log entry containing the archive path and current depth
3. WHEN processing nested archives THEN the System SHALL use iterative depth-first traversal with explicit stack data structure to prevent stack overflow
4. WHEN tracking extraction state THEN the System SHALL maintain an ExtractionContext structure containing current_depth, parent_archive_path, accumulated_size, and accumulated_file_count fields
5. WHERE extraction depth limit is reached for one branch THEN the System SHALL continue processing sibling archives at the same nesting level without terminating the entire extraction operation

### Requirement 3

**User Story:** As a security engineer, I want the system to detect and prevent zip bomb attacks, so that malicious archives cannot exhaust system resources.

#### Acceptance Criteria

1. WHEN the System extracts any file from an archive THEN the System SHALL calculate the compression ratio as uncompressed_size divided by compressed_size
2. WHEN the compression ratio exceeds the configured threshold (default: 100.0) THEN the System SHALL flag the file as suspicious and halt extraction pending explicit user confirmation
3. WHEN the cumulative extracted size exceeds the configured per-archive limit (default: 10 gigabytes) or per-workspace limit (default: 50 gigabytes) THEN the System SHALL halt extraction and emit a zip bomb detection event
4. WHEN detecting nested archives with elevated compression ratios THEN the System SHALL calculate risk score as compression_ratio raised to the power of nesting_depth and reject archives with risk score exceeding 1,000,000
5. WHERE suspicious compression patterns are detected THEN the System SHALL log compression_ratio, nesting_depth, extracted_size, and extraction_time metrics for security audit purposes

### Requirement 4

**User Story:** As a system operator, I want intelligent path length management, so that extraction succeeds even when full paths would exceed OS limits.

#### Acceptance Criteria

1. WHEN constructing extraction paths THEN the System SHALL calculate the predicted final path length before creating any directories
2. WHEN the predicted path length exceeds 80 percent of the operating system limit (Windows: 208 characters, Unix: 3,276 characters) THEN the System SHALL automatically apply path shortening
3. WHEN shortening paths THEN the System SHALL apply hierarchical shortening in the following order: first shorten timestamp suffixes, then apply hash-based shortening to the longest path components
4. WHERE multiple archives would extract to identical shortened paths THEN the System SHALL append a unique collision counter suffix (format: _001, _002, _003) to ensure filesystem uniqueness
5. WHEN path shortening is applied THEN the System SHALL persist the mapping in a SQLite database table with columns: workspace_id, short_path, original_path, created_at, access_count

### Requirement 5

**User Story:** As a developer, I want comprehensive extraction progress reporting, so that I can monitor long-running extraction operations and diagnose issues.

#### Acceptance Criteria

1. WHEN extraction begins THEN the System SHALL emit progress events containing fields: current_file, files_processed, bytes_processed, current_depth, and estimated_remaining_time
2. WHEN processing nested archives THEN the System SHALL include hierarchical_path field in progress events showing parent-child archive relationships
3. WHEN errors occur during extraction THEN the System SHALL categorize each error as one of: path_too_long, unsupported_format, corrupted_archive, permission_denied, or zip_bomb_detected, and continue processing remaining files
4. WHERE extraction is paused or cancelled THEN the System SHALL write checkpoint metadata enabling resumption from the last successfully extracted file
5. WHEN extraction completes THEN the System SHALL generate a summary report containing: total_files, total_bytes, max_depth_reached, errors_by_category, path_shortenings_applied, and suspicious_files_detected

### Requirement 6

**User Story:** As a system architect, I want configurable extraction policies, so that different environments can apply appropriate security and performance trade-offs.

#### Acceptance Criteria

1. WHEN the System initializes THEN the System SHALL load extraction policies from a TOML format configuration file with validation against defined constraints
2. WHERE no configuration file is provided THEN the System SHALL use secure default values: max_depth equals 10, max_file_size equals 100 megabytes, max_total_size equals 10 gigabytes, compression_ratio_threshold equals 100, and enable_long_paths equals true
3. WHEN configuration is updated THEN the System SHALL validate all constraints (max_depth within range 1 to 20, max_file_size positive, max_total_size positive) before applying the new configuration
4. WHERE configuration validation fails THEN the System SHALL reject the invalid configuration, emit detailed validation error logs, and continue using the previous valid configuration
5. WHEN policies are applied or updated THEN the System SHALL log the complete active policy set at INFO severity level for audit purposes

### Requirement 7

**User Story:** As a quality assurance engineer, I want the system to handle edge cases gracefully, so that extraction remains robust under adverse conditions.

#### Acceptance Criteria

1. WHEN encountering paths with Unicode normalization differences (NFC versus NFD forms) THEN the System SHALL normalize all paths to NFC form before processing
2. WHEN processing archives containing duplicate filenames (case-insensitive comparison on Windows) THEN the System SHALL append a numeric suffix to ensure each extracted file has a unique name
3. WHEN extraction is interrupted by process crash or power loss THEN the System SHALL detect incomplete extractions on restart and present cleanup or resume options to the operator
4. WHERE available disk space is insufficient for extraction THEN the System SHALL detect the condition before extraction begins, calculate required space, and fail immediately with a descriptive error message
5. WHEN processing archives containing circular symlink references (symlinks pointing to parent directories) THEN the System SHALL detect the cycle and skip the problematic symlink entries with a warning log entry

### Requirement 8

**User Story:** As a performance engineer, I want efficient resource utilization during extraction, so that the system can handle large-scale operations without degradation.

#### Acceptance Criteria

1. WHEN extracting multiple archives concurrently THEN the System SHALL limit the number of concurrent extractions to a configurable maximum (default: CPU core count divided by 2) to prevent resource exhaustion
2. WHEN processing large archives THEN the System SHALL use streaming extraction with a configurable buffer size (default: 64 kilobytes) to minimize memory footprint
3. WHEN creating extraction directories THEN the System SHALL batch directory creation operations to reduce the number of filesystem system calls
4. WHERE temporary files are created during extraction THEN the System SHALL store them in a dedicated temporary directory with automatic cleanup on process exit or after a configurable time-to-live (default: 24 hours)
5. WHEN extraction completes THEN the System SHALL release all file handles and memory buffers within 5 seconds to prevent resource leaks

### Requirement 9

**User Story:** As a compliance officer, I want detailed audit logs of extraction operations, so that I can track data access and security incidents.

#### Acceptance Criteria

1. WHEN any extraction operation begins THEN the System SHALL log the following fields: timestamp, user_id, workspace_id, archive_path, and extraction_policy_applied
2. WHEN security events occur (zip bomb detected, path traversal attempt detected, forbidden file extension detected) THEN the System SHALL log at WARN severity level with complete contextual information
3. WHEN extraction completes or fails THEN the System SHALL log the following metrics: duration, files_extracted, bytes_extracted, errors_encountered, and security_flags_raised
4. WHERE audit logs are written THEN the System SHALL use structured JSON format with consistent field names to enable automated log analysis
5. WHEN log rotation is triggered THEN the System SHALL rotate logs daily or when log file size exceeds 100 megabytes, and retain logs for a configurable period (default: 90 days)

### Requirement 10

**User Story:** As an integration developer, I want a clean API for archive extraction, so that I can easily integrate the enhanced extraction capabilities into existing workflows.

#### Acceptance Criteria

1. WHEN calling the extraction API THEN the System SHALL provide both synchronous and asynchronous interface variants with consistent error handling behavior
2. WHEN extraction is in progress THEN the System SHALL support cancellation via a cancellation token with graceful resource cleanup
3. WHEN multiple extraction requests target the same archive file THEN the System SHALL deduplicate the requests and share the single extraction result among all requesters
4. WHERE extraction fails THEN the System SHALL return structured error information containing: error_code, error_message, failed_file_path, and suggested_remediation
5. WHEN extraction succeeds THEN the System SHALL return an ExtractionResult structure containing: extracted_files list, metadata_mappings dictionary, warnings list, performance_metrics object, and security_events list
