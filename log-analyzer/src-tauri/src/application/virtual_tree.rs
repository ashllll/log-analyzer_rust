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
        let node = build_archive_node_indexed(archive, &archive_children, &file_children).await?;
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

/// Build archive node with pre-built HashMap indexes, O(n) total instead of O(n²)
fn build_archive_node_indexed<'a>(
    archive: &'a la_storage::ArchiveMetadata,
    archive_children: &'a std::collections::HashMap<i64, Vec<&'a la_storage::ArchiveMetadata>>,
    file_children: &'a std::collections::HashMap<i64, Vec<&'a la_storage::FileMetadata>>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<VirtualTreeNode, String>> + Send + 'a>>
{
    Box::pin(async move {
        let mut children = Vec::new();

        // O(1) HashMap lookup instead of O(n) linear scan
        if let Some(child_archives) = archive_children.get(&archive.id) {
            for child_archive in child_archives {
                let child_node =
                    build_archive_node_indexed(child_archive, archive_children, file_children)
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
}
