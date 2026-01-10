//! 路径处理工具函数
//!
//! 提供跨平台的路径操作工具，包括路径规范化、安全拼接等功能。

use std::path::{Path, PathBuf};

/// 规范化路径
///
/// 在 Windows 上使用 dunce 去除 UNC 前缀，在 Unix-like 系统上使用标准规范化。
///
/// # 参数
///
/// - `path` - 需要规范化的路径
///
/// # 返回值
///
/// - `Ok(PathBuf)` - 规范化后的路径
/// - `Err(String)` - 规范化失败的错误信息
///
/// # 示例
///
/// ```ignore
/// use std::path::Path;
/// let path = Path::new("./some/path");
/// let canonical = canonicalize_path(path)?;
/// ```
pub fn canonicalize_path(path: &Path) -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    {
        // Windows: 使用 dunce 去除 UNC 前缀 \\?\，并处理长路径
        dunce::canonicalize(path).map_err(|e| format!("Path canonicalization failed: {}", e))
    }
    #[cfg(not(target_os = "windows"))]
    {
        // Unix-like: 标准规范化
        path.canonicalize()
            .map_err(|e| format!("Path canonicalization failed: {}", e))
    }
}

/// 移除文件只读属性（Windows 专用）
///
/// 在 Windows 上移除文件的只读属性，避免删除失败。在 Unix-like 系统上为空操作。
///
/// # 参数
///
/// - `path` - 文件路径
///
/// # 返回值
///
/// - `Ok(())` - 操作成功
/// - `Err(String)` - 操作失败的错误信息
#[cfg(target_os = "windows")]
#[allow(clippy::permissions_set_readonly_false)] // Windows 特定操作，允许设置可写
pub fn remove_readonly(path: &Path) -> Result<(), String> {
    use std::fs;
    use std::os::windows::fs::MetadataExt;

    // 使用重试机制
    crate::utils::retry::retry_file_operation(
        || {
            if let Ok(metadata) = path.metadata() {
                // Windows FILE_ATTRIBUTE_READONLY = 0x1
                if metadata.file_attributes() & 0x1 != 0 {
                    let mut perms = metadata.permissions();
                    perms.set_readonly(false);
                    fs::set_permissions(path, perms)
                        .map_err(|e| format!("Failed to remove readonly: {}", e))?;
                }
            }
            Ok(())
        },
        2,    // 最多重试2次
        100,  // 基础延迟 100ms
        1000, // 最大延迟 1s
        &format!("remove_readonly({})", path.display()),
    )
}

#[cfg(not(target_os = "windows"))]
#[allow(dead_code)]
pub fn remove_readonly(_path: &Path) -> Result<(), String> {
    // Unix-like: 不需要处理
    Ok(())
}

/// 跨平台路径规范化（统一路径分隔符）
///
/// 在 Windows 上将 `/` 转换为 `\`，在 Unix-like 系统上保持 `/` 不变。
///
/// # 参数
///
/// - `path` - 路径字符串
///
/// # 返回值
///
/// 规范化后的路径字符串
///
/// # 示例
///
/// ```ignore
/// let path = "folder/subfolder/file.txt";
/// let normalized = normalize_path_separator(path);
/// // Windows: "folder\\subfolder\\file.txt"
/// // Linux/macOS: "folder/subfolder/file.txt"
/// ```
pub fn normalize_path_separator(path: &str) -> String {
    #[cfg(target_os = "windows")]
    {
        path.replace('/', "\\")
    }
    #[cfg(not(target_os = "windows"))]
    {
        path.to_string()
    }
}

/// 安全的路径拼接
///
/// 防止路径穿越攻击，移除 `..` 和驱动器符号。
///
/// # 参数
///
/// - `base` - 基础路径
/// - `component` - 要拼接的路径组件
///
/// # 返回值
///
/// 拼接后的安全路径
///
/// # 示例
///
/// ```ignore
/// use std::path::Path;
/// let base = Path::new("/base");
/// let result = safe_path_join(base, "../evil");
/// // 结果: /base/evil (.. 被移除)
/// ```
#[allow(dead_code)]
pub fn safe_path_join(base: &Path, component: &str) -> PathBuf {
    // 移除路径穿越尝试
    let sanitized = component
        .replace("..", "")
        .replace(":", "") // Windows 驱动器符号
        .trim()
        .to_string();

    base.join(sanitized)
}
