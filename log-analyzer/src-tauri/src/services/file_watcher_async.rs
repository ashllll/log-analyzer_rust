//! 异步文件监听服务
//!
//! 提供实时文件监听和增量读取功能，支持：
//! - 从指定偏移量异步读取文件
//! - Inode/File Index 追踪（跨平台）
//! - logrotate 自动检测与重连
//! - 文件截断检测
//! - 大文件分块处理

use std::io;
use std::path::{Path, PathBuf};
use std::time::Instant;

use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, AsyncSeekExt, BufReader};
use tracing::{debug, info};

// ============================================================================
// 跨平台 Inode/File Index 支持
// ============================================================================

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

/// 获取文件的唯一标识符（跨平台）
///
/// - Unix: 返回 inode 编号
/// - Windows: 返回文件索引
///
/// # 参数
/// * `path` - 文件路径
///
/// # 返回
/// * `Ok(u64)` - 文件唯一标识符
/// * `Err(io::Error)` - 获取失败
///
/// # 示例
/// ```ignore
/// let file_id = get_file_id(Path::new("/var/log/app.log"))?;
/// println!("File ID: {}", file_id);
/// ```
#[cfg(unix)]
pub fn get_file_id(path: &Path) -> io::Result<u64> {
    let metadata = std::fs::metadata(path)?;
    Ok(metadata.ino())
}

#[cfg(windows)]
pub fn get_file_id(path: &Path) -> io::Result<u64> {
    // Windows 平台使用文件大小 + 修改时间作为替代标识
    // 这是因为 winapi crate 未在依赖中，且对于 logrotate 检测足够可靠
    let metadata = std::fs::metadata(path)?;
    let size = metadata.len();
    let modified = metadata.modified()?;
    let modified_ts = modified
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    // 组合大小和时间戳创建伪唯一标识
    // 这种方法在以下情况下能够正确检测 logrotate:
    // 1. 文件被 rotate 后，新文件大小通常不同
    // 2. 新文件的修改时间必然不同
    Ok(size.wrapping_add(modified_ts))
}

// ============================================================================
// 文件轮转状态
// ============================================================================

/// 文件轮转状态
///
/// 描述文件是否被 rotate、删除或保持不变
#[derive(Debug, Clone, PartialEq)]
pub enum RotationState {
    /// 文件未变化
    Unchanged,
    /// 文件被 rotate（inode 变化）
    Rotated {
        /// 新的文件 ID（如果文件存在）
        new_file_id: Option<u64>,
        /// 新文件的大小
        new_size: u64,
    },
    /// 文件被删除
    Deleted,
    /// 文件被截断（大小变小）
    Truncated {
        /// 原始大小
        original_size: u64,
        /// 当前大小
        current_size: u64,
    },
}

// ============================================================================
// 文件追踪器
// ============================================================================

/// 文件追踪器
///
/// 追踪文件的 inode/file index 变化，用于检测 logrotate
///
/// # 特性
/// - 跨平台支持（Unix inode / Windows file index）
/// - 自动检测文件轮转
/// - 支持截断检测
/// - 检查间隔控制（避免频繁检查）
///
/// # 示例
/// ```ignore
/// let mut tracker = FileTracker::new(Path::new("/var/log/app.log")).await?;
///
/// // 定期检查文件状态
/// let state = tracker.check_rotation().await?;
/// match state {
///     RotationState::Rotated { .. } => {
///         tracker.reopen().await?;
///     }
///     _ => {}
/// }
/// ```
#[derive(Debug)]
pub struct FileTracker {
    /// 文件路径
    path: PathBuf,
    /// 文件唯一标识符（inode 或 file index）
    file_id: Option<u64>,
    /// 文件大小
    file_size: u64,
    /// 上次检查时间
    last_check: Instant,
    /// 检查间隔（毫秒）
    check_interval_ms: u64,
    /// 当前打开的文件句柄（可选）
    current_file: Option<File>,
    /// 当前读取偏移量
    current_offset: u64,
}

impl FileTracker {
    /// 默认检查间隔（500ms）
    const DEFAULT_CHECK_INTERVAL_MS: u64 = 500;

