use crate::archive::archive_handler::{ArchiveHandler, ExtractionSummary};
use crate::error::{AppError, Result};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::process::Command;
use tokio::fs;
use tracing::{debug, error, info, warn};

/**
 * RAR文件处理器 - 纯Rust优先方案
 *
 * 采用双模式策略：
 * 1. 优先使用 rar crate (纯Rust实现) - 基础RAR4支持
 * 2. Fallback到 unrar 二进制 (处理复杂RAR5/多部分/加密)
 *
 * 支持平台：Windows、macOS、Linux
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

        // 尝试使用纯Rust的 rar crate
        match RarHandler::extract_with_rar_crate(
            &source_str,
            &target_str,
            max_file_size,
            max_total_size,
            max_file_count,
        )
        .await
        {
            Ok(summary) => {
                if summary.files_extracted > 0 {
                    info!("Successfully extracted RAR using rar crate (pure Rust)");
                    return Ok(summary);
                }
                warn!("rar crate returned empty result, trying unrar binary fallback");
            }
            Err(e) => {
                debug!("rar crate extraction failed: {}, trying fallback", e);
            }
        }

        // Fallback: 使用 unrar 二进制处理
        RarHandler::extract_with_unrar_fallback(
            &source_str,
            &target_str,
            max_file_size,
            max_total_size,
            max_file_count,
        )
        .await
    }

    #[allow(dead_code)]
    async fn extract(&self, source: &Path, target_dir: &Path) -> Result<ExtractionSummary> {
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

impl RarHandler {
    /**
     * 使用纯Rust的 rar crate 提取RAR文件
     */
    async fn extract_with_rar_crate(
        source: &str,
        target_dir: &str,
        max_file_size: u64,
        max_total_size: u64,
        max_file_count: usize,
    ) -> Result<ExtractionSummary> {
        // rar v0.4 API: extract_all returns Result<Archive>
        let archive = rar::Archive::extract_all(source, target_dir, "")
            .map_err(|e| AppError::archive_error(format!("RAR extraction failed: {}", e), None))?;

        let mut summary = ExtractionSummary::new();
        let mut total_size = 0u64;
        let mut file_count = 0usize;

        // 遍历已提取的文件
        for entry in archive.files.into_iter() {
            // rar crate 返回 FileBlock，使用文件名作为路径
            // FileBlock 的文件名存储在 header 中
            let entry_name = format!("{:?}", entry.head);
            let entry_path = PathBuf::from(&entry_name);

            // 跳过目录 (检查文件名是否以/结尾或 attributes 表明是目录)
            if entry_name.ends_with('/') || entry_path.is_dir() {
                continue;
            }

            // 检查文件大小
            let file_size = entry_path.metadata().map(|m| m.len()).unwrap_or(0);

            if file_size > max_file_size {
                summary.add_error(format!(
                    "File {} exceeds maximum size limit, skipped",
                    entry_name
                ));
                continue;
            }

            if total_size + file_size > max_total_size {
                summary.add_error("Extraction would exceed total size limit, stopping".to_string());
                break;
            }

            if file_count + 1 > max_file_count {
                summary.add_error("Extraction would exceed file count limit, stopping".to_string());
                break;
            }

            total_size += file_size;
            file_count += 1;
            summary.add_file(entry_path, file_size);
            debug!("Extracted (rar crate): {}", entry_name);
        }

        info!(
            "rar crate extracted {} files, {} bytes",
            file_count, total_size
        );
        Ok(summary)
    }

    /**
     * Fallback: 使用 unrar 二进制提取
     */
    async fn extract_with_unrar_fallback(
        source: &str,
        target_dir: &str,
        max_file_size: u64,
        max_total_size: u64,
        max_file_count: usize,
    ) -> Result<ExtractionSummary> {
        let mut summary = ExtractionSummary::new();

        // 获取 unrar 二进制路径
        let unrar_path = get_unrar_path().map_err(|e| {
            summary.add_error(format!("RAR extraction failed: {}", e));
            e
        })?;
        debug!("Using unrar binary fallback: {}", unrar_path.display());

        // 检查 unrar 二进制是否存在
        if !unrar_path.exists() {
            summary.add_error(format!(
                "RAR extraction failed: unrar binary not found at {}",
                unrar_path.display()
            ));
            return Ok(summary);
        }

        // 安全验证：确保路径不包含危险字符
        // 使用业内成熟的 sanitize-filename 库进行路径验证
        let source_path = PathBuf::from(source);
        let target_path = PathBuf::from(target_dir);

        // 验证源文件路径
        if !source_path.exists() || !source_path.is_file() {
            return Err(AppError::archive_error(
                format!("Source file does not exist or is not a file: {}", source),
                Some(source_path),
            ));
        }

        // 验证源文件扩展名
        if !source_path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("rar"))
        {
            return Err(AppError::archive_error(
                format!("Source file is not a RAR archive: {}", source),
                Some(source_path),
            ));
        }

        // 验证目标目录
        if !target_path.exists() || !target_path.is_dir() {
            return Err(AppError::archive_error(
                format!(
                    "Target directory does not exist or is not a directory: {}",
                    target_dir
                ),
                Some(target_path),
            ));
        }

        // 使用参数化API调用 unrar，防止命令注入
        // Command::args() 会自动处理参数转义，不经过 shell
        let output = Command::new(&unrar_path)
            .arg("x") // 提取命令
            .arg("-y") // 假设对所有询问回答是
            .arg(source) // 源文件（作为独立参数，安全）
            .arg(target_dir) // 目标目录（作为独立参数，安全）
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

            if stderr.contains("encrypted") || stderr.contains("password") {
                summary.add_error("RAR file is password protected".to_string());
            } else if stderr.contains("is not RAR archive") {
                summary.add_error(format!("File is not a valid RAR archive: {}", source));
            } else {
                summary.add_error(format!("RAR extraction failed: {}", stderr));
            }
            return Ok(summary);
        }

        // 解析 unrar 输出获取解压结果
        let stdout = String::from_utf8_lossy(&output.stdout);
        let extracted_files = parse_unrar_output(&stdout);

        // 统计解压结果
        for file_info in extracted_files {
            if file_info.size > max_file_size {
                summary.add_error(format!(
                    "File {} exceeds maximum size limit, skipped",
                    file_info.name
                ));
                continue;
            }

            if summary.total_size + file_info.size > max_total_size {
                summary.add_error("Extraction would exceed total size limit, stopping".to_string());
                break;
            }

            if summary.files_extracted + 1 > max_file_count {
                summary.add_error("Extraction would exceed file count limit, stopping".to_string());
                break;
            }

            let file_path = PathBuf::from(&file_info.name);
            if file_info.name.ends_with('/') || file_info.name.ends_with('\\') {
                summary.add_file(file_path, 0);
            } else {
                summary.add_file(file_path, file_info.size);
                debug!("Extracted (fallback): {}", file_info.name);
            }
        }

        info!("unrar fallback extracted {} files", summary.files_extracted);
        Ok(summary)
    }
}

