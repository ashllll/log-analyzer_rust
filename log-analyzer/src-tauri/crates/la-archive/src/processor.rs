//! 递归路径处理器
//!
//! 核心递归逻辑，负责：
//! - 识别文件类型（压缩文件 vs 普通文件）
//! - 调用统一的 ArchiveManager 处理器或增强提取系统
//! - 支持异步递归解压嵌套压缩包
//! - 收集元数据（增量索引）
//! - 错误处理和进度报告

#[cfg(feature = "enhanced-extraction")]
use crate::checkpoint_manager::{Checkpoint, CheckpointManager};
#[cfg(feature = "enhanced-extraction")]
use crate::extraction_engine::ExtractionPolicy;
use crate::internal::file_type_filter::FileTypeFilter;
#[cfg(feature = "enhanced-extraction")]
use crate::public_api::extract_archive_async_with_security_policy;
#[cfg(feature = "enhanced-extraction")]
use crate::security_detector::SecurityPolicy;
use crate::ArchiveManager;
use la_core::error::{AppError, ErrorCategory, Result};
use la_core::models::config::{ArchiveConfig, FileFilterConfig};
use la_core::storage_types::{ArchiveMetadata, FileMetadata};
use la_core::traits::AppConfigProvider;
use la_core::utils::path::normalize_path_separator;
use la_core::utils::path_security::has_parent_directory_component;
use la_storage::{ContentAddressableStorage, MetadataStore};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};
use walkdir::WalkDir;

const DIRECTORY_METADATA_BATCH_SIZE: usize = 500;
const SMALL_FILE_INLINE_CAS_THRESHOLD_BYTES: u64 = 1024 * 1024;
type FileStats = (Option<i64>, Option<i64>, Option<u8>);

fn parse_boolean_override(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn boolean_env_override(name: &str, default: bool) -> bool {
    match std::env::var(name) {
        Ok(value) => match parse_boolean_override(&value) {
            Some(value) => value,
            None => {
                warn!(
                    variable = name,
                    value, default, "Ignoring invalid boolean environment override"
                );
                default
            }
        },
        Err(std::env::VarError::NotPresent) => default,
        Err(std::env::VarError::NotUnicode(_)) => {
            warn!(
                variable = name,
                default, "Ignoring non-Unicode boolean environment override"
            );
            default
        }
    }
}

fn ensure_not_cancelled(context: &CasProcessingContext, path: &Path) -> Result<()> {
    if context.cancellation_token.is_cancelled() {
        return Err(AppError::Archive {
            category: ErrorCategory::CancellationRequested,
            _message: "Import cancelled".to_string(),
            _path: Some(path.to_path_buf()),
        });
    }
    Ok(())
}

async fn ensure_archive_not_cancelled(
    context: &CasProcessingContext,
    archive_id: i64,
    archive_path: &Path,
    extract_dir: &Path,
) -> Result<()> {
    if !context.cancellation_token.is_cancelled() {
        return Ok(());
    }

    context
        .metadata_store
        .update_archive_status(archive_id, "cancelled")
        .await?;
    if let Err(error) = fs::remove_dir_all(extract_dir).await {
        tracing::warn!(
            path = %extract_dir.display(),
            error = %error,
            "Failed to remove cancelled extraction directory"
        );
    }
    ensure_not_cancelled(context, archive_path)
}

/// 检查是否启用增强提取系统
///
/// 优先级：
/// 1. 环境变量 USE_ENHANCED_EXTRACTION（用于测试和临时覆盖）
/// 2. 编译 enhanced-extraction 功能时默认启用
#[cfg(feature = "enhanced-extraction")]
fn is_enhanced_extraction_enabled() -> bool {
    boolean_env_override("USE_ENHANCED_EXTRACTION", true)
}

#[cfg(feature = "enhanced-extraction")]
fn extraction_policy_from_archive_config(config: &ArchiveConfig) -> ExtractionPolicy {
    ExtractionPolicy {
        max_depth: config.max_extraction_depth,
        max_file_size: config.max_file_size,
        max_total_size: config.max_total_size,
        buffer_size: config.streaming_buffer_size as usize,
        dir_batch_size: config.directory_batch_size,
        max_parallel_files: config.max_parallel_files,
        max_file_count: config.max_file_count,
    }
}

#[cfg(feature = "enhanced-extraction")]
fn security_policy_from_archive_config(
    config: &ArchiveConfig,
    existing_workspace_bytes: u64,
) -> SecurityPolicy {
    SecurityPolicy {
        max_compression_ratio: config.max_compression_ratio,
        max_cumulative_size: config.max_archive_total_size,
        max_workspace_size: config
            .max_workspace_size
            .saturating_sub(existing_workspace_bytes),
        exponential_backoff_threshold: config.exponential_backoff_threshold,
    }
}

/// 检查是否跟随目录中的符号链接
///
/// 优先级：
/// 1. 环境变量 FOLLOW_SYMLINKS（用于测试和临时覆盖）
/// 2. 默认为 false（不跟随符号链接）
///
/// # 安全说明
///
/// 启用此选项可能存在安全风险，因为符号链接可能指向目录外的位置。
/// 路径遍历检查会捕获大多数此类情况，但仍需谨慎使用。
fn should_follow_symlinks() -> bool {
    boolean_env_override("FOLLOW_SYMLINKS", false)
}

/// 检查路径是否安全（防止路径遍历攻击）
///
/// # 参数
///
/// - `path` - 要检查的路径（可以是相对路径或绝对路径）
/// - `base_dir` - 基础目录，用于验证路径是否在允许范围内
///
/// # 返回
///
/// - `Ok(())` - 路径安全
/// - `Err(AppError)` - 路径不安全
fn validate_path_safety(path: &Path, base_dir: &Path) -> Result<()> {
    // 规范化基础目录
    let canonical_base = base_dir.canonicalize().map_err(|e| {
        AppError::validation_error(format!(
            "Failed to canonicalize base dir {}: {}",
            base_dir.display(),
            e
        ))
    })?;

    // 处理相对路径和绝对路径
    // 如果 path 是相对路径，将其与 base_dir 组合得到完整路径
    let full_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        // 相对路径：与 base_dir 组合
        canonical_base.join(path)
    };

    // 规范化完整路径
    let canonical_path = full_path.canonicalize().map_err(|e| {
        AppError::validation_error(format!(
            "Failed to canonicalize path {}: {}",
            full_path.display(),
            e
        ))
    })?;

    // 验证路径是否在基础目录内
    if !canonical_path.starts_with(&canonical_base) {
        return Err(AppError::validation_error(format!(
            "Path traversal detected: {} (resolved to {}) is outside of {}",
            path.display(),
            canonical_path.display(),
            base_dir.display()
        )));
    }

    // 只拒绝真正的父目录组件；普通文件名中的 ".."（如 release..log）是合法的。
    if path
        .components()
        .any(|component| component == Component::ParentDir)
    {
        return Err(AppError::validation_error(format!(
            "Parent directory component detected: {}",
            path.display()
        )));
    }

    Ok(())
}

/// Helper struct to track directory processing statistics
#[derive(Default)]
struct DirectoryProcessingStats {
    processed_count: usize,
    skipped_error_count: usize,
    skipped_symlink_count: usize,
}

impl DirectoryProcessingStats {
    fn log_summary(&self, path: &Path, context: &str) {
        debug!(
            path = %path.display(),
            processed = self.processed_count,
            skipped_symlinks = self.skipped_symlink_count,
            skipped_errors = self.skipped_error_count,
            "{} completed",
            context
        );
    }
}

struct PendingFileImport {
    metadata: FileMetadata,
    source_path: PathBuf,
}

