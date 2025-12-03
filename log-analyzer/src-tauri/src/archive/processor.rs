//! 递归路径处理器
//!
//! 核心递归逻辑，负责：
//! - 识别文件类型（压缩文件 vs 普通文件）
//! - 调用相应的处理器
//! - 收集元数据（增量索引）
//! - 错误处理和进度报告

use crate::archive::context::ArchiveContext;
use crate::models::config::FileMetadata;
use crate::models::log_entry::TaskProgress;
use crate::services::file_watcher::get_file_metadata;
use crate::utils::path::normalize_path_separator;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use tauri::{AppHandle, Emitter};
use walkdir::WalkDir;

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
///
/// # 行为
///
/// - 内部调用 `process_path_recursive_inner` 进行实际处理
/// - 捕获错误，记录日志，发送警告事件
/// - 单个文件失败不中断整体流程
pub fn process_path_recursive(
    path: &Path,
    virtual_path: &str,
    target_root: &Path,
    map: &mut HashMap<String, String>,
    app: &AppHandle,
    task_id: &str,
) {
    // 错误处理：如果处理失败，不中断整个流程
    if let Err(e) = process_path_recursive_inner(path, virtual_path, target_root, map, app, task_id)
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
pub fn process_path_recursive_with_metadata(
    path: &Path,
    virtual_path: &str,
    target_root: &Path,
    map: &mut HashMap<String, String>,
    metadata_map: &mut HashMap<String, FileMetadata>,
    app: &AppHandle,
    task_id: &str,
) {
    if let Err(e) = process_path_recursive_inner_with_metadata(
        path,
        virtual_path,
        target_root,
        map,
        metadata_map,
        app,
        task_id,
    ) {
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
/// 2. 如果是文件：
///    - 识别压缩格式（ZIP/RAR/TAR/TAR.GZ/GZ）
///    - 调用相应的处理器
///    - 普通文件：添加到索引
pub fn process_path_recursive_inner(
    path: &Path,
    virtual_path: &str,
    target_root: &Path,
    map: &mut HashMap<String, String>,
    app: &AppHandle,
    task_id: &str,
) -> Result<(), String> {
    // 处理目录
    if path.is_dir() {
        for entry in WalkDir::new(path)
            .min_depth(1)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let entry_name = entry.file_name().to_string_lossy();
            let new_virtual = format!("{}/{}", virtual_path, entry_name);
            process_path_recursive(entry.path(), &new_virtual, target_root, map, app, task_id);
        }
        return Ok(());
    }

    let path_str = path.to_string_lossy();
    let file_name = path
        .file_name()
        .ok_or("Invalid filename")?
        .to_string_lossy();
    let lower_path = path_str.to_lowercase();

    let _ = app.emit(
        "task-update",
        TaskProgress {
            task_id: task_id.to_string(),
            task_type: "Import".to_string(),
            target: file_name.to_string(),
            status: "RUNNING".to_string(),
            message: format!("Processing: {}", file_name),
            progress: 50,
            workspace_id: None, // 文件处理进度
        },
    );

    // 判断文件类型
    let is_zip = lower_path.ends_with(".zip");
    let is_rar = lower_path.ends_with(".rar");
    let is_tar = lower_path.ends_with(".tar");
    let is_tar_gz = lower_path.ends_with(".tar.gz") || lower_path.ends_with(".tgz");
    let is_plain_gz = lower_path.ends_with(".gz") && !is_tar_gz;

    // --- 处理 ZIP ---
    if is_zip {
        return crate::archive::zip::process_zip_archive(
            path,
            &file_name,
            virtual_path,
            target_root,
            map,
            app,
            task_id,
        );
    }

    // --- 处理 RAR ---
    if is_rar {
        return crate::archive::rar::process_rar_archive(
            path,
            &file_name,
            virtual_path,
            target_root,
            map,
            app,
            task_id,
        );
    }

    // --- 处理 TAR / TAR.GZ ---
    if is_tar || is_tar_gz {
        let file = File::open(path).map_err(|e| e.to_string())?;
        let reader = BufReader::new(file);
        let mut ctx = ArchiveContext {
            target_root,
            virtual_path,
            map,
            app,
            task_id,
        };
        if is_tar_gz {
            let tar = flate2::read::GzDecoder::new(reader);
            let mut archive = tar::Archive::new(tar);
            return crate::archive::tar::process_tar_archive(
                &mut archive,
                path,
                &file_name,
                &mut ctx,
            );
        } else {
            let mut archive = tar::Archive::new(reader);
            return crate::archive::tar::process_tar_archive(
                &mut archive,
                path,
                &file_name,
                &mut ctx,
            );
        }
    }

    // --- 处理纯 GZ ---
    if is_plain_gz {
        return crate::archive::gz::process_gz_file(
            path,
            &file_name,
            virtual_path,
            target_root,
            map,
            app,
            task_id,
        );
    }

    // --- 普通文件 ---
    let real_path = path.to_string_lossy().to_string();

    // ✅ 使用 normalize_path_separator 统一路径分隔符
    let normalized_virtual = normalize_path_separator(virtual_path);

    map.insert(real_path, normalized_virtual.clone());
    // 注意：为了性能考虑，不再逐个文件输出日志
    // 日志汇总在处理完成后输出
    Ok(())
}

/// 带元数据收集的内部处理函数
///
/// # 行为
///
/// - 压缩文件：使用原始处理逻辑（不收集元数据）
/// - 普通文件：收集元数据并添加到 `metadata_map`
pub fn process_path_recursive_inner_with_metadata(
    path: &Path,
    virtual_path: &str,
    target_root: &Path,
    map: &mut HashMap<String, String>,
    metadata_map: &mut HashMap<String, FileMetadata>,
    app: &AppHandle,
    task_id: &str,
) -> Result<(), String> {
    // 处理目录
    if path.is_dir() {
        for entry in WalkDir::new(path)
            .min_depth(1)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let entry_name = entry.file_name().to_string_lossy();
            let new_virtual = format!("{}/{}", virtual_path, entry_name);
            process_path_recursive_with_metadata(
                entry.path(),
                &new_virtual,
                target_root,
                map,
                metadata_map,
                app,
                task_id,
            );
        }
        return Ok(());
    }

    let path_str = path.to_string_lossy();
    let file_name = path
        .file_name()
        .ok_or("Invalid filename")?
        .to_string_lossy();
    let lower_path = path_str.to_lowercase();

    let _ = app.emit(
        "task-update",
        TaskProgress {
            task_id: task_id.to_string(),
            task_type: "Import".to_string(),
            target: file_name.to_string(),
            status: "RUNNING".to_string(),
            message: format!("Processing: {}", file_name),
            progress: 50,
            workspace_id: None, // 文件处理进度
        },
    );

    // 判断文件类型
    let is_zip = lower_path.ends_with(".zip");
    let is_rar = lower_path.ends_with(".rar");
    let is_tar = lower_path.ends_with(".tar");
    let is_tar_gz = lower_path.ends_with(".tar.gz") || lower_path.ends_with(".tgz");
    let is_plain_gz = lower_path.ends_with(".gz") && !is_tar_gz;

    // 压缩文件不收集元数据，只处理普通文件
    if is_zip || is_rar || is_tar || is_tar_gz || is_plain_gz {
        // 递归调用原始的处理函数（不收集元数据）
        return process_path_recursive_inner(path, virtual_path, target_root, map, app, task_id);
    }

    // --- 普通文件：收集元数据 ---
    let real_path = path.to_string_lossy().to_string();

    // ✅ 使用 normalize_path_separator 统一路径分隔符
    let normalized_virtual = normalize_path_separator(virtual_path);

    map.insert(real_path.clone(), normalized_virtual.clone());

    // 收集文件元数据
    if let Ok(metadata) = get_file_metadata(path) {
        metadata_map.insert(real_path.clone(), metadata);
        // 注意：为了性能考虑，不再逐个文件输出日志
    }
    // 日志汇总在处理完成后输出

    Ok(())
}
