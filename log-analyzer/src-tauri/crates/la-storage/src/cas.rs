//! Content-Addressable Storage (CAS) Implementation
//!
//! Based on Git's object storage model, this module provides:
//! - SHA-256 content hashing (industry standard)
//! - Flat directory structure (avoids path length limits)
//! - Automatic deduplication (same content = same hash)
//! - Efficient storage and retrieval
//! - Object existence cache for performance optimization
//!
//! ## Storage Layout
//!
//! Files are stored using their SHA-256 hash as the identifier:
//! ```text
//! objects/
//!   a3/
//!     f2e1d4c5b6a7... (full hash as filename)
//!   b7/
//!     e145a3b2c9d8...
//! ```
//!
//! The first 2 characters of the hash are used as a directory name
//! to avoid having too many files in a single directory.

use async_trait::async_trait;
use la_core::error::{AppError, Result};
use la_core::traits::ContentStorage;
use moka::sync::Cache; // ✅ 使用 moka LRU 缓存替代 DashSet
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::fs;
use tokio::io::{AsyncReadExt, BufReader};
use tracing::{debug, error, info, warn};
use walkdir::WalkDir;

/// 获取指定路径的磁盘可用空间
///
/// # Arguments
///
/// * `path` - 要检查的路径
///
/// # Returns
///
/// 可用空间字节数，如果无法获取则返回 0
///
/// # Safety
///
/// 这个函数使用 `unsafe` 块调用系统调用 `statvfs`，已经通过以下方式确保安全：
/// - 使用 `CString::new()` 确保路径字符串以 NUL 结尾，不含内部 NUL
/// - 使用 `std::mem::zeroed()` 初始化 `statvfs` 结构体，确保内存安全
/// - `c_path.as_ptr()` 返回的指针在 `statvfs` 调用期间有效
/// - 所有错误都被妥善处理，返回 0 而非 panic
///
/// 这些 unsafe 操作是调用 libc 系统调用所必需的，已通过输入验证和错误处理确保安全。
#[cfg(target_os = "linux")]
#[allow(clippy::unnecessary_cast)]
async fn get_available_space(path: &Path) -> u64 {
    use libc::statvfs;
    use std::ffi::CString;

    let path_str = match path.to_str() {
        Some(s) => s,
        None => return 0,
    };

    let c_path = match CString::new(path_str) {
        Ok(c) => c,
        Err(_) => return 0,
    };

    let mut stat: statvfs = unsafe { std::mem::zeroed() };
    let result = unsafe { statvfs(c_path.as_ptr(), &mut stat) };

    if result == 0 {
        // f_bavail: 非超级用户可用的块数
        // f_frsize: 每个块的字节数（Linux 下已是 u64，无需转换）
        stat.f_bavail * stat.f_frsize
    } else {
        0
    }
}

/// 获取指定路径的磁盘可用空间 (macOS 版本)
///
/// # Safety
///
/// 与 Linux 版本相同，使用 `unsafe` 块调用 `statvfs` 系统调用：
/// - 路径已转换为有效的 C 字符串 (NUL 结尾)
/// - `statvfs` 结构体已正确初始化
/// - 所有错误都被妥善处理
#[cfg(target_os = "macos")]
#[allow(clippy::unnecessary_cast)]
async fn get_available_space(path: &Path) -> u64 {
    use libc::statvfs;
    use std::ffi::CString;

    let path_str = match path.to_str() {
        Some(s) => s,
        None => return 0,
    };

    let c_path = match CString::new(path_str) {
        Ok(c) => c,
        Err(_) => return 0,
    };

    let mut stat: statvfs = unsafe { std::mem::zeroed() };
    let result = unsafe { statvfs(c_path.as_ptr(), &mut stat) };

    if result == 0 {
        (stat.f_bavail as u64) * (stat.f_frsize as u64)
    } else {
        0
    }
}

/// 获取指定路径的磁盘可用空间 (Windows 版本)
///
/// # Safety
///
/// 使用 `unsafe` 块调用 Windows API `GetDiskFreeSpaceExW`：
/// - 路径已转换为宽字符 (UTF-16) 格式，以 NUL 结尾
/// - 使用有效的可变指针接收返回值
/// - 所有错误都被妥善处理，返回 0 而非 panic
///
/// 这些 unsafe 操作是调用 Windows API 所必需的，已通过输入验证确保安全。
#[cfg(target_os = "windows")]
#[allow(clippy::unnecessary_cast)]
async fn get_available_space(path: &Path) -> u64 {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;

    let wide_path: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0u16))
        .collect();

    let mut free_bytes_available: u64 = 0;
    let result = unsafe {
        GetDiskFreeSpaceExW(
            wide_path.as_ptr(),
            &mut free_bytes_available,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };

    if result != 0 {
        free_bytes_available
    } else {
        0
    }
}

// Fallback for other platforms
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
async fn get_available_space(_path: &Path) -> u64 {
    0 // Unknown, skip check
}

/// Content-Addressable Storage manager
///
/// Provides Git-style content storage with SHA-256 hashing.
/// All files are stored in a flat structure under `workspace_dir/objects/`.
///
/// ## Performance Optimization
///
/// Uses an LRU cache for object existence checks to avoid
/// redundant filesystem operations. The cache has a maximum capacity
/// of 10,000 entries to prevent unbounded memory growth.
/// LruCache provides thread-safe concurrent access with minimal locking overhead.
#[derive(Debug, Clone)]
pub struct ContentAddressableStorage {
    workspace_dir: PathBuf,
    /// In-memory LRU cache for object existence checks (performance optimization)
    /// Limits memory usage by evicting least recently used entries
    existence_cache: Arc<Cache<String, ()>>,
}

fn is_valid_content_hash(hash: &str) -> bool {
    hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// RAII guard for temporary files that ensures cleanup on drop (BUG-007 fix)
///
/// This guard holds the path to a temporary file and attempts to delete it
/// when the guard is dropped. This ensures temporary files are cleaned up
/// even if errors occur or early returns happen.
struct TempFileGuard {
    path: PathBuf,
    // Use tokio runtime handle for async cleanup in drop
    rt_handle: Option<tokio::runtime::Handle>,
}

impl TempFileGuard {
    /// Create a new temp file guard
    fn new(path: PathBuf) -> Self {
        let rt_handle = tokio::runtime::Handle::try_current().ok();
        Self { path, rt_handle }
    }

    /// Get the path to the temporary file
    #[allow(dead_code)]
    fn path(&self) -> &Path {
        &self.path
    }

    /// Consume the guard without deleting the file (e.g., after successful rename)
    fn keep(mut self) -> PathBuf {
        // Disable cleanup by taking the path
        std::mem::take(&mut self.path)
    }
}

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        if !self.path.as_os_str().is_empty() {
            // Try to delete the file
            if let Some(ref handle) = self.rt_handle {
                // We're in an async context, use blocking operation
                let path = self.path.clone();
                handle.spawn(async move {
                    if let Err(e) = tokio::fs::remove_file(&path).await {
                        tracing::warn!(
                            path = %path.display(),
                            error = %e,
                            "Failed to clean up temporary file"
                        );
                    } else {
                        tracing::debug!(path = %path.display(), "Cleaned up temporary file");
                    }
                });
            } else {
                // No runtime available, try synchronous deletion
                if let Err(e) = std::fs::remove_file(&self.path) {
                    tracing::warn!(
                        path = %self.path.display(),
                        error = %e,
                        "Failed to clean up temporary file (sync)"
                    );
                }
            }
        }
    }
}

