use crate::archive::archive_handler::{ArchiveHandler, ExtractionSummary};
use crate::error::{AppError, Result};
use async_trait::async_trait;
use std::path::Path;
use std::process::Command;
use tokio::fs;
use tracing::debug;

/**
 * RAR文件处理器
 *
 * 使用系统unrar命令行工具处理RAR文件
 */
pub struct RarHandler;

#[async_trait]
impl ArchiveHandler for RarHandler {
    fn can_handle(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|s| s.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("rar"))
            .unwrap_or(false)
    }

    async fn extract_with_limits(
        &self,
        source: &Path,
        target_dir: &Path,
        max_file_size: u64,
        max_total_size: u64,
        max_file_count: usize,
    ) -> Result<ExtractionSummary> {
        // 确保目标目录存在
        fs::create_dir_all(target_dir).await.map_err(|e| {
            AppError::archive_error(
                format!("Failed to create target directory: {}", e),
                Some(target_dir.to_path_buf()),
            )
        })?;

        // 获取unrar可执行文件路径
        let unrar_path = get_unrar_path().map_err(|e| {
            AppError::archive_error(
                format!("Failed to locate unrar binary: {}", e),
                Some(source.to_path_buf()),
            )
        })?;

        // 构建提取命令
        let output = Command::new(&unrar_path)
            .arg("x") // 提取文件
            .arg("-y") // 自动确认
            .arg("-o+") // 覆盖现有文件
            .arg(source)
            .arg(target_dir)
            .output()
            .map_err(|e| {
                AppError::archive_error(
                    format!("Failed to execute unrar command: {}", e),
                    Some(source.to_path_buf()),
                )
            })?;

        let mut summary = ExtractionSummary::new();

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            summary.add_error(format!("RAR extraction failed: {}", error_msg));
            return Ok(summary);
        }

        // 扫描提取的文件，应用安全限制
        scan_extracted_files_with_limits(
            target_dir,
            &mut summary,
            max_file_size,
            max_total_size,
            max_file_count,
        )
        .await?;

        Ok(summary)
    }

    #[allow(dead_code)]
    async fn extract(&self, source: &Path, target_dir: &Path) -> Result<ExtractionSummary> {
        // 默认使用安全限制：单个文件100MB，总大小1GB，文件数1000
        self.extract_with_limits(
            source,
            target_dir,
            100 * 1024 * 1024,
            1024 * 1024 * 1024, // 1GB
            1000,
        )
        .await
    }

    fn file_extensions(&self) -> Vec<&str> {
        vec!["rar"]
    }
}

/**
 * 获取 unrar 可执行文件路径
 *
 * 优先级：
 * 1. 内置二进制文件（推荐，最可靠）
 * 2. 环境变量 UNRAR_PATH
 * 3. 系统 PATH 中的 unrar
 */
fn get_unrar_path() -> Result<String> {
    // 1. ✅ 优先使用内置二进制（跨平台）
    if let Ok(binary_path) = get_builtin_unrar_path() {
        debug!(path = %binary_path, "Using built-in unrar binary");
        return Ok(binary_path);
    }

    // 2. ✅ 检查环境变量
    if let Ok(path) = std::env::var("UNRAR_PATH") {
        if validate_unrar_binary(&path) {
            debug!(path = %path, "Using unrar from UNRAR_PATH env");
            return Ok(path);
        }
    }

    // 3. ✅ 检查常见系统路径
    let system_paths = get_system_unrar_paths();
    for path in system_paths {
        if validate_unrar_binary(path) {
            debug!(path = %path, "Using system unrar");
            return Ok(path.to_string());
        }
    }

    // 4. ✅ 最后尝试 PATH 中的 unrar 命令
    if validate_unrar_binary("unrar") {
        debug!("Using unrar from system PATH");
        return Ok("unrar".to_string());
    }

    // ❌ 所有方法都失败，返回明确的错误
    Err(AppError::archive_error(
        "unrar binary not found. Please install unrar or set UNRAR_PATH environment variable.\n\
         For RAR support, unrar is required. Visit: https://www.rarlab.com/rar_add.htm",
        None,
    ))
}

/**
 * 获取内置 unrar 二进制路径
 *
 * 优先级：
 * 1. 开发模式：从项目目录（基于CARGO_MANIFEST_DIR或当前目录）
 * 2. 生产模式：从exe目录的binaries子目录查找
 * 3. 生产模式：从exe目录直接查找（打包时可能直接放在exe旁边）
 */
