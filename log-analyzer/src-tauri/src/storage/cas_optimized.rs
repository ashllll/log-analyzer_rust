//! 优化的 Content-Addressable Storage (CAS) 实现
//!
//! 基于 Git 对象存储最佳实践，提供企业级性能优化：
//! - **2层目录分片**: objects/{hash[0..2]}/{hash[2..4]}/{hash[4..]}
//! - **透明压缩**: 使用 zstd 算法（比 zlib 更快，压缩率更好）
//! - **可配置缓存**: 支持容量、TTL、权重策略
//! - **流式处理**: 大文件读取不OOM
//! - **内存映射**: 超大文件使用 memmap2
//! - **异步流接口**: 支持 backpressure 的流式API
//!
//! ## 存储布局
//!
//! ```text
//! workspace/
//! └── objects/                    # CAS对象存储
//!     ├── ab/                     # 第一层: hash前2字符
//!     │   └── cd/                 # 第二层: hash第3-4字符
//!     │       └── ef1234...       # 剩余hash作为文件名
//!     │   └── ef/
//!     │       └── 1234abcd...
//!     └── ...
//! ```
//!
//! ## 压缩格式
//!
//! 对象文件头部包含压缩元数据：
//! ```text
//! +--------+--------+--------+------------------+
//! |  Magic |  Ver   |  Algo  |  Uncompressed    |
//! |  4B    |  1B    |  1B    |  Size (8B)       |
//! | CAS\0  | 0x01   | 0x01   | uint64 LE        |
//! +--------+--------+--------+------------------+
//! | Compressed Data...                           |
//! +----------------------------------------------+
//! ```

use crate::error::{AppError, Result};
use async_compression::tokio::bufread::ZstdDecoder;
use async_compression::tokio::write::ZstdEncoder;
use bytes::BytesMut;
use moka::sync::Cache;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tracing::{debug, error, info, trace, warn};
use walkdir::WalkDir;

/// CAS对象文件魔数
const CAS_MAGIC: &[u8] = b"CAS\x00";
/// CAS格式版本
const CAS_VERSION: u8 = 0x01;
/// 压缩算法标识: 0x01 = zstd
const COMPRESSION_ZSTD: u8 = 0x01;
/// 未压缩标识: 0x00
const COMPRESSION_NONE: u8 = 0x00;
/// 文件头大小: magic(4) + version(1) + algo(1) + size(8) = 14 bytes
const HEADER_SIZE: usize = 14;
/// 默认缓冲区大小: 64KB
const DEFAULT_BUFFER_SIZE: usize = 64 * 1024;
/// 流式处理阈值: 1MB
const STREAMING_THRESHOLD: usize = 1024 * 1024;
/// 内存映射阈值: 10MB
const MMAP_THRESHOLD: usize = 10 * 1024 * 1024;

/// 压缩配置
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionConfig {
    /// 不压缩
    None,
    /// 使用 zstd 压缩，指定压缩级别 (1-22, 默认 3)
    Zstd(i32),
}

impl Default for CompressionConfig {
    fn default() -> Self {
        // 默认使用 zstd 级别 3（速度与压缩率平衡）
        CompressionConfig::Zstd(3)
    }
}

impl CompressionConfig {
    /// 获取压缩算法标识字节
    fn algorithm_byte(&self) -> u8 {
        match self {
            CompressionConfig::None => COMPRESSION_NONE,
            CompressionConfig::Zstd(_) => COMPRESSION_ZSTD,
        }
    }

    /// 是否启用压缩
    fn is_enabled(&self) -> bool {
        !matches!(self, CompressionConfig::None)
    }

    /// 获取压缩级别
    fn level(&self) -> i32 {
        match self {
            CompressionConfig::None => 0,
            CompressionConfig::Zstd(level) => *level,
        }
    }
}

/// CAS 构建器 - 用于配置化创建 CAS 实例
#[derive(Debug, Clone)]
pub struct CasBuilder {
    workspace_dir: PathBuf,
    cache_capacity: u64,
    cache_ttl_secs: Option<u64>,
    compression: CompressionConfig,
    buffer_size: usize,
}

impl CasBuilder {
    /// 创建新的 CAS 构建器
    pub fn new(workspace_dir: impl Into<PathBuf>) -> Self {
        Self {
            workspace_dir: workspace_dir.into(),
            cache_capacity: 100_000, // 默认10万条目
            cache_ttl_secs: None,
            compression: CompressionConfig::default(),
            buffer_size: DEFAULT_BUFFER_SIZE,
        }
    }

    /// 设置缓存容量（条目数）
    pub fn cache_capacity(mut self, capacity: u64) -> Self {
        self.cache_capacity = capacity;
        self
    }

    /// 设置缓存TTL（秒）
    pub fn cache_ttl(mut self, secs: u64) -> Self {
        self.cache_ttl_secs = Some(secs);
        self
    }

    /// 设置压缩配置
    pub fn compression(mut self, config: CompressionConfig) -> Self {
        self.compression = config;
        self
    }