/// Validate virtual path to prevent path traversal attacks
///
/// # Arguments
///
/// * `virtual_path` - The virtual path to validate
///
/// # Returns
///
/// * `Ok(())` - Path is safe
/// * `Err(AppError)` - Path contains suspicious components
///
/// # Requirements
///
/// Validates: Requirements 1.5, 7.2, 8.3
fn validate_virtual_path(virtual_path: &str) -> Result<()> {
    if virtual_path.is_empty() || virtual_path.trim().is_empty() {
        return Err(AppError::validation_error(
            "Virtual path cannot be empty".to_string(),
        ));
    }

    // Check for actual parent-directory components on both supported separators.
    if has_parent_directory_component(virtual_path) {
        return Err(AppError::validation_error(format!(
            "Path traversal detected in virtual path: {virtual_path}"
        )));
    }

    // Check for absolute paths (virtual paths should be relative)
    if virtual_path.starts_with('/') || virtual_path.starts_with('\\') {
        return Err(AppError::validation_error(format!(
            "Virtual path should be relative, not absolute: {virtual_path}"
        )));
    }

    // Check for Windows drive letters
    if virtual_path.len() >= 2 && virtual_path.chars().nth(1) == Some(':') {
        return Err(AppError::validation_error(format!(
            "Virtual path should not contain drive letters: {virtual_path}"
        )));
    }

    // Check path length (though CAS removes most limits, we still want reasonable paths)
    if virtual_path.len() > 4096 {
        warn!(
            path = %virtual_path,
            length = virtual_path.len(),
            "Virtual path is very long"
        );
    }

    Ok(())
}

// ========== NEW CAS-BASED PROCESSING FUNCTIONS ==========

/// Context for CAS processing with checkpoint support
pub struct CasProcessingContext {
    pub workspace_dir: PathBuf,
    pub cas: Arc<ContentAddressableStorage>,
    pub metadata_store: Arc<MetadataStore>,
    #[cfg(feature = "enhanced-extraction")]
    pub checkpoint_manager: Option<Arc<Mutex<CheckpointManager>>>,
    #[cfg(feature = "enhanced-extraction")]
    pub checkpoint: Option<Arc<Mutex<Checkpoint>>>,
    #[cfg(feature = "enhanced-extraction")]
    pub files_since_checkpoint: Arc<Mutex<usize>>,
    #[cfg(feature = "enhanced-extraction")]
    pub bytes_since_checkpoint: Arc<Mutex<u64>>,
    pub cancellation_token: CancellationToken,
    file_stats_updates: Arc<Mutex<Vec<tokio::task::JoinHandle<Result<()>>>>>,
}