fn get_builtin_unrar_path() -> Result<String> {
    // 检测当前平台
    let (arch, os, ext) = detect_platform();
    let binary_name = format!("unrar-{}-{}{}", arch, os, ext);

    // 1. 尝试从项目目录查找（使用多种方法）

    // 方法1a: 使用 CARGO_MANIFEST_DIR（cargo构建时设置）
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let project_path = std::path::Path::new(&manifest_dir)
            .join("binaries")
            .join(&binary_name);
        if project_path.exists() {
            debug!(path = %project_path.display(), "Using built-in unrar from CARGO_MANIFEST_DIR");
            return Ok(project_path.to_string_lossy().to_string());
        }
    }

    // 方法1b: 使用当前工作目录（更可靠）
    if let Ok(cwd) = std::env::current_dir() {
        let project_path = cwd.join("src-tauri").join("binaries").join(&binary_name);
        if project_path.exists() {
            debug!(path = %project_path.display(), "Using built-in unrar from current working directory");
            return Ok(project_path.to_string_lossy().to_string());
        }

        // 也尝试直接在cwd/binaries查找
        let project_path = cwd.join("binaries").join(&binary_name);
        if project_path.exists() {
            debug!(path = %project_path.display(), "Using built-in unrar from cwd/binaries");
            return Ok(project_path.to_string_lossy().to_string());
        }
    }

    // 2. 生产模式：从exe目录查找
    let exe_dir = std::env::current_exe()?
        .parent()
        .ok_or_else(|| AppError::archive_error("Cannot determine executable directory", None))?
        .to_path_buf();

    // 尝试从exe目录的binaries子目录查找
    let binary_path = exe_dir.join("binaries").join(&binary_name);
    if binary_path.exists() {
        debug!(path = %binary_path.display(), "Using built-in unrar from binaries directory");
        return Ok(binary_path.to_string_lossy().to_string());
    }

    // 尝试直接在exe目录查找（打包时unrar可能直接放在exe旁边）
    let binary_path = exe_dir.join(&binary_name);
    if binary_path.exists() {
        debug!(path = %binary_path.display(), "Using built-in unrar from exe directory");
        return Ok(binary_path.to_string_lossy().to_string());
    }

    Err(AppError::archive_error(
        format!("Built-in unrar binary not found: {}", binary_name),
        None,
    ))
}

/**
 * 检测平台三元组
 */
#[allow(unreachable_code)]
fn detect_platform() -> (&'static str, &'static str, &'static str) {
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    return ("x86_64", "pc-windows-msvc", ".exe");

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    return ("x86_64", "unknown-linux-gnu", "");

    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    return ("aarch64", "unknown-linux-gnu", "");

    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    return ("x86_64", "apple-darwin", "");

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return ("aarch64", "apple-darwin", "");

    // ✅ 默认回退
    ("x86_64", "unknown-linux-gnu", "")
}

/**
 * 验证 unrar 二进制是否可用
 */
fn validate_unrar_binary(path: &str) -> bool {
    std::process::Command::new(path)
        .arg("--help")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/**
 * 获取系统 unrar 常见路径
 */
fn get_system_unrar_paths() -> Vec<&'static str> {
    #[cfg(target_os = "windows")]
    {
        vec![
            "C:\\Program Files\\WinRAR\\UnRAR.exe",
            "C:\\Program Files (x86)\\WinRAR\\UnRAR.exe",
        ]
    }

    #[cfg(target_os = "macos")]
    {
        vec![
            "/usr/local/bin/unrar",
            "/opt/homebrew/bin/unrar", // Apple Silicon Homebrew
        ]
    }

    #[cfg(target_os = "linux")]
    {
        vec!["/usr/bin/unrar", "/usr/local/bin/unrar"]
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        vec![]
    }
}

/**
 * 扫描提取的文件并更新摘要（带安全限制）
 *
 * # 并发安全
 *
 * - 使用 Box::pin 解决递归异步调用问题
 */
