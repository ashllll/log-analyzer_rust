# Enhanced Archive Handling - Developer Guide

## Overview

This guide is for developers who want to integrate with, extend, or contribute to the Enhanced Archive Handling system.

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [API Reference](#api-reference)
3. [Extension Points](#extension-points)
4. [Development Setup](#development-setup)
5. [Testing](#testing)
6. [Contributing](#contributing)
7. [Code Examples](#code-examples)

## Architecture Overview

### System Components

```
┌─────────────────────────────────────────────────────────────┐
│                     Public API Layer                         │
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

### Key Design Principles

1. **Iterative Processing**: Uses explicit stack instead of recursion to prevent stack overflow
2. **Streaming**: Processes large files in chunks to minimize memory usage
3. **Security First**: Multiple layers of validation and detection
4. **Extensibility**: Plugin architecture for custom handlers and validators
5. **Observability**: Comprehensive logging and metrics

## API Reference

### Public API

#### Synchronous Extraction

```rust
use log_analyzer::archive::{extract_archive_sync, ExtractionResult};
use std::path::Path;

/// Extract an archive synchronously
///
/// # Arguments
/// * `archive_path` - Path to the archive file
/// * `target_dir` - Directory to extract files to
/// * `workspace_id` - Unique identifier for the workspace
///
/// # Returns
/// * `Result<ExtractionResult>` - Extraction result with metadata
///
/// # Example
/// ```rust
/// let result = extract_archive_sync(
///     Path::new("archive.zip"),
///     Path::new("./output"),
///     "workspace_123"
/// )?;
///
/// println!("Extracted {} files", result.extracted_files.len());
/// ```
pub fn extract_archive_sync(
    archive_path: &Path,
    target_dir: &Path,
    workspace_id: &str,
) -> Result<ExtractionResult>;
```

#### Asynchronous Extraction

```rust
use log_analyzer::archive::{extract_archive_async, ExtractionResult};
use std::path::Path;

/// Extract an archive asynchronously
///
/// # Arguments
/// * `archive_path` - Path to the archive file
/// * `target_dir` - Directory to extract files to
/// * `workspace_id` - Unique identifier for the workspace
///
/// # Returns
/// * `Result<ExtractionResult>` - Extraction result with metadata
///
/// # Example
/// ```rust
/// let result = extract_archive_async(
///     Path::new("archive.zip"),
///     Path::new("./output"),
///     "workspace_123"
/// ).await?;
///
/// println!("Extracted {} files", result.extracted_files.len());
/// ```
pub async fn extract_archive_async(
    archive_path: &Path,
    target_dir: &Path,
    workspace_id: &str,
) -> Result<ExtractionResult>;
```

#### Extraction with Options

```rust
use log_analyzer::archive::{ExtractionOptions, extract_with_options};

/// Extract with custom options
///
/// # Example
/// ```rust
/// let options = ExtractionOptions {
///     max_depth: 5,
///     max_file_size: 50_000_000, // 50MB
///     enable_progress: true,
///     cancellation_token: Some(token),
/// };
///
/// let result = extract_with_options(
///     Path::new("archive.zip"),
///     Path::new("./output"),
///     "workspace_123",
///     options
/// ).await?;
/// ```
pub async fn extract_with_options(
    archive_path: &Path,
    target_dir: &Path,
    workspace_id: &str,
    options: ExtractionOptions,
) -> Result<ExtractionResult>;
```

### Data Structures

#### ExtractionResult

```rust
/// Result of an extraction operation
#[derive(Debug, Clone)]
pub struct ExtractionResult {
    /// Unique identifier for the workspace
    pub workspace_id: String,
    
    /// List of extracted file paths
    pub extracted_files: Vec<PathBuf>,
    
    /// Mapping from shortened paths to original paths
    pub metadata_mappings: HashMap<PathBuf, PathBuf>,
    
    /// Warnings encountered during extraction
    pub warnings: Vec<ExtractionWarning>,
    
    /// Performance metrics
    pub performance_metrics: PerformanceMetrics,
    
    /// Security events detected
    pub security_events: Vec<SecurityEvent>,
}
```

#### ExtractionOptions

```rust
/// Options for customizing extraction behavior
#[derive(Debug, Clone)]
pub struct ExtractionOptions {
    /// Maximum nesting depth (1-20)
    pub max_depth: usize,
    
    /// Maximum size for a single file (bytes)
    pub max_file_size: u64,
    
    /// Maximum total extraction size (bytes)
    pub max_total_size: u64,
    
    /// Enable progress reporting
    pub enable_progress: bool,
    
    /// Progress callback function
    pub progress_callback: Option<Box<dyn Fn(ProgressEvent) + Send>>,
    
    /// Cancellation token
    pub cancellation_token: Option<CancellationToken>,
    
    /// Custom policy configuration
    pub policy: Option<ExtractionPolicy>,
}
```

#### ExtractionError

```rust
/// Error types for extraction operations
#[derive(Debug, thiserror::Error)]
pub enum ExtractionError {
    #[error("Path too long: {path}")]
    PathTooLong { path: String },
    
    #[error("Unsupported archive format: {format}")]
    UnsupportedFormat { format: String },
    
    #[error("Corrupted archive: {details}")]
    CorruptedArchive { details: String },
    
    #[error("Permission denied: {path}")]
    PermissionDenied { path: String },
    
    #[error("Zip bomb detected: compression ratio {ratio}")]
    ZipBombDetected { ratio: f64 },
    
    #[error("Depth limit exceeded: {depth} > {limit}")]
    DepthLimitExceeded { depth: usize, limit: usize },
    
    #[error("Disk space exhausted")]
    DiskSpaceExhausted,
    
    #[error("Extraction cancelled")]
    CancellationRequested,
}
```

### Progress Tracking

```rust
use log_analyzer::archive::{ProgressEvent, ProgressCallback};

/// Progress event structure
#[derive(Debug, Clone)]
pub struct ProgressEvent {
    pub workspace_id: String,
    pub current_file: String,
    pub files_processed: usize,
    pub bytes_processed: u64,
    pub current_depth: usize,
    pub estimated_remaining_time: Option<Duration>,
    pub hierarchical_path: Vec<String>,
}

/// Example: Custom progress handler
fn my_progress_handler(event: ProgressEvent) {
    println!(
        "Progress: {}/{} files, {} MB processed",
        event.files_processed,
        event.files_processed + 100, // estimate
        event.bytes_processed / 1_000_000
    );
}

// Use with extraction
let options = ExtractionOptions {
    enable_progress: true,
    progress_callback: Some(Box::new(my_progress_handler)),
    ..Default::default()
};
```

### Path Management

```rust
use log_analyzer::archive::PathManager;

/// Resolve shortened path to original
///
/// # Example
/// ```rust
/// let path_manager = PathManager::new(config);
/// 
/// let original = path_manager.resolve_original_path(
///     "workspace_123",
///     Path::new("a3f5b2c8d1e4f6a9.txt")
/// ).await?;
///
/// println!("Original path: {}", original.display());
/// ```
pub async fn resolve_original_path(
    &self,
    workspace_id: &str,
    short_path: &Path,
) -> Result<PathBuf>;

/// Get all path mappings for a workspace
pub async fn get_workspace_mappings(
    &self,
    workspace_id: &str,
) -> Result<Vec<PathMapping>>;
```

## Extension Points

### Custom Archive Handlers

Implement the `ArchiveHandler` trait to support custom archive formats:

```rust
use async_trait::async_trait;
use log_analyzer::archive::{ArchiveHandler, ExtractionSummary};

pub struct CustomArchiveHandler {
    // Handler-specific fields
}

#[async_trait]
impl ArchiveHandler for CustomArchiveHandler {
    fn can_handle(&self, path: &Path) -> bool {
        // Check if this handler can process the file
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext == "custom")
            .unwrap_or(false)
    }

    async fn extract_with_limits(
        &self,
        source: &Path,
        target_dir: &Path,
        max_file_size: u64,
        max_total_size: u64,
        max_file_count: usize,
    ) -> Result<ExtractionSummary> {
        // Implement extraction logic
        let mut summary = ExtractionSummary::new();
        
        // ... extraction code ...
        
        Ok(summary)
    }

    fn file_extensions(&self) -> Vec<&str> {
        vec!["custom", "cst"]
    }
}

// Register the handler
let mut registry = ArchiveHandlerRegistry::new();
registry.register(Box::new(CustomArchiveHandler::new()));
```

### Custom Security Validators

Implement custom security checks:

```rust
use log_analyzer::archive::{SecurityValidator, SecurityViolation};

pub struct CustomSecurityValidator {
    // Validator-specific fields
}

impl SecurityValidator for CustomSecurityValidator {
    fn validate_file(
        &self,
        file_path: &Path,
        metadata: &FileMetadata,
    ) -> Result<(), SecurityViolation> {
        // Implement custom validation logic
        
        if metadata.size > 1_000_000_000 {
            return Err(SecurityViolation::FileTooLarge {
                path: file_path.to_path_buf(),
                size: metadata.size,
            });
        }
        
        Ok(())
    }
    
    fn validate_archive(
        &self,
        archive_path: &Path,
        entries: &[ArchiveEntry],
    ) -> Result<(), SecurityViolation> {
        // Validate entire archive structure
        Ok(())
    }
}

// Register the validator
let mut engine = ExtractionEngine::new(config);
engine.add_security_validator(Box::new(CustomSecurityValidator::new()));
```

### Custom Progress Reporters

Implement custom progress reporting:

```rust
use log_analyzer::archive::{ProgressReporter, ProgressEvent};

pub struct CustomProgressReporter {
    // Reporter-specific fields
}

impl ProgressReporter for CustomProgressReporter {
    fn report_progress(&self, event: ProgressEvent) {
        // Send progress to custom destination
        // e.g., WebSocket, message queue, database
        
        self.send_to_websocket(&event);
    }
    
    fn report_error(&self, error: &ExtractionError) {
        // Report errors to monitoring system
        self.send_to_monitoring(error);
    }
}

// Use with extraction
let options = ExtractionOptions {
    progress_reporter: Some(Box::new(CustomProgressReporter::new())),
    ..Default::default()
};
```

### Plugin System

Create plugins for extended functionality:

```rust
use log_analyzer::archive::Plugin;

pub struct MyPlugin {
    name: String,
}

impl Plugin for MyPlugin {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn on_extraction_start(&self, context: &ExtractionContext) {
        // Hook into extraction start
        println!("Starting extraction: {}", context.archive_path.display());
    }
    
    fn on_extraction_complete(&self, result: &ExtractionResult) {
        // Hook into extraction completion
        println!("Completed: {} files extracted", result.extracted_files.len());
    }
    
    fn on_file_extracted(&self, file_path: &Path, size: u64) {
        // Hook into individual file extraction
    }
}

// Register plugin
let mut engine = ExtractionEngine::new(config);
engine.register_plugin(Box::new(MyPlugin {
    name: "my-plugin".to_string(),
}));
```

## Development Setup

### Prerequisites

- Rust 1.70+
- Node.js 18+
- SQLite 3.x
- Git

### Clone and Build

```bash
# Clone repository
git clone https://github.com/your-org/log-analyzer.git
cd log-analyzer

# Install dependencies
npm install

# Build Rust backend
cd src-tauri
cargo build

# Run tests
cargo test

# Build frontend
cd ..
npm run build
```

### Development Workflow

```bash
# Start development server with hot reload
npm run tauri dev

# Run tests in watch mode
cargo watch -x test

# Run specific test
cargo test test_name

# Run benchmarks
cargo bench

# Format code
cargo fmt

# Lint code
cargo clippy
```

### Project Structure

```
log-analyzer/
├── src/                    # Frontend (React/TypeScript)
│   ├── components/
│   ├── pages/
│   └── services/
├── src-tauri/             # Backend (Rust)
│   ├── src/
│   │   ├── archive/       # Archive handling
│   │   ├── models/        # Data models
│   │   ├── services/      # Business logic
│   │   └── utils/         # Utilities
│   ├── tests/             # Integration tests
│   ├── benches/           # Benchmarks
│   └── migrations/        # Database migrations
├── docs/                  # Documentation
└── config/                # Configuration files
```

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_shortening() {
        let manager = PathManager::new(PathConfig::default());
        let long_path = "a".repeat(300);
        let short = manager.hash_path_component(&long_path);
        
        assert_eq!(short.len(), 16);
        assert!(short.chars().all(|c| c.is_alphanumeric()));
    }

    #[tokio::test]
    async fn test_extraction() {
        let temp_dir = TempDir::new().unwrap();
        let archive = create_test_archive(&temp_dir);
        
        let result = extract_archive_async(
            &archive,
            &temp_dir.path().join("output"),
            "test_workspace"
        ).await;
        
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.extracted_files.len() > 0);
    }
}
```

### Property-Based Tests

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
            
            // Shorten path
            let short_path = manager.resolve_extraction_path(
                "test_workspace",
                Path::new(&original_path)
            ).await.unwrap();
            
            // Retrieve original
            let retrieved = manager.resolve_original_path(
                "test_workspace",
                &short_path
            ).await.unwrap();
            
            prop_assert_eq!(retrieved.to_str().unwrap(), original_path);
        });
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_deep_nesting_integration() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create 15-level nested archive
    let archive = create_nested_archive(&temp_dir, 15);
    
    let engine = ExtractionEngine::new(ExtractionPolicy {
        max_depth: 10,
        ..Default::default()
    });
    
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

### Benchmarks

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_extraction(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    
    c.bench_function("extract_1mb_archive", |b| {
        b.iter(|| {
            runtime.block_on(async {
                let archive = create_test_archive(1_000_000);
                extract_archive_async(
                    black_box(&archive),
                    black_box(&Path::new("./output")),
                    black_box("test")
                ).await.unwrap()
            })
        })
    });
}

criterion_group!(benches, bench_extraction);
criterion_main!(benches);
```

## Contributing

### Code Style

Follow Rust standard style:

```bash
# Format code
cargo fmt

# Check style
cargo fmt -- --check

# Lint
cargo clippy -- -D warnings
```

### Commit Messages

Use conventional commits:

```
feat: Add support for 7z archives
fix: Resolve path traversal vulnerability
docs: Update API documentation
test: Add property tests for path manager
perf: Optimize streaming extraction
```

### Pull Request Process

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Update documentation
6. Run full test suite
7. Submit pull request

### Code Review Checklist

- [ ] Tests added/updated
- [ ] Documentation updated
- [ ] No compiler warnings
- [ ] Clippy passes
- [ ] Benchmarks run (if performance-critical)
- [ ] Security implications considered
- [ ] Backward compatibility maintained

## Code Examples

### Example 1: Basic Extraction

```rust
use log_analyzer::archive::extract_archive_async;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let result = extract_archive_async(
        Path::new("archive.zip"),
        Path::new("./output"),
        "my_workspace"
    ).await?;
    
    println!("Extracted {} files", result.extracted_files.len());
    println!("Total size: {} bytes", result.performance_metrics.bytes_extracted);
    
    Ok(())
}
```

### Example 2: Extraction with Progress

```rust
use log_analyzer::archive::{extract_with_options, ExtractionOptions, ProgressEvent};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = ExtractionOptions {
        enable_progress: true,
        progress_callback: Some(Box::new(|event: ProgressEvent| {
            println!(
                "Progress: {} files, {} MB",
                event.files_processed,
                event.bytes_processed / 1_000_000
            );
        })),
        ..Default::default()
    };
    
    let result = extract_with_options(
        Path::new("large_archive.zip"),
        Path::new("./output"),
        "my_workspace",
        options
    ).await?;
    
    Ok(())
}
```

### Example 3: Extraction with Cancellation

```rust
use log_analyzer::archive::{extract_with_options, ExtractionOptions};
use tokio_util::sync::CancellationToken;
use std::path::Path;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = CancellationToken::new();
    let token_clone = token.clone();
    
    // Spawn task to cancel after 10 seconds
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(10)).await;
        token_clone.cancel();
    });
    
    let options = ExtractionOptions {
        cancellation_token: Some(token),
        ..Default::default()
    };
    
    match extract_with_options(
        Path::new("archive.zip"),
        Path::new("./output"),
        "my_workspace",
        options
    ).await {
        Ok(result) => println!("Completed: {} files", result.extracted_files.len()),
        Err(e) if e.to_string().contains("cancelled") => {
            println!("Extraction cancelled");
        }
        Err(e) => return Err(e.into()),
    }
    
    Ok(())
}
```

### Example 4: Custom Archive Handler

```rust
use log_analyzer::archive::{ArchiveHandler, ExtractionSummary, ArchiveHandlerRegistry};
use async_trait::async_trait;
use std::path::Path;

