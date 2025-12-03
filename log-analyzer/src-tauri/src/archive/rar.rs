//! RAR 压缩文件处理器
//!
//! 使用内置 unrar 二进制文件进行解压

use crate::archive::processor::process_path_recursive;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;
use tauri::{AppHandle, Manager};
use uuid::Uuid;
use walkdir::WalkDir;

/// 获取内置unrar可执行文件的路径
///
/// # 功能
///
/// 返回打包在应用中的unrar二进制文件路径（Tauri sidecar）
///
/// # 参数
///
/// - `app`: Tauri AppHandle引用
///
/// # 返回值
///
/// - `Ok(PathBuf)`: unrar可执行文件的完整路径
/// - `Err(String)`: 获取路径失败
fn get_bundled_unrar_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    // 获取应用资源目录
    let resource_path = app
        .path()
        .resource_dir()
        .map_err(|e| format!("Failed to get resource directory: {}", e))?;

    // 根据平台确定二进制文件名
    let binary_name = if cfg!(target_os = "windows") {
        "unrar-x86_64-pc-windows-msvc.exe"
    } else if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") {
            "unrar-aarch64-apple-darwin"
        } else {
            "unrar-x86_64-apple-darwin"
        }
    } else {
        "unrar-x86_64-unknown-linux-gnu"
    };

    // 尝试在资源目录中查找
    let sidecar_path = resource_path.join("binaries").join(binary_name);
    if sidecar_path.exists() {
        eprintln!("[DEBUG] Found bundled unrar at: {}", sidecar_path.display());
        return Ok(sidecar_path);
    }

    // 开发模式：尝试在 src-tauri/binaries 目录查找
    let exe_path =
        std::env::current_exe().map_err(|e| format!("Failed to get current exe path: {}", e))?;
    let exe_dir = exe_path.parent().ok_or("Failed to get exe directory")?;

    // 开发模式路径：target/debug/binaries 或直接在 src-tauri/binaries
    let dev_paths = [
        exe_dir.join("binaries").join(binary_name),
        exe_dir.join("../binaries").join(binary_name),
        exe_dir.join("../../binaries").join(binary_name),
        exe_dir
            .join("../../../src-tauri/binaries")
            .join(binary_name),
    ];

    for path in &dev_paths {
        if path.exists() {
            let canonical = dunce::canonicalize(path)
                .map_err(|e| format!("Failed to canonicalize path: {}", e))?;
            eprintln!(
                "[DEBUG] Found bundled unrar (dev mode) at: {}",
                canonical.display()
            );
            return Ok(canonical);
        }
    }

    Err(format!(
        "Bundled unrar not found. Checked paths:\n  - {}\n  - {:?}",
        sidecar_path.display(),
        dev_paths
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
    ))
}

/// 处理 RAR 归档文件（使用内置 unrar 二进制文件）
///
/// # Arguments
///
/// * `path` - RAR 文件路径
/// * `file_name` - 文件名
/// * `virtual_path` - 虚拟路径
/// * `target_root` - 解压目标根目录
/// * `map` - 路径映射表
/// * `app` - Tauri 应用句柄
/// * `task_id` - 任务 ID
///
/// # Returns
///
/// - `Ok(())`: 解压成功
/// - `Err(String)`: 解压失败
///
/// # 依赖
///
/// - 需要打包内置的 unrar 二进制文件
/// - 支持 Windows、macOS、Linux 平台
///
/// # 命令参数
///
/// - `x`: 解压并保持路径结构
/// - `-o+`: 覆盖已存在文件
/// - `-y`: 自动确认所有提示
pub fn process_rar_archive(
    path: &Path,
    file_name: &str,
    virtual_path: &str,
    target_root: &Path,
    map: &mut HashMap<String, String>,
    app: &AppHandle,
    task_id: &str,
) -> Result<(), String> {
    // 获取内置的 unrar 路径
    let unrar_path = get_bundled_unrar_path(app)?;

    let extract_folder_name = format!("{}_extracted_{}", file_name, Uuid::new_v4());
    let extract_path = target_root.join(&extract_folder_name);
    fs::create_dir_all(&extract_path)
        .map_err(|e| format!("Failed to create extract dir: {}", e))?;

    eprintln!(
        "[DEBUG] Extracting RAR: {} -> {}",
        path.display(),
        extract_path.display()
    );
    eprintln!("[DEBUG] Using bundled unrar: {}", unrar_path.display());

    // 执行 unrar 命令
    let output = Command::new(&unrar_path)
        .arg("x") // 解压并保持路径
        .arg("-o+") // 覆盖已存在文件
        .arg("-y") // 自动确认
        .arg(path) // 源文件
        .arg(&extract_path) // 目标目录
        .output()
        .map_err(|e| format!("Failed to execute unrar: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("unrar failed: {}", stderr));
    }

    eprintln!(
        "[DEBUG] RAR extracted successfully: {}",
        extract_path.display()
    );

    // 递归处理解压后的内容
    for entry in WalkDir::new(&extract_path)
        .min_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let entry_path = entry.path();
        let relative = entry_path
            .strip_prefix(&extract_path)
            .map_err(|e| format!("Path strip failed: {}", e))?;
        let relative_str = relative.to_string_lossy().replace('\\', "/");
        let new_virtual = format!("{}/{}", virtual_path, relative_str);

        if entry_path.is_file() {
            process_path_recursive(entry_path, &new_virtual, target_root, map, app, task_id);
        }
    }

    Ok(())
}
