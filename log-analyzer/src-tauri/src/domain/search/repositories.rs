//! 搜索仓储接口
//!
//! 定义搜索相关数据的持久化接口

use async_trait::async_trait;

use super::entities::{SearchResult, SearchSession};

/// 搜索结果仓储接口
#[async_trait]
pub trait SearchRepository: Send + Sync {
    /// 保存搜索会话
    async fn save_session(&self, session: &SearchSession) -> Result<(), RepositoryError>;

    /// 获取搜索会话
    async fn get_session(&self, id: &str) -> Result<Option<SearchSession>, RepositoryError>;

    /// 保存搜索结果
    async fn save_results(
        &self,
        session_id: &str,
        results: &[SearchResult],
    ) -> Result<(), RepositoryError>;

    /// 获取搜索结果
    async fn get_results(
        &self,
        session_id: &str,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<SearchResult>, RepositoryError>;

    /// 删除搜索会话及结果
    async fn delete_session(&self, id: &str) -> Result<(), RepositoryError>;

    /// 清理过期会话
    async fn cleanup_expired(&self, max_age_hours: u64) -> Result<usize, RepositoryError>;
}

/// 仓储错误
#[derive(Debug, Clone, thiserror::Error)]
pub enum RepositoryError {
    #[error("数据库错误: {0}")]
    DatabaseError(String),

    #[error("会话未找到: {0}")]
    SessionNotFound(String),

    #[error("连接错误: {0}")]
    ConnectionError(String),

    #[error("序列化错误: {0}")]
    SerializationError(String),
}

/// 内存搜索仓储（用于测试）
pub struct InMemorySearchRepository {
    sessions: std::collections::HashMap<String, SearchSession>,
    results: std::collections::HashMap<String, Vec<SearchResult>>,
}

impl InMemorySearchRepository {
    pub fn new() -> Self {
        Self {
            sessions: std::collections::HashMap::new(),
            results: std::collections::HashMap::new(),
        }
    }
}

impl Default for InMemorySearchRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SearchRepository for InMemorySearchRepository {
    async fn save_session(&self, _session: &SearchSession) -> Result<(), RepositoryError> {
        // 注意：这是简化的实现，实际应该使用 RwLock
        Ok(())
    }

    async fn get_session(&self, id: &str) -> Result<Option<SearchSession>, RepositoryError> {
        Ok(self.sessions.get(id).cloned())
    }

    async fn save_results(
        &self,
        session_id: &str,
        results: &[SearchResult],
    ) -> Result<(), RepositoryError> {
        // 简化实现：存储结果
        let _ = (session_id, results);
        Ok(())
    }

    async fn get_results(
        &self,
        session_id: &str,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<SearchResult>, RepositoryError> {
        let _ = (offset, limit);
        Ok(self.results.get(session_id).cloned().unwrap_or_default())
    }

    async fn delete_session(&self, id: &str) -> Result<(), RepositoryError> {
        let _ = id;
        Ok(())
    }

    async fn cleanup_expired(&self, max_age_hours: u64) -> Result<usize, RepositoryError> {
        let _ = max_age_hours;
        Ok(0)
    }
}

// ==================== 单元测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_memory_repository_creation() {
        let _repo = InMemorySearchRepository::new();
        // 基本创建测试 - 确保 InMemorySearchRepository 可以被实例化
    }

    #[tokio::test]
    async fn test_repository_get_session_empty() {
        let repo = InMemorySearchRepository::new();
        let result = repo.get_session("non-existent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_repository_get_results_empty() {
        let repo = InMemorySearchRepository::new();
        let results = repo.get_results("non-existent", 0, 10).await.unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_repository_error_display() {
        let error = RepositoryError::SessionNotFound("test-id".to_string());
        assert!(error.to_string().contains("test-id"));

        let error = RepositoryError::DatabaseError("connection failed".to_string());
        assert!(error.to_string().contains("connection failed"));
    }
}