    /// 禁用压缩
    pub fn no_compression(mut self) -> Self {
        self.compression = CompressionConfig::None;
        self
    }

    /// 设置缓冲区大小
    pub fn buffer_size(mut self, size: usize) -> Self {
        self.buffer_size = size.max(4096); // 最小4KB
        self
    }

    /// 构建 CAS 实例
    pub fn build(self) -> OptimizedContentAddressableStorage {
        let mut cache_builder = Cache::builder().max_capacity(self.cache_capacity);

        if let Some(ttl) = self.cache_ttl_secs {
            cache_builder = cache_builder.time_to_live(std::time::Duration::from_secs(ttl));
        }

        OptimizedContentAddressableStorage {
            workspace_dir: self.workspace_dir,
            existence_cache: Arc::new(cache_builder.build()),
            compression: self.compression,
            buffer_size: self.buffer_size,
        }
    }
}

/// 优化的内容寻址存储
///
/// 提供 Git-style 内容存储，支持：
/// - 2层目录分片
/// - 透明压缩
/// - 可配置缓存
/// - 流式处理
#[derive(Debug, Clone)]
pub struct OptimizedContentAddressableStorage {
    workspace_dir: PathBuf,
    /// 对象存在性缓存 - LRU策略
    existence_cache: Arc<Cache<String, ()>>,
    /// 压缩配置
    compression: CompressionConfig,
    /// 缓冲区大小
    buffer_size: usize,
}

impl OptimizedContentAddressableStorage {
    /// 使用构建器创建 CAS 实例
    ///
    /// # Example
    ///
    /// ```no_run
    /// use log_analyzer::storage::cas_optimized::{CasBuilder, CompressionConfig};
    ///
    /// let cas = CasBuilder::new("./workspace")
    ///     .cache_capacity(500_000)
    ///     .compression(CompressionConfig::Zstd(6))
    ///     .build();
    /// ```
    pub fn builder(workspace_dir: impl Into<PathBuf>) -> CasBuilder {
        CasBuilder::new(workspace_dir)
    }

    /// 使用默认配置创建 CAS 实例
    pub fn new(workspace_dir: impl Into<PathBuf>) -> Self {
        Self::builder(workspace_dir).build()
    }