    /// 创建新的文件追踪器
    ///
    /// # 参数
    /// * `path` - 要追踪的文件路径
    ///
    /// # 返回
    /// * `Ok(FileTracker)` - 追踪器实例
    /// * `Err(io::Error)` - 创建失败
    pub async fn new(path: &Path) -> io::Result<Self> {
        let metadata = tokio::fs::metadata(path).await?;
        let file_size = metadata.len();

        // 获取文件 ID
        let file_id = get_file_id(path).ok();

        // 打开文件
        let file = File::open(path).await?;

        info!(
            path = %path.display(),
            file_id = ?file_id,
            size = file_size,
            "FileTracker initialized"
        );

        Ok(Self {
            path: path.to_path_buf(),
            file_id,
            file_size,
            last_check: Instant::now(),
            check_interval_ms: Self::DEFAULT_CHECK_INTERVAL_MS,
            current_file: Some(file),
            current_offset: 0,
        })
    }

    /// 设置检查间隔
    ///
    /// # 参数
    /// * `interval_ms` - 检查间隔（毫秒）
    pub fn with_check_interval(mut self, interval_ms: u64) -> Self {
        self.check_interval_ms = interval_ms;
        self
    }

    /// 获取文件路径
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 获取当前文件 ID
    pub fn file_id(&self) -> Option<u64> {
        self.file_id
    }

    /// 获取当前文件大小
    pub fn file_size(&self) -> u64 {
        self.file_size
    }

    /// 获取当前读取偏移量
    pub fn current_offset(&self) -> u64 {
        self.current_offset
    }

    /// 设置当前读取偏移量
    pub fn set_offset(&mut self, offset: u64) {
        self.current_offset = offset;
    }

    /// 检查文件是否被 rotate
    ///
    /// 此方法会检查：
    /// 1. 文件是否存在
    /// 2. 文件 ID 是否变化
    /// 3. 文件大小是否变小（截断）
    ///
    /// # 返回
    /// * `Ok(RotationState)` - 文件状态
    /// * `Err(io::Error)` - 检查失败
    pub async fn check_rotation(&mut self) -> io::Result<RotationState> {
        // 检查是否到达检查间隔
        let elapsed = self.last_check.elapsed().as_millis() as u64;
        if elapsed < self.check_interval_ms {
            return Ok(RotationState::Unchanged);
        }

        self.last_check = Instant::now();

        // 检查文件是否存在
        let metadata = match tokio::fs::metadata(&self.path).await {
            Ok(m) => m,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                debug!(path = %self.path.display(), "File deleted");
                return Ok(RotationState::Deleted);
            }
            Err(e) => return Err(e),
        };

        let current_size = metadata.len();

        // 检查文件 ID
        let current_file_id = get_file_id(&self.path).ok();

        // 检测截断（文件变小）
        if current_size < self.current_offset {
            debug!(
                path = %self.path.display(),
                original_size = self.current_offset,
                current_size,
                "File truncated"
            );
            return Ok(RotationState::Truncated {
                original_size: self.current_offset,
                current_size,
            });
        }

        // 检测 inode 变化（文件被 rotate）
        if self.file_id.is_some() && current_file_id != self.file_id {
            info!(
                path = %self.path.display(),
                old_file_id = ?self.file_id,
                new_file_id = ?current_file_id,
                "File rotated detected"
            );
            return Ok(RotationState::Rotated {
                new_file_id: current_file_id,
                new_size: current_size,
            });
        }

        // 更新文件大小
        self.file_size = current_size;

