use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, AsyncSeekExt, BufReader};
use std::path::Path;
use tracing::warn;

/**
 * 异步文件读取器
 * 
 * 提供异步文件I/O操作，提升性能和响应性
 */
pub struct AsyncFileReader;

impl AsyncFileReader {
    /**
     * 从指定偏移量异步读取文件
     * 
     * # 参数
     * * `path` - 文件路径
     * * `offset` - 起始偏移量
     * 
     * # 返回
     * * `Ok((Vec<String>, u64))` - (行列表, 文件大小)
     * * `Err(String)` - 错误信息
     */
    pub async fn read_file_from_offset(
        path: &Path,
        offset: u64,
    ) -> Result<(Vec<String>, u64), String> {
        // 打开文件
        let mut file = File::open(path)
            .await
            .map_err(|e| format!("Failed to open file: {}", e))?;
        
        // 获取文件元数据
        let metadata = file
            .metadata()
            .await
            .map_err(|e| format!("Failed to get metadata: {}", e))?;
        
        let file_size = metadata.len();
        
        // 计算实际起始偏移量
        // 读取偏移量超出文件大小时，返回空结果而不是重新读取整个文件
        let start_offset = file_size.min(offset);
        
        if start_offset >= file_size {
            return Ok((Vec::new(), file_size));
        }
        
        // 移动到指定偏移量
        file.seek(std::io::SeekFrom::Start(start_offset))
            .await
            .map_err(|e| format!("Failed to seek: {}", e))?;
        
        // 创建缓冲读取器
        let reader = BufReader::new(file);
        let mut lines = Vec::new();
        let mut lines_stream = reader.lines();
        
        // 异步读取所有行
        while let Some(line) = lines_stream.next_line().await {
            match line {
                Ok(l) => lines.push(l),
                Err(e) => {
                    warn!(error = %e, "Error reading line");
                    break;
                }
            }
        }
        
        Ok((lines, file_size))
    }
    
    /**
     * 异步读取文件的前N行
     * 
     * # 参数
     * * `path` - 文件路径
     * * `max_lines` - 最大行数
     * 
     * # 返回
     * * `Ok(Vec<String>)` - 行列表
     * * `Err(String)` - 错误信息
     */
    pub async fn read_file_head(
        path: &Path,
        max_lines: usize,
    ) -> Result<Vec<String>, String> {
        let mut file = File::open(path)
            .await
            .map_err(|e| format!("Failed to open file: {}", e))?;
        
        let reader = BufReader::new(file);
        let mut lines = Vec::new();
        let mut lines_stream = reader.lines();
        let mut count = 0;
        
        while let Some(line) = lines_stream.next_line().await {
            if count >= max_lines {
                break;
            }
            
            match line {
                Ok(l) => {
                    lines.push(l);
                    count += 1;
                }
                Err(e) => {
                    warn!(error = %e, "Error reading line");
                    break;
                }
            }
        }
        
        Ok(lines)
    }
    
    /**
     * 检查文件是否存在且可读
     * 
     * # 参数
     * * `path` - 文件路径
     * 
     * # 返回
     * * `Ok(bool)` - 是否存在且可读
     * * `Err(String)` - 错误信息
     */
    pub async fn check_file_readable(path: &Path) -> Result<bool, String> {
        match tokio::fs::metadata(path).await {
            Ok(metadata) => Ok(metadata.is_file() && metadata.len() > 0),
            Err(e) => Err(format!("Failed to check file: {}", e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[tokio::test]
    async fn test_read_file_from_offset() {
        // 创建临时文件
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "Line 1").unwrap();
        writeln!(temp_file, "Line 2").unwrap();
        writeln!(temp_file, "Line 3").unwrap();
        writeln!(temp_file, "Line 4").unwrap();
        writeln!(temp_file, "Line 5").unwrap();
        
        let path = temp_file.path();
        
        // 从偏移量0读取
        let (lines, size) = AsyncFileReader::read_file_from_offset(path, 0)
            .await
            .unwrap();
        
        assert_eq!(lines.len(), 5);
        assert!(size > 0);
        
        // 从偏移量10读取（跳过一些内容）
        let (lines_partial, _) = AsyncFileReader::read_file_from_offset(path, 10)
            .await
            .unwrap();

        assert!(lines_partial.len() > 0);
        assert!(lines_partial.len() <= 5);

        // 从超过文件大小的偏移量读取时应该返回空结果
        let (empty_lines, size_after_end) = AsyncFileReader::read_file_from_offset(path, u64::MAX)
            .await
            .unwrap();

        assert!(empty_lines.is_empty());
        assert_eq!(size, size_after_end);
    }

    #[tokio::test]
    async fn test_read_file_head() {
        // 创建临时文件
        let mut temp_file = NamedTempFile::new().unwrap();
        for i in 0..100 {
            writeln!(temp_file, "Line {}", i).unwrap();
        }
        
        let path = temp_file.path();
        
        // 读取前10行
        let lines = AsyncFileReader::read_file_head(path, 10)
            .await
            .unwrap();
        
        assert_eq!(lines.len(), 10);
        assert!(lines[0].contains("Line 0"));
        assert!(lines[9].contains("Line 9"));
    }

    #[tokio::test]
    async fn test_check_file_readable() {
        // 创建临时文件
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        
        // 检查文件可读
        let readable = AsyncFileReader::check_file_readable(path)
            .await
            .unwrap();
        
        assert!(readable);
        
        // 检查不存在的文件
        let non_existent = Path::new("/non/existent/file.txt");
        let readable = AsyncFileReader::check_file_readable(non_existent)
            .await;
        
        assert!(readable.is_err() || readable.unwrap() == false);
    }

    #[tokio::test]
    async fn test_read_empty_file() {
        // 创建空临时文件
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        
        // 读取空文件
        let (lines, size) = AsyncFileReader::read_file_from_offset(path, 0)
            .await
            .unwrap();
        
        assert_eq!(lines.len(), 0);
        assert_eq!(size, 0);
    }

    #[tokio::test]
    async fn test_read_large_file() {
        // 创建大临时文件
        let mut temp_file = NamedTempFile::new().unwrap();
        let large_content = "A".repeat(1000) + "\n";
        
        for _ in 0..100 {
            write!(temp_file, "{}", large_content).unwrap();
        }
        
        let path = temp_file.path();
        
        let start = std::time::Instant::now();
        let (lines, _) = AsyncFileReader::read_file_from_offset(path, 0)
            .await
            .unwrap();
        let duration = start.elapsed();
        
        assert_eq!(lines.len(), 100);
        assert!(duration.as_millis() < 1000); // 应该在1秒内完成
    }
}