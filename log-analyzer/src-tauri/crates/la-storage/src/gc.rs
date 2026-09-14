//! 对象级垃圾回收：删除不再被元数据引用的 CAS 孤儿对象。

use std::collections::HashSet;

use la_core::error::{AppError, Result};

use crate::{ContentAddressableStorage, MetadataStore};

/// 删除所有未被任何文件/归档引用的 CAS 对象，返回删除数量。
///
/// Readers and producers must hold `cas.activity_guard()` for their entire
/// operation. Skip this collection if any are active; acquire exclusive access
/// before taking the reference snapshot so a new producer cannot race the sweep.
pub async fn gc_orphaned_objects(
    cas: &ContentAddressableStorage,
    metadata: &MetadataStore,
) -> Result<usize> {
    let Some(guard) = cas.try_gc_guard() else {
        tracing::debug!("Skipping CAS GC while workspace operations are active");
        return Ok(0);
    };
    let files = metadata
        .get_all_files()
        .await
        .map_err(|e| AppError::database_error(format!("Failed to enumerate files for GC: {e}")))?;
    let archives = metadata.get_all_archives().await.map_err(|e| {
        AppError::database_error(format!("Failed to enumerate archives for GC: {e}"))
    })?;

    let mut referenced: HashSet<String> = HashSet::with_capacity(files.len() + archives.len());
    for file in &files {
        referenced.insert(file.sha256_hash.clone());
    }
    for archive in &archives {
        referenced.insert(archive.sha256_hash.clone());
    }

    let cas = cas.clone();
    tokio::task::spawn_blocking(move || {
        let _guard = guard;
        let mut removed = 0usize;
        for hash in cas.list_object_hashes() {
            if !referenced.contains(&hash) {
                cas.remove_object(&hash)?;
                removed += 1;
            }
        }
        Ok(removed)
    })
    .await
    .map_err(|error| AppError::internal_error(format!("CAS GC worker failed: {error}")))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FileMetadata;
    use tempfile::TempDir;

    #[tokio::test]
    async fn gc_defers_for_uncommitted_producer_and_shared_cas_clone() {
        let temp = TempDir::new().unwrap();
        let cas = ContentAddressableStorage::new(temp.path().to_path_buf());
        let metadata = MetadataStore::new(temp.path()).await.unwrap();
        let clone = cas.clone();
        let producer = clone.activity_guard().await.unwrap();
        let hash = cas.store_content(b"not committed yet").await.unwrap();
        assert_eq!(gc_orphaned_objects(&cas, &metadata).await.unwrap(), 0);
        assert!(cas.exists(&hash));
        drop(producer);
        assert_eq!(gc_orphaned_objects(&cas, &metadata).await.unwrap(), 1);
        assert!(!cas.exists(&hash));
    }

    #[tokio::test]
    async fn exclusive_gc_blocks_new_operations_until_sweep_finishes() {
        use futures::FutureExt;
        let temp = TempDir::new().unwrap();
        let cas = ContentAddressableStorage::new(temp.path().to_path_buf());
        let gc = cas.try_gc_guard().unwrap();
        assert!(cas.activity_guard().now_or_never().is_none());
        drop(gc);
        assert!(cas.activity_guard().now_or_never().is_some());
    }

    #[tokio::test]
    async fn gc_removes_orphans_and_keeps_referenced_objects() {
        let temp = TempDir::new().unwrap();
        let workspace = temp.path().join("workspace");
        let cas = ContentAddressableStorage::new(workspace.clone());
        let metadata = MetadataStore::new(&workspace).await.unwrap();

        // 两个被引用的对象(一个文件、一个归档)
        let kept_file_hash = cas.store_content(b"referenced file content").await.unwrap();
        let kept_archive_hash = cas
            .store_content(b"referenced archive bytes")
            .await
            .unwrap();
        // 一个孤儿对象
        let orphan_hash = cas.store_content(b"orphan bytes").await.unwrap();

        metadata
            .insert_file(&FileMetadata {
                id: 0,
                sha256_hash: kept_file_hash.clone(),
                virtual_path: "a.log".to_string(),
                original_name: "a.log".to_string(),
                size: 20,
                modified_time: 0,
                mime_type: None,
                parent_archive_id: None,
                depth_level: 0,
                min_timestamp: None,
                max_timestamp: None,
                level_mask: None,
                analysis_status: la_core::storage_types::AnalysisStatus::Ready,
            })
            .await
            .unwrap();
        metadata
            .insert_archive(&crate::ArchiveMetadata {
                id: 0,
                sha256_hash: kept_archive_hash.clone(),
                virtual_path: "a.zip".to_string(),
                original_name: "a.zip".to_string(),
                archive_type: "zip".to_string(),
                parent_archive_id: None,
                depth_level: 0,
                extraction_status: "done".to_string(),
            })
            .await
            .unwrap();

        let removed = gc_orphaned_objects(&cas, &metadata).await.unwrap();

        assert_eq!(removed, 1, "only the orphan must be removed");
        assert!(cas.exists(&kept_file_hash));
        assert!(cas.exists(&kept_archive_hash));
        assert!(!cas.exists(&orphan_hash));
    }
}