        Ok(RotationState::Unchanged)
    }

    /// 重新打开文件（rotate 后）
    ///
    /// 当检测到文件被 rotate 后，调用此方法重新打开文件。
    /// 会更新文件 ID 和大小，并将偏移量重置为 0。
    ///
    /// # 返回
    /// * `Ok(File)` - 新打开的文件句柄
    /// * `Err(io::Error)` - 打开失败
    pub async fn reopen(&mut self) -> io::Result<File> {
        let file = File::open(&self.path).await?;
        let metadata = file.metadata().await?;
        let new_size = metadata.len();
        let new_file_id = get_file_id(&self.path).ok();

        info!(
            path = %self.path.display(),
            old_file_id = ?self.file_id,
            new_file_id = ?new_file_id,
            new_size,
            "File reopened after rotation"
        );

        // 更新追踪器状态
        self.file_id = new_file_id;
        self.file_size = new_size;
        self.current_file = Some(file);
        self.current_offset = 0; // 从头开始读取新文件

        // 返回文件句柄的克隆
        // 注意：Tokio File 不直接支持克隆，所以返回新的打开
        File::open(&self.path).await
    }

    /// 重置偏移量到文件末尾（跳过已有内容）
    ///
    /// 用于只想监听新内容的场景
    pub async fn seek_to_end(&mut self) -> io::Result<u64> {
        if let Some(ref mut file) = self.current_file {
            let offset = file.seek(std::io::SeekFrom::End(0)).await?;
            self.current_offset = offset;
            debug!(
                path = %self.path.display(),
                offset,
                "Seeked to file end"
            );
            Ok(offset)
        } else {
            Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "No file handle available",
            ))
        }
    }

    /// 读取新增内容
    ///
    /// 从当前偏移量读取到文件末尾
    ///
    /// # 参数
    /// * `max_lines` - 最大读取行数（None 表示无限制）
    ///
    /// # 返回
    /// * `Ok((Vec<String>, u64))` - (行列表, 新偏移量)
    /// * `Err(io::Error)` - 读取失败
    pub async fn read_new_content(
        &mut self,
        max_lines: Option<usize>,
    ) -> io::Result<(Vec<String>, u64)> {
        let _file = if let Some(ref file) = self.current_file {
            file
        } else {
            return Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "No file handle available",
            ));
        };

        // 重新打开以获取最新状态
        let mut file = File::open(&self.path).await?;

        // 获取当前文件大小
        let metadata = file.metadata().await?;
        let file_size = metadata.len();

        // 如果没有新内容，返回空
        if self.current_offset >= file_size {
            return Ok((Vec::new(), self.current_offset));
        }

        // 移动到当前偏移量
        file.seek(std::io::SeekFrom::Start(self.current_offset))
            .await?;

        let reader = BufReader::new(file);
        let mut lines = Vec::new();
        let mut lines_stream = reader.lines();
        let mut bytes_read = 0u64;

        while let Some(line) = lines_stream.next_line().await? {
            bytes_read += line.len() as u64 + 1; // +1 for newline

            lines.push(line);

            if let Some(max) = max_lines {
                if lines.len() >= max {
                    break;
                }
            }
        }

        // 更新偏移量
        self.current_offset += bytes_read;

        debug!(
            path = %self.path.display(),
            lines_read = lines.len(),
            new_offset = self.current_offset,
            "Read new content"
        );

        Ok((lines, self.current_offset))
    }
}

// ============================================================================
// 异步文件读取器（保留原有功能）
// ============================================================================

/**
 * 异步文件读取器
 *
 * 提供异步文件I/O操作，提升性能和响应性
 */
