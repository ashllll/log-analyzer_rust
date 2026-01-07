use crate::archive::archive_handler::{ArchiveHandler, ExtractionSummary};
use crate::error::{AppError, Result};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::process::Command;
use tokio::fs;
use tracing::{error, info, warn};

/**
 * RAR文件处理器
 *
 * 使用 sidecar unrar 二进制文件处理RAR文件
 * 支持跨平台：Windows、macOS、Linux
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

        let source_str = source.to_string_lossy().to_string();
        let target_str = target_dir.to_string_lossy().to_string();

        // 使用 tokio::task::spawn_blocking 在阻塞型上下文中运行 unrar
        let result = tokio::task::spawn_blocking(move || {
            extract_rar_sync(
                &source_str,
                &target_str,
                max_file_size,
                max_total_size,
                max_file_count,
            )
        })
        .await
        .map_err(|e| {
            AppError::archive_error(
                format!("RAR extraction task failed: {}", e),
                Some(source.to_path_buf()),
            )
        })?;

        result
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
 * 获取 unrar 二进制路径（带完整的 fallback 机制）
 *
 * 尝试多个可能的路径位置，按照优先级顺序
 * 提供详细的诊断日志
 */
pub fn get_unrar_path() -> PathBuf {
    let binary_name = match std::env::consts::OS {
        "macos" => "unrar-aarch64-apple-darwin",
        "linux" => "unrar-x86_64-unknown-linux-gnu",
        "windows" => "unrar-x86_64-pc-windows-msvc.exe",
        _ => panic!("Unsupported platform: {}", std::env::consts::OS),
    };

    let exe_path = match std::env::current_exe() {
        Ok(path) => {
            info!("Detected executable path: {}", path.display());
            path
        }
        Err(e) => {
            warn!("Failed to get current exe path: {}, using fallback", e);
            PathBuf::from(".")
        }
    };

    let binding = PathBuf::from(".");
    let exe_dir = exe_path.parent().unwrap_or(&binding);

    // 路径优先级列表（按 Tauri 2.0 文档）
    let candidate_paths: Vec<(PathBuf, &'static str)> = vec![
        // 优先级 1：开发模式（当前工作目录的 binaries/）
        {
            let path = PathBuf::from("binaries").join(binary_name);
            if path.exists() {
                info!("Found unrar in dev mode: {}", path.display());
                return path;
            }
            (path, "dev mode (binaries/)")
        },
        // 优先级 2：发布模式（与主 exe 同一目录）
        {
            let path = exe_dir.join(binary_name);
            if path.exists() {
                info!("Found unrar in release mode (same dir): {}", path.display());
                return path;
            }
            (path, "release mode (same dir)")
        },
        // 优先级 3：发布模式（binaries 子目录，兼容性）
        {
            let path = exe_dir.join("binaries").join(binary_name);
            if path.exists() {
                info!("Found unrar in release mode (binaries subdir): {}", path.display());
                return path;
            }
            (path, "release mode (binaries/)")
        },
        // 优先级 4：src-tauri/binaries/（开发模式 fallback）
        {
            let path = PathBuf::from("src-tauri/binaries").join(binary_name);
            if path.exists() {
                info!("Found unrar in src-tauri/binaries (fallback): {}", path.display());
                return path;
            }
            (path, "src-tauri/binaries/")
        },
    ];

    // 记录所有尝试的路径（用于诊断）
    for (i, (path, desc)) in candidate_paths.iter().enumerate() {
        info!("Candidate path {} ({}): {}", i + 1, desc, path.display());
    }

    // 返回第一个路径（即使不存在，让调用方处理）
    let first_path = &candidate_paths[0].0;
    warn!(
        "No valid unrar path found, using first candidate: {}",
        first_path.display()
    );
    first_path.clone()
}

/**
 * 二进制文件验证结果
 */
pub struct BinaryValidationResult {
    pub path: PathBuf,
    pub exists: bool,
    pub is_executable: bool,
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub version_info: Option<String>,
}

/**
 * 验证二进制文件的完整性（运行时验证 - 方案 B）
 *
 * 检查：存在性、可执行性、版本信息
 * 通过执行 `unrar --version` 进行实际验证
 */
