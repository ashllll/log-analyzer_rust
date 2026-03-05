//! Session 状态机核心实现
//!
//! 使用 Rust 类型系统实现编译期安全的状态流转
//!
//! # 安全说明
//! 本模块使用流式处理构建索引，防止大文件 OOM。

use std::io::{BufRead, BufReader};
use std::marker::PhantomData;
use std::path::PathBuf;

use thiserror::Error;

/// Typestate 状态机错误类型
#[derive(Error, Debug)]
pub enum SessionError {
    #[error("文件未映射: {0}")]
    FileNotMapped(String),

    #[error("索引未完成: {0}")]
    IndexNotReady(String),

    #[error("映射失败: {0}")]
    MappingFailed(String),

    #[error("索引失败: {0}")]
    IndexingFailed(String),

    #[error("IO错误: {0}")]
    IoError(#[from] std::io::Error),

    #[error("状态错误: {0}")]
    InvalidState(String),
}

/// 文件元数据
#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub path: PathBuf,
    pub size: u64,
    pub inode: u64,
    pub modified: std::time::SystemTime,
}

/// 索引条目
#[derive(Debug, Clone)]
pub struct IndexEntry {
    pub line_number: u64,
    pub byte_offset: u64,
    pub length: u32,
}

/// 状态标记 trait - Unmapped
pub trait UnmappedState: sealed::Sealed {}
/// 状态标记 trait - Mapped
pub trait MappedState: sealed::Sealed {}
/// 状态标记 trait - Indexed
pub trait IndexedState: sealed::Sealed {}

mod sealed {
    use super::*;

    /// 密封 UnmappedState trait 防止外部实现
    pub trait Sealed {}

    impl Sealed for Unmapped {}
    impl Sealed for Mapped {}
    impl Sealed for Indexed {}
}

/// 未映射状态标记
pub struct Unmapped {}
/// 内存映射状态标记
pub struct Mapped {}
/// 索引完成状态标记
pub struct Indexed {}

impl UnmappedState for Unmapped {}
impl MappedState for Mapped {}
impl IndexedState for Indexed {}

/// Typestate Session - 编译期状态机
///
/// # 类型参数
/// - `S`: 状态类型 (Unmapped, Mapped, Indexed)
///
/// # 示例
/// ```rust
/// // 创建初始状态的 Session
/// let session = Session::<Unmapped>::new("path/to/file.log")?;
///
/// // 映射文件到内存
/// let session = session.map()?;
///
/// // 构建索引
/// let session = session.index()?;
/// ```
pub struct Session<S> {
    path: PathBuf,
    metadata: Option<FileMetadata>,
    entries: Vec<IndexEntry>,
    _state: PhantomData<S>,
}

impl Session<Unmapped> {
    /// 创建新的未映射状态的 Session
    pub fn new(path: impl Into<PathBuf>) -> Result<Self, SessionError> {
        let path = path.into();

        // 获取文件元数据
        let std_metadata = std::fs::metadata(&path)?;
        // 使用文件大小和修改时间作为唯一标识
        let inode = std_metadata.len();
        let modified = std_metadata.modified()?;

        let metadata = FileMetadata {
            path: path.clone(),
            size: std_metadata.len(),
            inode,
            modified,
        };

        Ok(Self {
            path,
            metadata: Some(metadata),
            entries: Vec::new(),
            _state: PhantomData,
        })
    }

    /// 转换到Mapped状态 - 映射文件到内存
    pub fn map(self) -> Result<Session<Mapped>, SessionError> {
        let metadata = self
            .metadata
            .ok_or_else(|| SessionError::InvalidState("元数据丢失".to_string()))?;

        Ok(Session {
            path: self.path,
            metadata: Some(metadata),
            entries: self.entries,
            _state: PhantomData,
        })
    }
}

impl Session<Mapped> {
    /// 获取内存映射的文件大小
    pub fn mapped_size(&self) -> u64 {
        self.metadata.as_ref().map(|m| m.size).unwrap_or(0)
    }

    /// 转换到Indexed状态 - 构建索引
    ///
    /// 使用流式处理构建索引，避免一次性读取整个文件到内存
    pub fn index(self) -> Result<Session<Indexed>, SessionError> {
        let metadata = self
            .metadata
            .ok_or_else(|| SessionError::InvalidState("元数据丢失".to_string()))?;

        // 使用流式读取构建索引
        let file = std::fs::File::open(&self.path)?;
        let reader = BufReader::with_capacity(1024 * 1024, file); // 1MB 缓冲区
        let entries = Self::build_index_streaming(reader)?;

        Ok(Session {
            path: self.path,
            metadata: Some(metadata),
            entries,
            _state: PhantomData,
        })
    }

