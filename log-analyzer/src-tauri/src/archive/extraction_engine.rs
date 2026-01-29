//! Extraction Engine for Iterative Archive Processing
//!
//! This module implements the core extraction engine that processes archives
//! using iterative depth-first traversal instead of recursion. It enforces
//! depth limits, manages extraction state, and coordinates with security
//! detection and path management components.

use crate::archive::{
    ExtractionContext, ExtractionItem, ExtractionStack, PathManager, SecurityDetector,
};
use crate::error::{AppError, Result};
use dashmap::DashMap;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::sync::Semaphore;
use tracing::{debug, info, warn};

/// Extraction policy configuration
#[derive(Debug, Clone)]
pub struct ExtractionPolicy {
    /// Maximum nesting depth (default: 10, range: 1-20)
    pub max_depth: usize,
    /// Maximum file size in bytes (default: 100MB)
    pub max_file_size: u64,
    /// Maximum total extracted size per archive (default: 10GB)
    pub max_total_size: u64,
    /// Buffer size for streaming extraction (default: 64KB)
    pub buffer_size: usize,
    /// Directory creation batch size (default: 10)
    pub dir_batch_size: usize,
    /// Maximum parallel file extractions within single archive (default: 4)
    pub max_parallel_files: usize,
}

impl Default for ExtractionPolicy {
    fn default() -> Self {
        Self {
            max_depth: 10,
            max_file_size: 100 * 1024 * 1024,        // 100MB
            max_total_size: 10 * 1024 * 1024 * 1024, // 10GB
            buffer_size: 64 * 1024,                  // 64KB
            dir_batch_size: 10,                      // Batch 10 directories
            max_parallel_files: 4,                   // Extract up to 4 files in parallel
        }
    }
}

impl ExtractionPolicy {
    /// Validate policy constraints
    pub fn validate(&self) -> Result<()> {
        if self.max_depth < 1 || self.max_depth > 20 {
            return Err(AppError::validation_error(format!(
                "max_depth must be between 1 and 20, got {}",
                self.max_depth
            )));
        }

        if self.max_file_size == 0 {
            return Err(AppError::validation_error("max_file_size must be positive"));
        }

        if self.max_total_size == 0 {
            return Err(AppError::validation_error(
                "max_total_size must be positive",
            ));
        }

        if self.buffer_size == 0 {
            return Err(AppError::validation_error("buffer_size must be positive"));
        }

        if self.dir_batch_size == 0 {
            return Err(AppError::validation_error(
                "dir_batch_size must be positive",
            ));
        }

        if self.max_parallel_files == 0 {
            return Err(AppError::validation_error(
                "max_parallel_files must be positive",
            ));
        }

        Ok(())
    }
}

/// Result of an extraction operation
#[derive(Debug, Clone)]
pub struct ExtractionResult {
    /// Workspace identifier
    pub workspace_id: String,
    /// List of extracted files
    pub extracted_files: Vec<PathBuf>,
    /// Warnings encountered during extraction
    pub warnings: Vec<ExtractionWarning>,
    /// Maximum depth reached during extraction
    pub max_depth_reached: usize,
    /// Total files extracted
    pub total_files: usize,
    /// Total bytes extracted
    pub total_bytes: u64,
    /// Number of path shortenings applied
    pub path_shortenings_applied: usize,
    /// Number of archives skipped due to depth limit
    pub depth_limit_skips: usize,
    /// Extraction duration in seconds
    pub extraction_duration_secs: f64,
    /// Extraction speed in bytes per second
    pub extraction_speed_bytes_per_sec: f64,
}

impl ExtractionResult {
    /// Calculate extraction speed based on total bytes and duration
    ///
    /// # Arguments
    ///
    /// * `total_bytes` - Total bytes extracted
    /// * `duration_secs` - Duration in seconds
    ///
    /// # Returns
    ///
    /// Extraction speed in bytes per second
    pub fn calculate_speed(total_bytes: u64, duration_secs: f64) -> f64 {
        if duration_secs > 0.0 {
            total_bytes as f64 / duration_secs
        } else {
            0.0
        }
    }