impl CasProcessingContext {
    /// Create a new processing context
    pub fn new(
        workspace_dir: PathBuf,
        cas: Arc<ContentAddressableStorage>,
        metadata_store: Arc<MetadataStore>,
    ) -> Self {
        Self {
            workspace_dir,
            cas,
            metadata_store,
            #[cfg(feature = "enhanced-extraction")]
            checkpoint_manager: None,
            #[cfg(feature = "enhanced-extraction")]
            checkpoint: None,
            #[cfg(feature = "enhanced-extraction")]
            files_since_checkpoint: Arc::new(Mutex::new(0)),
            #[cfg(feature = "enhanced-extraction")]
            bytes_since_checkpoint: Arc::new(Mutex::new(0)),
            cancellation_token: CancellationToken::new(),
            file_stats_updates: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Attach the cancellation token owned by the importing task.
    pub fn with_cancellation_token(mut self, cancellation_token: CancellationToken) -> Self {
        self.cancellation_token = cancellation_token;
        self
    }

    async fn queue_file_stats_update(&self, hash: String, virtual_path: String) {
        let cas = Arc::clone(&self.cas);
        let metadata_store = Arc::clone(&self.metadata_store);
        let task = tokio::task::spawn(async move {
            let content = cas.read_content(&hash).await.map_err(|error| {
                AppError::io_error(
                    format!("Failed to read CAS content for file statistics: {error}"),
                    None,
                )
            })?;
            let (min_ts, max_ts, level_mask) =
                tokio::task::spawn_blocking(move || la_core::utils::compute_file_stats(&content))
                    .await
                    .map_err(|error| {
                        AppError::internal_error(format!("File statistics worker failed: {error}"))
                    })?;

            metadata_store
                .update_file_ready(&virtual_path, min_ts, max_ts, level_mask)
                .await?;
            tracing::debug!(
                virtual_path = %virtual_path,
                "File marked as READY for incremental search"
            );
            Ok(())
        });

        self.file_stats_updates.lock().await.push(task);
    }

    async fn finish_file_stats_updates(&self) -> Result<()> {
        let tasks = std::mem::take(&mut *self.file_stats_updates.lock().await);
        let mut first_error = None;

        for task in tasks {
            let error = match task.await {
                Ok(Ok(())) => None,
                Ok(Err(error)) => Some(error),
                Err(error) => Some(AppError::internal_error(format!(
                    "File statistics task failed: {error}"
                ))),
            };
            if first_error.is_none() {
                first_error = error;
            }
        }

        match first_error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    /// Enable checkpoint support
    #[cfg(feature = "enhanced-extraction")]
    pub fn with_checkpoints(
        mut self,
        checkpoint_manager: Arc<Mutex<CheckpointManager>>,
        checkpoint: Arc<Mutex<Checkpoint>>,
    ) -> Self {
        self.checkpoint_manager = Some(checkpoint_manager);
        self.checkpoint = Some(checkpoint);
        self
    }

    /// Update checkpoint if needed
    async fn maybe_save_checkpoint(&self) -> Result<()> {
        #[cfg(feature = "enhanced-extraction")]
        {
            if let (Some(manager), Some(checkpoint)) = (&self.checkpoint_manager, &self.checkpoint)
            {
                let files = *self.files_since_checkpoint.lock().await;
                let bytes = *self.bytes_since_checkpoint.lock().await;

                let manager_guard = manager.lock().await;
                if manager_guard.should_write_checkpoint(files, bytes) {
                    let checkpoint_guard = checkpoint.lock().await;
                    manager_guard.save_checkpoint(&checkpoint_guard).await?;

                    // Reset counters
                    *self.files_since_checkpoint.lock().await = 0;
                    *self.bytes_since_checkpoint.lock().await = 0;

                    info!(
                        files = checkpoint_guard.metrics.files_extracted,
                        bytes = checkpoint_guard.metrics.bytes_extracted,
                        "Checkpoint saved"
                    );
                }
            }
        }
        Ok(())
    }

    /// Update checkpoint with new file
    async fn update_checkpoint(&self, file_path: PathBuf, file_size: u64) -> Result<()> {
        #[cfg(feature = "enhanced-extraction")]
        {
            if let Some(checkpoint) = &self.checkpoint {
                let mut checkpoint_guard = checkpoint.lock().await;
                checkpoint_guard.update_file(file_path, file_size);

                // Update counters
                *self.files_since_checkpoint.lock().await += 1;
                *self.bytes_since_checkpoint.lock().await += file_size;
            }
        }
        #[cfg(not(feature = "enhanced-extraction"))]
        let _ = (file_path, file_size);
        Ok(())
    }

    /// Check if file was already extracted
    async fn is_file_extracted(&self, file_path: &Path) -> bool {
        #[cfg(feature = "enhanced-extraction")]
        {
            if let Some(checkpoint) = &self.checkpoint {
                let checkpoint_guard = checkpoint.lock().await;
                return checkpoint_guard.is_file_extracted(file_path);
            }
        }
        #[cfg(not(feature = "enhanced-extraction"))]
        let _ = file_path;
        false
    }
}

/// Process path recursively using CAS and MetadataStore with checkpoint support
///
/// This is the new CAS-based implementation that replaces the old HashMap-based approach.
/// It includes checkpoint support for resumable processing.
///
/// # Arguments
///
/// * `path` - Path to process (file or directory)
/// * `virtual_path` - Virtual path for indexing
/// * `context` - Processing context with CAS, metadata store, and optional checkpoints
/// * `provider` - 应用配置提供者（解耦 Tauri 依赖）
/// * `task_id` - Task ID for progress reporting
/// * `workspace_id` - Workspace ID
/// * `parent_archive_id` - Parent archive ID (None for root level)
/// * `depth_level` - Current nesting depth
///
/// # Requirements
///
/// Validates: Requirements 1.1, 1.2, 1.3, 4.1, 4.2, 4.3, 8.4
#[allow(clippy::too_many_arguments)]
pub async fn process_path_with_cas_and_checkpoints(
    path: &Path,
    virtual_path: &str,
    context: &CasProcessingContext,
    provider: &dyn AppConfigProvider,
    task_id: &str,
    workspace_id: &str,
    parent_archive_id: Option<i64>,
    depth_level: i32,
) -> Result<()> {
    let processing_result = process_path_with_cas_and_checkpoints_inner(
        path,
        virtual_path,
        context,
        provider,
        task_id,
        workspace_id,
        parent_archive_id,
        depth_level,
    )
    .await;
    let stats_result = context.finish_file_stats_updates().await;

    match (processing_result, stats_result) {
        (Ok(()), stats_result) => stats_result,
        (Err(processing_error), Ok(())) => Err(processing_error),
        (Err(processing_error), Err(stats_error)) => {
            warn!(
                error = %stats_error,
                "File statistics also failed while the import was already failing"
            );
            Err(processing_error)
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn process_path_with_cas_and_checkpoints_inner(
    path: &Path,
    virtual_path: &str,
    context: &CasProcessingContext,
    provider: &dyn AppConfigProvider,
    task_id: &str,
    workspace_id: &str,
    parent_archive_id: Option<i64>,
    depth_level: i32,
) -> Result<()> {
    ensure_not_cancelled(context, path)?;

    // Check if file was already extracted (checkpoint resume)
    if context.is_file_extracted(path).await {
        debug!(
            path = %path.display(),
            "Skipping already extracted file (checkpoint resume)"
        );
        return Ok(());
    }

    // Validate virtual path
    validate_virtual_path(virtual_path)?;

    // Validate file exists before processing
    if !path.exists() {
        warn!(
            path = %path.display(),
            "File does not exist, skipping"
        );
        return Ok(());
    }

    // Handle directories
    if path.is_dir() {
        // Track processing statistics for debugging
        let mut stats = DirectoryProcessingStats::default();
        let follow_symlinks = should_follow_symlinks();

        // 使用可配置的最大遍历深度，默认无限制（遍历所有子目录）
        // 通过环境变量 PROCESSOR_MAX_DEPTH 可设置深度限制
        const DEFAULT_MAX_DEPTH: usize = usize::MAX;
        let max_depth = std::env::var("PROCESSOR_MAX_DEPTH")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_MAX_DEPTH);

        let walkdir_iter = WalkDir::new(path)
            .min_depth(1)
            .max_depth(max_depth)
            .follow_links(follow_symlinks)
            .into_iter();
        let mut pending_files = Vec::with_capacity(DIRECTORY_METADATA_BATCH_SIZE);

        for entry_result in walkdir_iter {
            ensure_not_cancelled(context, path)?;

            match entry_result {
                Ok(entry) => {
                    // 处理符号链接
                    let target_path: Option<PathBuf> = if entry.path_is_symlink() {
                        if follow_symlinks {
                            // 尝试解析符号链接目标
                            match entry.path().read_link() {
                                Ok(target) => {
                                    let resolved = if target.is_absolute() {
                                        target
                                    } else {
                                        entry
                                            .path()
                                            .parent()
                                            .unwrap_or(std::path::Path::new(""))
                                            .join(&target)
                                    };
                                    // 验证符号链接目标不逃逸基础目录（B-H1 修复）
                                    let canonical_base =
                                        path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
                                    let check_path = resolved
                                        .canonicalize()
                                        .unwrap_or_else(|_| resolved.clone());
                                    if !check_path.starts_with(&canonical_base) {
                                        warn!(
                                            symlink = %entry.path().display(),
                                            target = %resolved.display(),
                                            "符号链接目标逃逸提取目录，已忽略"
                                        );
                                        stats.skipped_symlink_count += 1;
                                        continue;
                                    }
                                    info!(
                                        symlink = %entry.path().display(),
                                        target = %resolved.display(),
                                        "Following symlink to target during CAS processing"
                                    );
                                    Some(resolved)
                                }
                                Err(e) => {
                                    warn!(
                                        path = %entry.path().display(),
                                        error = %e,
                                        "Failed to read symlink target, skipping"
                                    );
                                    stats.skipped_symlink_count += 1;
                                    continue;
                                }
                            }
                        } else {
                            warn!(
                                path = %entry.path().display(),
                                "Skipping symlink during CAS directory processing (set FOLLOW_SYMLINKS=true to follow)"
                            );
                            stats.skipped_symlink_count += 1;
                            continue;
                        }
                    } else {
                        None
                    };

                    // 确定实际要处理的路径。WalkDir 已经递归遍历子目录；
                    // 如果这里再递归目录，会让嵌套文件被重复处理。
                    let path_to_process = target_path.as_deref().unwrap_or_else(|| entry.path());
                    if path_to_process.is_dir() {
                        continue;
                    }

                    let relative_path = entry.path().strip_prefix(path).unwrap_or(entry.path());
                    let new_virtual = build_child_virtual_path(virtual_path, relative_path);

                    if is_archive_file(path_to_process) {
                        flush_pending_directory_files(context, &mut pending_files).await?;
                        if let Err(e) = Box::pin(process_path_with_cas_and_checkpoints_inner(
                            path_to_process,
                            &new_virtual,
                            context,
                            provider,
                            task_id,
                            workspace_id,
                            parent_archive_id,
                            depth_level,
                        ))
                        .await
                        {
                            if e.category() == ErrorCategory::CancellationRequested {
                                return Err(e);
                            }
                            warn!(
                                file = %path_to_process.display(),
                                error = %e,
                                "Failed to process directory archive entry, continuing with others"
                            );
                        }
                    } else {
                        let file_name = path_to_process
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();
                        let import_allowed =
                            should_import_file_defensive(path_to_process, provider).await;

                        if !import_allowed {
                            tracing::info!(
                                file = %file_name,
                                path = %path_to_process.display(),
                                "File skipped by filter configuration (user-configured)"
                            );
                            continue;
                        }

                        match prepare_regular_file_import(
                            path_to_process,
                            &new_virtual,
                            context,
                            parent_archive_id,
                            depth_level,
                        )
                        .await
                        {
                            Ok(pending) => pending_files.push(pending),
                            Err(e) => {
                                warn!(
                                    file = %path_to_process.display(),
                                    error = %e,
                                    "Failed to stage directory file import, continuing with others"
                                );
                            }
                        }

                        if pending_files.len() >= DIRECTORY_METADATA_BATCH_SIZE {
                            flush_pending_directory_files(context, &mut pending_files).await?;
                        }
                    }
                    stats.processed_count += 1;
                }
                Err(e) => {
                    // Log the error instead of silently skipping
                    warn!(
                        path = %path.display(),
                        error = %e,
                        "Failed to read directory entry during CAS processing, skipping"
                    );
                    stats.skipped_error_count += 1;
                    // Continue processing other entries instead of failing entirely
                }
            }
        }

        flush_pending_directory_files(context, &mut pending_files).await?;
        stats.log_summary(path, "CAS directory processing");
        return Ok(());
    }

    let file_name = path
        .file_name()
        .ok_or_else(|| AppError::validation_error("Invalid filename"))?
        .to_string_lossy()
        .to_string();

    // ========== 防御性集成：文件类型过滤检查 ==========
    // 设计要点：
    // 1. 不修改现有逻辑
    // 2. 过滤失败时自动降级到旧行为（允许文件通过）
    // 3. 详细日志记录决策过程
    if !path.is_dir() && !is_archive_file(path) {
        // 可选的文件过滤检查（防御性：失败时允许文件通过）
        let import_allowed = should_import_file_defensive(path, provider).await;

        if !import_allowed {
            tracing::info!(
                file = %file_name,
                path = %path.display(),
                "File skipped by filter configuration (user-configured)"
            );
            return Ok(()); // 跳过此文件，但继续处理其他文件
        }

        tracing::debug!(
            file = %file_name,
            "File passed filter check, proceeding with import"
        );
    }
    // ========== 集成结束 ==========

    // Report progress
    tracing::debug!(
        task_id = %task_id,
        file = %file_name,
        "Processing file with CAS (event sent via TaskManager)"
    );

    // Check if this is an archive file
    if is_archive_file(path) {
        // Process as archive
        return Box::pin(extract_and_process_archive_with_cas_and_checkpoints(
            path,
            virtual_path,
            context,
            provider,
            task_id,
            workspace_id,
            parent_archive_id,
            depth_level,
        ))
        .await;
    }

    let pending =
        prepare_regular_file_import(path, virtual_path, context, parent_archive_id, depth_level)
            .await?;
    ensure_not_cancelled(context, path)?;
    let file_id = context
        .metadata_store
        .insert_file(&pending.metadata)
        .await?;
    finalize_regular_file_import(context, pending, Some(file_id)).await?;

    Ok(())
}

async fn prepare_regular_file_import(
    path: &Path,
    virtual_path: &str,
    context: &CasProcessingContext,
    parent_archive_id: Option<i64>,
    depth_level: i32,
) -> Result<PendingFileImport> {
    let file_name = path
        .file_name()
        .ok_or_else(|| AppError::validation_error("Invalid filename"))?
        .to_string_lossy()
        .to_string();
    let metadata = fs::metadata(path).await.ok();
    let file_size_bytes = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
    let (hash, inline_stats) = store_regular_file_content(context, path, file_size_bytes).await?;
    let file_size = file_size_bytes as i64;
    let modified_time = metadata
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let mime_type = detect_mime_type(path);
    let (min_timestamp, max_timestamp, level_mask, analysis_status) = match inline_stats {
        Some((min_timestamp, max_timestamp, level_mask)) => (
            min_timestamp,
            max_timestamp,
            level_mask,
            la_core::storage_types::AnalysisStatus::Ready,
        ),
        None => (
            None,
            None,
            None,
            la_core::storage_types::AnalysisStatus::Pending,
        ),
    };

    Ok(PendingFileImport {
        metadata: FileMetadata {
            id: 0,
            sha256_hash: hash,
            virtual_path: normalize_path_separator(virtual_path),
            original_name: file_name,
            size: file_size,
            modified_time,
            mime_type,
            parent_archive_id,
            depth_level,
            min_timestamp,
            max_timestamp,
            level_mask,
            analysis_status,
        },
        source_path: path.to_path_buf(),
    })
}

async fn flush_pending_directory_files(
    context: &CasProcessingContext,
    pending_files: &mut Vec<PendingFileImport>,
) -> Result<()> {
    if pending_files.is_empty() {
        return Ok(());
    }
    ensure_not_cancelled(context, &context.workspace_dir)?;

    let imports = std::mem::take(pending_files);
    let files: Vec<FileMetadata> = imports
        .iter()
        .map(|pending| pending.metadata.clone())
        .collect();

    match context.metadata_store.insert_files_batch_smart(files).await {
        Ok(_) => {
            for pending in imports {
                ensure_not_cancelled(context, &pending.source_path)?;
                finalize_regular_file_import(context, pending, None).await?;
            }
        }
        Err(batch_error) => {
            warn!(
                error = %batch_error,
                count = imports.len(),
                "Batch metadata insert failed, falling back to per-file inserts"
            );
            for pending in imports {
                match context.metadata_store.insert_file(&pending.metadata).await {
                    Ok(file_id) => {
                        finalize_regular_file_import(context, pending, Some(file_id)).await?;
                    }
                    Err(error) => {
                        warn!(
                            path = %pending.metadata.virtual_path,
                            error = %error,
                            "Failed to insert file metadata during fallback"
                        );
                    }
                }
            }
        }
    }

    Ok(())
}

async fn finalize_regular_file_import(
    context: &CasProcessingContext,
    pending: PendingFileImport,
    file_id: Option<i64>,
) -> Result<()> {
    context
        .update_checkpoint(pending.source_path, pending.metadata.size as u64)
        .await?;
    context.maybe_save_checkpoint().await?;

    debug!(
        file_id = ?file_id,
        hash = %pending.metadata.sha256_hash,
        virtual_path = %pending.metadata.virtual_path,
        size = pending.metadata.size,
        depth = pending.metadata.depth_level,
        "Stored file in CAS and metadata"
    );

    if pending.metadata.analysis_status == la_core::storage_types::AnalysisStatus::Pending {
        context
            .queue_file_stats_update(
                pending.metadata.sha256_hash.clone(),
                pending.metadata.virtual_path.clone(),
            )
            .await;
    }
    Ok(())
}

async fn store_regular_file_content(
    context: &CasProcessingContext,
    path: &Path,
    file_size: u64,
) -> Result<(String, Option<FileStats>)> {
    if file_size <= SMALL_FILE_INLINE_CAS_THRESHOLD_BYTES {
        let content = fs::read(path).await.map_err(|e| {
            AppError::io_error(
                format!("Failed to read small file for CAS storage: {e}"),
                Some(path.to_path_buf()),
            )
        })?;
        let stats = la_core::utils::compute_file_stats(&content);
        let hash = context.cas.store_content(&content).await?;
        return Ok((hash, Some(stats)));
    }

    Ok((context.cas.store_file_zero_copy(path).await?, None))
}

fn detect_mime_type(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .and_then(|ext| match ext.to_lowercase().as_str() {
            "log" | "txt" => Some("text/plain".to_string()),
            "json" => Some("application/json".to_string()),
            "xml" => Some("application/xml".to_string()),
            "gz" => Some("application/gzip".to_string()),
            "zip" => Some("application/zip".to_string()),
            _ => None,
        })
}

fn build_child_virtual_path(parent_virtual_path: &str, relative_path: &Path) -> String {
    format!(
        "{}/{}",
        parent_virtual_path,
        normalize_path_separator(&relative_path.to_string_lossy())
    )
}

/// Process path recursively using CAS and MetadataStore
///
/// This is the new CAS-based implementation that replaces the old HashMap-based approach.
///
/// # Arguments
///
/// * `path` - Path to process (file or directory)
/// * `virtual_path` - Virtual path for indexing
/// * `workspace_dir` - Workspace directory (contains CAS and metadata.db)
/// * `cas` - Content-Addressable Storage instance
/// * `metadata_store` - Metadata store instance (wrapped in Arc)
/// * `provider` - 应用配置提供者（解耦 Tauri 依赖）
/// * `task_id` - Task ID for progress reporting
/// * `workspace_id` - Workspace ID
/// * `parent_archive_id` - Parent archive ID (None for root level)
/// * `depth_level` - Current nesting depth
///
/// # Requirements
///
/// Validates: Requirements 1.1, 1.2, 1.3, 4.1, 4.2, 4.3
#[allow(clippy::too_many_arguments)]
pub async fn process_path_with_cas(
    path: &Path,
    virtual_path: &str,
    workspace_dir: &Path,
    cas: &ContentAddressableStorage,
    metadata_store: Arc<MetadataStore>,
    provider: &dyn AppConfigProvider,
    task_id: &str,
    workspace_id: &str,
    parent_archive_id: Option<i64>,
    depth_level: i32,
) -> Result<()> {
    process_path_with_cas_cancellable(
        path,
        virtual_path,
        workspace_dir,
        cas,
        metadata_store,
        provider,
        task_id,
        workspace_id,
        parent_archive_id,
        depth_level,
        CancellationToken::new(),
    )
    .await
}

/// Process a path while observing the cancellation token owned by the import task.
#[allow(clippy::too_many_arguments)]
pub async fn process_path_with_cas_cancellable(
    path: &Path,
    virtual_path: &str,
    workspace_dir: &Path,
    cas: &ContentAddressableStorage,
    metadata_store: Arc<MetadataStore>,
    provider: &dyn AppConfigProvider,
    task_id: &str,
    workspace_id: &str,
    parent_archive_id: Option<i64>,
    depth_level: i32,
    cancellation_token: CancellationToken,
) -> Result<()> {
    // Wrap CAS in Arc for the context
    let cas_arc = Arc::new(cas.clone());

    // Create context without checkpoints for backward compatibility
    let context = CasProcessingContext::new(workspace_dir.to_path_buf(), cas_arc, metadata_store)
        .with_cancellation_token(cancellation_token);

    process_path_with_cas_and_checkpoints(
        path,
        virtual_path,
        &context,
        provider,
        task_id,
        workspace_id,
        parent_archive_id,
        depth_level,
    )
    .await
}

async fn extract_with_legacy_manager(
    archive_manager: &ArchiveManager,
    archive_path: &Path,
    extract_dir: &Path,
    context: &CasProcessingContext,
    archive_id: i64,
) -> Result<Vec<PathBuf>> {
    match archive_manager
        .extract_archive(archive_path, extract_dir)
        .await
    {
        Ok(summary) => {
            info!(
                files = summary.files_extracted,
                bytes = summary.total_size,
                "Legacy extraction completed"
            );
            Ok(summary.extracted_files)
        }
        Err(error) => {
            context
                .metadata_store
                .update_archive_status(archive_id, "failed")
                .await?;
            let _ = fs::remove_dir_all(extract_dir).await;
            Err(AppError::archive_error(
                format!("Legacy extraction failed: {error}"),
                Some(archive_path.to_path_buf()),
            ))
        }
    }
}

/// Extract and process archive using CAS with checkpoint support
///
/// This function handles nested archive extraction with depth tracking and checkpoint support.
///
/// # Arguments
///
/// * `archive_path` - Path to the archive file
/// * `virtual_path` - Virtual path prefix
/// * `context` - Processing context with CAS, metadata store, and optional checkpoints
/// * `provider` - 应用配置提供者（解耦 Tauri 依赖）
/// * `task_id` - Task ID
/// * `workspace_id` - Workspace ID
/// * `parent_archive_id` - Parent archive ID (None for root)
/// * `depth_level` - Current nesting depth
///
/// # Requirements
///
/// Validates: Requirements 4.1, 4.2, 4.3, 8.4
#[allow(clippy::too_many_arguments)]
async fn extract_and_process_archive_with_cas_and_checkpoints(
    archive_path: &Path,
    virtual_path: &str,
    context: &CasProcessingContext,
    provider: &dyn AppConfigProvider,
    task_id: &str,
    workspace_id: &str,
    parent_archive_id: Option<i64>,
    depth_level: i32,
) -> Result<()> {
    let archive_config = load_archive_config_safe(provider).await;
    let max_extraction_depth = archive_config.max_extraction_depth as i32;

    // Check depth limit - if reached, still extract but don't recurse into nested archives
    let is_at_max_depth = depth_level >= max_extraction_depth;

    if is_at_max_depth {
        warn!(
            archive = %archive_path.display(),
            depth = depth_level,
            max_extraction_depth = max_extraction_depth,
            "Maximum nesting depth reached, will extract but not recurse into nested archives"
        );
    }

    let file_name = archive_path
        .file_name()
        .ok_or_else(|| AppError::validation_error("Invalid archive filename"))?
        .to_string_lossy()
        .to_string();

    info!(
        archive = %file_name,
        depth = depth_level,
        parent_id = ?parent_archive_id,
        "Processing archive with checkpoint support"
    );

    // Store the archive file itself in CAS
    let archive_hash = context.cas.store_file_zero_copy(archive_path).await?;

    // Detect archive type
    let archive_type = archive_path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase())
        .unwrap_or_else(|| "unknown".to_string());

    // Create archive metadata
    let archive_metadata = ArchiveMetadata {
        id: 0, // Will be auto-generated
        sha256_hash: archive_hash.clone(),
        virtual_path: normalize_path_separator(virtual_path),
        original_name: file_name.clone(),
        archive_type: archive_type.clone(),
        parent_archive_id,
        depth_level,
        extraction_status: "pending".to_string(),
    };

    // Insert archive metadata
    let archive_id = context
        .metadata_store
        .insert_archive(&archive_metadata)
        .await?;

    debug!(
        archive_id = archive_id,
        hash = %archive_hash,
        archive_type = %archive_type,
        "Inserted archive metadata"
    );

    // CR-15: async_zip streaming path removed; all ZIP processing now uses sync zip via spawn_blocking

    // Create extraction directory
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let extract_dir_name = format!("{}_{}", file_name.replace('.', "_"), timestamp);
    let extract_dir = context
        .workspace_dir
        .join("extracted")
        .join(&extract_dir_name);

    fs::create_dir_all(&extract_dir).await.map_err(|e| {
        AppError::archive_error(
            format!("Failed to create extraction directory: {e}"),
            Some(extract_dir.clone()),
        )
    })?;

    // Extract archive
    let archive_manager = ArchiveManager::with_config(archive_config.clone());
    #[cfg(feature = "enhanced-extraction")]
    let extracted_files = if is_enhanced_extraction_enabled() {
        let policy = extraction_policy_from_archive_config(&archive_config);
        let existing_workspace_bytes = context.metadata_store.sum_file_sizes().await?.max(0) as u64;
        let security_policy =
            security_policy_from_archive_config(&archive_config, existing_workspace_bytes);
        match extract_archive_async_with_security_policy(
            archive_path,
            &extract_dir,
            workspace_id,
            policy,
            security_policy,
        )
        .await
        {
            Ok(result) => {
                info!(
                    files = result.extracted_files.len(),
                    bytes = result.performance_metrics.bytes_extracted,
                    "Enhanced extraction completed"
                );
                result.extracted_files
            }
            Err(e) => {
                context
                    .metadata_store
                    .update_archive_status(archive_id, "failed")
                    .await?;
                // 清理失败解压的临时目录
                let _ = fs::remove_dir_all(&extract_dir).await;
                return Err(AppError::archive_error(
                    format!("Enhanced extraction failed: {e}"),
                    Some(archive_path.to_path_buf()),
                ));
            }
        }
    } else {
        extract_with_legacy_manager(
            &archive_manager,
            archive_path,
            &extract_dir,
            context,
            archive_id,
        )
        .await?
    };
    #[cfg(not(feature = "enhanced-extraction"))]
    let extracted_files = extract_with_legacy_manager(
        &archive_manager,
        archive_path,
        &extract_dir,
        context,
        archive_id,
    )
    .await?;

    ensure_archive_not_cancelled(context, archive_id, archive_path, &extract_dir).await?;

    // Update archive status to extracting
    context
        .metadata_store
        .update_archive_status(archive_id, "extracting")
        .await?;

    // Process extracted files recursively
    let total_files = extracted_files.len();
    info!(
        archive_id = archive_id,
        total_files = total_files,
        "Starting to process extracted files"
    );

    for (index, extracted_file) in extracted_files.iter().enumerate() {
        ensure_archive_not_cancelled(context, archive_id, archive_path, &extract_dir).await?;

        let file_name = extracted_file
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        debug!(
            archive_id = archive_id,
            current = index + 1,
            total = total_files,
            file = %file_name,
            path = %extracted_file.display(),
            "Processing extracted file"
        );

        // Validate path safety
        if let Err(e) = validate_path_safety(extracted_file, &extract_dir) {
            warn!(
                file = %extracted_file.display(),
                error = %e,
                "Skipping unsafe file"
            );
            continue;
        }

        // Calculate relative path
        // extracted_file may be relative or absolute, handle both cases
        let relative_path = if extracted_file.is_absolute() {
            // Absolute path: strip the extract_dir prefix
            extracted_file.strip_prefix(&extract_dir).map_err(|_| {
                AppError::validation_error(format!(
                    "Failed to compute relative path for {}",
                    extracted_file.display()
                ))
            })?
        } else {
            // Relative path: use as-is (already relative to extract_dir)
            extracted_file
        };

        // Calculate full path for file operations
        // extracted_file may be relative or absolute
        let full_path = if extracted_file.is_absolute() {
            extracted_file.to_path_buf()
        } else {
            extract_dir.join(extracted_file)
        };

        // 修复虚拟路径重复：virtual_path 已包含压缩包名，只需拼接 relative_path
        let new_virtual = format!("{}/{}", virtual_path, relative_path.to_string_lossy());

        // 虚拟路径长度守卫：防止嵌套压缩包路径无限膨胀（DoS）
        const MAX_VIRTUAL_PATH_LENGTH: usize = 1024;
        if new_virtual.len() > MAX_VIRTUAL_PATH_LENGTH {
            warn!(
                path = %new_virtual,
                length = new_virtual.len(),
                max_allowed = MAX_VIRTUAL_PATH_LENGTH,
                "虚拟路径超出最大长度限制，跳过该条目"
            );
            continue;
        }

        // Recursively process (supports nested archives)
        // Skip recursion if at max depth to prevent infinite loops
        if is_at_max_depth && is_archive_file(&full_path) {
            warn!(
                file = %file_name,
                depth = depth_level,
                "Reached max depth: storing nested archive as regular file without extraction"
            );
            // 将深层嵌套压缩包作为普通文件存入 CAS，使其可被搜索，但不再递归解压
            let hash = context.cas.store_file_zero_copy(&full_path).await?;
            let file_size = fs::metadata(&full_path)
                .await
                .map(|m| m.len() as i64)
                .unwrap_or(0);
            let nested_metadata = FileMetadata {
                id: 0,
                sha256_hash: hash.clone(),
                virtual_path: normalize_path_separator(&new_virtual),
                original_name: file_name.clone(),
                size: file_size,
                modified_time: 0,
                mime_type: Some("application/octet-stream".to_string()),
                parent_archive_id: Some(archive_id),
                depth_level,
                min_timestamp: None,
                max_timestamp: None,
                level_mask: None,
                analysis_status: la_core::storage_types::AnalysisStatus::Pending,
            };
            context.metadata_store.insert_file(&nested_metadata).await?;

            // 增量分析：深层嵌套压缩包也计算统计
            let vp = normalize_path_separator(&new_virtual);
            context.queue_file_stats_update(hash, vp).await;
            continue;
        }

        match Box::pin(process_path_with_cas_and_checkpoints_inner(
            &full_path, // Use full path for file operations
            &new_virtual,
            context,
            provider,
            task_id,
            workspace_id,
            Some(archive_id),
            depth_level + 1,
        ))
        .await
        {
            Ok(_) => {
                debug!(
                    current = index + 1,
                    total = total_files,
                    file = %file_name,
                    "Successfully processed file"
                );
            }
            Err(e) => {
                if e.category() == ErrorCategory::CancellationRequested {
                    ensure_archive_not_cancelled(context, archive_id, archive_path, &extract_dir)
                        .await?;
                    return Err(e);
                }
                error!(
                    current = index + 1,
                    total = total_files,
                    file = %file_name,
                    error = %e,
                    "Failed to process file"
                );
                // Continue processing other files instead of failing the entire batch
            }
        }
    }

    // Update archive status to completed
    context
        .metadata_store
        .update_archive_status(archive_id, "completed")
        .await?;

    info!(
        archive_id = archive_id,
        files = extracted_files.len(),
        "Archive processing completed"
    );

    // 清理解压临时目录：文件已存入 CAS，临时目录不再需要
    if extract_dir.exists() {
        if let Err(e) = fs::remove_dir_all(&extract_dir).await {
            warn!(
                path = %extract_dir.display(),
                error = %e,
                "Failed to remove extraction temp directory"
            );
        } else {
            debug!(path = %extract_dir.display(), "Removed extraction temp directory");
        }
    }

    Ok(())
}

/// 检查文件是否为压缩文件
///
/// # 支持的格式
///
/// - ZIP (.zip)
/// - RAR (.rar)
/// - TAR (.tar, .tar.gz, .tgz)
/// - GZ (.gz)
fn is_archive_file(path: &Path) -> bool {
    let _archive_manager = ArchiveManager::new();
    _archive_manager.supported_extensions().iter().any(|ext| {
        // 检查扩展名是否匹配
        if let Some(file_ext) = path.extension().and_then(|s| s.to_str()) {
            if file_ext.eq_ignore_ascii_case(ext) {
                return true;
            }
        }

        // 检查复合扩展名如 .tar.gz
        if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
            let lower_name = name.to_lowercase();
            if lower_name.ends_with(&format!(".{ext}")) {
                return true;
            }
        }

        false
    })
}

// / 提取压缩文件并递归处理内容 (Legacy HashMap-based version)
// /
// / **Note**: This is the legacy implementation for backward compatibility.
// / New code should use `extract_and_process_archive_with_cas` instead.
// /
// / # 参数
// /
// / - `archive_manager`: ArchiveManager 实例
// / - `archive_path`: 压缩文件路径
// / - `virtual_path`: 虚拟路径前缀
// / - `target_root`: 解压目标根目录
// / - `map`: 路径映射表
// / - `provider`: 应用配置提供者（解耦 Tauri 依赖）
// / - `task_id`: 任务 ID
// / - `workspace_id`: 工作区 ID
// /

// ========== 防御性集成：文件过滤守卫函数 ==========

/// 文件过滤守卫（失败安全：任何错误都返回 true，允许文件通过）
///
/// 防御性设计原则：
/// 1. 配置加载失败 → 返回 true（允许所有文件）
/// 2. 过滤逻辑异常 → 返回 true（允许当前文件）
/// 3. 记录详细日志 → 便于问题排查
async fn should_import_file_defensive(path: &Path, provider: &dyn AppConfigProvider) -> bool {
    // Step 1: 安全加载配置（失败时返回 true）
    let filter_config = match load_file_filter_config_safe(provider).await {
        Ok(config) => config,
        Err(e) => {
            tracing::warn!(
                error = %e,
                "Failed to load file filter config, allowing all files (fail-safe)"
            );
            return true; // 失败安全：允许所有文件
        }
    };

    // Step 2: 如果过滤未启用，直接允许
    if !filter_config.enabled && !filter_config.binary_detection_enabled {
        return true;
    }

    // Step 3: 执行过滤检查（捕获所有异常）
    match FileTypeFilter::new(filter_config).should_import_file_safe(path) {
        Ok(should_import) => should_import,
        Err(e) => {
            tracing::warn!(
                file = %path.display(),
                error = %e,
                "File filter check failed, allowing file (fail-safe)"
            );
            true // 失败安全：允许当前文件
        }
    }
}

/// 安全加载配置（失败时返回默认配置）
async fn load_file_filter_config_safe(
    provider: &dyn AppConfigProvider,
) -> Result<FileFilterConfig> {
    match load_config_from_provider(provider).await {
        Ok(config) => Ok(config.file_filter),
        Err(e) => {
            tracing::warn!(
                error = %e,
                "Failed to load config, using default file filter config"
            );
            Ok(FileFilterConfig::default())
        }
    }
}

async fn load_archive_config_safe(provider: &dyn AppConfigProvider) -> ArchiveConfig {
    match load_config_from_provider(provider).await {
        Ok(config) => config.archive,
        Err(e) => {
            tracing::warn!(
                error = %e,
                "Failed to load config, using default archive config"
            );
            ArchiveConfig::default()
        }
    }
}

/// 从 AppConfigProvider 加载应用配置
async fn load_config_from_provider(
    provider: &dyn AppConfigProvider,
) -> std::result::Result<la_core::models::config::AppConfig, String> {
    use la_core::models::config::ConfigLoader as AppConfigLoader;
    let config_dir = provider.config_dir()?;
    let config_path = config_dir.join("config.json");
    if config_path.exists() {
        match AppConfigLoader::load(Some(config_path.clone())) {
            Ok(loader) => Ok(loader.get_config().clone()),
            Err(e) => {
                tracing::warn!(
                    path = %config_path.display(),
                    error = %e,
                    "配置文件解析失败，使用默认配置"
                );
                Ok(la_core::models::config::AppConfig::default())
            }
        }
    } else {
        Ok(la_core::models::config::AppConfig::default())
    }
}

// ========== 防御性集成结束 ==========

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "enhanced-extraction")]
    use flate2::write::GzEncoder;
    #[cfg(feature = "enhanced-extraction")]
    use flate2::Compression;
    use std::io::Write;
    use std::sync::Arc;
    use tempfile::TempDir;