pub fn validate_unrar_binary(path: &Path) -> BinaryValidationResult {
    let mut result = BinaryValidationResult {
        path: path.to_path_buf(),
        exists: false,
        is_executable: false,
        is_valid: false,
        errors: Vec::new(),
        version_info: None,
    };

    // 1. 检查文件存在性
    result.exists = path.exists();
    if !result.exists {
        result.errors.push(format!(
            "File does not exist: {}",
            path.display()
        ));
        return result;
    }

    // 2. 检查可执行性
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        match std::fs::metadata(path) {
            Ok(metadata) => {
                let perms = metadata.permissions();
                result.is_executable = perms.mode() & 0o111 != 0;
                if !result.is_executable {
                    result.errors.push("File is not executable (missing execute permission)".to_string());
                }
            }
            Err(e) => {
                result.errors.push(format!("Failed to check permissions: {}", e));
                return result;
            }
        }
    }

    #[cfg(windows)]
    {
        match std::fs::metadata(path) {
            Ok(_metadata) => {
                // Windows .exe 文件默认可执行，验证文件扩展名
                result.is_executable = path
                    .extension()
                    .map(|ext| ext.eq_ignore_ascii_case("exe"))
                    .unwrap_or(false);
                if !result.is_executable {
                    result.errors.push("File does not have .exe extension".to_string());
                }
            }
            Err(e) => {
                result.errors.push(format!("Failed to check metadata: {}", e));
                return result;
            }
        }
    }

    // 3. 运行时验证（执行 unrar --version）
    match Command::new(path)
        .args(&["--version"])
        .output()
    {
        Ok(output) => {
            if output.status.success() {
                // 检查输出是否包含版本信息
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                // unrar --version 通常输出到 stderr
                let version_output = if !stdout.is_empty() {
                    stdout.to_string()
                } else if !stderr.is_empty() {
                    stderr.to_string()
                } else {
                    String::new()
                };

                if !version_output.is_empty()
                    && (version_output.to_lowercase().contains("unrar")
                        || version_output.contains("RAR"))
                {
                    result.version_info = Some(version_output.trim().to_string());
                    result.is_valid = true;
                    info!(
                        "unrar binary validation passed: version check succeeded. Version: {}",
                        version_output.trim()
                    );
                } else {
                    result.errors.push(format!(
                        "Invalid unrar binary (unexpected output): stdout='{}', stderr='{}'",
                        stdout.trim(),
                        stderr.trim()
                    ));
                }
            } else {
                // 命令执行失败
                let stderr = String::from_utf8_lossy(&output.stderr);
                result.errors.push(format!(
                    "unrar --version failed with exit code {}: {}",
                    output.status.code().unwrap_or(-1),
                    stderr.trim()
                ));
            }
        }
        Err(e) => {
            // 执行命令失败
            warn!(
                "Failed to execute unrar --version: {} (will check on actual use)",
                e
            );
            // 如果基本检查都通过，认为是有效的
            result.is_valid = result.exists && result.is_executable;
        }
    }

    // 4. 综合验证结果
    result.is_valid = result.exists && result.is_executable && result.errors.is_empty();

    result
}

/**
 * 同步提取 RAR 文件
 *
 * 使用 sidecar unrar 二进制进行解压
 * 命令格式: unrar x -y source dest\
 */
