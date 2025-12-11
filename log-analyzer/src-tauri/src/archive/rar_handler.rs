use async_trait::async_trait;
use std::path::{Path, PathBuf};
use crate::archive::archive_handler::{ArchiveHandler, ExtractionSummary};
use crate::error::{AppError, Result};
use tokio::fs;
use std::process::Command;

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
    
    async fn extract(
        &self,
        source: &Path,
        target_dir: &Path,
    ) -> Result<ExtractionSummary> {
        // 确保目标目录存在
        fs::create_dir_all(target_dir)
            .await
            .map_err(|e| AppError::archive_error(
                format!("Failed to create target directory: {}", e),
                Some(target_dir.to_path_buf())
            ))?;
        
        // 获取unrar可执行文件路径
        let unrar_path = get_unrar_path();
        
        // 构建提取命令
        let output = Command::new(&unrar_path)
            .arg("x") // 提取文件
            .arg("-y") // 自动确认
            .arg("-o+") // 覆盖现有文件
            .arg(source)
            .arg(target_dir)
            .output()
            .map_err(|e| AppError::archive_error(
                format!("Failed to execute unrar command: {}", e),
                Some(source.to_path_buf())
            ))?;
        
        let mut summary = ExtractionSummary::new();
        
        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            summary.add_error(format!("RAR extraction failed: {}", error_msg));
            return Ok(summary);
        }
        
        // 扫描提取的文件
        scan_extracted_files(target_dir, &mut summary).await?;
        
        Ok(summary)
    }
    
    fn file_extensions(&self) -> Vec<&str> {
        vec!["rar"]
    }
}

/**
 * 获取unrar可执行文件路径
 */
fn get_unrar_path() -> String {
    // 检查环境变量
    if let Ok(path) = std::env::var("UNRAR_PATH") {
        return path;
    }
    
    // 检查常见路径
    let possible_paths = [
        "unrar",
        "/usr/bin/unrar",
        "/usr/local/bin/unrar",
        "C:\\Program Files\\WinRAR\\UnRAR.exe",
        "C:\\Program Files (x86)\\WinRAR\\UnRAR.exe",
    ];
    
    for path in &possible_paths {
        if Command::new(path).arg("--help").output().is_ok() {
            return path.to_string();
        }
    }
    
    // 默认返回unrar，依赖PATH
    "unrar".to_string()
}

/**
 * 扫描提取的文件并更新摘要
 */
async fn scan_extracted_files(
    dir: &Path,
    summary: &mut ExtractionSummary,
) -> Result<()> {
    let mut entries = fs::read_dir(dir)
        .await
        .map_err(|e| AppError::archive_error(
            format!("Failed to read directory: {}", e),
            Some(dir.to_path_buf())
        ))?;
    
    while let Some(entry) = entries.next_entry().await
        .map_err(|e| AppError::archive_error(
            format!("Failed to read directory entry: {}", e),
            Some(dir.to_path_buf())
        ))? 
    {
        let path = entry.path();
        let metadata = fs::metadata(&path)
            .await
            .map_err(|e| AppError::archive_error(
                format!("Failed to get metadata: {}", e),
                Some(path.clone())
            ))?;
        
        if metadata.is_file() {
            summary.add_file(path.clone(), metadata.len());
        } else if metadata.is_dir() {
            // 递归扫描子目录
            scan_extracted_files(&path, summary).await?;
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs::File;
    use std::io::Write;

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
        fs::create_dir_all(temp_dir.path().join("subdir")).await.unwrap();
        
        let file1 = temp_dir.path().join("file1.txt");
        let file2 = temp_dir.path().join("subdir").join("file2.txt");
        
        fs::write(&file1, "content1").await.unwrap();
        fs::write(&file2, "content2").await.unwrap();
        
        let mut summary = ExtractionSummary::new();
        scan_extracted_files(temp_dir.path(), &mut summary).await.unwrap();
        
        assert_eq!(summary.files_extracted, 2);
        assert!(summary.total_size > 0);
    }

    #[test]
    fn test_get_unrar_path() {
        let path = get_unrar_path();
        assert!(!path.is_empty());
    }
}