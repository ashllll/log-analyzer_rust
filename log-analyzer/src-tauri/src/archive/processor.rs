//! 递归路径处理器
//!
//! 核心递归逻辑，负责：
//! - 识别文件类型（压缩文件 vs 普通文件）
//! - 调用统一的 ArchiveManager 处理器或增强提取系统
//! - 支持异步递归解压嵌套压缩包
//! - 收集元数据（增量索引）
//! - 错误处理和进度报告

use crate::archive::checkpoint_manager::{Checkpoint, CheckpointManager};
use crate::archive::extraction_engine::ExtractionPolicy;
use crate::archive::public_api::extract_archive_async;
use crate::archive::ArchiveManager;
use crate::error::{AppError, Result};
use crate::models::FileFilterConfig;
use crate::services::file_type_filter::FileTypeFilter;
use crate::services::file_watcher::get_file_metadata;
use crate::storage::{ArchiveMetadata, ContentAddressableStorage, FileMetadata, MetadataStore};
use crate::utils::path::normalize_path_separator;
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use tauri::AppHandle;
use tokio::fs;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};
use walkdir::WalkDir;

/// 检查是否启用增强提取系统
///
/// 优先级：
/// 1. 环境变量 USE_ENHANCED_EXTRACTION（用于测试和临时覆盖）
/// 2. 配置文件中的 use_enhanced_extraction 标志
/// 3. 默认为 false（使用旧系统）以保持向后兼容性
fn is_enhanced_extraction_enabled() -> bool {
    // 首先检查环境变量（最高优先级）
    if let Ok(env_value) = std::env::var("USE_ENHANCED_EXTRACTION") {
        return env_value.to_lowercase() == "true";
    }

    // Configuration file support can be added in the future if needed
    // Currently defaults to false for backward compatibility
    false
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

    // 检查路径组件中是否包含可疑的遍历尝试
    for component in path.components() {
        if let Component::Normal(os_str) = component {
            if let Some(str) = os_str.to_str() {
                // 检查是否包含路径遍历序列
                if str.contains("..") || str.contains("/") || str.contains("\\") {
                    return Err(AppError::validation_error(format!(
                        "Suspicious path component detected: {}",
                        str
                    )));
                }
            }
        }
    }

    Ok(())
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
    // Check for path traversal sequences
    if virtual_path.contains("..") {
        return Err(AppError::validation_error(format!(
            "Path traversal detected in virtual path: {}",
            virtual_path
        )));
    }

    // Check for absolute paths (virtual paths should be relative)
    if virtual_path.starts_with('/') || virtual_path.starts_with('\\') {
        return Err(AppError::validation_error(format!(
            "Virtual path should be relative, not absolute: {}",
            virtual_path
        )));
    }

    // Check for Windows drive letters
    if virtual_path.len() >= 2 && virtual_path.chars().nth(1) == Some(':') {
        return Err(AppError::validation_error(format!(
            "Virtual path should not contain drive letters: {}",
            virtual_path
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

/// 递归处理路径（公共接口，带错误处理）
///
/// # 参数
///
/// - `path`: 要处理的路径（文件或目录）
/// - `virtual_path`: 虚拟路径（用于索引）
/// - `target_root`: 临时目录根路径
/// - `map`: 真实路径到虚拟路径的映射表
/// - `app`: Tauri 应用句柄
/// - `task_id`: 任务 ID
/// - `workspace_id`: 工作区ID(用于解压目录命名)
///
/// # 行为
///
/// - 内部调用 `process_path_recursive_inner` 进行实际处理
/// - 支持异步递归解压嵌套压缩包
/// - 捕获错误，记录日志，发送警告事件
/// - 单个文件失败不中断整体流程
pub async fn process_path_recursive(
    path: &Path,
    virtual_path: &str,
    target_root: &Path,
    map: &mut HashMap<String, String>,
    app: &AppHandle,
    task_id: &str,
    workspace_id: &str,
) {
    // 错误处理：如果处理失败，不中断整个流程
    if let Err(e) = process_path_recursive_inner(
        path,
        virtual_path,
        target_root,
        map,
        app,
        task_id,
        workspace_id,
    )
    .await
    {
        // 老王备注：移除事件发送，所有事件都通过 TaskManager 发送
        tracing::warn!(
            path = %path.display(),
            error = %e,
            task_id = %task_id,
            "Failed to process archive (event sent via TaskManager)"
        );
    }
}

/// 带元数据收集的递归处理（公共接口，带错误处理）
///
/// # 参数
///
/// - `path`: 要处理的路径
/// - `virtual_path`: 虚拟路径
/// - `target_root`: 临时目录根路径
/// - `map`: 路径映射表
/// - `metadata_map`: 文件元数据映射表（用于增量索引）
/// - `app`: Tauri 应用句柄
/// - `task_id`: 任务 ID
/// - `workspace_id`: 工作区ID
#[allow(clippy::too_many_arguments)]
pub async fn process_path_recursive_with_metadata(
    path: &Path,
    virtual_path: &str,
    target_root: &Path,
    map: &mut HashMap<String, String>,
    metadata_map: &mut HashMap<String, crate::storage::FileMetadata>,
    app: &AppHandle,
    task_id: &str,
    workspace_id: &str,
) {
    if let Err(e) = process_path_recursive_inner_with_metadata(
        path,
        virtual_path,
        target_root,
        map,
        metadata_map,
        app,
        task_id,
        workspace_id,
    )
    .await
    {
        eprintln!("[WARNING] Failed to process {}: {}", path.display(), e);
        tracing::warn!(
            error = %e,
            task_id = %task_id,
            "Warning during archive processing (event sent via TaskManager)"
        );
    }
}

/// 递归处理路径（内部实现）
///
/// # 行为
///
/// 1. 如果是目录：递归处理子项
/// 2. 如果是压缩文件：
///    - 使用 ArchiveManager 统一接口解压
///    - 递归处理解压后的文件（支持嵌套压缩包）
/// 3. 如果是普通文件：添加到索引
///
/// # 并发安全
///
/// - 使用 Box::pin 解决递归异步调用问题
/// - 所有类型满足 Send trait 要求
async fn process_path_recursive_inner(
    path: &Path,
    virtual_path: &str,
    target_root: &Path,
    map: &mut HashMap<String, String>,
    app: &AppHandle,
    task_id: &str,
    workspace_id: &str,
) -> Result<()> {
    // 处理目录
    if path.is_dir() {
        for entry in WalkDir::new(path)
            .min_depth(1)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let entry_name = entry.file_name().to_string_lossy().to_string();
            let new_virtual = format!("{}/{}", virtual_path, entry_name);

            // 使用 Box::pin 解决递归异步调用
            Box::pin(process_path_recursive(
                entry.path(),
                &new_virtual,
                target_root,
                map,
                app,
                task_id,
                workspace_id,
            ))
            .await;
        }
        return Ok(());
    }

    let file_name = path
        .file_name()
        .ok_or_else(|| AppError::validation_error("Invalid filename"))?
        .to_string_lossy()
        .to_string();

    tracing::debug!(
        task_id = %task_id,
        file = %file_name,
        "Processing file (event sent via TaskManager)"
    );

    // 创建 ArchiveManager 实例
    let archive_manager = ArchiveManager::new();

    // 检查是否为压缩文件
    if is_archive_file(path) {
        // 使用统一接口处理压缩文件
        match extract_and_process_archive(
            &archive_manager,
            path,
            virtual_path,
            target_root,
            map,
            app,
            task_id,
            workspace_id,
        )
        .await
        {
            Ok(_) => return Ok(()),
            Err(e) => {
                eprintln!(
                    "[WARNING] Failed to extract archive {}: {}",
                    path.display(),
                    e
                );
                tracing::warn!(
                    file = %file_name,
                    error = %e,
                    task_id = %task_id,
                    "Failed to extract archive (event sent via TaskManager)"
                );
                return Err(e);
            }
        }
    }

    // --- 普通文件 ---
    let real_path = path.to_string_lossy().to_string();
    let normalized_virtual = normalize_path_separator(virtual_path);

    // Validate file exists before processing
    if !path.exists() {
        warn!(
            file = %real_path,
            "File does not exist when processing, skipping"
        );
        return Ok(());
    }

    debug!(
        real_path = %real_path,
        virtual_path = %normalized_virtual,
        "Processing file for CAS storage"
    );

    map.insert(real_path, normalized_virtual.clone());

    Ok(())
}

/// 带元数据收集的内部处理函数
///
/// # 行为
///
/// - 压缩文件：使用统一 ArchiveManager 接口处理
/// - 普通文件：收集元数据并添加到 `metadata_map`
///
/// # 并发安全
///
/// - 使用 Box::pin 解决递归异步调用问题
#[allow(clippy::too_many_arguments)]
async fn process_path_recursive_inner_with_metadata(
    path: &Path,
    virtual_path: &str,
    target_root: &Path,
    map: &mut HashMap<String, String>,
    metadata_map: &mut HashMap<String, crate::storage::FileMetadata>,
    app: &AppHandle,
    task_id: &str,
    workspace_id: &str,
) -> Result<()> {
    // 处理目录
    if path.is_dir() {
        for entry in WalkDir::new(path)
            .min_depth(1)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let entry_name = entry.file_name().to_string_lossy().to_string();
            let new_virtual = format!("{}/{}", virtual_path, entry_name);

            // 使用 Box::pin 解决递归异步调用
            Box::pin(process_path_recursive_with_metadata(
                entry.path(),
                &new_virtual,
                target_root,
                map,
                metadata_map,
                app,
                task_id,
                workspace_id,
            ))
            .await;
        }
        return Ok(());
    }

    let file_name = path
        .file_name()
        .ok_or_else(|| AppError::validation_error("Invalid filename"))?
        .to_string_lossy()
        .to_string();

    tracing::debug!(
        task_id = %task_id,
        file = %file_name,
        "Processing file with metadata (event sent via TaskManager)"
    );

    // 压缩文件不收集元数据，使用原始处理逻辑
    if is_archive_file(path) {
        return Box::pin(process_path_recursive_inner(
            path,
            virtual_path,
            target_root,
            map,
            app,
            task_id,
            workspace_id,
        ))
        .await;
    }

    // --- 普通文件：收集元数据 ---
    let real_path = path.to_string_lossy().to_string();
    let normalized_virtual = normalize_path_separator(virtual_path);

    map.insert(real_path.clone(), normalized_virtual.clone());

    // 收集文件元数据
    if let Ok(metadata) = get_file_metadata(path) {
        metadata_map.insert(real_path.clone(), metadata);
    }

    Ok(())
}

// ========== NEW CAS-BASED PROCESSING FUNCTIONS ==========

/// Context for CAS processing with checkpoint support
pub struct CasProcessingContext {
    pub workspace_dir: PathBuf,
    pub cas: Arc<ContentAddressableStorage>,
    pub metadata_store: Arc<MetadataStore>,
    pub checkpoint_manager: Option<Arc<Mutex<CheckpointManager>>>,
    pub checkpoint: Option<Arc<Mutex<Checkpoint>>>,
    pub files_since_checkpoint: Arc<Mutex<usize>>,
    pub bytes_since_checkpoint: Arc<Mutex<u64>>,
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
            checkpoint_manager: None,
            checkpoint: None,
            files_since_checkpoint: Arc::new(Mutex::new(0)),
            bytes_since_checkpoint: Arc::new(Mutex::new(0)),
        }
    }

    /// Enable checkpoint support
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
        if let (Some(manager), Some(checkpoint)) = (&self.checkpoint_manager, &self.checkpoint) {
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
        Ok(())
    }

    /// Update checkpoint with new file
    async fn update_checkpoint(&self, file_path: PathBuf, file_size: u64) -> Result<()> {
        if let Some(checkpoint) = &self.checkpoint {
            let mut checkpoint_guard = checkpoint.lock().await;
            checkpoint_guard.update_file(file_path, file_size);

            // Update counters
            *self.files_since_checkpoint.lock().await += 1;
            *self.bytes_since_checkpoint.lock().await += file_size;
        }
        Ok(())
    }

    /// Check if file was already extracted
    async fn is_file_extracted(&self, file_path: &Path) -> bool {
        if let Some(checkpoint) = &self.checkpoint {
            let checkpoint_guard = checkpoint.lock().await;
            checkpoint_guard.is_file_extracted(file_path)
        } else {
            false
        }
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
/// * `app` - Tauri app handle
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
    app: &AppHandle,
    task_id: &str,
    workspace_id: &str,
    parent_archive_id: Option<i64>,
    depth_level: i32,
) -> Result<()> {
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
        for entry in WalkDir::new(path)
            .min_depth(1)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let entry_name = entry.file_name().to_string_lossy().to_string();
            let new_virtual = format!("{}/{}", virtual_path, entry_name);

            Box::pin(process_path_with_cas_and_checkpoints(
                entry.path(),
                &new_virtual,
                context,
                app,
                task_id,
                workspace_id,
                parent_archive_id,
                depth_level,
            ))
            .await?;
        }
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
        let import_allowed = should_import_file_defensive(path, app).await;

        if !import_allowed {
            tracing::info!(
                file = %file_name,
                path = %path.display(),
                "File skipped by filter configuration (user-configured)"
            );
            return Ok(());  // 跳过此文件，但继续处理其他文件
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
            app,
            task_id,
            workspace_id,
            parent_archive_id,
            depth_level,
        ))
        .await;
    }

    // --- Regular file: Store in CAS and add to metadata ---

    // Store file content in CAS using streaming (memory-efficient)
    let hash = context.cas.store_file_streaming(path).await?;

    // Get file metadata
    let file_size = fs::metadata(path)
        .await
        .map(|m| m.len() as i64)
        .unwrap_or(0);

    let modified_time = fs::metadata(path)
        .await
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    // Detect MIME type (simple heuristic based on extension)
    let mime_type = path
        .extension()
        .and_then(|ext| ext.to_str())
        .and_then(|ext| match ext.to_lowercase().as_str() {
            "log" | "txt" => Some("text/plain".to_string()),
            "json" => Some("application/json".to_string()),
            "xml" => Some("application/xml".to_string()),
            "gz" => Some("application/gzip".to_string()),
            "zip" => Some("application/zip".to_string()),
            _ => None,
        });

    // Create file metadata
    let file_metadata = FileMetadata {
        id: 0, // Will be auto-generated
        sha256_hash: hash.clone(),
        virtual_path: normalize_path_separator(virtual_path),
        original_name: file_name.clone(),
        size: file_size,
        modified_time,
        mime_type,
        parent_archive_id,
        depth_level,
    };

    // Insert into metadata store
    let file_id = context.metadata_store.insert_file(&file_metadata).await?;

    // Update checkpoint
    context
        .update_checkpoint(path.to_path_buf(), file_size as u64)
        .await?;

    // Maybe save checkpoint
    context.maybe_save_checkpoint().await?;

    debug!(
        file_id = file_id,
        hash = %hash,
        virtual_path = %virtual_path,
        size = file_size,
        depth = depth_level,
        "Stored file in CAS and metadata"
    );

    Ok(())
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
/// * `app` - Tauri app handle
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
    app: &AppHandle,
    task_id: &str,
    workspace_id: &str,
    parent_archive_id: Option<i64>,
    depth_level: i32,
) -> Result<()> {
    // Wrap CAS in Arc for the context
    let cas_arc = Arc::new(cas.clone());

    // Create context without checkpoints for backward compatibility
    let context = CasProcessingContext::new(workspace_dir.to_path_buf(), cas_arc, metadata_store);

    process_path_with_cas_and_checkpoints(
        path,
        virtual_path,
        &context,
        app,
        task_id,
        workspace_id,
        parent_archive_id,
        depth_level,
    )
    .await
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
/// * `app` - Tauri app handle
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
    app: &AppHandle,
    task_id: &str,
    workspace_id: &str,
    parent_archive_id: Option<i64>,
    depth_level: i32,
) -> Result<()> {
    const MAX_NESTING_DEPTH: i32 = 10;

    // Check depth limit to prevent infinite recursion
    if depth_level >= MAX_NESTING_DEPTH {
        warn!(
            archive = %archive_path.display(),
            depth = depth_level,
            "Maximum nesting depth reached, skipping archive"
        );
        return Ok(());
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
    let archive_hash = context.cas.store_file_streaming(archive_path).await?;

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
            format!("Failed to create extraction directory: {}", e),
            Some(extract_dir.clone()),
        )
    })?;

    // Extract archive
    let archive_manager = ArchiveManager::new();
    let extracted_files = if is_enhanced_extraction_enabled() {
        let policy = ExtractionPolicy::default();
        match extract_archive_async(archive_path, &extract_dir, workspace_id, Some(policy)).await {
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
                return Err(AppError::archive_error(
                    format!("Enhanced extraction failed: {}", e),
                    Some(archive_path.to_path_buf()),
                ));
            }
        }
    } else {
        match archive_manager
            .extract_archive(archive_path, &extract_dir)
            .await
        {
            Ok(summary) => {
                info!(
                    files = summary.files_extracted,
                    bytes = summary.total_size,
                    "Legacy extraction completed"
                );
                summary.extracted_files
            }
            Err(e) => {
                context
                    .metadata_store
                    .update_archive_status(archive_id, "failed")
                    .await?;
                return Err(AppError::archive_error(
                    format!("Legacy extraction failed: {}", e),
                    Some(archive_path.to_path_buf()),
                ));
            }
        }
    };

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
        let file_name = extracted_file
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        info!(
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

        let new_virtual = format!(
            "{}/{}/{}",
            virtual_path,
            file_name,
            relative_path.to_string_lossy()
        );

        // Recursively process (supports nested archives)
        match Box::pin(process_path_with_cas_and_checkpoints(
            &full_path, // Use full path for file operations
            &new_virtual,
            context,
            app,
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

    Ok(())
}

/// Extract and process archive using CAS
///
/// This function handles nested archive extraction with depth tracking.
///
/// # Arguments
///
/// * `archive_path` - Path to the archive file
/// * `virtual_path` - Virtual path prefix
/// * `workspace_dir` - Workspace directory
/// * `cas` - Content-Addressable Storage instance
/// * `metadata_store` - Metadata store instance (wrapped in Arc)
/// * `app` - Tauri app handle
/// * `task_id` - Task ID
/// * `workspace_id` - Workspace ID
/// * `parent_archive_id` - Parent archive ID (None for root)
/// * `depth_level` - Current nesting depth
///
/// # Requirements
///
/// Validates: Requirements 4.1, 4.2, 4.3
#[allow(clippy::too_many_arguments)]
#[allow(dead_code)]
async fn extract_and_process_archive_with_cas(
    archive_path: &Path,
    virtual_path: &str,
    workspace_dir: &Path,
    cas: &ContentAddressableStorage,
    metadata_store: Arc<MetadataStore>,
    app: &AppHandle,
    task_id: &str,
    workspace_id: &str,
    parent_archive_id: Option<i64>,
    depth_level: i32,
) -> Result<()> {
    // Wrap CAS in Arc for the context
    let cas_arc = Arc::new(cas.clone());

    // Create context without checkpoints for backward compatibility
    let context = CasProcessingContext::new(workspace_dir.to_path_buf(), cas_arc, metadata_store);

    extract_and_process_archive_with_cas_and_checkpoints(
        archive_path,
        virtual_path,
        &context,
        app,
        task_id,
        workspace_id,
        parent_archive_id,
        depth_level,
    )
    .await
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
            if lower_name.ends_with(&format!(".{}", ext)) {
                return true;
            }
        }

        false
    })
}

