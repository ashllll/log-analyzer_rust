//! Virtual File Tree — business logic
//!
//! Encapsulates the tree-building algorithm that constructs a hierarchical
//! `VirtualTreeNode` representation from flat metadata (archive + file lists).

use la_storage::MetadataStore;
use serde::{Deserialize, Serialize};

/// Virtual file tree node
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum VirtualTreeNode {
    #[serde(rename = "file")]
    File {
        name: String,
        path: String,
        hash: String,
        size: i64,
        #[serde(rename = "mimeType")]
        mime_type: Option<String>,
    },
    #[serde(rename = "archive")]
    Archive {
        name: String,
        path: String,
        hash: String,
        #[serde(rename = "archiveType")]
        archive_type: String,
        children: Vec<VirtualTreeNode>,
    },
}

/// Build hierarchical tree structure from flat data
pub async fn build_tree_structure(
    archives: &[la_storage::ArchiveMetadata],
    files: &[la_storage::FileMetadata],
    _metadata_store: &MetadataStore,
) -> Result<Vec<VirtualTreeNode>, String> {
    let mut tree = Vec::new();

    // 预构建索引：parent_archive_id -> 子 archive/file 列表，O(n) → O(1) 查找
    use std::collections::HashMap;
    let mut archive_children: HashMap<i64, Vec<&la_storage::ArchiveMetadata>> = HashMap::new();
    for a in archives {
        if let Some(parent_id) = a.parent_archive_id {
            archive_children.entry(parent_id).or_default().push(a);
        }
    }
    let mut file_children: HashMap<i64, Vec<&la_storage::FileMetadata>> = HashMap::new();
    for f in files {
        if let Some(parent_id) = f.parent_archive_id {
            file_children.entry(parent_id).or_default().push(f);
        }
    }

    // Find root-level archives (no parent)
    let root_archives: Vec<_> = archives
        .iter()
        .filter(|a| a.parent_archive_id.is_none())
        .collect();

    // Find root-level files (no parent archive)
    let root_files: Vec<_> = files
        .iter()
        .filter(|f| f.parent_archive_id.is_none())
        .collect();

    // Add root archives with their children
    for archive in root_archives {
        let node = build_archive_node_indexed(
            archive,
            &archive_children,
            &file_children,
            std::collections::HashSet::new(),
            0,
        )
        .await?;
        tree.push(node);
    }

    // Add root files
    for file in root_files {
        tree.push(VirtualTreeNode::File {
            name: file.original_name.clone(),
            path: file.virtual_path.clone(),
            hash: file.sha256_hash.clone(),
            size: file.size,
            mime_type: file.mime_type.clone(),
        });
    }

    Ok(tree)
}

/// 归档树最大递归深度防御上限（metadata 层上限远小于此，防止数据
/// 损坏导致的自引用/成环无限递归）。
const MAX_ARCHIVE_TREE_DEPTH: usize = 128;

