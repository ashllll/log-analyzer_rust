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

/// 为 Windows 长路径场景添加 UNC 前缀（`\\?\`）
///
/// Windows 默认路径上限为 260 字节（MAX_PATH）。超过该上限时，
/// 系统调用返回 `ERROR_PATH_NOT_FOUND`（错误码 3），导致文件提取失败。
/// 本函数在路径超过 260 字节时自动追加 `\\?\` 前缀，
/// 将有效上限提升至 32,767 个字符。
///
/// 规则：
/// - 非 Windows：原样返回，无任何修改
/// - Windows，路径 ≤ 260 字节：原样返回
/// - Windows，已有 `\\?\` 前缀：原样返回（防止双重前缀）
/// - Windows，路径 > 260 字节：使用 `dunce::simplified` 规范化后追加 `\\?\`
///
/// # 参数
///
/// - `path` - 待处理的路径（通常是提取目标目录 `target_dir`）
///
/// # 返回值
///
/// - Windows 超长路径：`\\?\{simplified_path}`
/// - 其他情况：原始路径
///
/// # 示例
///
/// ```ignore
/// // Windows，路径 > 260 字节时
/// let long = Path::new(r"C:\Users\...\deeply\nested\...\target");
/// let result = apply_unc_prefix_if_needed(long);
/// // result: r"\\?\C:\Users\...\deeply\nested\...\target"
/// ```
pub fn apply_unc_prefix_if_needed(path: &Path) -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        let path_str = path.to_string_lossy();
        // 已含 UNC 前缀或路径不超限，直接返回
        if path_str.starts_with(r"\\?\") || path_str.len() <= 260 {
            return path.to_path_buf();
        }
        // 使用 dunce 规范化（消除多余的 . 和 ..）后追加前缀
        let simplified = dunce::simplified(path);
        PathBuf::from(format!(r"\\?\{}", simplified.display()))
    }
    #[cfg(not(target_os = "windows"))]
    {
        path.to_path_buf()
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
    let mut safe_path = base.to_path_buf();

    // 遍历组件并只添加安全的部分
    for comp in Path::new(component).components() {
        match comp {
            std::path::Component::Normal(c) => {
                safe_path.push(c);
            }
            // 忽略根目录、驱动器符号和父目录引用
            _ => continue,
        }
    }

    safe_path
}

#[cfg(test)]
mod tests {
    use super::*;

    /// apply_unc_prefix_if_needed：非 Windows 平台应原样返回
    #[test]
    #[cfg(not(target_os = "windows"))]
    fn test_unc_prefix_noop_on_non_windows() {
        let path =
            Path::new("/home/user/some/very/long/path/that/exceeds/260/characters/definitely");
        let result = apply_unc_prefix_if_needed(path);
        assert_eq!(result, path.to_path_buf(), "非 Windows 不应修改路径");
    }

    /// apply_unc_prefix_if_needed：Windows 上短路径不应追加前缀
    #[test]
    #[cfg(target_os = "windows")]
    fn test_unc_prefix_short_path_unchanged() {
        let path = Path::new(r"C:\Users\user\logs\target");
        let result = apply_unc_prefix_if_needed(path);
        // 路径 <= 260 字节，不应追加前缀
        let result_str = result.to_string_lossy();
        assert!(
            !result_str.starts_with(r"\\?\"),
            "短路径不应追加 UNC 前缀，实际：{}",
            result_str
        );
    }

    /// apply_unc_prefix_if_needed：Windows 上已有 UNC 前缀不应重复追加
    #[test]
    #[cfg(target_os = "windows")]
    fn test_unc_prefix_no_double_prefix() {
        let path = Path::new(r"\\?\C:\Users\user\logs");
        let result = apply_unc_prefix_if_needed(path);
        let result_str = result.to_string_lossy();
        assert_eq!(
            result_str.matches(r"\\?\").count(),
            1,
            "不应出现双重 UNC 前缀，实际：{}",
            result_str
        );
    }

    /// apply_unc_prefix_if_needed：Windows 上 > 260 字节的路径应追加前缀
    #[test]
    #[cfg(target_os = "windows")]
    fn test_unc_prefix_long_path_gets_prefix() {
        // 构造一个超过 260 字节的路径
        let long_segment = "a".repeat(50);
        let path_str = format!(
            r"C:\Users\user\{}\{}\{}\{}\{}\{}",
            long_segment, long_segment, long_segment, long_segment, long_segment, long_segment
        );
        assert!(path_str.len() > 260, "前置条件：路径应超过 260 字节");

        let path = Path::new(&path_str);
        let result = apply_unc_prefix_if_needed(path);
        let result_str = result.to_string_lossy();

        assert!(
            result_str.starts_with(r"\\?\"),
            "超过 260 字节的路径应追加 UNC 前缀，实际：{}",
            result_str
        );
    }

    /// safe_path_join 防路径穿越基础测试
    #[test]
    fn test_safe_path_join_prevents_traversal() {
        let base = Path::new("/base");
        let result = safe_path_join(base, "../evil");
        assert_eq!(result, Path::new("/base/evil"), ".. 应被过滤");
    }
}