/// 提取压缩文件并递归处理内容 (Legacy HashMap-based version)
///
/// **Note**: This is the legacy implementation for backward compatibility.
/// New code should use `extract_and_process_archive_with_cas` instead.
///
/// # 参数
///
/// - `archive_manager`: ArchiveManager 实例
/// - `archive_path`: 压缩文件路径
/// - `virtual_path`: 虚拟路径前缀
/// - `target_root`: 解压目标根目录
/// - `map`: 路径映射表
/// - `app`: Tauri 应用句柄
/// - `task_id`: 任务 ID
/// - `workspace_id`: 工作区 ID
///
/// # 行为
///
/// 1. 检查是否启用增强提取系统
/// 2. 如果启用，使用 extract_archive_async；否则使用 ArchiveManager
/// 3. 递归处理解压后的文件（支持嵌套压缩包）
/// 4. 清理临时目录（失败时）
/// 5. Track nesting depth and prevent infinite recursion
///
/// # Requirements
///
/// Validates: Requirements 4.1, 4.2, 4.3 (legacy implementation)
#[allow(clippy::too_many_arguments)]
async fn extract_and_process_archive(
    archive_manager: &ArchiveManager,
    archive_path: &Path,
    virtual_path: &str,
    target_root: &Path,
    map: &mut HashMap<String, String>,
    app: &AppHandle,
    task_id: &str,
    workspace_id: &str,
) -> Result<()> {
    const MAX_NESTING_DEPTH: i32 = 10;

    // Track depth to prevent infinite recursion
    // For legacy implementation, we estimate depth from virtual_path
    // Count the number of archive separators to determine nesting level
    let depth_level = virtual_path.matches('/').count() as i32;

    if depth_level >= MAX_NESTING_DEPTH {
        warn!(
            archive = %archive_path.display(),
            depth = depth_level,
            "Maximum nesting depth reached in legacy mode, skipping archive"
        );
        return Ok(());
    }
    let file_name = archive_path
        .file_name()
        .ok_or_else(|| AppError::validation_error("Invalid archive filename"))?
        .to_string_lossy()
        .to_string();

    info!(
        archive = %file_name,
        depth = depth_level,
        mode = "legacy",
        "Processing archive (legacy HashMap mode)"
    );

    // 创建工作区解压目录 (archive_name_timestamp)
    // 该目录将持久化保存，直到工作区删除时统一清理
    // 路径：app_data_dir/extracted/workspaceId/archive_name_timestamp
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let extract_dir_name = format!("{}_{}", file_name.replace('.', "_"), timestamp);
    let extract_dir = target_root.join(&extract_dir_name);

    // 确保解压目录存在
    fs::create_dir_all(&extract_dir).await.map_err(|e| {
        AppError::archive_error(
            format!("Failed to create extraction directory: {}", e),
            Some(extract_dir.clone()),
        )
    })?;

    // 检查是否使用增强提取系统
    let extracted_files = if is_enhanced_extraction_enabled() {
        // 使用增强提取系统
        eprintln!("[INFO] Using enhanced extraction system for {}", file_name);

        let policy = ExtractionPolicy::default();

        match extract_archive_async(archive_path, &extract_dir, workspace_id, Some(policy)).await {
            Ok(result) => {
                eprintln!(
                    "[INFO] Enhanced extraction: {} files from {} (total size: {} bytes)",
                    result.extracted_files.len(),
                    file_name,
                    result.performance_metrics.bytes_extracted
                );

                // Log warnings
                for warning in &result.warnings {
                    warn!(
                        category = ?warning.category,
                        message = %warning.message,
                        "Extraction warning"
                    );
                }

                // Log security events
                for event in &result.security_events {
                    warn!(
                        event_type = ?event.event_type,
                        details = ?event.details,
                        "Security event detected"
                    );
                }

                // Log path mappings
                for (short_path, original_path) in &result.metadata_mappings {
                    debug!(
                        short_path = %short_path.display(),
                        original_path = %original_path.display(),
                        "Path mapping created"
                    );
                }

                result.extracted_files
            }
            Err(e) => {
                return Err(AppError::archive_error(
                    format!("Enhanced extraction failed for {}: {}", file_name, e),
                    Some(archive_path.to_path_buf()),
                ));
            }
        }
    } else {
        // 使用旧的 ArchiveManager
        eprintln!("[INFO] Using legacy extraction system for {}", file_name);

        let summary = archive_manager
            .extract_archive(archive_path, &extract_dir)
            .await
            .map_err(|e| {
                AppError::archive_error(
                    format!("Failed to extract {}: {}", file_name, e),
                    Some(archive_path.to_path_buf()),
                )
            })?;

        // 报告提取结果
        eprintln!(
            "[INFO] Extracted {} files from {} (total size: {} bytes)",
            summary.files_extracted, file_name, summary.total_size
        );

        if summary.has_errors() {
            eprintln!("[WARNING] Extraction errors: {:?}", summary.errors);
        }

        summary.extracted_files
    };

    // Recursively process extracted files (supports nested archives)
    debug!(
        files_count = extracted_files.len(),
        archive = %file_name,
        "Processing extracted files"
    );

    for extracted_file in &extracted_files {
        // Validate path safety: prevent path traversal attacks
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

        let new_virtual = format!(
            "{}/{}/{}",
            virtual_path,
            file_name,
            relative_path.to_string_lossy()
        );

        debug!(
            file = %full_path.display(),
            virtual_path = %new_virtual,
            exists = full_path.exists(),
            "Processing extracted file"
        );

        // 使用 Box::pin 递归处理（支持嵌套压缩包）
        Box::pin(process_path_recursive(
            &full_path, // Use full path for file operations
            &new_virtual,
            target_root,
            map,
            app,
            task_id,
            workspace_id,
        ))
        .await;
    }

    Ok(())
}

// ========== 防御性集成：文件过滤守卫函数 ==========

/// 文件过滤守卫（失败安全：任何错误都返回 true，允许文件通过）
///
/// 防御性设计原则：
/// 1. 配置加载失败 → 返回 true（允许所有文件）
/// 2. 过滤逻辑异常 → 返回 true（允许当前文件）
/// 3. 记录详细日志 → 便于问题排查
async fn should_import_file_defensive(
    path: &Path,
    app: &AppHandle,
) -> bool {
    // Step 1: 安全加载配置（失败时返回 true）
    let filter_config = match load_file_filter_config_safe(app).await {
        Ok(config) => config,
        Err(e) => {
            tracing::warn!(
                error = %e,
                "Failed to load file filter config, allowing all files (fail-safe)"
            );
            return true;  // 失败安全：允许所有文件
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
            true  // 失败安全：允许当前文件
        }
    }
}

/// 安全加载配置（失败时返回默认配置）
async fn load_file_filter_config_safe(
    app: &AppHandle,
) -> Result<FileFilterConfig> {
    match crate::commands::config::load_config(app.clone()) {
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

// ========== 防御性集成结束 ==========
