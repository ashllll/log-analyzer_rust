//! 文件变更检测服务
//!
//! 使用 CAS (Content-Addressed Storage) 哈希系统检测文件变更，
//! 避免对未变更的文件进行不必要的重新索引。
//!
//! ## 功能
//!
//! - 基于 SHA-256 哈希的变更检测
//! - 支持增量索引偏移量恢复
//! - 文件大小和修改时间快速预检
//! - 与现有的 CAS 系统集成
//!
//! ## 使用场景
//!
//! - 应用重启后从上次位置继续读取
//! - 避免对未变更文件重新索引
//! - 检测文件截断和内容变更

use crate::error::{AppError, Result};
use crate::storage::ContentAddressableStorage;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::fs;
use tracing::debug;

/// 文件变更检测结果
#[derive(Debug, Clone)]
pub enum FileChangeStatus {
    /// 新文件，需要索引
    NewFile,
    /// 文件内容已变更，需要重新索引
    ContentChanged,
    /// 文件未变更，可以使用缓存索引
    Unchanged { last_offset: u64 },
    /// 文件被截断，需要从头索引
    Truncated,
}

/// 文件变更检测器
///
/// 使用 CAS 哈希系统检测文件是否需要重新索引
pub struct FileChangeDetector {
    /// 文件变更缓存（可选优化）
    /// 映射：file_path -> (modified_time, file_size)
    change_cache: Arc<Mutex<HashMap<String, (i64, u64)>>>,
}

