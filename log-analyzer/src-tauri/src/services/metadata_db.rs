//! Metadata Database - Stub Implementation
//!
//! This is a minimal stub implementation to maintain compatibility with
//! the archive extraction system's path shortening feature.
//!
//! **Note**: This is a temporary solution during the CAS migration.
//! The path shortening functionality may be refactored in the future.

use dashmap::DashMap;
use std::sync::Arc;

/// Metadata database for path shortening mappings
///
/// This is a stub implementation that uses in-memory storage only.
/// It provides the interface needed by PathManager for path shortening.
pub struct MetadataDB {
    /// In-memory storage for path mappings (workspace_id:short_path -> original_path)
    mappings: Arc<DashMap<String, String>>,
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
    pub async fn new(_db_path: &str) -> eyre::Result<Self> {
        Ok(Self {
            mappings: Arc::new(DashMap::new()),
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
    ) -> eyre::Result<Option<String>> {
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
    ) -> eyre::Result<Option<String>> {
        // Search through mappings to find the original path
        let prefix = format!("{}:", workspace_id);
        for entry in self.mappings.iter() {
            if entry.key().starts_with(&prefix) && entry.value() == short_path {
                // Extract original path from key (remove workspace_id prefix)
                let original = entry.key().strip_prefix(&prefix).unwrap_or("");
                return Ok(Some(original.to_string()));
            }
        }
        Ok(None)
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
    ) -> eyre::Result<()> {
        let key = format!("{}:{}", workspace_id, original_path);
        self.mappings.insert(key, short_path.to_string());
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
    pub async fn cleanup_workspace(&self, workspace_id: &str) -> eyre::Result<usize> {
        let prefix = format!("{}:", workspace_id);
        let mut count = 0;

        // Collect keys to remove
        let keys_to_remove: Vec<String> = self
            .mappings
            .iter()
            .filter(|entry| entry.key().starts_with(&prefix))
            .map(|entry| entry.key().clone())
            .collect();

        // Remove them
        for key in keys_to_remove {
            self.mappings.remove(&key);
            count += 1;
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
    ) -> eyre::Result<Vec<(String, String)>> {
        let prefix = format!("{}:", workspace_id);
        let mut mappings = Vec::new();

        for entry in self.mappings.iter() {
            if entry.key().starts_with(&prefix) {
                let original = entry.key().strip_prefix(&prefix).unwrap_or("");
                mappings.push((entry.value().clone(), original.to_string()));
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