/**
 * 获取对应平台的 unrar 二进制路径 (Fallback方案)
 *
 * # Returns
 * * `Ok(PathBuf)` - 平台支持时返回二进制路径
 * * `Err(AppError)` - 平台不支持时返回错误
 */
fn get_unrar_path() -> Result<PathBuf> {
    let binary_name = match std::env::consts::OS {
        "macos" => "unrar-aarch64-apple-darwin",
        "linux" => "unrar-x86_64-unknown-linux-gnu",
        "windows" => "unrar-x86_64-pc-windows-msvc.exe",
        os => return Err(AppError::archive_error(
            format!("Unsupported platform for unrar fallback: {}", os),
            None
        )),
    };

    let resource_dir = if cfg!(debug_assertions) {
        PathBuf::from("binaries")
    } else {
        std::env::current_exe()
            .unwrap_or_else(|_| PathBuf::from("."))
            .parent()
            .unwrap_or(&PathBuf::from("."))
            .to_path_buf()
    };

    Ok(resource_dir.join(binary_name))
}

/**
 * 解析 unrar 命令输出
 */
#[derive(Debug)]
struct FileInfo {
    name: String,
    size: u64,
}

fn parse_unrar_output(output: &str) -> Vec<FileInfo> {
    let mut files = Vec::new();

    for line in output.lines() {
        if line.starts_with("Extracting from")
            || line.starts_with("Creating")
            || line.starts_with("All OK")
        {
            continue;
        }
        if line.trim().is_empty() {
            continue;
        }

        if let Some(ok_pos) = line.find(" OK") {
            let content = &line[..ok_pos];
            let name;
            let mut size: u64 = 0;

            let bytes = content.as_bytes();
            let mut digit_start = None;

            for i in (0..bytes.len()).rev() {
                if bytes[i].is_ascii_digit() {
                    digit_start = Some(i);
                } else if digit_start.is_some() {
                    break;
                }
            }

            if let Some(d_pos) = digit_start {
                if d_pos > 0 && bytes[d_pos - 1] == b' ' {
                    let num_str = &content[d_pos..];
                    if let Ok(s) = num_str.parse::<u64>() {
                        size = s;
                    }
                    let before_digits = &content[..d_pos];
                    name = if let Some(stripped) = before_digits.strip_prefix("Extracting  ") {
                        stripped.trim_end().to_string()
                    } else if let Some(stripped) = before_digits.strip_prefix("Extracting ") {
                        stripped.trim_end().to_string()
                    } else {
                        before_digits.trim_end().to_string()
                    };
                } else {
                    name = if let Some(stripped) = content.strip_prefix("Extracting  ") {
                        stripped.trim_end().to_string()
                    } else {
                        content.trim_end().to_string()
                    };
                }
            } else {
                name = if let Some(stripped) = content.strip_prefix("Extracting  ") {
                    stripped.trim_end().to_string()
                } else {
                    content.trim_end().to_string()
                };
            }

            files.push(FileInfo { name, size });
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
}