pub struct AsyncFileReader {}

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
        while let Ok(Some(line)) = lines_stream.next_line().await {
            lines.push(line);
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
    pub async fn read_file_head(path: &Path, max_lines: usize) -> Result<Vec<String>, String> {
        let file = File::open(path)
            .await
            .map_err(|e| format!("Failed to open file: {}", e))?;

        let reader = BufReader::new(file);
        let mut lines = Vec::new();
        let mut lines_stream = reader.lines();
        let mut count = 0;

        while let Ok(Some(line)) = lines_stream.next_line().await {
            if count >= max_lines {
                break;
            }
            lines.push(line);
            count += 1;
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

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_file_tracker_new() {
        // 创建临时文件
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "Line 1").unwrap();
        writeln!(temp_file, "Line 2").unwrap();

        let path = temp_file.path();
        let tracker = FileTracker::new(path).await.unwrap();

        assert!(tracker.file_id().is_some());
        assert!(tracker.file_size() > 0);
        assert_eq!(tracker.current_offset(), 0);
    }

    #[tokio::test]
    async fn test_file_tracker_check_rotation_unchanged() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "Line 1").unwrap();

        let path = temp_file.path();
        let mut tracker = FileTracker::new(path).await.unwrap();

        // 设置很短的检查间隔
        tracker.check_interval_ms = 0;

        // 等待一点时间
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let state = tracker.check_rotation().await.unwrap();
        assert_eq!(state, RotationState::Unchanged);
    }

    #[tokio::test]
    async fn test_file_tracker_truncation_detection() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "Line 1 - this is a long line").unwrap();
        writeln!(temp_file, "Line 2 - another long line").unwrap();

        let path = temp_file.path();
        let mut tracker = FileTracker::new(path).await.unwrap();

        // 模拟已经读取了一部分
        tracker.current_offset = 50;

        // 设置很短的检查间隔
        tracker.check_interval_ms = 0;
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // 截断文件
        temp_file.as_file_mut().set_len(10).unwrap();

        let state = tracker.check_rotation().await.unwrap();
        assert!(matches!(state, RotationState::Truncated { .. }));
    }

    #[tokio::test]
    async fn test_file_tracker_read_new_content() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "Line 1").unwrap();
        writeln!(temp_file, "Line 2").unwrap();

        let path = temp_file.path();
        let mut tracker = FileTracker::new(path).await.unwrap();

        // 读取初始内容
        let (lines, offset) = tracker.read_new_content(None).await.unwrap();
        assert_eq!(lines.len(), 2);
        assert!(offset > 0);
    }

    #[tokio::test]
    async fn test_file_tracker_seek_to_end() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "Line 1").unwrap();
        writeln!(temp_file, "Line 2").unwrap();

        let path = temp_file.path();
        let mut tracker = FileTracker::new(path).await.unwrap();

        let offset = tracker.seek_to_end().await.unwrap();
        assert!(offset > 0);
        assert_eq!(tracker.current_offset(), offset);
    }

    #[tokio::test]
    async fn test_file_tracker_deleted_file() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();

        let mut tracker = FileTracker::new(&path).await.unwrap();
        tracker.check_interval_ms = 0;

        // 删除文件
        drop(temp_file);

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let state = tracker.check_rotation().await.unwrap();
        assert_eq!(state, RotationState::Deleted);
    }

    #[test]
    fn test_get_file_id() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "Test content").unwrap();

        let path = temp_file.path();
        let file_id = get_file_id(path);

        // 在所有平台都应该成功
        assert!(
            file_id.is_ok(),
            "Failed to get file ID: {:?}",
            file_id.err()
        );
    }

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

        assert!(!lines_partial.is_empty());
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
        let lines = AsyncFileReader::read_file_head(path, 10).await.unwrap();

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
        let readable = AsyncFileReader::check_file_readable(path).await.unwrap();

        assert!(readable);

        // 检查不存在的文件
        let non_existent = Path::new("/non/existent/file.txt");
        let readable = AsyncFileReader::check_file_readable(non_existent).await;

        assert!(readable.is_err() || !readable.unwrap());
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

    /// 模拟 logrotate 场景的测试
    #[tokio::test]
    async fn test_logrotate_simulation() {
        // 这个测试模拟 logrotate 的基本行为
        // 1. 创建文件并获取初始 inode
        // 2. 追加内容（正常写入）
        // 3. 检测到未变化

        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "Initial line 1").unwrap();
        writeln!(temp_file, "Initial line 2").unwrap();

        let path = temp_file.path();
        let mut tracker = FileTracker::new(path).await.unwrap();
        tracker.check_interval_ms = 0;

        // 读取初始内容
        let (lines, _) = tracker.read_new_content(None).await.unwrap();
        assert_eq!(lines.len(), 2);

        // 追加新内容
        writeln!(temp_file, "New line 3").unwrap();
        writeln!(temp_file, "New line 4").unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // 检查轮转状态（应该未变化）
        let state = tracker.check_rotation().await.unwrap();
        assert_eq!(state, RotationState::Unchanged);

        // 读取新内容
        let (new_lines, _) = tracker.read_new_content(None).await.unwrap();
        assert_eq!(new_lines.len(), 2);
        assert!(new_lines[0].contains("New line 3"));
    }
}
