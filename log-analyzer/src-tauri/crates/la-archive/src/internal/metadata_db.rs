//! Metadata Database - 内存实现
//!
//! 从主 crate services/metadata_db.rs 复制的最小实现，
//! 用于 archive 模块内部的路径缩短功能。

use dashmap::DashMap;
use la_core::error::Result;
use std::sync::Arc;

/// 元数据数据库（路径缩短映射）
///
/// 内存存储实现，提供 PathManager 所需的路径缩短接口。
pub struct MetadataDB {
    /// 正向映射：workspace_id:original_path -> short_path
    mappings: Arc<DashMap<String, String>>,
    /// 反向映射：workspace_id:short_path -> original_path（O(1) 反查）
    reverse_mappings: Arc<DashMap<String, String>>,
    /// 工作区键索引：workspace_id -> 所有 forward_key 列表（用于 O(1) cleanup）
    workspace_keys: Arc<DashMap<String, Vec<String>>>,
}

impl MetadataDB {
    /// 创建新的 MetadataDB 实例
    pub async fn new(_db_path: &str) -> Result<Self> {
        Ok(Self {
            mappings: Arc::new(DashMap::new()),
            reverse_mappings: Arc::new(DashMap::new()),
            workspace_keys: Arc::new(DashMap::new()),
        })
    }

    /// 通过原始路径获取缩短路径
    pub async fn get_short_path(
        &self,
        workspace_id: &str,
        original_path: &str,
    ) -> Result<Option<String>> {
        let key = format!("{}:{}", workspace_id, original_path);
        Ok(self.mappings.get(&key).map(|v| v.value().clone()))
    }

    /// 通过缩短路径获取原始路径
    pub async fn get_original_path(
        &self,
        workspace_id: &str,
        short_path: &str,
    ) -> Result<Option<String>> {
        let key = format!("{}:{}", workspace_id, short_path);
        Ok(self.reverse_mappings.get(&key).map(|v| v.value().clone()))
    }

    /// 存储路径映射
    pub async fn store_mapping(
        &self,
        workspace_id: &str,
        short_path: &str,
        original_path: &str,
    ) -> Result<()> {
        let forward_key = format!("{}:{}", workspace_id, original_path);
        self.mappings
            .insert(forward_key.clone(), short_path.to_string());

        self.workspace_keys
            .entry(workspace_id.to_string())
            .or_default()
            .push(forward_key);

        let reverse_key = format!("{}:{}", workspace_id, short_path);
        self.reverse_mappings
            .insert(reverse_key, original_path.to_string());

        Ok(())
    }

    /// 清理工作区的所有映射
    pub async fn cleanup_workspace(&self, workspace_id: &str) -> Result<usize> {
        let mut count = 0;

        if let Some((_, forward_keys)) = self.workspace_keys.remove(workspace_id) {
            for key in &forward_keys {
                self.mappings.remove(key);
                count += 1;
            }

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

    /// 获取工作区的所有映射
    pub async fn get_workspace_mappings(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<(String, String)>> {
        let mut mappings = Vec::new();

        if let Some(forward_keys) = self.workspace_keys.get(workspace_id) {
            let prefix = format!("{}:", workspace_id);
            for key in forward_keys.iter() {
                if let Some(short_path) = self.mappings.get(key) {
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

        db.store_mapping("ws1", "/short/path", "/very/long/original/path")
            .await
            .unwrap();

        let short = db
            .get_short_path("ws1", "/very/long/original/path")
            .await
            .unwrap();
        assert_eq!(short, Some("/short/path".to_string()));

        let original = db.get_original_path("ws1", "/short/path").await.unwrap();
        assert_eq!(original, Some("/very/long/original/path".to_string()));
    }

    #[tokio::test]
    async fn test_workspace_cleanup() {
        let db = MetadataDB::new(":memory:").await.unwrap();

        db.store_mapping("ws1", "/short1", "/original1")
            .await
            .unwrap();
        db.store_mapping("ws1", "/short2", "/original2")
            .await
            .unwrap();
        db.store_mapping("ws2", "/short3", "/original3")
            .await
            .unwrap();

        let count = db.cleanup_workspace("ws1").await.unwrap();
        assert_eq!(count, 2);

        let ws1_mappings = db.get_workspace_mappings("ws1").await.unwrap();
        assert_eq!(ws1_mappings.len(), 0);

        let ws2_mappings = db.get_workspace_mappings("ws2").await.unwrap();
        assert_eq!(ws2_mappings.len(), 1);
    }
}
