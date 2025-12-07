//! TAR / TAR.GZ 压缩文件处理器
//!
//! 支持格式：
//! - .tar (未压缩的 TAR 归档)
//! - .tar.gz / .tgz (Gzip 压缩的 TAR 归档)

use crate::archive::context::ArchiveContext;
use crate::archive::processor::process_path_recursive;
use crate::utils::path::{remove_readonly, safe_path_join};
use std::fs;
use std::path::Path;
use uuid::Uuid;

/// 处理 TAR 归档文件（泛型，支持压缩和未压缩）
///
/// # Arguments
///
/// * `archive` - TAR 归档读取器（可以是原始 TAR 或 Gzip 解码后的 TAR）
/// * `_path` - 归档文件路径（未使用，保留用于日志）
/// * `file_name` - 归档文件名
/// * `ctx` - 处理上下文（包含目标目录、映射表等）
///
/// # Returns
///
/// - `Ok(())`: 全部或部分提取成功
/// - `Err(String)`: 全部提取失败
///
/// # 错误容忍
///
/// - 单个条目提取失败不会中断整体流程
/// - 只要有部分文件成功提取，就返回 Ok
/// - 失败的条目会记录警告日志
///
/// # 安全性
///
/// - **路径穿越保护**：拒绝包含 ".." 的路径
/// - **Windows 兼容**：自动移除只读属性、规范化路径分隔符
pub fn process_tar_archive<R: std::io::Read>(
    archive: &mut tar::Archive<R>,
    _path: &Path,
    file_name: &str,
    ctx: &mut ArchiveContext,
) -> Result<(), String> {
    let extract_folder_name = format!("{}_extracted_{}", file_name, Uuid::new_v4());
    let extract_path = ctx.target_root.join(&extract_folder_name);
    fs::create_dir_all(&extract_path)
        .map_err(|e| format!("Failed to create extract dir: {}", e))?;

    let entries = archive
        .entries()
        .map_err(|e| format!("Failed to read tar entries: {}", e))?;

    let mut success_count = 0;
    let mut error_count = 0;
    let mut errors = Vec::new();

    for mut file in entries.flatten() {
        let entry_result = (|| -> Result<(), String> {
            let entry_path = file
                .path()
                .map_err(|e| format!("Invalid tar entry path: {}", e))?;
            let entry_name = entry_path.to_string_lossy().to_string();

            // 安全检查：阻止路径穿越
            if entry_name.contains("..") {
                return Err(format!("Unsafe path detected: {}", entry_name));
            }

            // Windows 兼容：处理路径分隔符
            let normalized_name = entry_name.replace('\\', "/");
            let out_path = safe_path_join(&extract_path, &normalized_name);

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

            file.unpack(&out_path)
                .map_err(|e| format!("Failed to unpack {}: {}", normalized_name, e))?;

            let new_virtual = format!("{}/{}", ctx.virtual_path, normalized_name);
            // 递归处理解压后的文件
            process_path_recursive(
                &out_path,
                &new_virtual,
                ctx.target_root,
                ctx.map,
                ctx.app,
                ctx.task_id,
                "", // workspace_id会在递归处理中自动生成
            );

            Ok(())
        })();

        match entry_result {
            Ok(_) => success_count += 1,
            Err(e) => {
                error_count += 1;
                eprintln!("[WARNING] TAR entry extraction failed: {}", e);
                errors.push(e);
                // 继续处理其他文件，不中断整体流程
            }
        }
    }

    eprintln!(
        "[INFO] TAR extraction complete: {} succeeded, {} failed",
        success_count, error_count
    );

    // 如果有部分文件成功，则返回Ok，但在消息中标注部分失败
    if success_count > 0 {
        if error_count > 0 {
            eprintln!(
                "[WARN] TAR archive partially extracted ({} errors)",
                error_count
            );
        }
        Ok(())
    } else if error_count > 0 {
        Err(format!(
            "All TAR entries failed to extract. First error: {}",
            errors.first().unwrap_or(&"Unknown error".to_string())
        ))
    } else {
        Ok(())
    }
}
