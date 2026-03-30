//! 路径处理工具函数
//!
//! 提供跨平台的路径操作工具，包括路径规范化、安全拼接等功能。

/// 规范化路径分隔符
///
/// Windows 上将 `/` 替换为 `\`，其他平台保持原样。
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
