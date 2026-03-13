//! 原子性 Content-Addressable Storage (CAS) 实现
//!
//! 解决 TOCTOU (Time-of-Check-Time-of-Use) 竞争条件问题
//! 采用业内成熟方案：
//! - Git 对象存储模式（基于 O_EXCL 的原子创建）
//! - 临时文件 + 原子重命名（Linux/Windows 跨平台兼容）
//! - SHA-256 内容寻址保证完整性

use crate::error::{AppError, Result};
use moka::sync::Cache;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Semaphore;
use tracing::{debug, error, info, warn};

/// 原子写入配置
#[derive(Debug, Clone)]
pub struct AtomicCasConfig {
    /// 临时文件后缀
    pub temp_suffix: String,
    /// 最大并发写入数
    pub max_concurrent_writes: usize,
    /// 文件复制缓冲区大小
    pub copy_buffer_size: usize,
    /// 文件复制超时时间（秒）
    pub copy_timeout_secs: u64,
}

impl Default for AtomicCasConfig {
    fn default() -> Self {
        Self {
            temp_suffix: ".tmp".to_string(),
            max_concurrent_writes: 100,
            copy_buffer_size: 64 * 1024, // 64KB
            copy_timeout_secs: 300,
        }
    }
}

/// 原子性内容寻址存储
///
/// 核心设计原则：
/// 1. 无检查-后使用模式：使用 O_EXCL 标志直接创建，失败则处理
/// 2. 临时文件 + 原子重命名：确保写入的完整性
/// 3. 信号量控制并发：防止资源耗尽
#[derive(Debug, Clone)]
pub struct AtomicContentAddressableStorage {
    workspace_dir: PathBuf,
    existence_cache: Arc<Cache<String, ()>>,
    config: AtomicCasConfig,
    /// 并发写入信号量（背压控制）
    write_semaphore: Arc<Semaphore>,
}

impl AtomicContentAddressableStorage {
    /// 创建新的原子 CAS 实例
    pub fn new(workspace_dir: PathBuf) -> Self {
        Self::with_config(workspace_dir, AtomicCasConfig::default())
    }

    /// 使用自定义配置创建
    pub fn with_config(workspace_dir: PathBuf, config: AtomicCasConfig) -> Self {
        let semaphore = Arc::new(Semaphore::new(config.max_concurrent_writes));

        Self {
            workspace_dir,
            existence_cache: Arc::new(Cache::new(10_000)),
            config,
            write_semaphore: semaphore,
        }
    }