    struct TestConfigProvider {
        dir: PathBuf,
    }

    impl AppConfigProvider for TestConfigProvider {
        fn config_dir(&self) -> std::result::Result<PathBuf, String> {
            Ok(self.dir.clone())
        }
    }

    #[test]
    fn boolean_environment_overrides_require_explicit_true_or_false() {
        assert_eq!(parse_boolean_override(" true "), Some(true));
        assert_eq!(parse_boolean_override("FALSE"), Some(false));
        assert_eq!(parse_boolean_override("1"), None);
        assert_eq!(parse_boolean_override(""), None);
    }

    #[test]
    fn child_virtual_path_preserves_nested_relative_path() {
        let relative = Path::new("nested").join("child.log");
        assert_eq!(
            build_child_virtual_path("root", &relative),
            "root/nested/child.log"
        );
    }

    #[cfg(feature = "enhanced-extraction")]
    #[test]
    fn enhanced_security_policy_tracks_archive_configuration() {
        let config = ArchiveConfig {
            max_compression_ratio: 42.0,
            max_archive_total_size: 123_456,
            max_workspace_size: 789_012,
            exponential_backoff_threshold: 9_876.0,
            ..ArchiveConfig::default()
        };

        let security_policy = security_policy_from_archive_config(&config, 12);

        assert_eq!(security_policy.max_compression_ratio, 42.0);
        assert_eq!(security_policy.max_cumulative_size, 123_456);
        assert_eq!(security_policy.max_workspace_size, 789_000);
        assert_eq!(security_policy.exponential_backoff_threshold, 9_876.0);
    }