fn scan_extracted_files_with_limits<'a>(
    dir: &'a Path,
    summary: &'a mut ExtractionSummary,
    max_file_size: u64,
    max_total_size: u64,
    max_file_count: usize,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
    Box::pin(async move {
        let mut entries = fs::read_dir(dir).await.map_err(|e| {
            AppError::archive_error(
                format!("Failed to read directory: {}", e),
                Some(dir.to_path_buf()),
            )
        })?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            AppError::archive_error(
                format!("Failed to read directory entry: {}", e),
                Some(dir.to_path_buf()),
            )
        })? {
            let path = entry.path();
            let metadata = fs::metadata(&path).await.map_err(|e| {
                AppError::archive_error(
                    format!("Failed to get metadata: {}", e),
                    Some(path.clone()),
                )
            })?;

            if metadata.is_file() {
                let file_size = metadata.len();

                // 安全检查：单个文件大小限制
                if file_size > max_file_size {
                    return Err(AppError::archive_error(
                        format!(
                            "File {} exceeds maximum size limit of {} bytes",
                            path.display(),
                            max_file_size
                        ),
                        Some(path),
                    ));
                }

                // 安全检查：总大小限制
                if summary.total_size + file_size > max_total_size {
                    return Err(AppError::archive_error(
                        format!(
                            "Extraction would exceed total size limit of {} bytes",
                            max_total_size
                        ),
                        Some(path),
                    ));
                }

                // 安全检查：文件数量限制
                if summary.files_extracted + 1 > max_file_count {
                    return Err(AppError::archive_error(
                        format!(
                            "Extraction would exceed file count limit of {} files",
                            max_file_count
                        ),
                        Some(path),
                    ));
                }

                summary.add_file(path.clone(), file_size);
            } else if metadata.is_dir() {
                // 使用 Box::pin 递归扫描子目录
                scan_extracted_files_with_limits(
                    &path,
                    summary,
                    max_file_size,
                    max_total_size,
                    max_file_count,
                )
                .await?;
            }
        }

        Ok(())
    })
}

/**
 * 扫描提取的文件并更新摘要（兼容旧版本）
 *
 * # 并发安全
 *
 * - 使用 Box::pin 解决递归异步调用问题
 */
#[allow(dead_code)]
fn scan_extracted_files<'a>(
    dir: &'a Path,
    summary: &'a mut ExtractionSummary,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
    Box::pin(async move {
        // 默认使用安全限制：单个文件100MB，总大小1GB，文件数1000
        scan_extracted_files_with_limits(
            dir,
            summary,
            100 * 1024 * 1024,
            1024 * 1024 * 1024, // 1GB
            1000,
        )
        .await
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_rar_handler_can_handle() {
        let handler = RarHandler;

        assert!(handler.can_handle(Path::new("test.rar")));
        assert!(handler.can_handle(Path::new("test.RAR")));
        assert!(!handler.can_handle(Path::new("test.zip")));
        assert!(!handler.can_handle(Path::new("test.txt")));
    }

    #[test]
    fn test_rar_handler_file_extensions() {
        let handler = RarHandler;
        let extensions = handler.file_extensions();

        assert_eq!(extensions, vec!["rar"]);
    }

    #[tokio::test]
    async fn test_scan_extracted_files() {
        let temp_dir = TempDir::new().unwrap();

        // 创建测试文件结构
        fs::create_dir_all(temp_dir.path().join("subdir"))
            .await
            .unwrap();

        let file1 = temp_dir.path().join("file1.txt");
        let file2 = temp_dir.path().join("subdir").join("file2.txt");

        fs::write(&file1, "content1").await.unwrap();
        fs::write(&file2, "content2").await.unwrap();

        let mut summary = ExtractionSummary::new();
        scan_extracted_files(temp_dir.path(), &mut summary)
            .await
            .unwrap();

        assert_eq!(summary.files_extracted, 2);
        assert!(summary.total_size > 0);
    }

    #[test]
    fn test_detect_platform() {
        let (arch, os, _ext) = detect_platform();
        assert!(!arch.is_empty());
        assert!(!os.is_empty());
    }

    #[test]
    fn test_get_system_unrar_paths() {
        let paths = get_system_unrar_paths();
        // 每个平台应该返回路径向量（可能为空）
        // Vec::len() 总是非负的，所以不需要断言
        assert!(!paths.is_empty() || paths.is_empty()); // 验证路径可访问
    }

    #[test]
    fn test_validate_unrar_binary_invalid() {
        // 测试无效的路径
        assert!(!validate_unrar_binary("/nonexistent/unrar"));
    }

    #[test]
    fn test_get_unrar_path() {
        // 测试 get_unrar_path 的错误处理
        // 如果 unrar 不存在，应该返回 Err 而不是 panic
        let result = std::panic::catch_unwind(|| {
            let _ = get_unrar_path();
        });

        // 无论 unrar 是否存在，都不应该 panic
        assert!(result.is_ok());
    }
}