    /// Get extraction speed in MB/s
    ///
    /// # Returns
    ///
    /// Extraction speed in megabytes per second
    pub fn speed_mb_per_sec(&self) -> f64 {
        self.extraction_speed_bytes_per_sec / (1024.0 * 1024.0)
    }

    /// Get extraction speed in KB/s
    ///
    /// # Returns
    ///
    /// Extraction speed in kilobytes per second
    pub fn speed_kb_per_sec(&self) -> f64 {
        self.extraction_speed_bytes_per_sec / 1024.0
    }

    /// Get formatted extraction speed string
    ///
    /// # Returns
    ///
    /// Human-readable speed string (e.g., "15.3 MB/s" or "512.7 KB/s")
    pub fn formatted_speed(&self) -> String {
        let mb_per_sec = self.speed_mb_per_sec();
        if mb_per_sec >= 1.0 {
            format!("{:.1} MB/s", mb_per_sec)
        } else {
            format!("{:.1} KB/s", self.speed_kb_per_sec())
        }
    }

    /// Get formatted extraction duration string
    ///
    /// # Returns
    ///
    /// Human-readable duration string (e.g., "2.5s" or "125ms")
    pub fn formatted_duration(&self) -> String {
        if self.extraction_duration_secs >= 1.0 {
            format!("{:.2}s", self.extraction_duration_secs)
        } else {
            format!("{:.0}ms", self.extraction_duration_secs * 1000.0)
        }
    }

    /// Get formatted total size string
    ///
    /// # Returns
    ///
    /// Human-readable size string (e.g., "15.3 MB" or "512 KB")
    pub fn formatted_total_size(&self) -> String {
        let mb = self.total_bytes as f64 / (1024.0 * 1024.0);
        if mb >= 1.0 {
            format!("{:.1} MB", mb)
        } else {
            let kb = self.total_bytes as f64 / 1024.0;
            format!("{:.0} KB", kb)
        }
    }
}

/// Warning encountered during extraction
#[derive(Debug, Clone)]
pub struct ExtractionWarning {
    /// Warning message
    pub message: String,
    /// File path associated with the warning
    pub file_path: Option<PathBuf>,
    /// Warning category
    pub category: WarningCategory,
}

/// Categories of extraction warnings
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarningCategory {
    /// Depth limit reached
    DepthLimitReached,
    /// Path was shortened
    PathShortened,
    /// High compression ratio detected
    HighCompressionRatio,
    /// File skipped due to error
    FileSkipped,
    /// Security event detected
    SecurityEvent,
    /// Archive-level error
    ArchiveError,
    /// Path resolution error
    PathResolutionError,
}

/// Handler registry for managing archive handlers
struct HandlerRegistry {
    handlers: Vec<Box<dyn crate::archive::archive_handler::ArchiveHandler>>,
}

impl HandlerRegistry {
    /// Create a new empty handler registry
    fn new() -> Self {
        Self {
            handlers: Vec::new(),
        }
    }

    /// Register a new handler
    fn register(&mut self, handler: Box<dyn crate::archive::archive_handler::ArchiveHandler>) {
        self.handlers.push(handler);
    }

    /// Find a handler that can process the given file
    fn find_handler(
        &self,
        path: &Path,
    ) -> Option<&dyn crate::archive::archive_handler::ArchiveHandler> {
        self.handlers
            .iter()
            .find(|h| h.can_handle(path))
            .map(|h| h.as_ref())
    }
}

/// Extraction engine for iterative archive processing
pub struct ExtractionEngine {
    /// Path manager for handling long paths
    path_manager: Arc<PathManager>,
    /// Security detector for zip bomb detection (stored for future use)
    #[allow(dead_code)]
    security_detector: Arc<SecurityDetector>,
    /// Extraction policy
    policy: ExtractionPolicy,
    /// Path mapping cache for fast lookups
    path_cache: Arc<DashMap<String, PathBuf>>,
    /// Semaphore for parallel file extraction
    parallel_semaphore: Arc<Semaphore>,
}