    #[cfg(feature = "enhanced-extraction")]
    #[tokio::test]
    async fn default_archive_import_enforces_configured_compression_ratio() {
        let temp = TempDir::new().unwrap();
        let archive_path = temp.path().join("suspicious.log.gz");
        let config_dir = temp.path().join("config");
        let workspace_dir = temp.path().join("workspace");

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&vec![b'A'; 64 * 1024]).unwrap();
        std::fs::write(&archive_path, encoder.finish().unwrap()).unwrap();

        let mut config = la_core::models::config::AppConfig::default();
        config.archive.max_compression_ratio = 2.0;
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::write(
            config_dir.join("config.json"),
            serde_json::to_vec_pretty(&config).unwrap(),
        )
        .unwrap();

        let cas = Arc::new(ContentAddressableStorage::new(workspace_dir.clone()));
        let metadata_store = Arc::new(MetadataStore::new(&workspace_dir).await.unwrap());
        let context = CasProcessingContext::new(workspace_dir, Arc::clone(&cas), metadata_store);
        let provider = TestConfigProvider { dir: config_dir };

        let error = process_path_with_cas_and_checkpoints(
            &archive_path,
            "suspicious.log.gz",
            &context,
            &provider,
            "task-1",
            "workspace-1",
            None,
            0,
        )
        .await
        .expect_err("default archive imports must enforce the configured compression-ratio limit");

