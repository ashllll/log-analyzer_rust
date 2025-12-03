//! 压缩文件处理上下文
//!
//! 定义压缩文件处理过程中的共享参数，减少函数参数数量

use std::collections::HashMap;
use std::path::Path;
use tauri::AppHandle;

/// 压缩文件处理上下文
///
/// # 字段
///
/// - `target_root`: 临时目录根路径（用于解压文件）
/// - `virtual_path`: 虚拟路径（用于索引）
/// - `map`: 真实路径到虚拟路径的映射表（可变引用）
/// - `app`: Tauri 应用句柄（用于发送进度事件）
/// - `task_id`: 任务 ID（用于进度跟踪）
///
/// # 生命周期
///
/// 'a: 所有引用的生命周期参数，确保上下文不超过引用的有效期
pub struct ArchiveContext<'a> {
    pub target_root: &'a Path,
    pub virtual_path: &'a str,
    pub map: &'a mut HashMap<String, String>,
    pub app: &'a AppHandle,
    pub task_id: &'a str,
}