/// Build archive node with pre-built HashMap indexes, O(n) total instead of O(n²)
///
/// P2#9 修复：携带 visited 集合与深度计数，parent_archive_id 自引用或
/// 成环时跳过而非无限递归；超深树返回错误而非栈溢出。
fn build_archive_node_indexed<'a>(
    archive: &'a la_storage::ArchiveMetadata,
    archive_children: &'a std::collections::HashMap<i64, Vec<&'a la_storage::ArchiveMetadata>>,
    file_children: &'a std::collections::HashMap<i64, Vec<&'a la_storage::FileMetadata>>,
    mut visited: std::collections::HashSet<i64>,
    depth: usize,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<VirtualTreeNode, String>> + Send + 'a>>
{
    Box::pin(async move {
        if depth > MAX_ARCHIVE_TREE_DEPTH {
            return Err(format!(
                "Archive tree nesting exceeds {MAX_ARCHIVE_TREE_DEPTH} levels"
            ));
        }
        visited.insert(archive.id);
        let mut children = Vec::new();

        // O(1) HashMap lookup instead of O(n) linear scan
        if let Some(child_archives) = archive_children.get(&archive.id) {
            for child_archive in child_archives {
                if visited.contains(&child_archive.id) {
                    tracing::warn!(
                        archive_id = child_archive.id,
                        "Cycle detected in archive tree, skipping child"
                    );
                    continue;
                }
                let child_node = build_archive_node_indexed(
                    child_archive,
                    archive_children,
                    file_children,
                    visited.clone(),
                    depth + 1,
                )
                .await?;
                children.push(child_node);
            }
        }

        if let Some(child_files) = file_children.get(&archive.id) {
            for file in child_files {
                children.push(VirtualTreeNode::File {
                    name: file.original_name.clone(),
                    path: file.virtual_path.clone(),
                    hash: file.sha256_hash.clone(),
                    size: file.size,
                    mime_type: file.mime_type.clone(),
                });
            }
        }

        Ok(VirtualTreeNode::Archive {
            name: archive.original_name.clone(),
            path: archive.virtual_path.clone(),
            hash: archive.sha256_hash.clone(),
            archive_type: archive.archive_type.clone(),
            children,
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_virtual_tree_node_serialization() {
        let file_node = VirtualTreeNode::File {
            name: "test.log".to_string(),
            path: "archive.zip/test.log".to_string(),
            hash: "abc123".to_string(),
            size: 1024,
            mime_type: Some("text/plain".to_string()),
        };

        let json = serde_json::to_string(&file_node)
            .expect("VirtualTreeNode::File should always be serializable");
        assert!(json.contains("\"type\":\"file\""));
        assert!(json.contains("\"name\":\"test.log\""));
    }

    #[test]
    fn test_archive_node_serialization() {
        let archive_node = VirtualTreeNode::Archive {
            name: "archive.zip".to_string(),
            path: "archive.zip".to_string(),
            hash: "def456".to_string(),
            archive_type: "zip".to_string(),
            children: vec![],
        };

        let json = serde_json::to_string(&archive_node)
            .expect("VirtualTreeNode::Archive should always be serializable");
        assert!(json.contains("\"type\":\"archive\""));
        assert!(json.contains("\"archiveType\":\"zip\""));
    }
    fn archive_meta(id: i64, parent: Option<i64>) -> la_storage::ArchiveMetadata {
        la_storage::ArchiveMetadata {
            id,
            sha256_hash: format!("hash-{id}"),
            virtual_path: format!("a{id}.zip"),
            original_name: format!("a{id}.zip"),
            archive_type: "zip".to_string(),
            parent_archive_id: parent,
            depth_level: 0,
            extraction_status: "done".to_string(),
        }
    }

    fn file_meta(id: i64, parent: i64) -> la_storage::FileMetadata {
        la_storage::FileMetadata {
            id,
            sha256_hash: format!("fhash-{id}"),
            virtual_path: format!("a{parent}.zip/f{id}.log"),
            original_name: format!("f{id}.log"),
            size: 10,
            modified_time: 0,
            mime_type: None,
            parent_archive_id: Some(parent),
            depth_level: 1,
            min_timestamp: None,
            max_timestamp: None,
            level_mask: None,
            analysis_status: la_core::storage_types::AnalysisStatus::Ready,
        }
    }

    // P2#9 regression: cycles must not recurse forever

    #[tokio::test]
    async fn self_referencing_archive_does_not_recurse_forever() {
        let archives = vec![archive_meta(1, Some(1))];
        let files: Vec<la_storage::FileMetadata> = vec![];

        let tree = build_tree_structure(&archives, &files, &make_store().await)
            .await
            .expect("self-referencing metadata must not hang or panic");

        assert!(tree.is_empty());
    }

    #[tokio::test]
    async fn mutually_recursive_archives_are_skipped_instead_of_hanging() {
        let archives = vec![archive_meta(1, Some(2)), archive_meta(2, Some(1))];
        let files: Vec<la_storage::FileMetadata> = vec![];

        let tree = build_tree_structure(&archives, &files, &make_store().await)
            .await
            .expect("cycle must be detected and skipped");

        assert!(tree.is_empty(), "both archives form a cycle with no root");
    }

    #[tokio::test]
    async fn normal_nested_archives_still_build() {
        let archives = vec![archive_meta(1, None), archive_meta(2, Some(1))];
        let files = vec![file_meta(1, 2)];

        let tree = build_tree_structure(&archives, &files, &make_store().await)
            .await
            .expect("normal tree should build");

        assert_eq!(tree.len(), 1);
        if let VirtualTreeNode::Archive { children, .. } = &tree[0] {
            assert_eq!(children.len(), 1);
        } else {
            panic!("root must be an archive");
        }
    }

    async fn make_store() -> MetadataStore {
        let dir = tempfile::tempdir().unwrap();
        MetadataStore::new(dir.path()).await.unwrap()
    }
}