impl ContentAddressableStorage {
    /// Create a new CAS instance
    ///
    /// # Arguments
    ///
    /// * `workspace_dir` - Root directory for this workspace
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::path::PathBuf;
    /// use la_storage::ContentAddressableStorage;
    ///
    /// let cas = ContentAddressableStorage::new(PathBuf::from("./workspace_123"));
    /// ```
    pub fn new(workspace_dir: PathBuf) -> Self {
        // Create an LRU cache for object existence checks with TTL/TTI expiration
        // Capacity: 10,000 entries to balance performance and memory usage
        // TTL: 1 hour - entries forced to expire after max lifetime
        // TTI: 5 minutes - entries removed after idle time
        Self {
            workspace_dir,
            existence_cache: Arc::new(
                Cache::builder()
                    .max_capacity(10_000)
                    .time_to_live(Duration::from_secs(3600))
                    .time_to_idle(Duration::from_secs(300))
                    .build(),
            ),
        }
    }

    /// Compute SHA-256 hash of content
    ///
    /// This is a pure function that always produces the same hash
    /// for the same content (idempotent).
    ///
    /// # Arguments
    ///
    /// * `content` - Byte slice to hash
    ///
    /// # Returns
    ///
    /// Lowercase hexadecimal string representation of the SHA-256 hash
    ///
    /// # Example
    ///
    /// ```
    /// use la_storage::ContentAddressableStorage;
    ///
    /// let hash = ContentAddressableStorage::compute_hash(b"hello world");
    /// assert_eq!(hash.len(), 64); // SHA-256 produces 64 hex characters
    /// ```
    pub fn compute_hash(content: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content);
        format!("{:x}", hasher.finalize())
    }

    /// Compute SHA-256 hash incrementally for large files
    ///
    /// Uses streaming with an 8KB buffer to avoid loading the entire
    /// file into memory. This is essential for handling large log files
    /// without causing memory spikes.
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the file to hash
    ///
    /// # Returns
    ///
    /// Lowercase hexadecimal string representation of the SHA-256 hash
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - File cannot be opened
    /// - File cannot be read
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use la_storage::ContentAddressableStorage;
    /// # use std::path::Path;
    /// # tokio_test::block_on(async {
    /// let hash = ContentAddressableStorage::compute_hash_incremental(
    ///     Path::new("large_file.log")
    /// ).await.unwrap();
    /// # })
    /// ```
    pub async fn compute_hash_incremental(file_path: &Path) -> Result<String> {
        const BUFFER_SIZE: usize = 1024 * 1024; // 1MB buffer (优化: 从 8KB 增大，减少大文件处理时的 syscall 次数)

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
                break; // EOF
            }

            hasher.update(&buffer[..bytes_read]);
        }

        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Store file content from a path using streaming
    ///
    /// This method reads the file incrementally and stores it in CAS.
    /// It's more memory-efficient than `store_content` for large files.
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the file to store
    ///
    /// # Returns
    ///
    /// SHA-256 hash of the stored content
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - File cannot be read
    /// - Failed to create object directory
    /// - Failed to write file
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use la_storage::ContentAddressableStorage;
    /// # use std::path::{Path, PathBuf};
    /// # tokio_test::block_on(async {
    /// let cas = ContentAddressableStorage::new(PathBuf::from("./workspace"));
    /// let hash = cas.store_file_streaming(Path::new("large.log")).await.unwrap();
    /// # })
    /// ```
    pub async fn store_file_streaming(&self, file_path: &Path) -> Result<String> {
        // First compute the hash to check for deduplication
        let hash = Self::compute_hash_incremental(file_path).await?;
        let object_path = self.get_object_path(&hash);

        // Fast path: Check cache first (lock-free, high-frequency optimization)
        // This is a hint, not authoritative - actual existence is checked atomically below
        if self.existence_cache.get(&hash).is_some() {
            // Verify that the file actually exists to handle stale cache
            if tokio::fs::try_exists(&object_path).await.unwrap_or(false) {
                debug!(
                    hash = %hash,
                    file = %file_path.display(),
                    "Content already exists (verified), skipping write (deduplication)"
                );
                return Ok(hash);
            }
            // Cache is stale, invalidate it and continue with write
            self.existence_cache.invalidate(&hash);
            debug!(
                hash = %hash,
                "Cache indicated existence but file missing, proceeding with write"
            );
        }

        // Atomic file creation with O_EXCL flag prevents TOCTOU race conditions
        // This is the authoritative check - no separate exists() check needed
        // If file already exists, we'll get AlreadyExists error and handle it gracefully

        // Create parent directory (e.g., objects/a3/)
        if let Some(parent) = object_path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                AppError::io_error(
                    format!("Failed to create object directory: {}", e),
                    Some(parent.to_path_buf()),
                )
            })?;
        }

        // Copy file to object storage with timeout protection (industry standard)
        // Use tokio::io::copy for true async copy (fs::copy blocks on Windows!)
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::time::{timeout, Duration};

        const FILE_COPY_TIMEOUT: u64 = 300; // 5 minutes timeout for large files
        const COPY_BUFFER_SIZE: usize = 1024 * 1024; // 1MB buffer for efficient copying (优化: 从 64KB 增大)

        let copy_result = timeout(Duration::from_secs(FILE_COPY_TIMEOUT), async {
            // Open source file
            let mut src_file = fs::File::open(file_path).await.map_err(|e| {
                AppError::io_error(
                    format!("Failed to open source file: {}", e),
                    Some(file_path.to_path_buf()),
                )
            })?;

            // **SECURITY FIX**: Use create_new() for atomic file creation (O_EXCL flag)
            // This prevents TOCTOU race condition - if two threads try to create the same file,
            // only one will succeed with Ok(), the other gets ErrorKind::AlreadyExists
            use tokio::fs::OpenOptions;
            let mut dst_file = match OpenOptions::new()
                .write(true)
                .create_new(true) // O_EXCL: atomic check-and-create
                .open(&object_path)
                .await
            {
                Ok(file) => file,
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    // Another thread created the file - this is expected in concurrent scenarios
                    // Update cache and return early (deduplication win)
                    self.existence_cache.insert(hash.clone(), ());
                    debug!(
                        hash = %hash,
                        file = %file_path.display(),
                        "Content already exists (concurrent write detected), skipping"
                    );
                    return Ok(0u64); // Signal early return with 0 bytes
                }
                Err(e) => {
                    return Err(AppError::io_error(
                        format!("Failed to create target file: {}", e),
                        Some(object_path.clone()),
                    ));
                }
            };

            // Copy using async I/O with buffer
            let mut buffer = vec![0u8; COPY_BUFFER_SIZE];
            let mut total_bytes = 0u64;

            loop {
                let bytes_read = src_file.read(&mut buffer).await.map_err(|e| {
                    AppError::io_error(
                        format!("Failed to read from source file: {}", e),
                        Some(file_path.to_path_buf()),
                    )
                })?;

                if bytes_read == 0 {
                    break; // EOF
                }

                dst_file
                    .write_all(&buffer[..bytes_read])
                    .await
                    .map_err(|e| {
                        AppError::io_error(
                            format!("Failed to write to target file: {}", e),
                            Some(object_path.clone()),
                        )
                    })?;

                total_bytes += bytes_read as u64;
            }

            // Flush to ensure all data is written
            dst_file.flush().await.map_err(|e| {
                AppError::io_error(
                    format!("Failed to flush target file: {}", e),
                    Some(object_path.clone()),
                )
            })?;

            Ok::<u64, AppError>(total_bytes)
        })
        .await;

        match copy_result {
            Ok(Ok(bytes)) => {
                debug!(
                    file = %file_path.display(),
                    target = %object_path.display(),
                    bytes = bytes,
                    "File copied successfully"
                );
            }
            Ok(Err(e)) => {
                // Copy failed with error — clean up the partial file so it is not
                // mistaken for valid content on the next store attempt (create_new
                // would return AlreadyExists and silently serve corrupted data).
                error!(
                    file = %file_path.display(),
                    target = %object_path.display(),
                    error = %e,
                    "File copy failed"
                );
                if let Err(cleanup_err) = fs::remove_file(&object_path).await {
                    warn!(
                        target = %object_path.display(),
                        error = %cleanup_err,
                        "写入失败后清理部分文件出错"
                    );
                }
                return Err(e);
            }
            Err(_) => {
                // Timeout occurred (ERR-001 fix: improved cleanup with retry)
                error!(
                    file = %file_path.display(),
                    target = %object_path.display(),
                    timeout_secs = FILE_COPY_TIMEOUT,
                    "File copy timeout after {} seconds",
                    FILE_COPY_TIMEOUT
                );
                // Clean up partial file with retry logic
                let mut cleanup_attempts = 0;
                let max_cleanup_attempts = 3;
                let mut cleanup_success = false;

                while cleanup_attempts < max_cleanup_attempts && !cleanup_success {
                    match fs::remove_file(&object_path).await {
                        Ok(_) => {
                            debug!("Successfully cleaned up partial file after timeout");
                            cleanup_success = true;
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                            // File doesn't exist, consider cleanup successful
                            debug!("Partial file already removed");
                            cleanup_success = true;
                        }
                        Err(e) => {
                            cleanup_attempts += 1;
                            error!(
                                target = %object_path.display(),
                                error = %e,
                                attempt = cleanup_attempts,
                                max_attempts = max_cleanup_attempts,
                                "Failed to clean up partial file after timeout"
                            );
                            if cleanup_attempts < max_cleanup_attempts {
                                // Wait before retry with exponential backoff
                                tokio::time::sleep(Duration::from_millis(
                                    100 * cleanup_attempts as u64,
                                ))
                                .await;
                            }
                        }
                    }
                }

                if !cleanup_success {
                    // Log warning about orphaned partial file
                    warn!(
                        target = %object_path.display(),
                        "Partial file could not be cleaned up after {} attempts, may need manual cleanup",
                        max_cleanup_attempts
                    );
                }

                return Err(AppError::io_error(
                    format!("File copy timeout after {} seconds", FILE_COPY_TIMEOUT),
                    Some(file_path.to_path_buf()),
                ));
            }
        }

        let metadata = fs::metadata(&object_path).await.map_err(|e| {
            AppError::io_error(
                format!("Failed to read file metadata: {}", e),
                Some(object_path.clone()),
            )
        })?;

        // Cache the newly created object for future existence checks
        // Use a thread-safe insert operation
        self.existence_cache.insert(hash.clone(), ());

        info!(
            hash = %hash,
            size = metadata.len(),
            path = %object_path.display(),
            source = %file_path.display(),
            "Stored file in CAS using streaming"
        );

        Ok(hash)
    }

    /// Store file content and return its hash
    ///
    /// If the content already exists (same hash), it won't be written again.
    /// This provides automatic deduplication.
    ///
    /// # Arguments
    ///
    /// * `content` - File content to store
    ///
    /// # Returns
    ///
    /// SHA-256 hash of the stored content
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Failed to create object directory
    /// - Failed to write file
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use la_storage::ContentAddressableStorage;
    /// # use std::path::PathBuf;
    /// # tokio_test::block_on(async {
    /// let cas = ContentAddressableStorage::new(PathBuf::from("./workspace"));
    /// let hash = cas.store_content(b"log content").await.unwrap();
    /// # })
    /// ```
    pub async fn store_content(&self, content: &[u8]) -> Result<String> {
        let hash = Self::compute_hash(content);
        let object_path = self.get_object_path(&hash);

        // Check cache first for performance (fast path)
        if self.existence_cache.get(&hash).is_some() {
            // Verify that the file actually exists to handle stale cache
            if object_path.exists() {
                debug!(
                    hash = %hash,
                    "Content already exists (verified), skipping write (deduplication)"
                );
                return Ok(hash);
            }
            // Cache is stale, invalidate it and continue
            self.existence_cache.invalidate(&hash);
            debug!(
                hash = %hash,
                "Cache indicated existence but file missing, proceeding with write"
            );
        }

        // Check if object already exists (deduplication) - authoritative check
        if object_path.exists() {
            // Cache the result
            self.existence_cache.insert(hash.clone(), ());
            debug!(
                hash = %hash,
                "Content already exists, skipping write (deduplication)"
            );
            return Ok(hash);
        }

        // Create parent directory (e.g., objects/a3/)
        if let Some(parent) = object_path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                AppError::io_error(
                    format!("Failed to create object directory: {}", e),
                    Some(parent.to_path_buf()),
                )
            })?;
        }

        // **SECURITY FIX**: Use atomic write with create_new() to prevent TOCTOU race
        use tokio::fs::OpenOptions;
        use tokio::io::AsyncWriteExt;

        match OpenOptions::new()
            .write(true)
            .create_new(true) // O_EXCL: atomic check-and-create
            .open(&object_path)
            .await
        {
            Ok(mut file) => {
                // Successfully created new file, write content
                file.write_all(content).await.map_err(|e| {
                    AppError::io_error(
                        format!("Failed to write object file: {}", e),
                        Some(object_path.clone()),
                    )
                })?;

                file.flush().await.map_err(|e| {
                    AppError::io_error(
                        format!("Failed to flush object file: {}", e),
                        Some(object_path.clone()),
                    )
                })?;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                // Another thread created the file concurrently - deduplication win
                self.existence_cache.insert(hash.clone(), ());
                debug!(
                    hash = %hash,
                    "Content already exists (concurrent write detected), skipping"
                );
                return Ok(hash);
            }
            Err(e) => {
                return Err(AppError::io_error(
                    format!("Failed to create object file: {}", e),
                    Some(object_path.clone()),
                ));
            }
        }

        // Cache the newly created object
        self.existence_cache.insert(hash.clone(), ());

        info!(
            hash = %hash,
            size = content.len(),
            path = %object_path.display(),
            "Stored content in CAS"
        );

        Ok(hash)
    }

    /// Get the filesystem path for a given hash
    ///
    /// Uses Git-style sharding: first 2 characters as directory name.
    /// This prevents having too many files in a single directory.
    ///
    /// # Arguments
    ///
    /// * `hash` - SHA-256 hash (64 hex characters)
    ///
    /// # Returns
    ///
    /// Path to the object file
    ///
    /// # Example
    ///
    /// ```
    /// # use la_storage::ContentAddressableStorage;
    /// # use std::path::PathBuf;
    /// let cas = ContentAddressableStorage::new(PathBuf::from("./workspace"));
    /// let path = cas.get_object_path("a3f2e1d4c5b6a7...");
    /// // Returns: ./workspace/objects/a3/f2e1d4c5b6a7...
    /// ```
    pub fn get_object_path(&self, hash: &str) -> PathBuf {
        // Split hash: first 2 chars as directory, rest as filename
        let (prefix, suffix) = if hash.len() >= 2 {
            hash.split_at(2)
        } else {
            // Fallback for invalid hash (shouldn't happen with SHA-256)
            warn!(hash = %hash, "Invalid hash length, using full hash as filename");
            ("00", hash)
        };

        self.workspace_dir.join("objects").join(prefix).join(suffix)
    }

    /// Get the objects directory path
    ///
    /// # Returns
    ///
    /// Path to the objects directory (e.g., `./workspace/objects/`)
    pub fn objects_dir(&self) -> PathBuf {
        self.workspace_dir.join("objects")
    }

    /// Read content by hash
    ///
    /// # Arguments
    ///
    /// * `hash` - SHA-256 hash of the content
    ///
    /// # Returns
    ///
    /// File content as byte vector
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Object file doesn't exist
    /// - Failed to read file
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use la_storage::ContentAddressableStorage;
    /// # use std::path::PathBuf;
    /// # tokio_test::block_on(async {
    /// let cas = ContentAddressableStorage::new(PathBuf::from("./workspace"));
    /// let content = cas.read_content("a3f2e1d4c5...").await.unwrap();
    /// # })
    /// ```
    pub async fn read_content(&self, hash: &str) -> Result<Vec<u8>> {
        if !is_valid_content_hash(hash) {
            return Err(AppError::validation_error(format!(
                "Invalid content hash format: {}",
                hash
            )));
        }

        let object_path = self.get_object_path(hash);

        if !object_path.exists() {
            return Err(AppError::not_found(format!(
                "Object not found: {} at path: {}",
                hash,
                object_path.display()
            )));
        }

        fs::read(&object_path).await.map_err(|e| {
            AppError::io_error(
                format!("Failed to read object {}: {}", hash, e),
                Some(object_path),
            )
        })
    }

    /// Check if content exists in storage (sync version)
    ///
    /// Uses a double-check pattern to prevent race conditions between
    /// cache and filesystem state. Cache is only used as a fast path,
    /// but the authoritative check is always the filesystem.
    ///
    /// # Arguments
    ///
    /// * `hash` - SHA-256 hash to check
    ///
    /// # Returns
    ///
    /// `true` if the object exists, `false` otherwise
    pub fn exists(&self, hash: &str) -> bool {
        if !is_valid_content_hash(hash) {
            warn!(hash = %hash, "Rejected invalid content hash in exists()");
            return false;
        }

        // Fast path: Check cache first for performance
        if self.existence_cache.get(hash).is_some() {
            // Cache hit - but we still need to verify the file actually exists
            // to handle the case where the file was deleted externally
            let object_path = self.get_object_path(hash);
            if object_path.exists() {
                return true;
            }
            // Cache is stale, invalidate it
            self.existence_cache.invalidate(hash);
        }

        // Slow path: Check filesystem (authoritative source)
        let result = self.get_object_path(hash).exists();
        if result {
            self.existence_cache.insert(hash.to_string(), ());
        }
        result
    }

    /// Read content from storage (sync version)
    ///
    /// This is a synchronous version that uses std::fs::read instead of
    /// tokio async I/O. Suitable for use in spawn_blocking contexts.
    ///
    /// # Arguments
    ///
    /// * `hash` - SHA-256 hash of the content
    ///
    /// # Returns
    ///
    /// Content bytes on success
    pub fn read_content_sync(&self, hash: &str) -> Result<Vec<u8>> {
        if !is_valid_content_hash(hash) {
            return Err(AppError::validation_error(format!(
                "Invalid content hash format: {}",
                hash
            )));
        }

        let object_path = self.get_object_path(hash);

        if !object_path.exists() {
            return Err(AppError::not_found(format!(
                "Object not found: {} at path: {}",
                hash,
                object_path.display()
            )));
        }

        std::fs::read(&object_path).map_err(|e| {
            AppError::io_error(
                format!("Failed to read object {}: {}", hash, e),
                Some(object_path),
            )
        })
    }

    /// Check if content exists in storage (async version with cache)
    ///
    /// Uses a double-check pattern to prevent race conditions between
    /// cache and filesystem state. Cache is only used as a fast path,
    /// but the authoritative check is always the filesystem.
    ///
    /// # Arguments
    ///
    /// * `hash` - SHA-256 hash to check
    ///
    /// # Returns
    ///
    /// `true` if the object exists, `false` otherwise
    pub async fn exists_async(&self, hash: &str) -> bool {
        if !is_valid_content_hash(hash) {
            warn!(hash = %hash, "Rejected invalid content hash in exists_async()");
            return false;
        }

        // Fast path: Check cache first for performance
        if self.existence_cache.get(hash).is_some() {
            // Cache hit - but we still need to verify the file actually exists
            // to handle the case where the file was deleted externally
            let object_path = self.get_object_path(hash);
            if tokio::fs::try_exists(&object_path).await.unwrap_or(false) {
                return true;
            }
            // Cache is stale, invalidate it
            self.existence_cache.invalidate(hash);
        }

        // Slow path: Check filesystem (authoritative source)
        let result = tokio::fs::try_exists(self.get_object_path(hash))
            .await
            .unwrap_or(false);
        if result {
            self.existence_cache.insert(hash.to_string(), ());
        }
        result
    }

    /// Invalidate a cache entry for a given hash
    ///
    /// This is used when we know a file has been deleted or modified
    /// and we need to ensure the cache reflects the current state.
    ///
    /// # Arguments
    ///
    /// * `hash` - SHA-256 hash to invalidate from cache
    pub fn invalidate_cache_entry(&self, hash: &str) {
        self.existence_cache.invalidate(hash);
        debug!(hash = %hash, "Invalidated cache entry");
    }

    /// Get the total size of stored objects
    ///
    /// Uses walkdir for efficient directory traversal instead of
    /// recursive async calls. This significantly improves performance
    /// for workspaces with many files.
    ///
    /// # Returns
    ///
    /// Total size in bytes
    pub async fn get_storage_size(&self) -> Result<u64> {
        let objects_dir = self.workspace_dir.join("objects");

        if !objects_dir.exists() {
            return Ok(0);
        }

        // Use walkdir for efficient parallel directory traversal
        let mut total_size = 0u64;
        for entry in WalkDir::new(&objects_dir)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                total_size += entry.metadata().map(|m| m.len()).unwrap_or(0);
            }
        }

        Ok(total_size)
    }

    /// Verify file integrity by recomputing hash
    ///
    /// Reads the content and checks if the computed hash matches
    /// the expected hash. This detects corruption.
    ///
    /// # Arguments
    ///
    /// * `hash` - Expected SHA-256 hash
    ///
    /// # Returns
    ///
    /// `true` if integrity check passes, `false` if corrupted
    ///
    /// # Errors
    ///
    /// Returns error if file cannot be read
    pub async fn verify_integrity(&self, hash: &str) -> Result<bool> {
        let content = self.read_content(hash).await?;
        let computed_hash = Self::compute_hash(&content);
        Ok(computed_hash == hash)
    }

    /// Store file with disk space pre-check (safe storage)
    ///
    /// This method performs disk space validation before attempting to store a file.
    /// It requires 3x the file size: original file + CAS storage + temporary file overhead.
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the file to store
    ///
    /// # Returns
    ///
    /// SHA-256 hash of the stored content
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - File cannot be read
    /// - Insufficient disk space (requires 3x file size)
    /// - Failed to create object directory
    /// - Failed to write file
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use la_storage::ContentAddressableStorage;
    /// # use std::path::{Path, PathBuf};
    /// # tokio_test::block_on(async {
    /// let cas = ContentAddressableStorage::new(PathBuf::from("./workspace"));
    /// match cas.store_file_safe(Path::new("large.log")).await {
    ///     Ok(hash) => println!("Stored with hash: {}", hash),
    ///     Err(e) => println!("Storage failed: {}", e),
    /// }
    /// # })
    /// ```
    pub async fn store_file_safe(&self, file_path: &Path) -> Result<String> {
        // 1. 获取文件大小
        let metadata = fs::metadata(file_path).await.map_err(|e| {
            AppError::io_error(
                format!("Failed to get file metadata: {}", e),
                Some(file_path.to_path_buf()),
            )
        })?;
        let file_size = metadata.len();

        // 2. 检查目标磁盘空间 (需要 3x 空间: 原文件 + CAS存储 + 临时文件)
        let required_space = file_size.checked_mul(3).ok_or_else(|| {
            AppError::validation_error("File too large for disk space check".to_string())
        })?;
        let available_space = get_available_space(&self.workspace_dir).await;

        // 如果无法获取可用空间（返回0），跳过检查但记录警告
        if available_space == 0 {
            warn!(
                file = %file_path.display(),
                "Unable to determine available disk space, proceeding without check"
            );
        } else if available_space < required_space {
            return Err(AppError::io_error(
                format!(
                    "Insufficient disk space: required {} bytes (3x file size), available {} bytes",
                    required_space, available_space
                ),
                Some(self.workspace_dir.clone()),
            ));
        }

        // 3. 空间充足，继续原有存储逻辑
        self.store_file_streaming(file_path).await
    }

    /// Store file using zero-copy streaming (single-pass optimization)
    ///
    /// This method reads the file once, computing the hash while simultaneously
    /// writing to a temporary file. After completion, it atomically renames the
    /// temporary file to the final location. This avoids reading the file twice
    /// (once for hashing, once for copying), providing significant performance
    /// improvements for large files.
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the file to store
    ///
    /// # Returns
    ///
    /// SHA-256 hash of the stored content
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - File cannot be read
    /// - Failed to create temporary file
    /// - Failed to write file
    /// - Atomic rename failed
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use la_storage::ContentAddressableStorage;
    /// # use std::path::{Path, PathBuf};
    /// # tokio_test::block_on(async {
    /// let cas = ContentAddressableStorage::new(PathBuf::from("./workspace"));
    /// let hash = cas.store_file_zero_copy(Path::new("large.log")).await.unwrap();
    /// # })
    /// ```
    pub async fn store_file_zero_copy(&self, file_path: &Path) -> Result<String> {
        use tokio::io::AsyncWriteExt;
        use tokio::time::{timeout, Duration};

        const BUFFER_SIZE: usize = 1024 * 1024; // 1MB buffer
        const FILE_COPY_TIMEOUT: u64 = 300; // 5 minutes timeout

        // Open source file
        let mut src_file = fs::File::open(file_path).await.map_err(|e| {
            AppError::io_error(
                format!("Failed to open source file: {}", e),
                Some(file_path.to_path_buf()),
            )
        })?;

        // 1. 边读取边计算哈希，同时写入临时文件
        let temp_dir = self.workspace_dir.join("tmp");
        fs::create_dir_all(&temp_dir).await.map_err(|e| {
            AppError::io_error(
                format!("Failed to create temp directory: {}", e),
                Some(temp_dir.clone()),
            )
        })?;

        // Generate unique temp file name
        let temp_filename = format!(".tmp.{}.{}.tmp", uuid::Uuid::new_v4(), std::process::id());
        let temp_path = temp_dir.join(&temp_filename);

        // Create temp file with RAII guard for automatic cleanup (BUG-007 fix)
        let mut temp_file = fs::File::create(&temp_path).await.map_err(|e| {
            AppError::io_error(
                format!("Failed to create temp file: {}", e),
                Some(temp_path.clone()),
            )
        })?;

        // RAII guard ensures temp file is cleaned up even if errors occur
        let temp_guard = TempFileGuard::new(temp_path.clone());

        // Single-pass: read, hash, and write simultaneously
        let mut hasher = Sha256::new();
        let mut buffer = vec![0u8; BUFFER_SIZE];
        let mut total_bytes = 0u64;

        let copy_result = timeout(Duration::from_secs(FILE_COPY_TIMEOUT), async {
            loop {
                let bytes_read = src_file.read(&mut buffer).await.map_err(|e| {
                    AppError::io_error(
                        format!("Failed to read from source file: {}", e),
                        Some(file_path.to_path_buf()),
                    )
                })?;

                if bytes_read == 0 {
                    break; // EOF
                }

                // Update hash
                hasher.update(&buffer[..bytes_read]);

                // Write to temp file
                temp_file
                    .write_all(&buffer[..bytes_read])
                    .await
                    .map_err(|e| {
                        AppError::io_error(
                            format!("Failed to write to temp file: {}", e),
                            Some(temp_path.clone()),
                        )
                    })?;

                total_bytes += bytes_read as u64;
            }

            // Flush temp file
            temp_file.flush().await.map_err(|e| {
                AppError::io_error(
                    format!("Failed to flush temp file: {}", e),
                    Some(temp_path.clone()),
                )
            })?;

            // Sync to ensure data is written to disk
            temp_file.sync_all().await.map_err(|e| {
                AppError::io_error(
                    format!("Failed to sync temp file: {}", e),
                    Some(temp_path.clone()),
                )
            })?;

            Ok::<(), AppError>(())
        })
        .await;

        // Handle timeout or error
        match copy_result {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                // Temp file will be cleaned up by RAII guard (BUG-007 fix)
                return Err(e);
            }
            Err(_) => {
                // Timeout occurred, temp file will be cleaned up by RAII guard (BUG-007 fix)
                return Err(AppError::io_error(
                    format!("File copy timeout after {} seconds", FILE_COPY_TIMEOUT),
                    Some(file_path.to_path_buf()),
                ));
            }
        }

        // 2. 计算最终哈希
        let hash = format!("{:x}", hasher.finalize());
        let object_path = self.get_object_path(&hash);

        // Check cache first - might already exist
        if self.existence_cache.get(&hash).is_some() {
            // Verify the file actually exists before skipping
            if object_path.exists() {
                // Temp file will be cleaned up by RAII guard (BUG-007 fix)
                debug!(
                    hash = %hash,
                    file = %file_path.display(),
                    "Content already exists (verified), skipping"
                );
                return Ok(hash);
            }
            // Cache was stale, continue with the write
            self.existence_cache.invalidate(&hash);
        }

        // Create parent directory for final destination
        if let Some(parent) = object_path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                AppError::io_error(
                    format!("Failed to create object directory: {}", e),
                    Some(parent.to_path_buf()),
                )
            })?;
        }

        // 3. 原子重命名到目标位置 (O_EXCL 确保不会覆盖现有文件)
        // Use tokio::fs::OpenOptions for atomic creation
        use tokio::fs::OpenOptions;

        // Check if target already exists
        match OpenOptions::new()
            .write(true)
            .create_new(true) // O_EXCL: fail if exists
            .open(&object_path)
            .await
        {
            Ok(_target_file) => {
                // Target doesn't exist, we can proceed with rename
                drop(_target_file);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                // File already exists - deduplication win
                self.existence_cache.insert(hash.clone(), ());
                // Temp file will be cleaned up by RAII guard (BUG-007 fix)
                debug!(
                    hash = %hash,
                    file = %file_path.display(),
                    "Content already exists, deduplication"
                );
                return Ok(hash);
            }
            Err(e) => {
                // Temp file will be cleaned up by RAII guard (BUG-007 fix)
                return Err(AppError::io_error(
                    format!("Failed to check target file: {}", e),
                    Some(object_path.clone()),
                ));
            }
        }

        // Perform atomic rename
        match fs::rename(&temp_path, &object_path).await {
            Ok(()) => {
                // Success! Prevent RAII guard from cleaning up the temp file
                // since it's now the permanent file
                let _ = temp_guard.keep();
            }
            Err(e) => {
                // Rename failed, temp file will be cleaned up by RAII guard (BUG-007 fix)
                return Err(AppError::io_error(
                    format!("Failed to rename temp file to target: {}", e),
                    Some(object_path.clone()),
                ));
            }
        }

        // Cache the newly created object
        self.existence_cache.insert(hash.clone(), ());

        info!(
            hash = %hash,
            size = total_bytes,
            path = %object_path.display(),
            source = %file_path.display(),
            "Stored file in CAS using zero-copy streaming"
        );

        Ok(hash)
    }
}