impl FileChangeDetector {
    /// 创建新的文件变更检测器
    pub fn new() -> Self {
        Self {
            change_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 检查文件是否需要重新索引
    ///
    /// # 参数
    ///
    /// * `path` - 文件路径
    /// * `workspace_id` - 工作区 ID
    /// * `indexed_file` - 已保存的索引文件记录（如果存在）
    ///
    /// # 返回值
    ///
    /// 返回文件变更状态，指示是否需要重新索引
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let detector = FileChangeDetector::new();
    /// let status = detector.check_file_change(
    ///     Path::new("/path/to/file.log"),
    ///     "workspace_123",
    ///     Some(&indexed_file)
    /// ).await?;
    ///
    /// match status {
    ///     FileChangeStatus::NewFile => { /* 索引整个文件 */ }
    ///     FileChangeStatus::ContentChanged => { /* 重新索引 */ }
    ///     FileChangeStatus::Unchanged { last_offset } => { /* 从 last_offset 继续 */ }
    ///     FileChangeStatus::Truncated => { /* 从头索引 */ }
    /// }
    /// ```
    pub async fn check_file_change(
        &self,
        path: &Path,
        _workspace_id: &str,
        indexed_file: Option<&crate::storage::IndexedFile>,
    ) -> Result<FileChangeStatus> {
        // 获取当前文件元数据
        let metadata = fs::metadata(path).await.map_err(|e| {
            AppError::io_error(
                format!("Failed to read file metadata: {}", e),
                Some(path.to_path_buf()),
            )
        })?;

        let current_size = metadata.len();
        let modified = metadata.modified().map_err(|e| {
            AppError::io_error(
                format!("Failed to get modified time: {}", e),
                Some(path.to_path_buf()),
            )
        })?;
        let current_modified_time = modified
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| AppError::validation_error(format!("Invalid timestamp: {}", e)))?
            .as_secs() as i64;

        // 新文件：没有索引记录
        let indexed = match indexed_file {
            Some(file) => file,
            None => {
                debug!(
                    file = %path.display(),
                    "New file detected, needs indexing"
                );
                return Ok(FileChangeStatus::NewFile);
            }
        };

        // 快速检查：如果文件大小变小，说明被截断了
        if current_size < indexed.file_size as u64 {
            debug!(
                file = %path.display(),
                old_size = indexed.file_size,
                new_size = current_size,
                "File truncated, needs full re-index"
            );
            return Ok(FileChangeStatus::Truncated);
        }

        // 快速检查：如果修改时间不同，计算哈希确认
        if current_modified_time != indexed.modified_time {
            // 计算当前文件哈希
            let current_hash = ContentAddressableStorage::compute_hash_incremental(path).await?;

            if current_hash != indexed.hash {
                debug!(
                    file = %path.display(),
                    old_hash = %indexed.hash,
                    new_hash = %current_hash,
                    "File content changed, needs re-index"
                );
                return Ok(FileChangeStatus::ContentChanged);
            }

            // 哈希相同但修改时间不同（可能是文件被替换为相同内容）
            // 更新修改时间记录，但不重新索引
            debug!(
                file = %path.display(),
                "File hash unchanged but modified time differs, treating as unchanged"
            );
        }

        // 文件未变更，可以从上次偏移量继续
        debug!(
            file = %path.display(),
            last_offset = indexed.last_offset,
            "File unchanged, can continue from last offset"
        );

        Ok(FileChangeStatus::Unchanged {
            last_offset: indexed.last_offset,
        })
    }

    /// 批量检查多个文件的变更状态
    ///
    /// # 参数
    ///
    /// * `files` - 文件路径和对应的索引记录列表
    /// * `workspace_id` - 工作区 ID
    ///
    /// # 返回值
    ///
    /// 返回每个文件的变更状态列表
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let files = vec![
    ///     (PathBuf::from("/path/file1.log"), Some(indexed_file1)),
    ///     (PathBuf::from("/path/file2.log"), None),
    /// ];
    ///
    /// let statuses = detector.check_files_change(&files, "workspace_123").await?;
    /// ```
    pub async fn check_files_change(
        &self,
        files: &[(impl AsRef<Path>, Option<&crate::storage::IndexedFile>)],
        workspace_id: &str,
    ) -> Result<Vec<FileChangeStatus>> {
        let mut results = Vec::with_capacity(files.len());

        for (path, indexed_file) in files {
            let status = self
                .check_file_change(path.as_ref(), workspace_id, *indexed_file)
                .await?;
            results.push(status);
        }

        Ok(results)
    }

    /// 更新变更缓存
    ///
    /// 用于优化性能，避免重复读取文件元数据
    pub fn update_cache(&self, file_path: String, modified_time: i64, file_size: u64) {
        let mut cache = self.change_cache.lock();
        cache.insert(file_path, (modified_time, file_size));
    }

    /// 清除变更缓存
    ///
    /// 当工作区删除或重新索引时调用
    pub fn clear_cache(&self) {
        let mut cache = self.change_cache.lock();
        cache.clear();
    }

    /// 从缓存中移除指定文件
    pub fn remove_from_cache(&self, file_path: &str) {
        let mut cache = self.change_cache.lock();
        cache.remove(file_path);
    }
}

impl Default for FileChangeDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::IndexedFile;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_detector() -> FileChangeDetector {
        FileChangeDetector::new()
    }

    #[tokio::test]
    async fn test_check_new_file() {
        let detector = create_test_detector();
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.log");

        // 创建测试文件
        let mut file = std::fs::File::create(&file_path).unwrap();
        file.write_all(b"test content").unwrap();

        // 检查新文件（没有索引记录）
        let status = detector
            .check_file_change(&file_path, "test_workspace", None)
            .await
            .unwrap();

        assert!(matches!(status, FileChangeStatus::NewFile));
    }

    #[tokio::test]
    async fn test_check_unchanged_file() {
        let detector = create_test_detector();
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.log");

        // 创建测试文件
        let mut file = std::fs::File::create(&file_path).unwrap();
        let content = b"test content";
        file.write_all(content).unwrap();

        // 计算文件哈希
        let hash = ContentAddressableStorage::compute_hash(content);
        let metadata = std::fs::metadata(&file_path).unwrap();
        let modified = metadata.modified().unwrap();
        let modified_time = modified
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        // 创建索引记录
        let indexed_file = IndexedFile {
            file_path: file_path.to_string_lossy().to_string(),
            workspace_id: "test_workspace".to_string(),
            last_offset: 100,
            file_size: metadata.len() as i64,
            modified_time,
            hash,
        };

        // 检查未变更文件
        let status = detector
            .check_file_change(&file_path, "test_workspace", Some(&indexed_file))
            .await
            .unwrap();

        assert!(matches!(status, FileChangeStatus::Unchanged { .. }));
        if let FileChangeStatus::Unchanged { last_offset } = status {
            assert_eq!(last_offset, 100);
        }
    }

    #[tokio::test]
    async fn test_check_content_changed_file() {
        let detector = create_test_detector();
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.log");

        // 创建初始文件
        let mut file = std::fs::File::create(&file_path).unwrap();
        let old_content = b"old content";
        file.write_all(old_content).unwrap();

        let old_hash = ContentAddressableStorage::compute_hash(old_content);
        let metadata = std::fs::metadata(&file_path).unwrap();
        let modified = metadata.modified().unwrap();
        let modified_time = modified
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        // 创建索引记录（使用旧哈希）
        let indexed_file = IndexedFile {
            file_path: file_path.to_string_lossy().to_string(),
            workspace_id: "test_workspace".to_string(),
            last_offset: 100,
            file_size: metadata.len() as i64,
            modified_time,
            hash: old_hash,
        };

        // 等待足够时间确保修改时间不同（某些文件系统精度较低）
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // 修改文件内容
        let mut file = std::fs::File::create(&file_path).unwrap();
        file.write_all(b"new content").unwrap();
        file.sync_all().unwrap(); // 确保内容写入磁盘

        // 再次等待以确保修改时间被更新
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // 检查已变更文件
        let status = detector
            .check_file_change(&file_path, "test_workspace", Some(&indexed_file))
            .await
            .unwrap();

        assert!(matches!(status, FileChangeStatus::ContentChanged));
    }

    #[tokio::test]
    async fn test_check_truncated_file() {
        let detector = create_test_detector();
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.log");

        // 创建大文件
        let mut file = std::fs::File::create(&file_path).unwrap();
        let large_content = b"large content that will be truncated";
        file.write_all(large_content).unwrap();

        let hash = ContentAddressableStorage::compute_hash(large_content);
        let metadata = std::fs::metadata(&file_path).unwrap();
        let modified = metadata.modified().unwrap();
        let modified_time = modified
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        // 创建索引记录（记录较大的文件大小）
        let indexed_file = IndexedFile {
            file_path: file_path.to_string_lossy().to_string(),
            workspace_id: "test_workspace".to_string(),
            last_offset: 100,
            file_size: metadata.len() as i64,
            modified_time,
            hash,
        };

        // 截断文件
        let mut file = std::fs::File::create(&file_path).unwrap();
        file.write_all(b"small").unwrap();

        // 检查截断文件
        let status = detector
            .check_file_change(&file_path, "test_workspace", Some(&indexed_file))
            .await
            .unwrap();

        assert!(matches!(status, FileChangeStatus::Truncated));
    }

    #[tokio::test]
    async fn test_check_files_change_batch() {
        let detector = create_test_detector();
        let temp_dir = TempDir::new().unwrap();

        let file1_path = temp_dir.path().join("file1.log");
        let file2_path = temp_dir.path().join("file2.log");

        // 创建文件1
        let mut file = std::fs::File::create(&file1_path).unwrap();
        file.write_all(b"content1").unwrap();

        // 创建文件2
        let mut file = std::fs::File::create(&file2_path).unwrap();
        file.write_all(b"content2").unwrap();

        let files: Vec<(std::path::PathBuf, Option<&IndexedFile>)> = vec![
            (file1_path.clone(), None), // 新文件
            (file2_path.clone(), None), // 新文件
        ];

        let statuses = detector
            .check_files_change(&files, "test_workspace")
            .await
            .unwrap();

        assert_eq!(statuses.len(), 2);
        assert!(matches!(statuses[0], FileChangeStatus::NewFile));
        assert!(matches!(statuses[1], FileChangeStatus::NewFile));
    }

    #[tokio::test]
    async fn test_cache_operations() {
        let detector = create_test_detector();

        // 测试更新缓存
        detector.update_cache("/path/to/file.log".to_string(), 12345, 678);

        // 验证缓存已更新
        let cache = detector.change_cache.lock();
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.get("/path/to/file.log"), Some(&(12345, 678)));

        drop(cache);

        // 测试清除缓存
        detector.clear_cache();
        let cache = detector.change_cache.lock();
        assert_eq!(cache.len(), 0);

        drop(cache);

        // 测试从缓存移除
        detector.update_cache("/path/to/file1.log".to_string(), 111, 222);
        detector.update_cache("/path/to/file2.log".to_string(), 333, 444);
        detector.remove_from_cache("/path/to/file1.log");

        let cache = detector.change_cache.lock();
        assert_eq!(cache.len(), 1);
        assert!(cache.get("/path/to/file1.log").is_none());
        assert_eq!(cache.get("/path/to/file2.log"), Some(&(333, 444)));
    }
}