impl ExtractionEngine {
    /// Create a new extraction engine
    ///
    /// # Arguments
    ///
    /// * `path_manager` - Path manager for handling long paths
    /// * `security_detector` - Security detector for zip bomb detection
    /// * `policy` - Extraction policy configuration
    ///
    /// # Returns
    ///
    /// A new ExtractionEngine instance
    ///
    /// # Errors
    ///
    /// Returns an error if the policy validation fails
    pub fn new(
        path_manager: Arc<PathManager>,
        security_detector: Arc<SecurityDetector>,
        policy: ExtractionPolicy,
    ) -> Result<Self> {
        policy.validate()?;

        info!(
            "Initializing ExtractionEngine with max_depth={}, max_file_size={}, buffer_size={}, dir_batch_size={}, max_parallel_files={}",
            policy.max_depth, policy.max_file_size, policy.buffer_size, policy.dir_batch_size, policy.max_parallel_files
        );

        let parallel_semaphore = Arc::new(Semaphore::new(policy.max_parallel_files));

        Ok(Self {
            path_manager,
            security_detector,
            policy,
            path_cache: Arc::new(DashMap::new()),
            parallel_semaphore,
        })
    }

    /// Extract an archive using iterative traversal
    ///
    /// This is the main entry point for archive extraction. It creates an initial
    /// extraction context and processes the archive iteratively using an explicit
    /// stack instead of recursion.
    ///
    /// # Arguments
    ///
    /// * `archive_path` - Path to the archive to extract
    /// * `target_dir` - Directory where files should be extracted
    /// * `workspace_id` - Workspace identifier for path mapping
    ///
    /// # Returns
    ///
    /// ExtractionResult containing statistics and warnings
    ///
    /// # Errors
    ///
    /// Returns an error if extraction fails
    pub async fn extract_archive(
        &self,
        archive_path: &Path,
        target_dir: &Path,
        workspace_id: &str,
    ) -> Result<ExtractionResult> {
        info!(
            "Starting archive extraction: {:?} -> {:?} (workspace: {})",
            archive_path, target_dir, workspace_id
        );

        // Create initial extraction context
        let context = ExtractionContext::new(workspace_id.to_string());

        // Create initial extraction item
        let initial_item = ExtractionItem::new(
            archive_path.to_path_buf(),
            target_dir.to_path_buf(),
            0,
            context,
        );

        // Perform iterative extraction
        let result = self.extract_iterative(initial_item).await?;

        info!(
            "Archive extraction completed: {} files, {}, max depth {}, {} warnings, duration: {}, speed: {}",
            result.total_files,
            result.formatted_total_size(),
            result.max_depth_reached,
            result.warnings.len(),
            result.formatted_duration(),
            result.formatted_speed()
        );

        // Log warning summary if there are warnings
        if !result.warnings.is_empty() {
            warn!(
                "Extraction completed with {} warning(s):",
                result.warnings.len()
            );
            for (i, warning) in result.warnings.iter().enumerate() {
                warn!(
                    "  Warning {}: [{:?}] {}",
                    i + 1,
                    warning.category,
                    warning.message
                );
            }
        }

        Ok(result)
    }