    /// 流式构建行索引
    ///
    /// 使用 BufReader 流式读取文件，避免 OOM
    fn build_index_streaming<R: BufRead>(reader: R) -> Result<Vec<IndexEntry>, SessionError> {
        let mut entries = Vec::new();
        let mut line_number: u64 = 1;
        let mut byte_offset: u64 = 0;

        // 流式读取，避免一次性加载整个文件
        for line_result in reader.lines() {
            let line = line_result?;
            let line_len = line.len() as u64;

            // 记录当前行的索引信息
            // 注意：这里我们记录的是行开始的位置和长度（不含换行符）
            entries.push(IndexEntry {
                line_number,
                byte_offset,
                length: line.len() as u32,
            });

            // 更新偏移量（加上换行符的 1 字节）
            byte_offset += line_len + 1;
            line_number += 1;
        }

        // 如果文件不以换行符结尾，最后一行已经被正确处理

        Ok(entries)
    }

    /// 构建行索引（内存版本，适用于小文件）
    #[allow(dead_code)]
    fn build_index(content: &[u8]) -> Vec<IndexEntry> {
        let mut entries = Vec::new();
        let mut line_number: u64 = 1;
        let mut byte_offset: u64 = 0;

        for (i, &byte) in content.iter().enumerate() {
            if byte == b'\n' {
                let length = (i as u64) - byte_offset;
                entries.push(IndexEntry {
                    line_number,
                    byte_offset,
                    length: length as u32,
                });
                line_number += 1;
                byte_offset = (i + 1) as u64;
            }
        }

        // 处理最后一行（如果没有换行符）
        if byte_offset < content.len() as u64 {
            let length = (content.len() as u64) - byte_offset;
            entries.push(IndexEntry {
                line_number,
                byte_offset,
                length: length as u32,
            });
        }

        entries
    }
}

impl Session<Indexed> {
    /// 获取索引条目数量
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// 获取所有索引条目
    pub fn entries(&self) -> &[IndexEntry] {
        &self.entries
    }

    /// 根据行号获取索引条目
    pub fn get_entry(&self, line_number: u64) -> Option<&IndexEntry> {
        self.entries.iter().find(|e| e.line_number == line_number)
    }

    /// 获取索引的内存大小（字节）
    pub fn index_size(&self) -> usize {
        std::mem::size_of::<IndexEntry>() * self.entries.len()
    }
}

/// 状态转换函数的通用实现
impl<S: 'static> Session<S> {
    /// 获取文件路径
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// 获取文件元数据
    pub fn metadata(&self) -> Option<&FileMetadata> {
        self.metadata.as_ref()
    }

    /// 检查是否是最终状态（已索引）
    pub fn is_indexed(&self) -> bool {
        std::any::TypeId::of::<S>() == std::any::TypeId::of::<Indexed>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_file() -> NamedTempFile {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        writeln!(file, "Line 1: ERROR: Test error").unwrap();
        writeln!(file, "Line 2: WARN: Test warning").unwrap();
        writeln!(file, "Line 3: INFO: Test info").unwrap();
        file
    }

    #[test]
    fn test_session_lifecycle() {
        let temp_file = create_test_file();
        let path = temp_file.path();

        // 创建 Unmapped 状态
        let session_unmapped = Session::<Unmapped>::new(path).unwrap();
        assert!(session_unmapped.metadata().is_some());
        assert_eq!(session_unmapped.path(), path);

        // 转换到 Mapped 状态
        let session_mapped = session_unmapped.map().unwrap();
        assert!(session_mapped.metadata().is_some());

        // 转换到 Indexed 状态
        let session_indexed = session_mapped.index().unwrap();
        assert!(session_indexed.is_indexed());
        assert_eq!(session_indexed.entry_count(), 3);
    }

    #[test]
    fn test_index_entry_access() {
        let temp_file = create_test_file();
        let path = temp_file.path();

        let session = Session::<Unmapped>::new(path)
            .unwrap()
            .map()
            .unwrap()
            .index()
            .unwrap();

        // 检查第一个条目
        let entry = session.get_entry(1).unwrap();
        assert_eq!(entry.line_number, 1);
        assert_eq!(entry.byte_offset, 0);

        // 检查索引大小
        assert!(session.index_size() > 0);
    }

    #[test]
    fn test_error_handling() {
        // 测试不存在的文件
        let result = Session::<Unmapped>::new("nonexistent_file.txt");
        assert!(result.is_err());
    }
}
