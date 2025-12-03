//! ZIP 压缩文件处理器
//!
//! 支持 Windows 编码优化（GBK/GB2312 文件名）

use crate::archive::processor::process_path_recursive;
use crate::utils::encoding::decode_filename;
use crate::utils::path::{remove_readonly, safe_path_join};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::Path;
use tauri::AppHandle;
use uuid::Uuid;

/// 处理 ZIP 归档文件（Windows 编码优化）
///
/// # Arguments
///
/// * `path` - ZIP 文件路径
/// * `file_name` - 文件名
/// * `virtual_path` - 虚拟路径
/// * `target_root` - 解压目标根目录
/// * `map` - 路径映射表
/// * `app` - Tauri 应用句柄
/// * `task_id` - 任务 ID
///
/// # Returns
///
/// - `Ok(())`: 全部或部分提取成功
/// - `Err(String)`: 全部提取失败
///
/// # 特性
///
/// - **多编码支持**：自动检测并解码 UTF-8/GBK/GB2312 文件名
/// - **错误容忍**：单个条目失败不中断整体流程
/// - **安全检查**：防止路径穿越攻击
/// - **Windows 兼容**：移除只读属性、规范化路径分隔符
pub fn process_zip_archive(
    path: &Path,
    file_name: &str,
    virtual_path: &str,
    target_root: &Path,
    map: &mut HashMap<String, String>,
    app: &AppHandle,
    task_id: &str,
) -> Result<(), String> {
    let file = File::open(path).map_err(|e| format!("Failed to open zip: {}", e))?;
    let reader = BufReader::new(file);
    let mut archive =
        zip::ZipArchive::new(reader).map_err(|e| format!("Invalid zip archive: {}", e))?;

    let extract_folder_name = format!("{}_extracted_{}", file_name, Uuid::new_v4());
    let extract_path = target_root.join(&extract_folder_name);
    fs::create_dir_all(&extract_path)
        .map_err(|e| format!("Failed to create extract dir: {}", e))?;

    let mut success_count = 0;
    let mut error_count = 0;
    let mut errors = Vec::new();

    for i in 0..archive.len() {
        let entry_result = (|| -> Result<(), String> {
            let mut file = archive
                .by_index(i)
                .map_err(|e| format!("Failed to read zip entry {}: {}", i, e))?;
            let name_raw = file.name_raw().to_vec();
            let name = decode_filename(&name_raw);

            // 安全检查
            if name.contains("..") {
                return Err(format!("Unsafe zip entry: {}", name));
            }

            // Windows 兼容：路径分隔符规范化
            let normalized_name = name.replace('\\', "/");
            let out_path = safe_path_join(&extract_path, &normalized_name);

            if file.is_dir() {
                fs::create_dir_all(&out_path).map_err(|e| {
                    format!("Failed to create directory {}: {}", normalized_name, e)
                })?;
            } else {
                if let Some(p) = out_path.parent() {
                    fs::create_dir_all(p).map_err(|e| {
                        format!("Failed to create parent dir for {}: {}", normalized_name, e)
                    })?;
                }

                // Windows 兼容：在解压前移除只读属性
                if out_path.exists() {
                    remove_readonly(&out_path).map_err(|e| {
                        format!("Failed to remove readonly for {}: {}", normalized_name, e)
                    })?;
                }

                let mut outfile = File::create(&out_path)
                    .map_err(|e| format!("Failed to create file {}: {}", normalized_name, e))?;
                std::io::copy(&mut file, &mut outfile)
                    .map_err(|e| format!("Failed to extract {}: {}", normalized_name, e))?;

                let new_virtual = format!("{}/{}", virtual_path, normalized_name);
                // 递归处理
                process_path_recursive(&out_path, &new_virtual, target_root, map, app, task_id);
            }

            Ok(())
        })();

        match entry_result {
            Ok(_) => success_count += 1,
            Err(e) => {
                error_count += 1;
                eprintln!("[WARNING] ZIP entry extraction failed: {}", e);
                errors.push(e);
                // 继续处理其他文件
            }
        }
    }

    eprintln!(
        "[INFO] ZIP extraction complete: {} succeeded, {} failed",
        success_count, error_count
    );

    // 如果有部分文件成功，则返回Ok
    if success_count > 0 {
        if error_count > 0 {
            eprintln!(
                "[WARN] ZIP archive partially extracted ({} errors)",
                error_count
            );
        }
        Ok(())
    } else if error_count > 0 {
        Err(format!(
            "All ZIP entries failed to extract. First error: {}",
            errors.first().unwrap_or(&"Unknown error".to_string())
        ))
    } else {
        Err("Empty ZIP archive".to_string())
    }
}