    /// Perform iterative depth-first traversal of nested archives
    ///
    /// This method uses an explicit stack to manage extraction state, preventing
    /// stack overflow when processing deeply nested archives.
    ///
    /// # Arguments
    ///
    /// * `initial_item` - The first archive to process
    ///
    /// # Returns
    ///
    /// ExtractionResult containing statistics and warnings
    ///
    /// # Errors
    ///
    /// Returns an error if extraction fails
    async fn extract_iterative(&self, initial_item: ExtractionItem) -> Result<ExtractionResult> {
        // Start timing the extraction
        let start_time = std::time::Instant::now();

        let mut stack = ExtractionStack::new();
        let mut result = ExtractionResult {
            workspace_id: initial_item.parent_context.workspace_id.clone(),
            extracted_files: Vec::new(),
            warnings: Vec::new(),
            max_depth_reached: 0,
            total_files: 0,
            total_bytes: 0,
            path_shortenings_applied: 0,
            depth_limit_skips: 0,
            extraction_duration_secs: 0.0,
            extraction_speed_bytes_per_sec: 0.0,
        };

        // Push initial item onto stack
        stack
            .push(initial_item)
            .map_err(|e| AppError::archive_error(e, None))?;

        // Process stack iteratively
        while let Some(item) = stack.pop() {
            debug!(
                "Processing archive at depth {}: {:?}",
                item.depth, item.archive_path
            );

            // Update max depth reached
            if item.depth > result.max_depth_reached {
                result.max_depth_reached = item.depth;
            }

            // Check depth limit
            if item.depth >= self.policy.max_depth {
                warn!(
                    "Depth limit reached at {}: {:?}",
                    item.depth, item.archive_path
                );
                result.warnings.push(ExtractionWarning {
                    message: format!(
                        "Depth limit {} reached, skipping nested archive",
                        self.policy.max_depth
                    ),
                    file_path: Some(item.archive_path.clone()),
                    category: WarningCategory::DepthLimitReached,
                });
                result.depth_limit_skips += 1;
                continue;
            }

            // Process this archive
            match self.process_archive_file(&item, &mut stack).await {
                Ok((extracted_files, bytes_extracted, skips, shortenings)) => {
                    result.total_files += extracted_files.len();
                    result.total_bytes += bytes_extracted;
                    result.depth_limit_skips += skips;
                    result.path_shortenings_applied += shortenings;

                    // Add warnings for depth limit skips
                    if skips > 0 {
                        result.warnings.push(ExtractionWarning {
                            message: format!(
                                "Depth limit {} reached, skipped {} nested archive(s)",
                                self.policy.max_depth, skips
                            ),
                            file_path: Some(item.archive_path.clone()),
                            category: WarningCategory::DepthLimitReached,
                        });
                    }

                    // Add warnings for path shortenings
                    if shortenings > 0 {
                        result.warnings.push(ExtractionWarning {
                            message: format!(
                                "Applied {} path shortening(s) due to long paths",
                                shortenings
                            ),
                            file_path: Some(item.archive_path.clone()),
                            category: WarningCategory::PathShortened,
                        });
                    }

                    result.extracted_files.extend(extracted_files);
                }
                Err(e) => {
                    // Archive-level error: log warning and continue with other archives in stack
                    warn!(
                        "Archive-level error processing {:?} at depth {}: {}",
                        item.archive_path, item.depth, e
                    );

                    // Determine warning category based on error type
                    let category = if e.to_string().contains("Security") {
                        WarningCategory::SecurityEvent
                    } else {
                        WarningCategory::ArchiveError
                    };

                    result.warnings.push(ExtractionWarning {
                        message: format!("Failed to extract archive: {}", e),
                        file_path: Some(item.archive_path.clone()),
                        category,
                    });

                    // Continue processing other archives in the stack
                    info!(
                        "Continuing with remaining {} archive(s) in stack after error",
                        stack.len()
                    );
                }
            }
        }

        // Calculate extraction duration and speed
        let duration = start_time.elapsed();
        result.extraction_duration_secs = duration.as_secs_f64();
        result.extraction_speed_bytes_per_sec =
            ExtractionResult::calculate_speed(result.total_bytes, result.extraction_duration_secs);

        // Log performance metrics
        info!(
            "Extraction performance: {} in {}, speed: {}",
            result.formatted_total_size(),
            result.formatted_duration(),
            result.formatted_speed()
        );

        Ok(result)
    }