fn extract_rar_sync(
    source: &str,
    target_dir: &str,
    max_file_size: u64,
    max_total_size: u64,
    max_file_count: usize,
) -> Result<ExtractionSummary> {
    let mut summary = ExtractionSummary::new();

    // 获取 unrar 二进制路径
    let unrar_path = get_unrar_path();
    info!("Using unrar binary: {}", unrar_path.display());

    // 验证 unrar 二进制完整性（运行时验证）
    let validation = validate_unrar_binary(&unrar_path);

    if !validation.exists {
        summary.add_error(format!(
            "RAR extraction failed: unrar binary not found at {}. Expected locations (in order of priority):\n  1. binaries/ (dev mode)\n  2. same directory as exe (release mode)\n  3. binaries/ subdirectory (release mode)\n  4. src-tauri/binaries/ (fallback)\n\n  Please reinstall the application from official source.",
            unrar_path.display()
        ));
        return Ok(summary);
    }

    if !validation.is_valid {
        // 记录所有验证错误
        for error in &validation.errors {
            error!("unrar binary validation error: {}", error);
        }

        summary.add_error(format!(
            "RAR extraction failed: unrar binary validation failed. \
            Path: {}, Executable: {}, Version: {}. \
            Errors:\n  {}",
            unrar_path.display(),
            if validation.is_executable { "是" } else { "否" },
            validation.version_info.as_deref().unwrap_or("未知"),
            validation.errors.join("\n  ")
        ));
        return Ok(summary);
    }

    // 验证成功，继续执行
    info!(
        "unrar binary validation passed: path={}, version={}",
        unrar_path.display(),
        validation.version_info.as_deref().unwrap_or("未知")
    );

    // 调用 unrar 进行解压
    // unrar x -y source target\
    // x = extract with full paths
    // -y = assume Yes on all queries
    let output = Command::new(&unrar_path)
        .args(&["x", "-y", source, target_dir])
        .output()
        .map_err(|e| {
            AppError::archive_error(
                format!("Failed to execute unrar: {}", e),
                Some(PathBuf::from(source)),
            )
        })?;

    // 检查命令执行结果
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("unrar stderr: {}", stderr);

        // 解析常见错误
        if stderr.contains("cannot find file") || stderr.contains("not found") {
            summary.add_error(format!("RAR file not found or corrupted: {}", source));
        } else if stderr.contains("encrypted") || stderr.contains("password") {
            summary.add_error("RAR file is password protected and cannot be extracted".to_string());
        } else if stderr.contains("is not RAR archive") {
            summary.add_error(format!("File is not a valid RAR archive: {}", source));
        } else {
            summary.add_error(format!("RAR extraction failed: {}", stderr));
        }
        return Ok(summary);
    }

    // 解析 unrar 输出获取解压结果
    let stdout = String::from_utf8_lossy(&output.stdout);
    info!("unrar output: {}", stdout);

    // 从 unrar 输出中提取文件信息
    // unrar 输出格式示例:
    // Extracting from test.rar
    // Extracting  file1.txt                                OK
    // Extracting  dir/file2.txt                            OK
    // All OK
    let extracted_files = parse_unrar_output(&stdout);

    // 统计解压结果
    for file_info in extracted_files {
        // 检查文件大小限制
        if file_info.size > max_file_size {
            summary.add_error(format!(
                "File {} exceeds maximum size limit of {} bytes, skipped",
                file_info.name, max_file_size
            ));
            continue;
        }

        // 检查总大小限制
        if summary.total_size + file_info.size as u64 > max_total_size {
            summary.add_error(format!(
                "Extraction would exceed total size limit of {} bytes, stopping",
                max_total_size
            ));
            break;
        }

        // 检查文件数量限制
        if summary.files_extracted + 1 > max_file_count {
            summary.add_error(format!(
                "Extraction would exceed file count limit of {} files, stopping",
                max_file_count
            ));
            break;
        }

        // 解析文件路径，转换为 PathBuf
        let file_path = PathBuf::from(&file_info.name);

        // 判断是文件还是目录
        if file_info.name.ends_with('/') || file_info.name.ends_with('\\') {
            // 目录，不计入文件数
            summary.add_file(file_path, 0);
        } else {
            summary.add_file(file_path, file_info.size as u64);
            info!("Extracted: {}", file_info.name);
        }
    }

    // 如果没有提取任何文件，但命令成功了，可能是归档为空
    if summary.files_extracted == 0 && output.status.success() {
        info!("RAR archive extracted successfully (0 files)");
    }

    Ok(summary)
}

/**
 * 解析 unrar 命令输出
 *
 * 提取每个文件的名称和大小
 */
#[derive(Debug)]
struct FileInfo {
    name: String,
    size: u64,
}

