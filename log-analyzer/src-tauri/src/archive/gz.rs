//! GZ 单文件压缩处理器
//!
//! 处理 .gz 文件（不包括 .tar.gz，那个由 tar 模块处理）

use crate::archive::processor::process_path_recursive;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use tauri::AppHandle;
use uuid::Uuid;

/// 处理 GZ 压缩文件
///
/// # Arguments
///
/// * `path` - GZ 文件路径
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
/// # 行为
///
/// 1. 解压 GZ 文件到临时目录
/// 2. 去除虚拟路径的 `.gz` 后缀
/// 3. 递归处理解压后的文件（可能是 TAR 或其他压缩格式）
///
/// # 示例
///
/// - `file.log.gz` → 解压后作为 `file.log` 处理
/// - `archive.tar.gz` → 通过 tar 模块处理（不会调用此函数）
#[allow(dead_code)] // Will be used when lib.rs is fully migrated
pub fn process_gz_file(
    path: &Path,
    file_name: &str,
    virtual_path: &str,
    target_root: &Path,
    map: &mut HashMap<String, String>,
    app: &AppHandle,
    task_id: &str,
) -> Result<(), String> {
    let file = File::open(path).map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);
    let mut gz = flate2::read::GzDecoder::new(reader);

    let base_name = file_name.trim_end_matches(".gz");
    let unique_name = format!(
        "{}_{}",
        base_name,
        Uuid::new_v4()
            .to_string()
            .split('-')
            .next()
            .unwrap_or("tmp")
    );
    let out_path = target_root.join(&unique_name);

    let mut out_file = File::create(&out_path).map_err(|e| e.to_string())?;
    std::io::copy(&mut gz, &mut out_file).map_err(|e| e.to_string())?;

    let decompressed_virtual = if virtual_path.ends_with(".gz") {
        virtual_path.trim_end_matches(".gz").to_string()
    } else {
        virtual_path.to_string()
    };

    // 关键：递归处理（解压后可能是 tar 或其他压缩格式）
    process_path_recursive(
        &out_path,
        &decompressed_virtual,
        target_root,
        map,
        app,
        task_id,
    );
    Ok(())
}
