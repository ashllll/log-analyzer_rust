//! 递归路径处理器
//!
//! 核心递归逻辑，负责：
//! - 识别文件类型（压缩文件 vs 普通文件）
//! - 调用统一的 ArchiveManager 处理器或增强提取系统
//! - 支持异步递归解压嵌套压缩包
//! - 收集元数据（增量索引）
//! - 错误处理和进度报告

use crate::archive::extraction_engine::ExtractionPolicy;
use crate::archive::public_api::extract_archive_async;
use crate::archive::ArchiveManager;
use crate::error::{AppError, Result};
use crate::models::config::FileMetadata;
use crate::models::log_entry::TaskProgress;
use crate::services::file_watcher::get_file_metadata;
use crate::utils::path::normalize_path_separator;
use std::collections::HashMap;
use std::path::{Component, Path};
use tauri::{AppHandle, Emitter};
use tokio::fs;
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

    // 然后检查配置文件
    // TODO: 从配置文件加载 ExtractionPolicy 并检查 use_enhanced_extraction 标志
    // 目前默认为 false
    false
}

/// 检查路径是否安全（防止路径遍历攻击）
///
/// # 参数
///
/// - `path` - 要检查的路径
/// - `base_dir` - 基础目录，用于验证路径是否在允许范围内
///
/// # 返回
///
/// - `Ok(())` - 路径安全
/// - `Err(AppError)` - 路径不安全
fn validate_path_safety(path: &Path, base_dir: &Path) -> Result<()> {
    // 规范化路径
    let canonical_path = path.canonicalize().map_err(|e| {
        AppError::validation_error(format!(
            "Failed to canonicalize path {}: {}",
            path.display(),
            e
        ))
    })?;

    let canonical_base = base_dir.canonicalize().map_err(|e| {
        AppError::validation_error(format!(
            "Failed to canonicalize base dir {}: {}",
            base_dir.display(),
            e
        ))
    })?;

    // 验证路径是否在基础目录内
    if !canonical_path.starts_with(&canonical_base) {
        return Err(AppError::validation_error(format!(
            "Path traversal detected: {} is outside of {}",
            path.display(),
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
        eprintln!("[WARNING] Failed to process {}: {}", path.display(), e);
        let _ = app.emit(
            "task-update",
            TaskProgress {
                task_id: task_id.to_string(),
                task_type: "Import".to_string(),
                target: "Processing".to_string(),
                status: "RUNNING".to_string(),
                message: format!("Warning: {}", e),
                progress: 50,
                workspace_id: None, // 这是内部进度更新，没有 workspace_id
            },
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
    metadata_map: &mut HashMap<String, FileMetadata>,
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
        let _ = app.emit(
            "task-update",
            TaskProgress {
                task_id: task_id.to_string(),
                task_type: "Import".to_string(),
                target: "Processing".to_string(),
                status: "RUNNING".to_string(),
                message: format!("Warning: {}", e),
                progress: 50,
                workspace_id: None, // 内部进度更新
            },
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

    let _ = app.emit(
        "task-update",
        TaskProgress {
            task_id: task_id.to_string(),
            task_type: "Import".to_string(),
            target: file_name.clone(),
            status: "RUNNING".to_string(),
            message: format!("Processing: {}", file_name),
            progress: 50,
            workspace_id: None,
        },
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
                // 压缩文件解压失败，记录错误但继续处理
                let _ = app.emit(
                    "task-update",
                    TaskProgress {
                        task_id: task_id.to_string(),
                        task_type: "Import".to_string(),
                        target: file_name.clone(),
                        status: "RUNNING".to_string(),
                        message: format!("Warning: Failed to extract {}", file_name),
                        progress: 50,
                        workspace_id: None,
                    },
                );
                return Err(e);
            }
        }
    }

    // --- 普通文件 ---
    let real_path = path.to_string_lossy().to_string();
    let normalized_virtual = normalize_path_separator(virtual_path);

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
    metadata_map: &mut HashMap<String, FileMetadata>,
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

    let _ = app.emit(
        "task-update",
        TaskProgress {
            task_id: task_id.to_string(),
            task_type: "Import".to_string(),
            target: file_name.clone(),
            status: "RUNNING".to_string(),
            message: format!("Processing: {}", file_name),
            progress: 50,
            workspace_id: None,
        },
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

/// 提取压缩文件并递归处理内容
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
    let file_name = archive_path
        .file_name()
        .ok_or_else(|| AppError::validation_error("Invalid archive filename"))?
        .to_string_lossy()
        .to_string();

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

                // 报告警告
                for warning in &result.warnings {
                    eprintln!("[WARNING] {:?}: {}", warning.category, warning.message);
                }

                // 报告安全事件
                for event in &result.security_events {
                    eprintln!("[SECURITY] {:?}: {:?}", event.event_type, event.details);
                }

                // 集成路径映射
                for (short_path, original_path) in &result.metadata_mappings {
                    let short_str = short_path.to_string_lossy().to_string();
                    let original_str = original_path.to_string_lossy().to_string();
                    eprintln!("[DEBUG] Path mapping: {} -> {}", short_str, original_str);
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

    // 递归处理解压后的文件（支持嵌套压缩包）
    for extracted_file in &extracted_files {
        // 验证路径安全：防止路径遍历攻击
        if let Err(e) = validate_path_safety(extracted_file, &extract_dir) {
            eprintln!(
                "[SECURITY] Skipping unsafe file {}: {}",
                extracted_file.display(),
                e
            );
            continue; // 跳过不安全的文件
        }

        let relative_path = extracted_file.strip_prefix(&extract_dir).map_err(|_| {
            AppError::validation_error(format!(
                "Failed to compute relative path for {}",
                extracted_file.display()
            ))
        })?;

        let new_virtual = format!(
            "{}/{}/{}",
            virtual_path,
            file_name,
            relative_path.to_string_lossy()
        );

        // 使用 Box::pin 递归处理（支持嵌套压缩包）
        Box::pin(process_path_recursive(
            extracted_file,
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