        assert!(
            error.to_string().contains("Compression ratio"),
            "unexpected error: {error}"
        );
    }

    fn create_many_file_zip(path: &Path, file_count: usize) {
        let file = std::fs::File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::FileOptions::<'_, ()>::default()
            .compression_method(zip::CompressionMethod::Stored);

        for i in 0..file_count {
            zip.start_file(format!("file-{i:04}.log"), options).unwrap();
            zip.write_all(format!("2026-06-15T00:00:00Z INFO line {i}\n").as_bytes())
                .unwrap();
        }

        zip.finish().unwrap();
    }

    #[tokio::test]
    async fn archive_import_preserves_safe_filename_containing_two_dots() {
        let temp = TempDir::new().unwrap();
        let zip_path = temp.path().join("release.zip");
        let content = b"2026-06-15T00:00:00Z INFO release\n";
        let file = std::fs::File::create(&zip_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::FileOptions::<'_, ()>::default()
            .compression_method(zip::CompressionMethod::Stored);
        zip.start_file("release..log", options).unwrap();
        zip.write_all(content).unwrap();
        zip.finish().unwrap();

        let workspace_dir = temp.path().join("workspace");
        let cas = Arc::new(ContentAddressableStorage::new(workspace_dir.clone()));
        let metadata_store = Arc::new(MetadataStore::new(&workspace_dir).await.unwrap());
        let context =
            CasProcessingContext::new(workspace_dir, Arc::clone(&cas), Arc::clone(&metadata_store));
        let provider = TestConfigProvider {
            dir: temp.path().join("config"),
        };

        process_path_with_cas_and_checkpoints(
            &zip_path,
            "release.zip",
            &context,
            &provider,
            "task-1",
            "workspace-1",
            None,
            0,
        )
        .await
        .unwrap();

        let files = metadata_store.get_all_files().await.unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].virtual_path, "release.zip/release..log");
        assert_eq!(files[0].original_name, "release..log");
        assert_eq!(
            cas.read_content(&files[0].sha256_hash).await.unwrap(),
            content
        );
    }

    #[tokio::test]
    async fn archive_import_rejects_parent_absolute_and_drive_member_paths() {
        let temp = TempDir::new().unwrap();
        let zip_path = temp.path().join("unsafe.zip");
        let file = std::fs::File::create(&zip_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::FileOptions::<'_, ()>::default()
            .compression_method(zip::CompressionMethod::Stored);
        for name in [
            "../escape.log",
            r"..\escape-windows.log",
            "/absolute.log",
            r"\absolute-windows.log",
            "C:/drive.log",
        ] {
            zip.start_file(name, options).unwrap();
            zip.write_all(b"unsafe\n").unwrap();
        }
        zip.finish().unwrap();

        let workspace_dir = temp.path().join("workspace");
        let cas = Arc::new(ContentAddressableStorage::new(workspace_dir.clone()));
        let metadata_store = Arc::new(MetadataStore::new(&workspace_dir).await.unwrap());
        let context =
            CasProcessingContext::new(workspace_dir, Arc::clone(&cas), Arc::clone(&metadata_store));
        let provider = TestConfigProvider {
            dir: temp.path().join("config"),
        };

        process_path_with_cas_and_checkpoints(
            &zip_path,
            "unsafe.zip",
            &context,
            &provider,
            "task-1",
            "workspace-1",
            None,
            0,
        )
        .await
        .unwrap();

        assert!(metadata_store.get_all_files().await.unwrap().is_empty());
        assert!(!temp.path().join("escape.log").exists());
        assert!(!temp.path().join("absolute.log").exists());
    }

    #[tokio::test]
    async fn directory_import_preserves_nested_virtual_paths() {
        let temp = TempDir::new().unwrap();
        let source_dir = temp.path().join("source");
        let nested_dir = source_dir.join("nested");
        tokio::fs::create_dir_all(&nested_dir).await.unwrap();
        tokio::fs::write(source_dir.join("top.log"), b"top\n")
            .await
            .unwrap();
        tokio::fs::write(nested_dir.join("child.log"), b"child\n")
            .await
            .unwrap();

        let workspace_dir = temp.path().join("workspace");
        let cas = Arc::new(ContentAddressableStorage::new(workspace_dir.clone()));
        let metadata_store = Arc::new(MetadataStore::new(&workspace_dir).await.unwrap());
        let context =
            CasProcessingContext::new(workspace_dir, Arc::clone(&cas), Arc::clone(&metadata_store));
        let provider = TestConfigProvider {
            dir: temp.path().join("config"),
        };

        process_path_with_cas_and_checkpoints(
            &source_dir,
            "source",
            &context,
            &provider,
            "task-1",
            "workspace-1",
            None,
            0,
        )
        .await
        .unwrap();

        let mut paths: Vec<String> = metadata_store
            .get_all_files()
            .await
            .unwrap()
            .into_iter()
            .map(|file| file.virtual_path)
            .collect();
        paths.sort();

        assert_eq!(
            paths,
            vec![
                "source/nested/child.log".to_string(),
                "source/top.log".to_string()
            ]
        );
    }

    #[tokio::test]
    async fn pre_cancelled_directory_import_writes_no_metadata() {
        let temp = TempDir::new().unwrap();
        let source_dir = temp.path().join("source");
        tokio::fs::create_dir_all(&source_dir).await.unwrap();
        tokio::fs::write(source_dir.join("cancelled.log"), b"should not import\n")
            .await
            .unwrap();

        let workspace_dir = temp.path().join("workspace");
        let cas = Arc::new(ContentAddressableStorage::new(workspace_dir.clone()));
        let metadata_store = Arc::new(MetadataStore::new(&workspace_dir).await.unwrap());
        let cancellation_token = CancellationToken::new();
        cancellation_token.cancel();
        let context =
            CasProcessingContext::new(workspace_dir, Arc::clone(&cas), Arc::clone(&metadata_store))
                .with_cancellation_token(cancellation_token);
        let provider = TestConfigProvider {
            dir: temp.path().join("config"),
        };

        let result = process_path_with_cas_and_checkpoints(
            &source_dir,
            "source",
            &context,
            &provider,
            "task-cancelled",
            "workspace-1",
            None,
            0,
        )
        .await;

        assert!(result.is_err(), "a pre-cancelled import must stop");
        assert_eq!(metadata_store.count_files().await.unwrap(), 0);
    }

    #[test]
    fn completed_import_has_ready_metadata_without_a_background_runtime() {
        let temp = TempDir::new().unwrap();
        let source_file = temp.path().join("app.log");
        let mut content = b"2026-07-30T12:00:00Z ERROR deterministic readiness\n".to_vec();
        content.resize(SMALL_FILE_INLINE_CAS_THRESHOLD_BYTES as usize + 1, b'x');
        std::fs::write(&source_file, content).unwrap();

        let workspace_dir = temp.path().join("workspace");
        let config_dir = temp.path().join("config");
        let metadata_store = {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            runtime.block_on(async {
                let cas = ContentAddressableStorage::new(workspace_dir.clone());
                let metadata_store = Arc::new(MetadataStore::new(&workspace_dir).await.unwrap());
                let provider = TestConfigProvider { dir: config_dir };

                process_path_with_cas(
                    &source_file,
                    "app.log",
                    &workspace_dir,
                    &cas,
                    Arc::clone(&metadata_store),
                    &provider,
                    "task-ready",
                    "workspace-ready",
                    None,
                    0,
                )
                .await
                .unwrap();

                metadata_store
            })
        };

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let file = runtime
            .block_on(metadata_store.get_file_by_virtual_path("app.log"))
            .unwrap()
            .unwrap();

        assert_eq!(
            file.analysis_status,
            la_core::storage_types::AnalysisStatus::Ready,
            "a successful import must not depend on a detached task to make metadata searchable"
        );
    }

    #[tokio::test]
    async fn directory_import_batches_many_regular_files() {
        let temp = TempDir::new().unwrap();
        let source_dir = temp.path().join("many");
        tokio::fs::create_dir_all(&source_dir).await.unwrap();

        for i in 0..12 {
            tokio::fs::write(
                source_dir.join(format!("file-{i:02}.log")),
                format!("line {i}\n"),
            )
            .await
            .unwrap();
        }

        let workspace_dir = temp.path().join("workspace");
        let cas = Arc::new(ContentAddressableStorage::new(workspace_dir.clone()));
        let metadata_store = Arc::new(MetadataStore::new(&workspace_dir).await.unwrap());
        let context =
            CasProcessingContext::new(workspace_dir, Arc::clone(&cas), Arc::clone(&metadata_store));
        let provider = TestConfigProvider {
            dir: temp.path().join("config"),
        };

        process_path_with_cas_and_checkpoints(
            &source_dir,
            "many",
            &context,
            &provider,
            "task-1",
            "workspace-1",
            None,
            0,
        )
        .await
        .unwrap();

        assert_eq!(metadata_store.count_files().await.unwrap(), 12);
        assert_eq!(
            metadata_store.get_ready_files().await.unwrap().len(),
            12,
            "directory import must wait for every parallel statistics update"
        );
    }

    #[tokio::test]
    #[ignore = "local performance smoke test; run explicitly when tuning import latency"]
    async fn directory_imports_1000_small_files_in_seconds() {
        let temp = TempDir::new().unwrap();
        let source_dir = temp.path().join("many");
        tokio::fs::create_dir_all(&source_dir).await.unwrap();

        for i in 0..1000 {
            tokio::fs::write(
                source_dir.join(format!("file-{i:04}.log")),
                format!("2026-06-15T00:00:00Z INFO line {i}\n"),
            )
            .await
            .unwrap();
        }

        let workspace_dir = temp.path().join("workspace");
        let cas = Arc::new(ContentAddressableStorage::new(workspace_dir.clone()));
        let metadata_store = Arc::new(MetadataStore::new(&workspace_dir).await.unwrap());
        let context =
            CasProcessingContext::new(workspace_dir, Arc::clone(&cas), Arc::clone(&metadata_store));
        let provider = TestConfigProvider {
            dir: temp.path().join("config"),
        };

        let started = std::time::Instant::now();
        process_path_with_cas_and_checkpoints(
            &source_dir,
            "many",
            &context,
            &provider,
            "task-1",
            "workspace-1",
            None,
            0,
        )
        .await
        .unwrap();
        let elapsed = started.elapsed();

        assert_eq!(metadata_store.count_files().await.unwrap(), 1000);
        assert!(
            elapsed < std::time::Duration::from_secs(2),
            "expected 1000-file import to complete in seconds, took {elapsed:?}"
        );
        eprintln!("1000-file import smoke elapsed: {elapsed:?}");
    }

    #[tokio::test]
    #[ignore = "local performance smoke test; run explicitly when tuning archive import latency"]
    async fn zip_imports_1000_small_files_in_seconds() {
        let temp = TempDir::new().unwrap();
        let zip_path = temp.path().join("many.zip");
        create_many_file_zip(&zip_path, 1000);

        let workspace_dir = temp.path().join("workspace");
        let cas = Arc::new(ContentAddressableStorage::new(workspace_dir.clone()));
        let metadata_store = Arc::new(MetadataStore::new(&workspace_dir).await.unwrap());
        let context =
            CasProcessingContext::new(workspace_dir, Arc::clone(&cas), Arc::clone(&metadata_store));
        let provider = TestConfigProvider {
            dir: temp.path().join("config"),
        };

        let started = std::time::Instant::now();
        process_path_with_cas_and_checkpoints(
            &zip_path,
            "many.zip",
            &context,
            &provider,
            "task-1",
            "workspace-1",
            None,
            0,
        )
        .await
        .unwrap();
        let elapsed = started.elapsed();

        assert_eq!(metadata_store.count_files().await.unwrap(), 1000);
        assert!(
            elapsed < std::time::Duration::from_secs(3),
            "expected 1000-file zip import to complete in seconds, took {elapsed:?}"
        );
        eprintln!("1000-file zip import smoke elapsed: {elapsed:?}");
    }
}