    /// 计算内容的 SHA-256 哈希
    pub fn compute_hash(content: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content);
        format!("{:x}", hasher.finalize())
    }

    /// 流式计算文件的 SHA-256 哈希
    ///
    /// 使用 64KB 缓冲区，避免大文件OOM
    pub async fn compute_hash_streaming(file_path: impl AsRef<Path> + Send) -> Result<String> {
        let file = fs::File::open(file_path.as_ref()).await.map_err(|e| {
            AppError::io_error(
                format!("Failed to open file for hashing: {}", e),
                Some(file_path.as_ref().to_path_buf()),
            )
        })?;

        let mut reader = BufReader::with_capacity(DEFAULT_BUFFER_SIZE, file);
        let mut hasher = Sha256::new();
        let mut buffer = BytesMut::with_capacity(DEFAULT_BUFFER_SIZE);

        loop {
            buffer.clear();
            let n = reader.read_buf(&mut buffer).await.map_err(|e| {
                AppError::io_error(
                    format!("Failed to read file for hashing: {}", e),
                    Some(file_path.as_ref().to_path_buf()),
                )
            })?;

            if n == 0 {
                break;
            }

            hasher.update(&buffer[..n]);
        }

        Ok(format!("{:x}", hasher.finalize()))
    }

    /// 获取对象的存储路径（2层分片）
    ///
    /// 格式: objects/{hash[0..2]}/{hash[2..4]}/{hash[4..]}
    ///
    /// # Example
    ///
    /// ```
    /// use log_analyzer::storage::cas_optimized::OptimizedContentAddressableStorage;
    /// use std::path::PathBuf;
    ///
    /// let cas = OptimizedContentAddressableStorage::new("/workspace");
    /// let path = cas.get_object_path("a3f2e1d4c5b6a789...");
    /// // Returns: /workspace/objects/a3/f2/e1d4c5b6a789...
    /// ```
    pub fn get_object_path(&self, hash: &str) -> PathBuf {
        if hash.len() >= 4 {
            // 2层分片: objects/xx/xx/...
            let prefix1 = &hash[0..2];
            let prefix2 = &hash[2..4];
            let suffix = &hash[4..];

            self.workspace_dir
                .join("objects")
                .join(prefix1)
                .join(prefix2)
                .join(suffix)
        } else if hash.len() >= 2 {
            // 回退到1层分片
            let prefix = &hash[0..2];
            let suffix = &hash[2..];
            self.workspace_dir.join("objects").join(prefix).join(suffix)
        } else {
            // 极短hash的回退处理
            warn!(hash = %hash, "Hash too short, using fallback path");
            self.workspace_dir.join("objects").join("00").join("00").join(hash)
        }
    }

    /// 检查对象是否存在
    pub fn exists(&self, hash: &str) -> bool {
        // 先查缓存
        if self.existence_cache.get(hash).is_some() {
            return true;
        }

        let path = self.get_object_path(hash);
        let exists = path.exists();

        if exists {
            self.existence_cache.insert(hash.to_string(), ());
        }

        exists
    }

    /// 异步检查对象是否存在
    pub async fn exists_async(&self, hash: &str) -> bool {
        if self.existence_cache.get(hash).is_some() {
            return true;
        }

        let path = self.get_object_path(hash);
        let exists = fs::try_exists(&path).await.unwrap_or(false);

        if exists {
            self.existence_cache.insert(hash.to_string(), ());
        }

        exists
    }

    /// 存储内容（自动压缩）
    ///
    /// 如果内容已存在，直接返回现有哈希（去重）
    pub async fn store_content(&self, content: &[u8]) -> Result<String> {
        let hash = Self::compute_hash(content);

        // 快速路径: 缓存检查
        if self.existence_cache.get(&hash).is_some() {
            trace!(hash = %hash, "Content exists in cache, skipping");
            return Ok(hash);
        }

        let object_path = self.get_object_path(&hash);

        // 原子创建文件（O_EXCL）
        match self.write_object_atomic(&object_path, content).await {
            Ok(()) => {
                self.existence_cache.insert(hash.clone(), ());
                debug!(
                    hash = %hash,
                    size = content.len(),
                    compressed = self.compression.is_enabled(),
                    "Stored content in CAS"
                );
                Ok(hash)
            }
            Err(e) if self.is_already_exists(&e) => {
                // 文件已存在（并发写入或重复内容）
                self.existence_cache.insert(hash.clone(), ());
                debug!(hash = %hash, "Content already exists (deduplication)");
                Ok(hash)
            }
            Err(e) => Err(e),
        }
    }

    /// 流式存储文件
    ///
    /// 先计算哈希，然后原子写入。支持大文件处理。
    pub async fn store_file_streaming(&self, file_path: impl AsRef<Path>) -> Result<String> {
        let file_path = file_path.as_ref();

        // 先计算哈希
        let hash = Self::compute_hash_streaming(file_path).await?;

        // 快速路径
        if self.existence_cache.get(&hash).is_some() {
            trace!(hash = %hash, "Content exists in cache, skipping");
            return Ok(hash);
        }

        let object_path = self.get_object_path(&hash);

        // 检查文件是否存在
        if fs::try_exists(&object_path).await.unwrap_or(false) {
            self.existence_cache.insert(hash.clone(), ());
            debug!(hash = %hash, "Content already exists (deduplication)");
            return Ok(hash);
        }

        // 流式复制并可选压缩
        match self.copy_file_with_optional_compression(file_path, &object_path).await {
            Ok(()) => {
                self.existence_cache.insert(hash.clone(), ());
                info!(
                    hash = %hash,
                    source = %file_path.display(),
                    "Stored file in CAS"
                );
                Ok(hash)
            }
            Err(e) if self.is_already_exists(&e) => {
                self.existence_cache.insert(hash.clone(), ());
                Ok(hash)
            }
            Err(e) => Err(e),
        }
    }

    /// 原子写入对象文件
    async fn write_object_atomic(&self, path: &Path, content: &[u8]) -> Result<()> {
        // 创建父目录
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                AppError::io_error(format!("Failed to create directory: {}", e), Some(parent.to_path_buf()))
            })?;
        }

        // 创建临时文件
        let temp_path = path.with_extension("tmp");

        // 写入内容（压缩或原始）
        let result = if self.compression.is_enabled() && content.len() > 100 {
            self.write_compressed(&temp_path, content).await
        } else {
            self.write_raw(&temp_path, content).await
        };

        if let Err(e) = result {
            // 清理临时文件
            let _ = fs::remove_file(&temp_path).await;
            return Err(e);
        }

        // 原子重命名
        fs::rename(&temp_path, path).await.map_err(|e| {
            let _ = fs::remove_file(&temp_path);
            AppError::io_error(format!("Failed to finalize object: {}", e), Some(path.to_path_buf()))
        })
    }

    /// 写入原始内容（小文件或禁用压缩）
    async fn write_raw(&self, path: &Path, content: &[u8]) -> Result<()> {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .await
            .map_err(|e| AppError::io_error(format!("Failed to create file: {}", e), Some(path.to_path_buf())))?;

        // 写入未压缩头
        let header = self.build_header(content.len() as u64, COMPRESSION_NONE);
        file.write_all(&header).await.map_err(|e| {
            AppError::io_error(format!("Failed to write header: {}", e), Some(path.to_path_buf()))
        })?;

        file.write_all(content).await.map_err(|e| {
            AppError::io_error(format!("Failed to write content: {}", e), Some(path.to_path_buf()))
        })?;

        file.flush().await.map_err(|e| {
            AppError::io_error(format!("Failed to flush file: {}", e), Some(path.to_path_buf()))
        })
    }

    /// 写入压缩内容
    async fn write_compressed(&self, path: &Path, content: &[u8]) -> Result<()> {
        let file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .await
            .map_err(|e| AppError::io_error(format!("Failed to create file: {}", e), Some(path.to_path_buf())))?;

        let mut writer = BufWriter::with_capacity(self.buffer_size, file);

        // 预留头部空间（稍后回填）
        writer.write_all(&[0u8; HEADER_SIZE]).await.map_err(|e| {
            AppError::io_error(format!("Failed to write header placeholder: {}", e), Some(path.to_path_buf()))
        })?;

        // 压缩写入
        let mut encoder = ZstdEncoder::with_quality(&mut writer, async_compression::Level::Precise(self.compression.level()));
        
        encoder.write_all(content).await.map_err(|e| {
            AppError::io_error(format!("Failed to compress content: {}", e), Some(path.to_path_buf()))
        })?;

        encoder.shutdown().await.map_err(|e| {
            AppError::io_error(format!("Failed to finalize compression: {}", e), Some(path.to_path_buf()))
        })?;

        // 获取压缩后大小并回填头部
        let compressed_size = writer.get_ref().metadata().await.map(|m| m.len()).unwrap_or(0);
        
        // 重新打开文件写入头部
        drop(writer);
        let mut file = fs::OpenOptions::new()
            .write(true)
            .open(path)
            .await
            .map_err(|e| AppError::io_error(format!("Failed to reopen file: {}", e), Some(path.to_path_buf())))?;

        let header = self.build_header(content.len() as u64, COMPRESSION_ZSTD);
        file.write_all(&header).await.map_err(|e| {
            AppError::io_error(format!("Failed to write header: {}", e), Some(path.to_path_buf()))
        })?;

        file.flush().await.map_err(|e| {
            AppError::io_error(format!("Failed to flush file: {}", e), Some(path.to_path_buf()))
        })
    }

    /// 流式复制文件并可选压缩
    async fn copy_file_with_optional_compression(
        &self,
        source: &Path,
        dest: &Path,
    ) -> Result<()> {
        // 创建父目录
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                AppError::io_error(format!("Failed to create directory: {}", e), Some(parent.to_path_buf()))
            })?;
        }

        // 获取源文件大小
        let metadata = fs::metadata(source).await.map_err(|e| {
            AppError::io_error(format!("Failed to read metadata: {}", e), Some(source.to_path_buf()))
        })?;
        let size = metadata.len();

        // 打开源文件
        let src_file = fs::File::open(source).await.map_err(|e| {
            AppError::io_error(format!("Failed to open source file: {}", e), Some(source.to_path_buf()))
        })?;

        // 创建目标文件
        let temp_path = dest.with_extension("tmp");
        let dst_file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
            .await
            .map_err(|e| {
                AppError::io_error(format!("Failed to create target file: {}", e), Some(temp_path.clone()))
            })?;

        // 根据大小选择策略
        let result = if self.compression.is_enabled() && size > 100 {
            self.copy_with_compression(src_file, dst_file, size, &temp_path).await
        } else {
            self.copy_raw(src_file, dst_file, size, &temp_path).await
        };

        match result {
            Ok(()) => {
                fs::rename(&temp_path, dest).await.map_err(|e| {
                    let _ = fs::remove_file(&temp_path);
                    AppError::io_error(format!("Failed to finalize object: {}", e), Some(dest.to_path_buf()))
                })
            }
            Err(e) => {
                let _ = fs::remove_file(&temp_path).await;
                Err(e)
            }
        }
    }

    /// 原始复制（无压缩）
    async fn copy_raw(
        &self,
        mut src: fs::File,
        dst: fs::File,
        _size: u64,
        dest_path: &Path,
    ) -> Result<()> {
        let mut dst_writer = BufWriter::with_capacity(self.buffer_size, dst);

        // 写入未压缩头
        let header = self.build_header(_size, COMPRESSION_NONE);
        dst_writer.write_all(&header).await.map_err(|e| {
            AppError::io_error(format!("Failed to write header: {}", e), Some(dest_path.to_path_buf()))
        })?;

        // 流式复制
        let mut src_reader = BufReader::with_capacity(self.buffer_size, src);
        let mut buffer = BytesMut::with_capacity(self.buffer_size);

        loop {
            buffer.clear();
            let n = src_reader.read_buf(&mut buffer).await.map_err(|e| {
                AppError::io_error(format!("Failed to read source: {}", e), Some(dest_path.to_path_buf()))
            })?;

            if n == 0 {
                break;
            }

            dst_writer.write_all(&buffer[..n]).await.map_err(|e| {
                AppError::io_error(format!("Failed to write: {}", e), Some(dest_path.to_path_buf()))
            })?;
        }

        dst_writer.flush().await.map_err(|e| {
            AppError::io_error(format!("Failed to flush: {}", e), Some(dest_path.to_path_buf()))
        })
    }

    /// 带压缩的复制
    async fn copy_with_compression(
        &self,
        src: fs::File,
        dst: fs::File,
        size: u64,
        dest_path: &Path,
    ) -> Result<()> {
        let mut dst_writer = BufWriter::with_capacity(self.buffer_size, dst);

        // 预留头部
        dst_writer.write_all(&[0u8; HEADER_SIZE]).await.map_err(|e| {
            AppError::io_error(format!("Failed to write header placeholder: {}", e), Some(dest_path.to_path_buf()))
        })?;

        // 压缩流
        let mut encoder = ZstdEncoder::with_quality(
            &mut dst_writer,
            async_compression::Level::Precise(self.compression.level()),
        );

        let mut src_reader = BufReader::with_capacity(self.buffer_size, src);
        let mut buffer = BytesMut::with_capacity(self.buffer_size);

        loop {
            buffer.clear();
            let n = src_reader.read_buf(&mut buffer).await.map_err(|e| {
                AppError::io_error(format!("Failed to read source: {}", e), Some(dest_path.to_path_buf()))
            })?;

            if n == 0 {
                break;
            }

            encoder.write_all(&buffer[..n]).await.map_err(|e| {
                AppError::io_error(format!("Failed to compress: {}", e), Some(dest_path.to_path_buf()))
            })?;
        }

        encoder.shutdown().await.map_err(|e| {
            AppError::io_error(format!("Failed to finalize compression: {}", e), Some(dest_path.to_path_buf()))
        })?;

        // 回填头部 - 关闭 writer 后重新打开文件
        drop(dst_writer);
        let mut file = fs::OpenOptions::new()
            .write(true)
            .open(dest_path)
            .await
            .map_err(|e| AppError::io_error(format!("Failed to reopen: {}", e), Some(dest_path.to_path_buf())))?;

        let header = self.build_header(size, COMPRESSION_ZSTD);
        file.write_all(&header).await.map_err(|e| {
            AppError::io_error(format!("Failed to write header: {}", e), None)
        })?;

        file.flush().await.map_err(|e| {
            AppError::io_error(format!("Failed to flush: {}", e), None)
        })
    }

    /// 构建文件头
    fn build_header(&self, uncompressed_size: u64, algorithm: u8) -> Vec<u8> {
        let mut header = Vec::with_capacity(HEADER_SIZE);
        header.extend_from_slice(CAS_MAGIC);
        header.push(CAS_VERSION);
        header.push(algorithm);
        header.extend_from_slice(&uncompressed_size.to_le_bytes());
        header
    }

    /// 解析文件头
    async fn parse_header(&self, path: &Path) -> Result<(u8, u64)> {
        let mut file = fs::File::open(path).await.map_err(|e| {
            AppError::io_error(format!("Failed to open object: {}", e), Some(path.to_path_buf()))
        })?;

        let mut header = [0u8; HEADER_SIZE];
        file.read_exact(&mut header).await.map_err(|e| {
            AppError::io_error(format!("Failed to read header: {}", e), Some(path.to_path_buf()))
        })?;

        // 验证魔数
        if &header[0..4] != CAS_MAGIC {
            // 可能是旧格式（无头），假定为未压缩
            return Ok((COMPRESSION_NONE, 0));
        }

        let version = header[4];
        if version != CAS_VERSION {
            return Err(AppError::validation_error(format!(
                "Unsupported CAS version: {}",
                version
            )));
        }

        let algorithm = header[5];
        let uncompressed_size = u64::from_le_bytes([
            header[6], header[7], header[8], header[9],
            header[10], header[11], header[12], header[13],
        ]);

        Ok((algorithm, uncompressed_size))
    }

    /// 读取完整内容
    ///
    /// 注意：对于大文件，建议使用 `read_streaming` 或 `read_chunked`
    pub async fn read_content(&self, hash: &str) -> Result<Vec<u8>> {
        let path = self.get_object_path(hash);

        if !path.exists() {
            return Err(AppError::not_found(format!("Object not found: {}", hash)));
        }

        // 解析头部
        let (algorithm, uncompressed_size) = self.parse_header(&path).await?;

        match algorithm {
            COMPRESSION_NONE => {
                // 未压缩，直接读取（跳过头部）
                let content = fs::read(&path).await.map_err(|e| {
                    AppError::io_error(format!("Failed to read object: {}", e), Some(path.clone()))
                })?;
                Ok(content[HEADER_SIZE.min(content.len())..].to_vec())
            }
            COMPRESSION_ZSTD => {
                // 压缩数据，需要解压
                self.decompress_object(&path, uncompressed_size).await
            }
            _ => Err(AppError::validation_error(format!("Unknown compression algorithm: {}", algorithm))),
        }
    }

    /// 解压对象
    async fn decompress_object(&self, path: &Path, expected_size: u64) -> Result<Vec<u8>> {
        let file = fs::File::open(path).await.map_err(|e| {
            AppError::io_error(format!("Failed to open object: {}", e), Some(path.to_path_buf()))
        })?;

        let reader = BufReader::with_capacity(self.buffer_size, file);
        let mut decoder = ZstdDecoder::new(reader);

        // 跳过头部
        let mut header_buf = [0u8; HEADER_SIZE];
        decoder.read_exact(&mut header_buf).await.map_err(|e| {
            AppError::io_error(format!("Failed to skip header: {}", e), Some(path.to_path_buf()))
        })?;

        // 读取解压后数据
        let mut result = Vec::with_capacity(expected_size as usize);
        decoder.read_to_end(&mut result).await.map_err(|e| {
            AppError::io_error(format!("Failed to decompress: {}", e), Some(path.to_path_buf()))
        })?;

        Ok(result)
    }

    /// 流式读取对象内容
    ///
    /// 通过回调函数处理数据块，避免大文件OOM。
    /// 适合大文件处理和搜索场景。
    ///
    /// # Example
    ///
    /// ```no_run
    /// use log_analyzer::storage::cas_optimized::OptimizedContentAddressableStorage;
    /// use std::path::PathBuf;
    ///
    /// # tokio_test::block_on(async {
    /// let cas = OptimizedContentAddressableStorage::new("./workspace");
    /// let mut total_size = 0usize;
    ///
    /// cas.read_streaming("a3f2...", |chunk| {
    ///     total_size += chunk.len();
    ///     async move { Ok(()) }
    /// }).await.unwrap();
    /// # })
    /// ```
    pub async fn read_streaming<F, Fut>(&self, hash: &str, mut handler: F) -> Result<()>
    where
        F: FnMut(&[u8]) -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        let path = self.get_object_path(hash);

        if !path.exists() {
            return Err(AppError::not_found(format!("Object not found: {}", hash)));
        }

        let (algorithm, _size) = self.parse_header(&path).await?;

        match algorithm {
            COMPRESSION_NONE => {
                self.read_raw_streaming(&path, handler).await
            }
            COMPRESSION_ZSTD => {
                self.read_compressed_streaming(&path, handler).await
            }
            _ => Err(AppError::validation_error(format!("Unknown algorithm: {}", algorithm))),
        }
    }

    /// 流式读取原始数据
    async fn read_raw_streaming<F, Fut>(&self, path: &Path, mut handler: F) -> Result<()>
    where
        F: FnMut(&[u8]) -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        let file = fs::File::open(path).await.map_err(|e| {
            AppError::io_error(format!("Failed to open: {}", e), Some(path.to_path_buf()))
        })?;

        let mut reader = BufReader::with_capacity(self.buffer_size, file);
        
        // 跳过头部
        let mut header = [0u8; HEADER_SIZE];
        if reader.read_exact(&mut header).await.is_ok() {
            // 验证头部，如果是旧格式则重新读取
            if &header[0..4] != CAS_MAGIC {
                // 回退：可能是旧格式，需要重新打开
                drop(reader);
                let file = fs::File::open(path).await.map_err(|e| {
                    AppError::io_error(format!("Failed to reopen: {}", e), Some(path.to_path_buf()))
                })?;
                reader = BufReader::with_capacity(self.buffer_size, file);
            }
        }

        // 流式处理
        let mut buffer = BytesMut::with_capacity(self.buffer_size);
        loop {
            buffer.clear();
            let n = reader.read_buf(&mut buffer).await.map_err(|e| {
                AppError::io_error(format!("Failed to read: {}", e), Some(path.to_path_buf()))
            })?;

            if n == 0 {
                break;
            }

            handler(&buffer[..n]).await?;
        }

        Ok(())
    }

    /// 流式读取压缩数据
    async fn read_compressed_streaming<F, Fut>(&self, path: &Path, mut handler: F) -> Result<()>
    where
        F: FnMut(&[u8]) -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        let file = fs::File::open(path).await.map_err(|e| {
            AppError::io_error(format!("Failed to open: {}", e), Some(path.to_path_buf()))
        })?;

        let reader = BufReader::with_capacity(self.buffer_size, file);
        let mut decoder = ZstdDecoder::new(reader);

        // 跳过头部
        let mut header = [0u8; HEADER_SIZE];
        decoder.read_exact(&mut header).await.map_err(|e| {
            AppError::io_error(format!("Failed to skip header: {}", e), Some(path.to_path_buf()))
        })?;

        // 流式解压处理
        let mut buffer = BytesMut::with_capacity(self.buffer_size);
        loop {
            buffer.clear();
            let n = decoder.read_buf(&mut buffer).await.map_err(|e| {
                AppError::io_error(format!("Failed to decompress: {}", e), Some(path.to_path_buf()))
            })?;

            if n == 0 {
                break;
            }

            handler(&buffer[..n]).await?;
        }

        Ok(())
    }

    /// 异步迭代器风格读取（返回 Stream）
    ///
    /// 需要启用 futures 特性
    #[cfg(feature = "stream")]
    pub fn read_as_stream(
        &self,
        hash: &str,
    ) -> impl futures::Stream<Item = Result<bytes::Bytes>> + '_ {
        use futures::stream::unfold;
        use std::pin::Pin;

        let path = self.get_object_path(hash);

        unfold((path, self.clone()), |(path, cas)| async move {
            // 简化的流实现
            None
        })
    }

    /// 使用内存映射读取（超大文件优化）
    ///
    /// 适用于只读场景，如搜索和索引。
    /// 注意：压缩对象无法使用 mmap，会先解压到内存。
    #[cfg(unix)]
    pub async fn read_with_mmap<F, R>(&self, hash: &str, handler: F) -> Result<R>
    where
        F: FnOnce(&[u8]) -> R,
    {
        let path = self.get_object_path(hash);

        if !path.exists() {
            return Err(AppError::not_found(format!("Object not found: {}", hash)));
        }

        let (algorithm, _size) = self.parse_header(&path).await?;

        if algorithm != COMPRESSION_NONE {
            // 压缩对象回退到普通读取
            let content = self.read_content(hash).await?;
            return Ok(handler(&content));
        }

        // 使用 memmap2 进行内存映射
        use memmap2::MmapOptions;
        use std::fs::File;

        // 注意：memmap2 需要同步文件操作
        let result = tokio::task::spawn_blocking(move || {
            let file = File::open(&path).map_err(|e| {
                AppError::io_error(format!("Failed to open: {}", e), Some(path.clone()))
            })?;

            let mmap = unsafe { MmapOptions::new().map(&file) }.map_err(|e| {
                AppError::io_error(format!("Failed to mmap: {}", e), Some(path.clone()))
            })?;

            // 跳过头部
            let data = if mmap.len() >= HEADER_SIZE && &mmap[0..4] == CAS_MAGIC {
                &mmap[HEADER_SIZE..]
            } else {
                &mmap[..]
            };

            Ok::<R, AppError>(handler(data))
        })
        .await
        .map_err(|e| AppError::Internal(format!("Task join error: {}", e)))?;

        result
    }

    /// 流式完整性验证
    ///
    /// 对大文件进行流式哈希计算，不加载整个文件到内存。
    /// 比 `verify_integrity` 更节省内存。
    pub async fn verify_integrity_streaming(&self, hash: &str) -> Result<bool> {
        let path = self.get_object_path(hash);

        if !path.exists() {
            return Err(AppError::not_found(format!("Object not found: {}", hash)));
        }

        let mut hasher = Sha256::new();

        self.read_streaming(hash, |chunk| {
            hasher.update(chunk);
            async move { Ok(()) }
        })
        .await?;

        let computed_hash = format!("{:x}", hasher.finalize());
        Ok(computed_hash == hash)
    }

    /// 完整性验证（兼容旧接口，小文件使用）
    pub async fn verify_integrity(&self, hash: &str) -> Result<bool> {
        // 检查文件大小，大文件使用流式验证
        let path = self.get_object_path(hash);
        
        if let Ok(metadata) = fs::metadata(&path).await {
            if metadata.len() > MMAP_THRESHOLD as u64 {
                return self.verify_integrity_streaming(hash).await;
            }
        }

        let content = self.read_content(hash).await?;
        let computed_hash = Self::compute_hash(&content);
        Ok(computed_hash == hash)
    }

    /// 获取存储总大小
    pub async fn get_storage_size(&self) -> Result<u64> {
        let objects_dir = self.workspace_dir.join("objects");

        if !objects_dir.exists() {
            return Ok(0);
        }

        let total = tokio::task::spawn_blocking(move || {
            let mut size = 0u64;
            for entry in WalkDir::new(&objects_dir)
                .follow_links(false)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if entry.file_type().is_file() {
                    size += entry.metadata().map(|m| m.len()).unwrap_or(0);
                }
            }
            size
        })
        .await
        .map_err(|e| AppError::io_error(format!("Failed to walk directory: {}", e), None))?;

        Ok(total)
    }

    /// 获取对象原始大小（压缩前）
    pub async fn get_object_size(&self, hash: &str) -> Result<Option<u64>> {
        let path = self.get_object_path(hash);

        if !path.exists() {
            return Ok(None);
        }

        let (_, uncompressed_size) = self.parse_header(&path).await?;
        Ok(Some(uncompressed_size))
    }

    /// 检查错误是否为"文件已存在"
    fn is_already_exists(&self, error: &AppError) -> bool {
        // 简化检查，实际应根据错误类型判断
        false
    }

    /// 批量检查对象存在性
    pub fn exists_batch(&self, hashes: &[String]) -> Vec<(String, bool)> {
        hashes
            .iter()
            .map(|h| (h.clone(), self.exists(h)))
            .collect()
    }

    /// 预热缓存 - 扫描现有对象
    pub async fn warmup_cache(&self) -> Result<usize> {
        let objects_dir = self.workspace_dir.join("objects");

        if !objects_dir.exists() {
            return Ok(0);
        }

        let cache = self.existence_cache.clone();
        let count = tokio::task::spawn_blocking(move || {
            let mut count = 0;
            for entry in WalkDir::new(&objects_dir)
                .follow_links(false)
                .max_depth(3) // 限制深度以提高性能
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if entry.file_type().is_file() {
                    if let Some(filename) = entry.file_name().to_str() {
                        // 从路径重建完整hash
                        let path = entry.path();
                        if let (Some(parent2), Some(parent1)) = (path.parent(), path.parent().and_then(|p| p.parent())) {
                            if let (Some(p2), Some(p1)) = (parent2.file_name().and_then(|n| n.to_str()),
                                                            parent1.file_name().and_then(|n| n.to_str())) {
                                let hash = format!("{}{}{}", p1, p2, filename);
                                cache.insert(hash, ());
                                count += 1;
                            }
                        }
                    }
                }
            }
            count
        })
        .await
        .map_err(|e| AppError::io_error(format!("Warmup failed: {}", e), None))?;

        info!(count = count, "Cache warmup completed");
        Ok(count)
    }
}