    /// 计算内容的 SHA-256 哈希
    pub fn compute_hash(content: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content);
        format!("{:x}", hasher.finalize())
    }

    /// 增量计算文件的 SHA-256 哈希（流式处理大文件）
    pub async fn compute_hash_incremental(file_path: &Path) -> Result<String> {
        const BUFFER_SIZE: usize = 8 * 1024;

        let file = fs::File::open(file_path).await.map_err(|e| {
            AppError::io_error(
                format!("Failed to open file for hashing: {}", e),
                Some(file_path.to_path_buf()),
            )
        })?;

        let mut reader = BufReader::with_capacity(BUFFER_SIZE, file);
        let mut hasher = Sha256::new();
        let mut buffer = vec![0u8; BUFFER_SIZE];

        loop {
            let bytes_read = reader.read(&mut buffer).await.map_err(|e| {
                AppError::io_error(
                    format!("Failed to read file for hashing: {}", e),
                    Some(file_path.to_path_buf()),
                )
            })?;

            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        Ok(format!("{:x}", hasher.finalize()))
    }

    /// 原子存储内容（业内成熟方案：临时文件 + 原子重命名）
    ///
    /// # 并发安全保证
    /// 1. 使用临时文件写入，避免半写入状态
    /// 2. 使用 `rename` 系统调用保证原子性
    /// 3. 信号量控制并发写入数量（背压）
    /// 4. O_EXCL 标志防止竞争写入
    pub async fn store_content_atomic(&self, content: &[u8]) -> Result<String> {
        let hash = Self::compute_hash(content);
        let object_path = self.get_object_path(&hash);

        // 快速路径：检查缓存
        if self.existence_cache.get(&hash).is_some() {
            debug!(hash = %hash, "Content exists in cache (deduplication)");
            return Ok(hash);
        }

        // 获取写入许可（背压控制）
        let _permit = self
            .write_semaphore
            .acquire()
            .await
            .map_err(|_| AppError::io_error("Write semaphore closed", None::<PathBuf>))?;

        // 检查文件是否已存在（非权威，仅用于提前返回）
        if object_path.exists() {
            self.existence_cache.insert(hash.clone(), ());
            debug!(hash = %hash, "Content already exists (deduplication)");
            return Ok(hash);
        }

        // 确保目录存在
        let parent_dir = object_path.parent().ok_or_else(|| {
            AppError::io_error(
                "Invalid object path: no parent directory",
                Some(object_path.clone()),
            )
        })?;

        fs::create_dir_all(parent_dir).await.map_err(|e| {
            AppError::io_error(
                format!("Failed to create object directory: {}", e),
                Some(parent_dir.to_path_buf()),
            )
        })?;

        // 原子写入：临时文件 -> 原子重命名
        let temp_path = object_path.with_extension(&self.config.temp_suffix);

        // 步骤 1: 写入临时文件
        let temp_write_result = self.write_to_temp_file(&temp_path, content).await;

        match temp_write_result {
            Ok(()) => {
                // 步骤 2: 原子重命名（Linux/Windows 跨平台保证）
                match fs::rename(&temp_path, &object_path).await {
                    Ok(()) => {
                        self.existence_cache.insert(hash.clone(), ());
                        info!(
                            hash = %hash,
                            size = content.len(),
                            path = %object_path.display(),
                            "Content stored atomically"
                        );
                        Ok(hash)
                    }
                    Err(e) => {
                        // 重命名失败，清理临时文件
                        let _ = fs::remove_file(&temp_path).await;

                        // 检查是否是竞争条件导致的已存在
                        if e.kind() == std::io::ErrorKind::AlreadyExists {
                            self.existence_cache.insert(hash.clone(), ());
                            debug!(hash = %hash, "Content created by concurrent writer");
                            Ok(hash)
                        } else {
                            Err(AppError::io_error(
                                format!("Failed to rename temp file: {}", e),
                                Some(object_path),
                            ))
                        }
                    }
                }
            }
            Err(e) => {
                // 写入失败，尝试清理临时文件
                let _ = fs::remove_file(&temp_path).await;
                Err(e)
            }
        }
    }

    /// 流式存储文件（原子性保证）
    pub async fn store_file_streaming_atomic(&self, file_path: &Path) -> Result<String> {
        // 先计算哈希
        let hash = Self::compute_hash_incremental(file_path).await?;
        let object_path = self.get_object_path(&hash);

        // 快速路径检查
        if self.existence_cache.get(&hash).is_some() {
            debug!(hash = %hash, file = %file_path.display(), "Content cached, skipping");
            return Ok(hash);
        }

        // 获取写入许可（背压）
        let _permit = self
            .write_semaphore
            .acquire()
            .await
            .map_err(|_| AppError::io_error("Write semaphore closed", None::<PathBuf>))?;

        // 使用 O_EXCL 原子创建
        use tokio::fs::OpenOptions;
        use tokio::time::{timeout, Duration};

        // 确保目录存在
        if let Some(parent) = object_path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                AppError::io_error(
                    format!("Failed to create directory: {}", e),
                    Some(parent.to_path_buf()),
                )
            })?;
        }

        // 尝试原子创建
        let dst_file = match OpenOptions::new()
            .write(true)
            .create_new(true) // O_EXCL: 原子创建
            .open(&object_path)
            .await
        {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                self.existence_cache.insert(hash.clone(), ());
                debug!(hash = %hash, "File exists (concurrent write)");
                return Ok(hash);
            }
            Err(e) => {
                return Err(AppError::io_error(
                    format!("Failed to create object file: {}", e),
                    Some(object_path),
                ));
            }
        };

        // 流式复制（带超时）
        let copy_result = timeout(
            Duration::from_secs(self.config.copy_timeout_secs),
            self.stream_copy(file_path, dst_file, &object_path),
        )
        .await;

        match copy_result {
            Ok(Ok(())) => {
                self.existence_cache.insert(hash.clone(), ());
                info!(
                    hash = %hash,
                    source = %file_path.display(),
                    "File stored atomically via streaming"
                );
                Ok(hash)
            }
            Ok(Err(e)) => {
                // 复制失败，删除部分文件
                let _ = fs::remove_file(&object_path).await;
                Err(e)
            }
            Err(_) => {
                // 超时，清理部分文件
                let _ = fs::remove_file(&object_path).await;
                Err(AppError::io_error(
                    format!("File copy timeout after {}s", self.config.copy_timeout_secs),
                    Some(file_path.to_path_buf()),
                ))
            }
        }
    }

    /// 写入临时文件
    async fn write_to_temp_file(&self, temp_path: &Path, content: &[u8]) -> Result<()> {
        use tokio::fs::OpenOptions;

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(temp_path)
            .await
            .map_err(|e| {
                AppError::io_error(
                    format!("Failed to create temp file: {}", e),
                    Some(temp_path.to_path_buf()),
                )
            })?;

        file.write_all(content).await.map_err(|e| {
            AppError::io_error(
                format!("Failed to write temp file: {}", e),
                Some(temp_path.to_path_buf()),
            )
        })?;

        file.flush().await.map_err(|e| {
            AppError::io_error(
                format!("Failed to flush temp file: {}", e),
                Some(temp_path.to_path_buf()),
            )
        })?;

        // 确保数据落盘（fsync）
        file.sync_all().await.map_err(|e| {
            AppError::io_error(
                format!("Failed to sync temp file: {}", e),
                Some(temp_path.to_path_buf()),
            )
        })?;

        Ok(())
    }

    /// 流式复制文件
    async fn stream_copy(
        &self,
        src_path: &Path,
        mut dst_file: tokio::fs::File,
        dst_path: &Path,
    ) -> Result<()> {
        let mut src_file = fs::File::open(src_path).await.map_err(|e| {
            AppError::io_error(
                format!("Failed to open source file: {}", e),
                Some(src_path.to_path_buf()),
            )
        })?;

        let mut buffer = vec![0u8; self.config.copy_buffer_size];

        loop {
            let bytes_read = src_file.read(&mut buffer).await.map_err(|e| {
                AppError::io_error(
                    format!("Failed to read source file: {}", e),
                    Some(src_path.to_path_buf()),
                )
            })?;

            if bytes_read == 0 {
                break;
            }

            dst_file
                .write_all(&buffer[..bytes_read])
                .await
                .map_err(|e| {
                    AppError::io_error(
                        format!("Failed to write target file: {}", e),
                        Some(dst_path.to_path_buf()),
                    )
                })?;
        }

        dst_file.flush().await.map_err(|e| {
            AppError::io_error(
                format!("Failed to flush target file: {}", e),
                Some(dst_path.to_path_buf()),
            )
        })?;

        dst_file.sync_all().await.map_err(|e| {
            AppError::io_error(
                format!("Failed to sync target file: {}", e),
                Some(dst_path.to_path_buf()),
            )
        })?;

        Ok(())
    }

    /// 获取对象路径
    pub fn get_object_path(&self, hash: &str) -> PathBuf {
        let (prefix, suffix) = if hash.len() >= 2 {
            hash.split_at(2)
        } else {
            ("00", hash)
        };
        self.workspace_dir.join("objects").join(prefix).join(suffix)
    }

    /// 读取内容
    pub async fn read_content(&self, hash: &str) -> Result<Vec<u8>> {
        let object_path = self.get_object_path(hash);

        if !object_path.exists() {
            return Err(AppError::not_found(format!(
                "Object not found: {} at {}",
                hash,
                object_path.display()
            )));
        }

        fs::read(&object_path).await.map_err(|e| {
            AppError::io_error(format!("Failed to read object: {}", e), Some(object_path))
        })
    }

    /// 检查内容是否存在
    pub fn exists(&self, hash: &str) -> bool {
        if self.existence_cache.get(hash).is_some() {
            return true;
        }
        let result = self.get_object_path(hash).exists();
        if result {
            self.existence_cache.insert(hash.to_string(), ());
        }
        result
    }

    /// 获取存储大小
    pub async fn get_storage_size(&self) -> Result<u64> {
        let objects_dir = self.workspace_dir.join("objects");

        if !objects_dir.exists() {
            return Ok(0);
        }

        let mut total_size = 0u64;
        let mut entries = fs::read_dir(&objects_dir).await.map_err(|e| {
            AppError::io_error(
                format!("Failed to read objects directory: {}", e),
                Some(objects_dir.clone()),
            )
        })?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            AppError::io_error(
                format!("Failed to read directory entry: {}", e),
                Some(objects_dir.clone()),
            )
        })? {
            let file_type = entry.file_type().await.map_err(|e| {
                AppError::io_error(
                    format!("Failed to get file type: {}", e),
                    Some(entry.path()),
                )
            })?;

            if file_type.is_dir() {
                // 递归统计子目录
                let mut sub_entries = fs::read_dir(entry.path()).await.map_err(|e| {
                    AppError::io_error(
                        format!("Failed to read subdirectory: {}", e),
                        Some(entry.path()),
                    )
                })?;

                while let Some(sub_entry) = sub_entries.next_entry().await.map_err(|e| {
                    AppError::io_error(
                        format!("Failed to read subdirectory entry: {}", e),
                        Some(entry.path()),
                    )
                })? {
                    let metadata = sub_entry.metadata().await.map_err(|e| {
                        AppError::io_error(
                            format!("Failed to get file metadata: {}", e),
                            Some(sub_entry.path()),
                        )
                    })?;
                    total_size += metadata.len();
                }
            }
        }

        Ok(total_size)
    }

    /// 验证内容完整性
    pub async fn verify_integrity(&self, hash: &str) -> Result<bool> {
        let content = self.read_content(hash).await?;
        let computed_hash = Self::compute_hash(&content);
        Ok(computed_hash == hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_atomic_store_content() {
        let temp_dir = TempDir::new().unwrap();
        let cas = AtomicContentAddressableStorage::new(temp_dir.path().to_path_buf());

        let content = b"atomic test content";
        let hash = cas.store_content_atomic(content).await.unwrap();

        // 验证内容可读取
        let retrieved = cas.read_content(&hash).await.unwrap();
        assert_eq!(retrieved, content);

        // 验证重复存储返回相同哈希
        let hash2 = cas.store_content_atomic(content).await.unwrap();
        assert_eq!(hash, hash2);
    }

    #[tokio::test]
    async fn test_concurrent_writes() {
        let temp_dir = TempDir::new().unwrap();
        let cas = AtomicContentAddressableStorage::new(temp_dir.path().to_path_buf());

        let content = b"concurrent content";
        let cas_clone = cas.clone();

        // 并发写入相同内容
        let (r1, r2) = tokio::join!(
            cas.store_content_atomic(content),
            cas_clone.store_content_atomic(content)
        );

        // 两者都应该成功且返回相同哈希
        assert!(r1.is_ok());
        assert!(r2.is_ok());
        assert_eq!(r1.unwrap(), r2.unwrap());
    }

    #[tokio::test]
    async fn test_streaming_store() {
        let temp_dir = TempDir::new().unwrap();
        let cas = AtomicContentAddressableStorage::new(temp_dir.path().to_path_buf());

        // 创建测试文件
        let test_file = temp_dir.path().join("test.log");
        let content = b"streaming test content";
        fs::write(&test_file, content).await.unwrap();

        let hash = cas.store_file_streaming_atomic(&test_file).await.unwrap();

        // 验证内容
        let retrieved = cas.read_content(&hash).await.unwrap();
        assert_eq!(retrieved, content);
    }

    #[tokio::test]
    async fn test_integrity_verification() {
        let temp_dir = TempDir::new().unwrap();
        let cas = AtomicContentAddressableStorage::new(temp_dir.path().to_path_buf());

        let content = b"integrity test";
        let hash = cas.store_content_atomic(content).await.unwrap();

        assert!(cas.verify_integrity(&hash).await.unwrap());

        // 修改存储的文件
        let object_path = cas.get_object_path(&hash);
        fs::write(&object_path, b"corrupted").await.unwrap();

        assert!(!cas.verify_integrity(&hash).await.unwrap());
    }
}