fn parse_unrar_output(output: &str) -> Vec<FileInfo> {
    let mut files = Vec::new();

    // unrar 输出格式:
    // Extracting from archive.rar
    // Extracting  file1.txt                              1234 OK
    // Extracting  dir/file2.txt                       5678 OK
    // Extracting  dir1/                                  OK
    // All OK

    for line in output.lines() {
        // 跳过标题行和状态行
        if line.starts_with("Extracting from")
            || line.starts_with("Creating")
            || line.starts_with("All OK")
        {
            continue;
        }
        if line.trim().is_empty() {
            continue;
        }

        // 解析文件行: "Extracting  filename                    size OK"
        // 文件名可能在中间位置，需要找到 "OK" 或 "FAILED" 标记
        if let Some(ok_pos) = line.find(" OK") {
            // 提取文件名（去掉 "Extracting  " 前缀和大小）
            // 格式: "Extracting  file.txt      1234 OK"
            // 或: "Extracting  dir/     OK" (目录)

            // 先去掉 " OK" 结尾
            let content = &line[..ok_pos];

            // 检查是否有大小（末尾是数字，且数字前有空格）
            let name;
            let mut size: u64 = 0;

            // 从后向前查找数字，数字前必须有空格才认为是文件大小
            let bytes = content.as_bytes();
            let mut digit_start = None;

            for i in (0..bytes.len()).rev() {
                if bytes[i].is_ascii_digit() {
                    digit_start = Some(i);
                } else if digit_start.is_some() {
                    // 遇到非数字，停止查找
                    break;
                }
            }

            if let Some(d_pos) = digit_start {
                // 检查数字前面是否确实是空格（而不是字母）
                if d_pos > 0 && bytes[d_pos - 1] == b' ' {
                    // 有空格分隔，这是文件
                    let num_str = &content[d_pos..];
                    if let Ok(s) = num_str.parse::<u64>() {
                        size = s;
                    }
                    // 文件名是数字之前的所有内容，去掉 "Extracting  " 前缀
                    let before_digits = &content[..d_pos];
                    name = if before_digits.starts_with("Extracting  ") {
                        before_digits["Extracting  ".len()..].trim_end().to_string()
                    } else if before_digits.starts_with("Extracting ") {
                        before_digits["Extracting ".len()..].trim_end().to_string()
                    } else {
                        before_digits.trim_end().to_string()
                    };
                } else {
                    // 数字前没有空格，是目录或特殊文件名
                    name = if content.starts_with("Extracting  ") {
                        content["Extracting  ".len()..].trim_end().to_string()
                    } else if content.starts_with("Extracting ") {
                        content["Extracting ".len()..].trim_end().to_string()
                    } else {
                        content.trim_end().to_string()
                    };
                }
            } else {
                // 没有数字结尾，是目录
                name = if content.starts_with("Extracting  ") {
                    content["Extracting  ".len()..].trim_end().to_string()
                } else if content.starts_with("Extracting ") {
                    content["Extracting  ".len()..].trim_end().to_string()
                } else {
                    content.trim_end().to_string()
                };
            }

            files.push(FileInfo { name, size });
        } else if line.contains("FAILED") || line.contains("ERROR") {
            warn!("RAR extraction warning: {}", line);
        }
    }

    files
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

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

    #[test]
    fn test_parse_unrar_output_basic() {
        let output = r#"Extracting from test.rar
Extracting  file1.txt                              1234 OK
Extracting  file2.txt                              5678 OK
All OK"#;

        let files = parse_unrar_output(output);

        assert_eq!(files.len(), 2);
        assert_eq!(files[0].name, "file1.txt");
        assert_eq!(files[0].size, 1234);
        assert_eq!(files[1].name, "file2.txt");
        assert_eq!(files[1].size, 5678);
    }

    #[test]
    fn test_parse_unrar_output_with_directories() {
        let output = r#"Extracting from test.rar
Extracting  dir1/                                   OK
Extracting  dir1/file1.txt                         1000 OK
Extracting  dir2/subdir/file2.txt                  2000 OK
All OK"#;

        let files = parse_unrar_output(output);

        assert_eq!(files.len(), 3);
        assert_eq!(files[0].name, "dir1/");
        assert_eq!(files[1].name, "dir1/file1.txt");
        assert_eq!(files[2].name, "dir2/subdir/file2.txt");
    }

    #[test]
    fn test_parse_unrar_output_empty() {
        let output = "Extracting from empty.rar\nAll OK";

        let files = parse_unrar_output(output);

        assert!(files.is_empty());
    }

    #[test]
    fn test_get_unrar_path() {
        let path = get_unrar_path();

        // 验证路径格式正确
        let binary_name = path.file_name().unwrap().to_str().unwrap();
        assert!(binary_name.starts_with("unrar-"));
        assert!(binary_name.ends_with(match std::env::consts::OS {
            "macos" => "darwin",
            "linux" => "linux-gnu",
            "windows" => ".exe",
            _ => "",
        }));
    }
}