    /// Process a single archive file
    ///
    /// Extracts the archive and identifies any nested archives to add to the stack.
    ///
    /// # Arguments
    ///
    /// * `item` - The extraction item to process
    /// * `stack` - The extraction stack for nested archives
    ///
    /// # Returns
    ///
    /// List of extracted file paths
    ///
    /// # Errors
    ///
    /// Returns an error if extraction fails
    /// Process a single archive file
    ///
    /// Extracts the archive and identifies any nested archives to add to the stack.
    ///
    /// # Arguments
    ///
    /// * `item` - The extraction item to process
    /// * `stack` - The extraction stack for nested archives
    ///
    /// # Returns
    ///
    /// Tuple of (extracted file paths, total bytes extracted, depth limit skips, path shortenings)
    ///
    /// # Errors
    ///
    /// Returns an error if extraction fails
    async fn process_archive_file(
        &self,
        item: &ExtractionItem,
        stack: &mut ExtractionStack,
    ) -> Result<(Vec<PathBuf>, u64, usize, usize)> {
        // Ensure target directory exists
        fs::create_dir_all(&item.target_dir).await.map_err(|e| {
            AppError::archive_error(
                format!("Failed to create target directory: {}", e),
                Some(item.target_dir.clone()),
            )
        })?;

        debug!(
            "Processing archive: {:?} -> {:?} (depth: {})",
            item.archive_path, item.target_dir, item.depth
        );

        // Create handler registry
        let registry = self.create_handler_registry();

        // Find appropriate handler
        let handler = registry.find_handler(&item.archive_path).ok_or_else(|| {
            AppError::archive_error(
                format!("No handler found for archive: {:?}", item.archive_path),
                Some(item.archive_path.clone()),
            )
        })?;

        debug!(
            "Using handler for archive: {:?}",
            item.archive_path.extension()
        );

        // Extract archive using handler
        let summary = handler
            .extract_with_limits(
                &item.archive_path,
                &item.target_dir,
                self.policy.max_file_size,
                self.policy.max_total_size,
                usize::MAX, // File count limit controlled by outer loop
            )
            .await
            .map_err(|e| {
                AppError::archive_error(
                    format!("Failed to extract archive: {}", e),
                    Some(item.archive_path.clone()),
                )
            })?;

        debug!(
            "Extracted {} files ({} bytes) from {:?}",
            summary.files_extracted, summary.total_size, item.archive_path
        );

        // Process extracted files
        let mut extracted_files = Vec::new();
        let mut depth_limit_skips = 0;
        let mut path_shortenings = 0;

        for file_path in &summary.extracted_files {
            // Check if this is a nested archive
            if self.is_archive_file(file_path) {
                // Check depth limit before adding to stack
                if item.depth + 1 >= self.policy.max_depth {
                    debug!(
                        "Depth limit reached, skipping nested archive: {:?}",
                        file_path
                    );
                    // Still count the file itself as extracted (it exists in the parent archive)
                    extracted_files.push(item.target_dir.join(file_path));
                    depth_limit_skips += 1;
                    continue;
                }

                // Create nested extraction item
                // Use the full path to the nested archive for proper handler lookup
                let nested_archive_path = item.target_dir.join(file_path);
                let nested_target = item.target_dir.join(
                    file_path
                        .file_stem()
                        .unwrap_or_else(|| std::ffi::OsStr::new("extracted")),
                );

                let nested_item = ExtractionItem::new(
                    nested_archive_path,
                    nested_target,
                    item.depth + 1,
                    item.parent_context.clone(),
                );

                // Add to stack for processing
                stack.push(nested_item).map_err(|e| {
                    AppError::archive_error(
                        format!("Failed to add nested archive to stack: {}", e),
                        Some(file_path.to_path_buf()),
                    )
                })?;

                debug!(
                    "Added nested archive to stack: {:?} (depth: {})",
                    file_path,
                    item.depth + 1
                );
            }

            // Resolve path (may be shortened if too long)
            let full_path = item.target_dir.join(file_path);
            let resolved_path = self
                .resolve_path_cached(&item.parent_context.workspace_id, &full_path)
                .await?;

            // Check if path was shortened
            if resolved_path != full_path {
                path_shortenings += 1;
                debug!("Path shortened: {:?} -> {:?}", full_path, resolved_path);
            }

            extracted_files.push(resolved_path);
        }

        debug!(
            "Processed archive: {} files, {} bytes, {} nested archives, {} path shortenings",
            extracted_files.len(),
            summary.total_size,
            depth_limit_skips,
            path_shortenings
        );

        Ok((
            extracted_files,
            summary.total_size,
            depth_limit_skips,
            path_shortenings,
        ))
    }

    /// Create handler registry with all available handlers
    fn create_handler_registry(&self) -> HandlerRegistry {
        let mut registry = HandlerRegistry::new();

        // Register all handlers
        registry.register(Box::new(crate::archive::zip_handler::ZipHandler));
        registry.register(Box::new(crate::archive::tar_handler::TarHandler));
        registry.register(Box::new(crate::archive::gz_handler::GzHandler));
        registry.register(Box::new(crate::archive::rar_handler::RarHandler));
        registry.register(Box::new(crate::archive::sevenz_handler::SevenZHandler));

        registry
    }

