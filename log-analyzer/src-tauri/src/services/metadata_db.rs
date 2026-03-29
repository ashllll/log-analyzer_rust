//! Metadata Database - Stub Implementation
//!
//! This is a minimal stub implementation to maintain compatibility with
//! the archive extraction system's path shortening feature.
//!
//! **Note**: This is a temporary solution during the CAS migration.
//! The path shortening functionality may be refactored in the future.

use crate::error::Result;
use dashmap::DashMap;
use std::sync::Arc;

/// Metadata database for path shortening mappings
///
/// This is a stub implementation that uses in-memory storage only.
/// It provides the interface needed by PathManager for path shortening.
pub struct MetadataDB {
    /// 正向映射：workspace_id:original_path -> short_path
    mappings: Arc<DashMap<String, String>>,
    /// 反向映射：workspace_id:short_path -> original_path（O(1) 反查）
    reverse_mappings: Arc<DashMap<String, String>>,
    /// 工作区键索引：workspace_id -> 所有 forward_key 列表（用于 O(1) cleanup）
    workspace_keys: Arc<DashMap<String, Vec<String>>>,
}

impl MetadataDB {
    /// Create a new MetadataDB instance
    ///
    /// # Arguments
    ///
    /// * `_db_path` - Database path (ignored in stub implementation)
    ///
    /// # Returns
    ///
    /// A new MetadataDB instance with in-memory storage
    pub async fn new(_db_path: &str) -> Result<Self> {
        Ok(Self {
            mappings: Arc::new(DashMap::new()),
            reverse_mappings: Arc::new(DashMap::new()),
            workspace_keys: Arc::new(DashMap::new()),
        })
    }

    /// Get shortened path for an original path
    ///
    /// # Arguments
    ///
    /// * `workspace_id` - Workspace identifier
    /// * `original_path` - Original (long) path
    ///
    /// # Returns
    ///
    /// Shortened path if exists, None otherwise
    pub async fn get_short_path(
        &self,
        workspace_id: &str,
        original_path: &str,
    ) -> Result<Option<String>> {
        let key = format!("{}:{}", workspace_id, original_path);
        Ok(self.mappings.get(&key).map(|v| v.value().clone()))
    }

    /// Get original path from shortened path
    ///
    /// # Arguments
    ///
    /// * `workspace_id` - Workspace identifier
    /// * `short_path` - Shortened path
    ///
    /// # Returns
    ///
    /// Original path if exists, None otherwise
    pub async fn get_original_path(
        &self,
        workspace_id: &str,
        short_path: &str,
    ) -> Result<Option<String>> {
        // 使用反向映射 O(1) 直接查找
        let key = format!("{}:{}", workspace_id, short_path);
        Ok(self.reverse_mappings.get(&key).map(|v| v.value().clone()))
    }

    /// Store a path mapping
    ///
    /// # Arguments
    ///
    /// * `workspace_id` - Workspace identifier
    /// * `short_path` - Shortened path
    /// * `original_path` - Original (long) path
    pub async fn store_mapping(
        &self,
        workspace_id: &str,
        short_path: &str,
        original_path: &str,
    ) -> Result<()> {
        let forward_key = format!("{}:{}", workspace_id, original_path);
        self.mappings
            .insert(forward_key.clone(), short_path.to_string());

        // 维护工作区键索引
        self.workspace_keys
            .entry(workspace_id.to_string())
            .or_default()
            .push(forward_key);

        // 同时维护反向映射
        let reverse_key = format!("{}:{}", workspace_id, short_path);
        self.reverse_mappings
            .insert(reverse_key, original_path.to_string());

        Ok(())
    }

    /// Clean up all mappings for a workspace
    ///
    /// # Arguments
    ///
    /// * `workspace_id` - Workspace identifier
    ///
    /// # Returns
    ///
    /// Number of mappings removed
    pub async fn cleanup_workspace(&self, workspace_id: &str) -> Result<usize> {
        // 使用 workspace_keys 索引进行 O(1) 查找，O(k) 删除（k 为该工作区文件数）
        let mut count = 0;

        if let Some((_, forward_keys)) = self.workspace_keys.remove(workspace_id) {
            // 删除正向映射
            for key in &forward_keys {
                self.mappings.remove(key);
                count += 1;
            }

            // 同时清理反向映射
            let prefix = format!("{}:", workspace_id);
            let reverse_keys_to_remove: Vec<String> = self
                .reverse_mappings
                .iter()
                .filter(|entry| entry.key().starts_with(&prefix))
                .map(|entry| entry.key().clone())
                .collect();

            for key in reverse_keys_to_remove {
                self.reverse_mappings.remove(&key);
            }
        }

        Ok(count)
    }

    /// Get all mappings for a workspace
    ///
    /// # Arguments
    ///
    /// * `workspace_id` - Workspace identifier
    ///
    /// # Returns
    ///
    /// Vector of (short_path, original_path) tuples
    pub async fn get_workspace_mappings(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<(String, String)>> {
        let mut mappings = Vec::new();

        // 使用 workspace_keys 索引进行 O(k) 查找，而非 O(n) 遍历所有条目
        if let Some(forward_keys) = self.workspace_keys.get(workspace_id) {
            let prefix = format!("{}:", workspace_id);
            for key in forward_keys.iter() {
                if let Some(short_path) = self.mappings.get(key) {
                    // key 格式为 "workspace_id:original_path"，需要提取 original_path
                    if let Some(original) = key.strip_prefix(&prefix) {
                        mappings.push((short_path.value().clone(), original.to_string()));
                    }
                }
            }
        }

        Ok(mappings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_metadata_db_basic_operations() {
        let db = MetadataDB::new(":memory:").await.unwrap();

        // Store a mapping
        db.store_mapping("ws1", "/short/path", "/very/long/original/path")
            .await
            .unwrap();

        // Retrieve by original path
        let short = db
            .get_short_path("ws1", "/very/long/original/path")
            .await
            .unwrap();
        assert_eq!(short, Some("/short/path".to_string()));

        // Retrieve by short path
        let original = db.get_original_path("ws1", "/short/path").await.unwrap();
        assert_eq!(original, Some("/very/long/original/path".to_string()));
    }

    #[tokio::test]
    async fn test_workspace_cleanup() {
        let db = MetadataDB::new(":memory:").await.unwrap();

        // Store mappings for multiple workspaces
        db.store_mapping("ws1", "/short1", "/original1")
            .await
            .unwrap();
        db.store_mapping("ws1", "/short2", "/original2")
            .await
            .unwrap();
        db.store_mapping("ws2", "/short3", "/original3")
            .await
            .unwrap();

        // Clean up ws1
        let count = db.cleanup_workspace("ws1").await.unwrap();
        assert_eq!(count, 2);

        // Verify ws1 mappings are gone
        let ws1_mappings = db.get_workspace_mappings("ws1").await.unwrap();
        assert_eq!(ws1_mappings.len(), 0);

        // Verify ws2 mappings still exist
        let ws2_mappings = db.get_workspace_mappings("ws2").await.unwrap();
        assert_eq!(ws2_mappings.len(), 1);
    }
}