pub struct SevenZipHandler;

#[async_trait]
impl ArchiveHandler for SevenZipHandler {
    fn can_handle(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext == "7z")
            .unwrap_or(false)
    }

    async fn extract_with_limits(
        &self,
        source: &Path,
        target_dir: &Path,
        max_file_size: u64,
        max_total_size: u64,
        max_file_count: usize,
    ) -> Result<ExtractionSummary> {
        // Use 7z library or command-line tool
        let mut summary = ExtractionSummary::new();
        
        // Implementation details...
        
        Ok(summary)
    }

    fn file_extensions(&self) -> Vec<&str> {
        vec!["7z"]
    }
}

// Register handler
fn main() {
    let mut registry = ArchiveHandlerRegistry::global();
    registry.register(Box::new(SevenZipHandler));
}
```

### Example 5: Path Mapping Lookup

```rust
use log_analyzer::archive::PathManager;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path_manager = PathManager::new(Default::default());
    
    // Get original path from shortened path
    let original = path_manager.resolve_original_path(
        "workspace_123",
        Path::new("output/a3f5b2c8d1e4f6a9.txt")
    ).await?;
    
    println!("Original path: {}", original.display());
    
    // Get all mappings for workspace
    let mappings = path_manager.get_workspace_mappings("workspace_123").await?;
    
    for mapping in mappings {
        println!("{} -> {}", mapping.short_path, mapping.original_path);
    }
    
    Ok(())
}
```

## API Versioning

The API follows semantic versioning:

- **Major version**: Breaking changes
- **Minor version**: New features, backward compatible
- **Patch version**: Bug fixes

Current version: `2.0.0`

### Deprecation Policy

- Deprecated features are marked with `#[deprecated]`
- Deprecated features are supported for at least 2 minor versions
- Migration guides provided for breaking changes

## Performance Considerations

### Memory Usage

- Streaming extraction uses fixed buffer size (default 64KB)
- Memory usage: ~100MB per concurrent extraction
- Path mapping cache uses LRU eviction

### CPU Usage

- Concurrent extractions limited to CPU cores / 2
- Compression/decompression is CPU-intensive
- Consider reducing concurrency for CPU-bound workloads

### Disk I/O

- Batch directory creation to reduce syscalls
- Use SSD storage for best performance
- Temporary files cleaned up automatically

## Security Considerations

### Input Validation

- All paths validated for traversal attempts
- Filenames checked for invalid characters
- Archive structure validated before extraction

### Resource Limits

- File size limits enforced
- Total extraction size limited
- Nesting depth limited
- Compression ratio checked

### Audit Logging

- All operations logged with user context
- Security events logged at WARN level
- Structured JSON format for analysis

## Support and Resources

- **Documentation**: https://docs.example.com
- **API Reference**: https://api-docs.example.com
- **GitHub**: https://github.com/your-org/log-analyzer
- **Issues**: https://github.com/your-org/log-analyzer/issues
- **Discussions**: https://github.com/your-org/log-analyzer/discussions

## License

This project is licensed under the MIT License - see the LICENSE file for details.