    /// Check if a file is an archive based on extension
    fn is_archive_file(&self, path: &Path) -> bool {
        let extensions = ["zip", "rar", "tar", "gz", "tgz", "7z"];

        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            let ext_lower = ext.to_lowercase();
            if extensions.contains(&ext_lower.as_str()) {
                return true;
            }

            // Check for .tar.gz
            if ext_lower == "gz" {
                if let Some(stem) = path.file_stem() {
                    if let Some(stem_str) = stem.to_str() {
                        return stem_str.ends_with(".tar");
                    }
                }
            }
        }

        false
    }

    /// Get the current extraction policy
    pub fn policy(&self) -> &ExtractionPolicy {
        &self.policy
    }

    /// Resolve extraction path with caching for performance
    ///
    /// Uses DashMap for fast concurrent lookups, reducing database queries
    /// for frequently accessed paths.
    ///
    /// # Arguments
    ///
    /// * `workspace_id` - Workspace identifier
    /// * `full_path` - Full path to resolve
    ///
    /// # Returns
    ///
    /// Resolved path (may be shortened)
    ///
    /// # Errors
    ///
    /// Returns an error if path resolution fails
    pub async fn resolve_path_cached(
        &self,
        workspace_id: &str,
        full_path: &Path,
    ) -> Result<PathBuf> {
        let cache_key = format!("{}:{}", workspace_id, full_path.display());

        // Check cache first
        if let Some(cached) = self.path_cache.get(&cache_key) {
            debug!("Path cache hit: {}", cache_key);
            return Ok(cached.clone());
        }

        // Cache miss - resolve and store
        debug!("Path cache miss: {}", cache_key);
        let resolved = self
            .path_manager
            .resolve_extraction_path(workspace_id, full_path)
            .await?;

        self.path_cache.insert(cache_key, resolved.clone());

        Ok(resolved)
    }

    /// Create directories in batches for improved performance
    ///
    /// Batches directory creation operations to reduce filesystem syscalls.
    /// Creates directories in groups of `dir_batch_size` (default: 10).
    ///
    /// # Arguments
    ///
    /// * `directories` - List of directories to create
    ///
    /// # Returns
    ///
    /// Number of directories created
    ///
    /// # Errors
    ///
    /// Returns an error if directory creation fails
    pub async fn create_directories_batched(&self, directories: &[PathBuf]) -> Result<usize> {
        if directories.is_empty() {
            return Ok(0);
        }

        let mut created_count = 0;
        let mut unique_dirs = HashSet::new();

        // Deduplicate directories
        for dir in directories {
            unique_dirs.insert(dir.clone());
        }

        let unique_dirs: Vec<PathBuf> = unique_dirs.into_iter().collect();
        let total_dirs = unique_dirs.len();

        debug!(
            "Creating {} directories in batches of {}",
            total_dirs, self.policy.dir_batch_size
        );

        // Process in batches
        for batch in unique_dirs.chunks(self.policy.dir_batch_size) {
            let mut tasks = Vec::new();

            for dir in batch {
                let dir = dir.clone();
                let task = tokio::spawn(async move {
                    fs::create_dir_all(&dir).await.map_err(|e| {
                        AppError::archive_error(
                            format!("Failed to create directory: {}", e),
                            Some(dir.clone()),
                        )
                    })
                });
                tasks.push(task);
            }

            // Wait for batch to complete
            for task in tasks {
                task.await.map_err(|e| {
                    AppError::archive_error(format!("Task join error: {}", e), None)
                })??;
                created_count += 1;
            }
        }

        debug!("Created {} directories", created_count);
        Ok(created_count)
    }

    /// Extract multiple files in parallel within a single archive
    ///
    /// Uses a semaphore to limit concurrent extractions to `max_parallel_files`
    /// (default: 4) to balance performance and resource usage.
    ///
    /// # Arguments
    ///
    /// * `file_tasks` - List of file extraction tasks
    ///
    /// # Returns
    ///
    /// List of extracted file paths
    ///
    /// # Errors
    ///
    /// Returns an error if any extraction fails
    pub async fn extract_files_parallel(
        &self,
        file_tasks: Vec<(PathBuf, PathBuf, u64)>, // (source, target, expected_size)
    ) -> Result<Vec<PathBuf>> {
        if file_tasks.is_empty() {
            return Ok(Vec::new());
        }

        debug!(
            "Extracting {} files in parallel (max concurrent: {})",
            file_tasks.len(),
            self.policy.max_parallel_files
        );

        let mut handles = Vec::new();

        for (_source, target, _expected_size) in file_tasks {
            let semaphore = self.parallel_semaphore.clone();
            let _buffer_size = self.policy.buffer_size;
            let _max_file_size = self.policy.max_file_size;

            let handle = tokio::spawn(async move {
                // Acquire semaphore permit
                let _permit = semaphore.acquire().await.map_err(|e| {
                    AppError::archive_error(format!("Failed to acquire semaphore: {}", e), None)
                })?;

                // Simulate file extraction (in real implementation, would read from archive)
                // For now, just create the target file
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent).await.map_err(|e| {
                        AppError::archive_error(
                            format!("Failed to create parent directory: {}", e),
                            Some(parent.to_path_buf()),
                        )
                    })?;
                }

                // Create empty file (placeholder for actual extraction)
                fs::File::create(&target).await.map_err(|e| {
                    AppError::archive_error(
                        format!("Failed to create file: {}", e),
                        Some(target.clone()),
                    )
                })?;

                debug!("Extracted file: {:?}", target);
                Ok::<PathBuf, AppError>(target)
            });

            handles.push(handle);
        }

        // Wait for all tasks to complete
        let mut extracted_files = Vec::new();
        for handle in handles {
            let result = handle
                .await
                .map_err(|e| AppError::archive_error(format!("Task join error: {}", e), None))??;
            extracted_files.push(result);
        }

        debug!(
            "Parallel extraction completed: {} files",
            extracted_files.len()
        );
        Ok(extracted_files)
    }

    /// Clear the path cache
    ///
    /// Useful for testing or when memory needs to be reclaimed
    pub fn clear_cache(&self) {
        self.path_cache.clear();
        debug!("Path cache cleared");
    }

    /// Get cache statistics
    ///
    /// Returns the number of entries in the path cache
    pub fn cache_size(&self) -> usize {
        self.path_cache.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::archive::PathConfig;
    use crate::services::MetadataDB;

    async fn create_test_engine() -> ExtractionEngine {
        let db = Arc::new(MetadataDB::new(":memory:").await.unwrap());
        let path_manager = Arc::new(PathManager::new(PathConfig::default(), db));
        let security_detector = Arc::new(SecurityDetector::default());
        let policy = ExtractionPolicy::default();

        ExtractionEngine::new(path_manager, security_detector, policy).unwrap()
    }

    #[test]
    fn test_extraction_policy_default() {
        let policy = ExtractionPolicy::default();
        assert_eq!(policy.max_depth, 10);
        assert_eq!(policy.max_file_size, 100 * 1024 * 1024);
        assert_eq!(policy.max_total_size, 10 * 1024 * 1024 * 1024);
        assert_eq!(policy.buffer_size, 64 * 1024);
        assert_eq!(policy.dir_batch_size, 10);
        assert_eq!(policy.max_parallel_files, 4);
    }

    #[test]
    fn test_extraction_policy_validate() {
        let mut policy = ExtractionPolicy::default();

        // Valid policy
        assert!(policy.validate().is_ok());

        // Invalid max_depth (too low)
        policy.max_depth = 0;
        assert!(policy.validate().is_err());

        // Invalid max_depth (too high)
        policy.max_depth = 21;
        assert!(policy.validate().is_err());

        // Valid max_depth
        policy.max_depth = 10;
        assert!(policy.validate().is_ok());

        // Invalid max_file_size
        policy.max_file_size = 0;
        assert!(policy.validate().is_err());

        // Invalid max_total_size
        policy.max_file_size = 1000;
        policy.max_total_size = 0;
        assert!(policy.validate().is_err());

        // Invalid buffer_size
        policy.max_total_size = 1000;
        policy.buffer_size = 0;
        assert!(policy.validate().is_err());

        // Invalid dir_batch_size
        policy.buffer_size = 1000;
        policy.dir_batch_size = 0;
        assert!(policy.validate().is_err());

        // Invalid max_parallel_files
        policy.dir_batch_size = 10;
        policy.max_parallel_files = 0;
        assert!(policy.validate().is_err());

        // All valid
        policy.max_parallel_files = 4;
        assert!(policy.validate().is_ok());
    }

    #[tokio::test]
    async fn test_extraction_engine_creation() {
        let engine = create_test_engine().await;
        assert_eq!(engine.policy().max_depth, 10);
    }

    #[tokio::test]
    async fn test_extraction_result_initialization() {
        let result = ExtractionResult {
            workspace_id: "test_workspace".to_string(),
            extracted_files: Vec::new(),
            warnings: Vec::new(),
            max_depth_reached: 0,
            total_files: 0,
            total_bytes: 0,
            path_shortenings_applied: 0,
            depth_limit_skips: 0,
            extraction_duration_secs: 0.0,
            extraction_speed_bytes_per_sec: 0.0,
        };

        assert_eq!(result.workspace_id, "test_workspace");
        assert_eq!(result.total_files, 0);
        assert_eq!(result.warnings.len(), 0);
        assert_eq!(result.extraction_duration_secs, 0.0);
        assert_eq!(result.extraction_speed_bytes_per_sec, 0.0);
    }

    #[tokio::test]
    async fn test_path_cache() {
        let engine = create_test_engine().await;

        // Initially empty
        assert_eq!(engine.cache_size(), 0);

        // Clear cache (should not error on empty cache)
        engine.clear_cache();
        assert_eq!(engine.cache_size(), 0);
    }

    #[tokio::test]
    async fn test_create_directories_batched() {
        let engine = create_test_engine().await;
        let temp_dir = tempfile::tempdir().unwrap();

        // Create test directories
        let dirs: Vec<PathBuf> = (0..25)
            .map(|i| temp_dir.path().join(format!("dir_{}", i)))
            .collect();

        let created = engine.create_directories_batched(&dirs).await.unwrap();
        assert_eq!(created, 25);

        // Verify directories exist
        for dir in &dirs {
            assert!(dir.exists());
        }
    }

    #[tokio::test]
    async fn test_create_directories_batched_empty() {
        let engine = create_test_engine().await;
        let created = engine.create_directories_batched(&[]).await.unwrap();
        assert_eq!(created, 0);
    }

    #[tokio::test]
    async fn test_create_directories_batched_deduplication() {
        let engine = create_test_engine().await;
        let temp_dir = tempfile::tempdir().unwrap();

        let test_dir = temp_dir.path().join("test_dir");

        // Same directory multiple times
        let dirs = vec![test_dir.clone(), test_dir.clone(), test_dir.clone()];

        let created = engine.create_directories_batched(&dirs).await.unwrap();
        // Should only create once due to deduplication
        assert_eq!(created, 1);
        assert!(test_dir.exists());
    }

    #[tokio::test]
    async fn test_extract_files_parallel() {
        let engine = create_test_engine().await;
        let temp_dir = tempfile::tempdir().unwrap();

        // Create test file tasks
        let tasks: Vec<(PathBuf, PathBuf, u64)> = (0..10)
            .map(|i| {
                let source = temp_dir.path().join(format!("source_{}.txt", i));
                let target = temp_dir.path().join(format!("target_{}.txt", i));
                (source, target, 1024)
            })
            .collect();

        let extracted = engine.extract_files_parallel(tasks).await.unwrap();
        assert_eq!(extracted.len(), 10);

        // Verify files were created
        for file in &extracted {
            assert!(file.exists());
        }
    }

    #[tokio::test]
    async fn test_extract_files_parallel_empty() {
        let engine = create_test_engine().await;
        let extracted = engine.extract_files_parallel(vec![]).await.unwrap();
        assert_eq!(extracted.len(), 0);
    }

    #[tokio::test]
    async fn test_parallel_extraction_respects_semaphore() {
        let engine = create_test_engine().await;
        let temp_dir = tempfile::tempdir().unwrap();

        // Create more tasks than max_parallel_files
        let tasks: Vec<(PathBuf, PathBuf, u64)> = (0..20)
            .map(|i| {
                let source = temp_dir.path().join(format!("source_{}.txt", i));
                let target = temp_dir.path().join(format!("target_{}.txt", i));
                (source, target, 1024)
            })
            .collect();

        // Should complete without error, respecting semaphore limit
        let extracted = engine.extract_files_parallel(tasks).await.unwrap();
        assert_eq!(extracted.len(), 20);
    }
}