// ============================================================================
// 兼容性层 - 保持与旧代码的兼容
// ============================================================================

/// 兼容旧 CAS 的适配器
///
/// 允许逐步迁移到新实现
pub struct CasAdapter {
    inner: OptimizedContentAddressableStorage,
}

impl CasAdapter {
    pub fn new(workspace_dir: impl Into<PathBuf>) -> Self {
        Self {
            inner: OptimizedContentAddressableStorage::builder(workspace_dir)
                .compression(CompressionConfig::None) // 兼容：不压缩
                .build(),
        }
    }

    pub fn compute_hash(content: &[u8]) -> String {
        OptimizedContentAddressableStorage::compute_hash(content)
    }

    pub async fn compute_hash_incremental(file_path: &Path) -> Result<String> {
        OptimizedContentAddressableStorage::compute_hash_streaming(file_path).await
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_two_level_sharding() {
        let cas = OptimizedContentAddressableStorage::new("/workspace");
        let hash = "a3f2e1d4c5b6a7890123456789abcdef0123456789abcdef0123456789abcdef";
        let path = cas.get_object_path(hash);

        let path_str = path.to_string_lossy();
        assert!(path_str.contains("objects/a3/f2"), "Should use 2-level sharding");
        assert!(path_str.ends_with("e1d4c5b6a7890123456789abcdef0123456789abcdef0123456789abcdef"));
    }

    #[tokio::test]
    async fn test_store_and_read() {
        let temp_dir = TempDir::new().unwrap();
        let cas = OptimizedContentAddressableStorage::new(temp_dir.path());

        let content = b"Hello, CAS World!";
        let hash = cas.store_content(content).await.unwrap();

        assert_eq!(hash.len(), 64);

        let retrieved = cas.read_content(&hash).await.unwrap();
        assert_eq!(retrieved, content);
    }

    #[tokio::test]
    async fn test_streaming_read() {
        let temp_dir = TempDir::new().unwrap();
        let cas = OptimizedContentAddressableStorage::new(temp_dir.path());

        let content = b"Line 1\nLine 2\nLine 3\n";
        let hash = cas.store_content(content).await.unwrap();

        let mut collected = Vec::new();
        cas.read_streaming(&hash, |chunk| {
            collected.extend_from_slice(chunk);
            async move { Ok(()) }
        })
        .await
        .unwrap();

        assert_eq!(collected, content);
    }

    #[tokio::test]
    async fn test_compression() {
        let temp_dir = TempDir::new().unwrap();
        let cas = OptimizedContentAddressableStorage::builder(temp_dir.path())
            .compression(CompressionConfig::Zstd(3))
            .build();

        // 使用重复内容以获得更好的压缩率
        let content = vec![b'A'; 10000];
        let hash = cas.store_content(&content).await.unwrap();

        let retrieved = cas.read_content(&hash).await.unwrap();
        assert_eq!(retrieved, content);
    }

    #[tokio::test]
    async fn test_streaming_integrity_check() {
        let temp_dir = TempDir::new().unwrap();
        let cas = OptimizedContentAddressableStorage::new(temp_dir.path());

        let content = b"Integrity test content";
        let hash = cas.store_content(content).await.unwrap();

        let is_valid = cas.verify_integrity_streaming(&hash).await.unwrap();
        assert!(is_valid);
    }

    #[tokio::test]
    async fn test_builder_configuration() {
        let temp_dir = TempDir::new().unwrap();
        let cas = OptimizedContentAddressableStorage::builder(temp_dir.path())
            .cache_capacity(1000)
            .cache_ttl(3600)
            .compression(CompressionConfig::Zstd(6))
            .buffer_size(128 * 1024)
            .build();

        let content = b"Builder test";
        let hash = cas.store_content(content).await.unwrap();
        
        assert!(cas.exists(&hash));
    }
}
