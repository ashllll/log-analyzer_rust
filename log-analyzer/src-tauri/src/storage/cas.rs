//! Content-Addressable Storage (CAS) Implementation
//!
//! Based on Git's object storage model, this module provides:
//! - SHA-256 content hashing (industry standard)
//! - Flat directory structure (avoids path length limits)
//! - Automatic deduplication (same content = same hash)
//! - Efficient storage and retrieval
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

use crate::error::{AppError, Result};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::{AsyncReadExt, BufReader};
use tracing::{debug, error, info, warn};

/// Content-Addressable Storage manager
///
/// Provides Git-style content storage with SHA-256 hashing.
/// All files are stored in a flat structure under `workspace_dir/objects/`.
#[derive(Debug, Clone)]
pub struct ContentAddressableStorage {
    workspace_dir: PathBuf,
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
    /// use log_analyzer::storage::ContentAddressableStorage;
    ///
    /// let cas = ContentAddressableStorage::new(PathBuf::from("./workspace_123"));
    /// ```
    pub fn new(workspace_dir: PathBuf) -> Self {
        Self { workspace_dir }
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
    /// use log_analyzer::storage::ContentAddressableStorage;
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
    /// # use log_analyzer::storage::ContentAddressableStorage;
    /// # use std::path::Path;
    /// # tokio_test::block_on(async {
    /// let hash = ContentAddressableStorage::compute_hash_incremental(
    ///     Path::new("large_file.log")
    /// ).await.unwrap();
    /// # })
    /// ```
    pub async fn compute_hash_incremental(file_path: &Path) -> Result<String> {
        const BUFFER_SIZE: usize = 8 * 1024; // 8KB buffer

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
    /// # use log_analyzer::storage::ContentAddressableStorage;
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

        // Check if object already exists (deduplication)
        if object_path.exists() {
            debug!(
                hash = %hash,
                file = %file_path.display(),
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

        // Copy file to object storage with timeout protection (industry standard)
        // Use tokio::io::copy for true async copy (fs::copy blocks on Windows!)
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::time::{timeout, Duration};

        const FILE_COPY_TIMEOUT: u64 = 300; // 5 minutes timeout for large files
        const COPY_BUFFER_SIZE: usize = 64 * 1024; // 64KB buffer for efficient copying

        let copy_result = timeout(Duration::from_secs(FILE_COPY_TIMEOUT), async {
            // Open source file
            let mut src_file = fs::File::open(file_path).await.map_err(|e| {
                AppError::io_error(
                    format!("Failed to open source file: {}", e),
                    Some(file_path.to_path_buf()),
                )
            })?;

            // Create target file
            let mut dst_file = fs::File::create(&object_path).await.map_err(|e| {
                AppError::io_error(
                    format!("Failed to create target file: {}", e),
                    Some(object_path.clone()),
                )
            })?;

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
                // Copy failed with error
                error!(
                    file = %file_path.display(),
                    target = %object_path.display(),
                    error = %e,
                    "File copy failed"
                );
                return Err(e);
            }
            Err(_) => {
                // Timeout occurred
                error!(
                    file = %file_path.display(),
                    target = %object_path.display(),
                    timeout_secs = FILE_COPY_TIMEOUT,
                    "File copy timeout after {} seconds",
                    FILE_COPY_TIMEOUT
                );
                // Clean up partial file
                match fs::remove_file(&object_path).await {
                    Ok(_) => {
                        debug!("Successfully cleaned up partial file after timeout");
                    }
                    Err(e) => {
                        error!(
                            target = %object_path.display(),
                            error = %e,
                            "Failed to clean up partial file after timeout: {}",
                            e
                        );
                        // 注意：部分文件残留，但已经返回错误给调用者
                        // 可以考虑添加到清理队列以便后续重试
                    }
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
    /// # use log_analyzer::storage::ContentAddressableStorage;
    /// # use std::path::PathBuf;
    /// # tokio_test::block_on(async {
    /// let cas = ContentAddressableStorage::new(PathBuf::from("./workspace"));
    /// let hash = cas.store_content(b"log content").await.unwrap();
    /// # })
    /// ```
    pub async fn store_content(&self, content: &[u8]) -> Result<String> {
        let hash = Self::compute_hash(content);
        let object_path = self.get_object_path(&hash);

        // Check if object already exists (deduplication)
        if object_path.exists() {
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

        // Write content to file
        fs::write(&object_path, content).await.map_err(|e| {
            AppError::io_error(
                format!("Failed to write object file: {}", e),
                Some(object_path.clone()),
            )
        })?;

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
    /// # use log_analyzer::storage::ContentAddressableStorage;
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
    /// # use log_analyzer::storage::ContentAddressableStorage;
    /// # use std::path::PathBuf;
    /// # tokio_test::block_on(async {
    /// let cas = ContentAddressableStorage::new(PathBuf::from("./workspace"));
    /// let content = cas.read_content("a3f2e1d4c5...").await.unwrap();
    /// # })
    /// ```
    pub async fn read_content(&self, hash: &str) -> Result<Vec<u8>> {
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

    /// Check if content exists in storage
    ///
    /// # Arguments
    ///
    /// * `hash` - SHA-256 hash to check
    ///
    /// # Returns
    ///
    /// `true` if the object exists, `false` otherwise
    pub fn exists(&self, hash: &str) -> bool {
        self.get_object_path(hash).exists()
    }

    /// Get the total size of stored objects
    ///
    /// Walks the objects directory and sums file sizes.
    /// Useful for monitoring storage usage.
    ///
    /// # Returns
    ///
    /// Total size in bytes
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
            let path = entry.path();
            if path.is_dir() {
                // Recursively sum subdirectory
                total_size += self.get_directory_size(&path).await?;
            }
        }

        Ok(total_size)
    }

    /// Helper: Get size of a directory recursively
    async fn get_directory_size(&self, dir: &Path) -> Result<u64> {
        let mut total_size = 0u64;
        let mut entries = fs::read_dir(dir).await.map_err(|e| {
            AppError::io_error(
                format!("Failed to read directory: {}", e),
                Some(dir.to_path_buf()),
            )
        })?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            AppError::io_error(
                format!("Failed to read directory entry: {}", e),
                Some(dir.to_path_buf()),
            )
        })? {
            let path = entry.path();
            if path.is_file() {
                if let Ok(metadata) = fs::metadata(&path).await {
                    total_size += metadata.len();
                }
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
}