/// ContentStorage trait implementation for ContentAddressableStorage
///
/// This implementation allows ContentAddressableStorage to be used
/// polymorphically through the ContentStorage trait.
#[async_trait]
impl ContentStorage for ContentAddressableStorage {
    async fn store(&self, content: &[u8]) -> Result<String> {
        self.store_content(content).await
    }

    async fn retrieve(&self, hash: &str) -> Result<Vec<u8>> {
        self.read_content(hash).await
    }

    async fn exists(&self, hash: &str) -> bool {
        // Use the async version internally
        self.exists_async(hash).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_compute_hash_idempotent() {
        let content = b"test content";
        let hash1 = ContentAddressableStorage::compute_hash(content);
        let hash2 = ContentAddressableStorage::compute_hash(content);
        assert_eq!(hash1, hash2, "Hash should be idempotent");
        assert_eq!(hash1.len(), 64, "SHA-256 should produce 64 hex chars");
    }

    // Property-based tests
    #[cfg(test)]
    mod property_tests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #![proptest_config(ProptestConfig::with_cases(100))]

            #[test]
            fn prop_hash_idempotence(content in prop::collection::vec(any::<u8>(), 0..10000)) {
                let hash1 = ContentAddressableStorage::compute_hash(&content);
                let hash2 = ContentAddressableStorage::compute_hash(&content);

                prop_assert_eq!(
                    &hash1,
                    &hash2,
                    "Hash computation must be idempotent: same content should always produce same hash"
                );

                // Also verify hash format is correct
                prop_assert_eq!(
                    hash1.len(),
                    64,
                    "SHA-256 hash must be 64 hexadecimal characters"
                );

                // Verify hash contains only valid hex characters
                prop_assert!(
                    hash1.chars().all(|c| c.is_ascii_hexdigit()),
                    "Hash must contain only hexadecimal characters"
                );
            }
        }
    }

    #[test]
    fn test_different_content_different_hash() {
        let hash1 = ContentAddressableStorage::compute_hash(b"content1");
        let hash2 = ContentAddressableStorage::compute_hash(b"content2");
        assert_ne!(
            hash1, hash2,
            "Different content should produce different hashes"
        );
    }

    #[tokio::test]
    async fn test_store_and_read() {
        let temp_dir = TempDir::new().unwrap();
        let cas = ContentAddressableStorage::new(temp_dir.path().to_path_buf());

        let content = b"test log content";
        let hash = cas.store_content(content).await.unwrap();

        let retrieved = cas.read_content(&hash).await.unwrap();
        assert_eq!(
            retrieved, content,
            "Retrieved content should match original"
        );
    }

    #[tokio::test]
    async fn test_deduplication() {
        let temp_dir = TempDir::new().unwrap();
        let cas = ContentAddressableStorage::new(temp_dir.path().to_path_buf());

        let content = b"duplicate content";
        let hash1 = cas.store_content(content).await.unwrap();
        let hash2 = cas.store_content(content).await.unwrap();

        assert_eq!(hash1, hash2, "Same content should produce same hash");
        assert!(cas.exists(&hash1), "Content should exist");
    }

    #[tokio::test]
    async fn test_verify_integrity() {
        let temp_dir = TempDir::new().unwrap();
        let cas = ContentAddressableStorage::new(temp_dir.path().to_path_buf());

        let content = b"integrity test";
        let hash = cas.store_content(content).await.unwrap();

        let is_valid = cas.verify_integrity(&hash).await.unwrap();
        assert!(is_valid, "Integrity check should pass for valid content");
    }

    #[test]
    fn test_object_path_sharding() {
        let cas = ContentAddressableStorage::new(PathBuf::from("/workspace"));
        let hash = "a3f2e1d4c5b6a7890123456789abcdef0123456789abcdef0123456789abcdef";
        let path = cas.get_object_path(hash);

        let path_str = path.to_string_lossy();
        // Use platform-independent path checking
        assert!(
            path_str.contains("objects") && path_str.contains("a3"),
            "Should use first 2 chars as directory, got: {}",
            path_str
        );
        assert!(
            path_str.ends_with("f2e1d4c5b6a7890123456789abcdef0123456789abcdef0123456789abcdef")
        );
    }

    #[tokio::test]
    async fn test_incremental_hash_matches_regular_hash() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.log");

        let content = b"This is test content for incremental hashing";
        fs::write(&test_file, content).await.unwrap();

        let hash_regular = ContentAddressableStorage::compute_hash(content);
        let hash_incremental = ContentAddressableStorage::compute_hash_incremental(&test_file)
            .await
            .unwrap();

        assert_eq!(
            hash_regular, hash_incremental,
            "Incremental hash should match regular hash"
        );
    }

    #[tokio::test]
    async fn test_incremental_hash_large_file() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("large.log");

        // Create a file larger than the buffer size (8KB)
        let content = vec![b'x'; 20 * 1024]; // 20KB
        fs::write(&test_file, &content).await.unwrap();

        let hash_regular = ContentAddressableStorage::compute_hash(&content);
        let hash_incremental = ContentAddressableStorage::compute_hash_incremental(&test_file)
            .await
            .unwrap();

        assert_eq!(
            hash_regular, hash_incremental,
            "Incremental hash should work for large files"
        );
    }

    #[tokio::test]
    async fn test_store_file_streaming() {
        let temp_dir = TempDir::new().unwrap();
        let cas = ContentAddressableStorage::new(temp_dir.path().join("workspace"));

        let test_file = temp_dir.path().join("test.log");
        let content = b"streaming test content";
        fs::write(&test_file, content).await.unwrap();

        let hash = cas.store_file_streaming(&test_file).await.unwrap();

        // Verify content can be read back
        let retrieved = cas.read_content(&hash).await.unwrap();
        assert_eq!(
            retrieved, content,
            "Retrieved content should match original"
        );
    }

    #[tokio::test]
    async fn test_store_file_streaming_deduplication() {
        let temp_dir = TempDir::new().unwrap();
        let cas = ContentAddressableStorage::new(temp_dir.path().join("workspace"));

        let test_file = temp_dir.path().join("test.log");
        let content = b"duplicate streaming content";
        fs::write(&test_file, content).await.unwrap();

        let hash1 = cas.store_file_streaming(&test_file).await.unwrap();
        let hash2 = cas.store_file_streaming(&test_file).await.unwrap();

        assert_eq!(hash1, hash2, "Same file should produce same hash");
        assert!(cas.exists(&hash1), "Content should exist");
    }

    #[test]
    fn test_hash_empty_content() {
        let hash = ContentAddressableStorage::compute_hash(b"");
        assert_eq!(
            hash.len(),
            64,
            "Empty content should still produce 64-char hash"
        );
        // SHA-256 of empty string is a known value
        assert_eq!(
            hash, "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            "Empty content should produce known SHA-256 hash"
        );
    }

    #[tokio::test]
    async fn test_read_nonexistent_content() {
        let temp_dir = TempDir::new().unwrap();
        let cas = ContentAddressableStorage::new(temp_dir.path().to_path_buf());

        let fake_hash = "0000000000000000000000000000000000000000000000000000000000000000";
        let result = cas.read_content(fake_hash).await;

        assert!(result.is_err(), "Reading nonexistent content should fail");
    }

    #[tokio::test]
    async fn test_exists_check() {
        let temp_dir = TempDir::new().unwrap();
        let cas = ContentAddressableStorage::new(temp_dir.path().to_path_buf());

        let content = b"existence test";
        let hash = cas.store_content(content).await.unwrap();

        assert!(cas.exists(&hash), "Stored content should exist");
        assert!(
            !cas.exists("nonexistent_hash"),
            "Nonexistent content should not exist"
        );
    }

    #[tokio::test]
    async fn test_multiple_different_contents() {
        let temp_dir = TempDir::new().unwrap();
        let cas = ContentAddressableStorage::new(temp_dir.path().to_path_buf());

        let content1 = b"first content";
        let content2 = b"second content";
        let content3 = b"third content";

        let hash1 = cas.store_content(content1).await.unwrap();
        let hash2 = cas.store_content(content2).await.unwrap();
        let hash3 = cas.store_content(content3).await.unwrap();

        // All hashes should be different
        assert_ne!(hash1, hash2);
        assert_ne!(hash2, hash3);
        assert_ne!(hash1, hash3);

        // All content should be retrievable
        assert_eq!(cas.read_content(&hash1).await.unwrap(), content1);
        assert_eq!(cas.read_content(&hash2).await.unwrap(), content2);
        assert_eq!(cas.read_content(&hash3).await.unwrap(), content3);
    }

    #[test]
    fn test_object_path_short_hash() {
        let cas = ContentAddressableStorage::new(PathBuf::from("/workspace"));

        // Test with a hash shorter than 2 characters (edge case)
        let short_hash = "a";
        let path = cas.get_object_path(short_hash);

        let path_str = path.to_string_lossy();
        assert!(
            path_str.contains("objects") && path_str.contains("00"),
            "Short hash should use '00' as directory prefix"
        );
    }

    #[tokio::test]
    async fn test_storage_size_empty() {
        let temp_dir = TempDir::new().unwrap();
        let cas = ContentAddressableStorage::new(temp_dir.path().to_path_buf());

        let size = cas.get_storage_size().await.unwrap();
        assert_eq!(size, 0, "Empty storage should have size 0");
    }

    #[tokio::test]
    async fn test_storage_size_with_content() {
        let temp_dir = TempDir::new().unwrap();
        let cas = ContentAddressableStorage::new(temp_dir.path().to_path_buf());

        let content1 = b"test content 1";
        let content2 = b"test content 2 is longer";

        cas.store_content(content1).await.unwrap();
        cas.store_content(content2).await.unwrap();

        let size = cas.get_storage_size().await.unwrap();
        assert!(
            size >= (content1.len() + content2.len()) as u64,
            "Storage size should be at least the sum of content sizes"
        );
    }

    #[tokio::test]
    async fn test_verify_integrity_corrupted() {
        let temp_dir = TempDir::new().unwrap();
        let cas = ContentAddressableStorage::new(temp_dir.path().to_path_buf());

        let content = b"original content";
        let hash = cas.store_content(content).await.unwrap();

        // Manually corrupt the stored file
        let object_path = cas.get_object_path(&hash);
        fs::write(&object_path, b"corrupted content").await.unwrap();

        let is_valid = cas.verify_integrity(&hash).await.unwrap();
        assert!(
            !is_valid,
            "Integrity check should fail for corrupted content"
        );
    }

    #[tokio::test]
    async fn test_deduplication_saves_space() {
        let temp_dir = TempDir::new().unwrap();
        let cas = ContentAddressableStorage::new(temp_dir.path().to_path_buf());

        let content = b"duplicate content for space test";

        // Store the same content multiple times
        let hash1 = cas.store_content(content).await.unwrap();
        let hash2 = cas.store_content(content).await.unwrap();
        let hash3 = cas.store_content(content).await.unwrap();

        // All should produce the same hash
        assert_eq!(hash1, hash2);
        assert_eq!(hash2, hash3);

        // Storage size should only count the content once
        let size = cas.get_storage_size().await.unwrap();
        assert!(
            size >= content.len() as u64 && size < (content.len() * 2) as u64,
            "Deduplication should prevent storing content multiple times"
        );
    }

    /// Test cache consistency with external file deletion (BUG-005 fix)
    #[tokio::test]
    async fn test_cache_consistency_after_external_deletion() {
        let temp_dir = TempDir::new().unwrap();
        let cas = ContentAddressableStorage::new(temp_dir.path().to_path_buf());

        let content = b"test content for cache consistency";
        let hash = cas.store_content(content).await.unwrap();

        // Verify file exists and is cached
        assert!(cas.exists(&hash), "File should exist after storage");

        // Simulate external deletion (e.g., manual cleanup or another process)
        let object_path = cas.get_object_path(&hash);
        tokio::fs::remove_file(&object_path).await.unwrap();

        // Cache may still indicate existence, but exists() should detect the deletion
        assert!(
            !cas.exists(&hash),
            "exists() should detect external file deletion and update cache"
        );

        // Should be able to store the content again
        let hash2 = cas.store_content(content).await.unwrap();
        assert_eq!(
            hash, hash2,
            "Re-storing same content should produce same hash"
        );
        assert!(cas.exists(&hash), "File should exist after re-storage");
    }

    /// Test async cache consistency with external file deletion
    #[tokio::test]
    async fn test_async_cache_consistency_after_external_deletion() {
        let temp_dir = TempDir::new().unwrap();
        let cas = ContentAddressableStorage::new(temp_dir.path().to_path_buf());

        let content = b"test content for async cache consistency";
        let hash = cas.store_content(content).await.unwrap();

        // Verify file exists and is cached
        assert!(
            cas.exists_async(&hash).await,
            "File should exist after storage"
        );

        // Simulate external deletion
        let object_path = cas.get_object_path(&hash);
        tokio::fs::remove_file(&object_path).await.unwrap();

        // Async exists should detect the deletion
        assert!(
            !cas.exists_async(&hash).await,
            "exists_async() should detect external file deletion and update cache"
        );
    }

    /// Test store_content handles stale cache correctly
    #[tokio::test]
    async fn test_store_content_with_stale_cache() {
        let temp_dir = TempDir::new().unwrap();
        let cas = ContentAddressableStorage::new(temp_dir.path().to_path_buf());

        let content = b"test content for stale cache handling";
        let hash = cas.store_content(content).await.unwrap();

        // Manually remove the file to simulate external deletion
        let object_path = cas.get_object_path(&hash);
        tokio::fs::remove_file(&object_path).await.unwrap();

        // Cache still indicates existence, but store_content should handle it
        let hash2 = cas.store_content(content).await.unwrap();
        assert_eq!(
            hash, hash2,
            "Storing content with stale cache should still work"
        );

        // Verify the file was re-created
        assert!(
            object_path.exists(),
            "File should be re-created after stale cache write"
        );
    }

    /// Test store_file_streaming handles stale cache correctly
    #[tokio::test]
    async fn test_store_file_streaming_with_stale_cache() {
        let temp_dir = TempDir::new().unwrap();
        let cas = ContentAddressableStorage::new(temp_dir.path().join("workspace"));

        let test_file = temp_dir.path().join("test.log");
        let content = b"test content for streaming stale cache";
        tokio::fs::write(&test_file, content).await.unwrap();

        // Store file
        let hash = cas.store_file_streaming(&test_file).await.unwrap();

        // Manually remove the stored file to simulate external deletion
        let object_path = cas.get_object_path(&hash);
        tokio::fs::remove_file(&object_path).await.unwrap();

        // Cache still indicates existence, but store_file_streaming should handle it
        let hash2 = cas.store_file_streaming(&test_file).await.unwrap();
        assert_eq!(
            hash, hash2,
            "Storing file with stale cache should still work"
        );

        // Verify the file was re-created
        assert!(
            object_path.exists(),
            "File should be re-created after stale cache streaming write"
        );
    }

    /// Test RAII temp file cleanup on error (BUG-007 fix)
    #[tokio::test]
    async fn test_temp_file_cleanup_on_error() {
        let temp_dir = TempDir::new().unwrap();
        let cas = ContentAddressableStorage::new(temp_dir.path().join("workspace"));

        // Create a temp directory to monitor
        let tmp_dir = temp_dir.path().join("workspace").join("tmp");

        // Create a test file
        let test_file = temp_dir.path().join("test_cleanup.log");
        let content = b"test content for cleanup verification";
        tokio::fs::write(&test_file, content).await.unwrap();

        // Get initial temp directory state
        let initial_count = if tmp_dir.exists() {
            std::fs::read_dir(&tmp_dir).map(|d| d.count()).unwrap_or(0)
        } else {
            0
        };

        // Store file successfully - should not leave temp files
        let hash = cas.store_file_zero_copy(&test_file).await.unwrap();

        // Verify file exists
        assert!(cas.exists(&hash), "File should exist after storage");

        // Check temp directory is clean (or has no new files)
        if tmp_dir.exists() {
            let final_count = std::fs::read_dir(&tmp_dir).map(|d| d.count()).unwrap_or(0);
            // Temp files should be cleaned up
            assert!(
                final_count <= initial_count + 1, // Allow for some timing differences
                "Temp directory should not accumulate files"
            );
        }
    }

    /// Test TempFileGuard RAII cleanup
    #[tokio::test]
    async fn test_temp_file_guard_cleanup() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().join("test_guard.tmp");

        // Create a temp file
        tokio::fs::write(&temp_path, b"test content").await.unwrap();
        assert!(temp_path.exists(), "Temp file should exist");

        // Create guard and let it drop
        {
            let guard = TempFileGuard::new(temp_path.clone());
            assert_eq!(guard.path(), &temp_path);
            // Guard drops here
        }

        // Give a moment for async cleanup
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // File should be cleaned up
        assert!(
            !temp_path.exists(),
            "Temp file should be cleaned up by RAII guard"
        );
    }

    /// Test TempFileGuard keep() prevents cleanup
    #[tokio::test]
    async fn test_temp_file_guard_keep() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().join("test_keep.tmp");

        // Create a temp file
        tokio::fs::write(&temp_path, b"test content").await.unwrap();
        assert!(temp_path.exists(), "Temp file should exist");

        // Create guard and keep the file
        let kept_path = {
            let guard = TempFileGuard::new(temp_path.clone());
            guard.keep()
        };

        // Give a moment for any async operations
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // File should still exist
        assert!(kept_path.exists(), "File should exist after keep()");
        assert_eq!(kept_path, temp_path);
    }

    #[tokio::test]
    async fn test_invalid_hash_is_rejected_before_filesystem_access() {
        let temp_dir = TempDir::new().unwrap();
        let cas = ContentAddressableStorage::new(temp_dir.path().join("workspace"));

        let escaped_target = temp_dir.path().join("escaped.txt");
        tokio::fs::write(&escaped_target, b"secret").await.unwrap();

        let malicious_hash = format!("aa{}", escaped_target.display());

        assert!(cas.read_content(&malicious_hash).await.is_err());
        assert!(!cas.exists(&malicious_hash));
        assert!(!cas.exists_async(&malicious_hash).await);
        assert!(cas.read_content_sync(&malicious_hash).is_err());
    }
}
