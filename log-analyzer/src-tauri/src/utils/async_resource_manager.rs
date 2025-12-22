//! 异步资源管理器
//!
//! 提供异步上下文中的资源管理功能，包括：
//! - 异步锁管理
//! - 取消令牌支持
//! - 资源生命周期管理
//! - 搜索操作取消
//! - 后台任务管理

use crate::utils::ResourceManager;
use eyre::Result;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex as AsyncMutex, RwLock as AsyncRwLock};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, instrument, warn};

/// 操作信息
#[derive(Debug, Clone)]
pub struct OperationInfo {
    pub id: String,
    pub operation_type: OperationType,
    pub started_at: Instant,
    pub workspace_id: Option<String>,
    pub cancelled: bool,
}

/// 操作类型
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OperationType {
    Search,
    FileWatch,
    ArchiveExtraction,
    IndexBuilding,
    BackgroundTask,
}

/// 异步资源管理器
///
/// 管理异步操作中的资源，支持取消和超时
pub struct AsyncResourceManager {
    /// 活跃的异步操作
    active_operations: Arc<AsyncMutex<HashMap<String, (CancellationToken, OperationInfo)>>>,
    /// 资源注册表
    resources: Arc<AsyncRwLock<HashMap<String, String>>>,
    /// 全局取消令牌
    global_cancellation: CancellationToken,
    /// 同步资源管理器集成
    sync_resource_manager: Option<Arc<ResourceManager>>,
}

impl AsyncResourceManager {
    /// 创建新的异步资源管理器
    pub fn new() -> Self {
        Self {
            active_operations: Arc::new(AsyncMutex::new(HashMap::new())),
            resources: Arc::new(AsyncRwLock::new(HashMap::new())),
            global_cancellation: CancellationToken::new(),
            sync_resource_manager: None,
        }
    }

    /// 创建带有同步资源管理器集成的异步资源管理器
    pub fn with_sync_manager(sync_manager: Arc<ResourceManager>) -> Self {
        Self {
            active_operations: Arc::new(AsyncMutex::new(HashMap::new())),
            resources: Arc::new(AsyncRwLock::new(HashMap::new())),
            global_cancellation: CancellationToken::new(),
            sync_resource_manager: Some(sync_manager),
        }
    }

    /// 注册一个新的异步操作
    #[instrument(skip(self))]
    pub async fn register_operation(
        &self,
        operation_id: String,
        operation_type: OperationType,
        workspace_id: Option<String>,
    ) -> CancellationToken {
        let token = self.global_cancellation.child_token();
        let operation_info = OperationInfo {
            id: operation_id.clone(),
            operation_type: operation_type.clone(),
            started_at: Instant::now(),
            workspace_id: workspace_id.clone(),
            cancelled: false,
        };

        let mut operations = self.active_operations.lock().await;
        operations.insert(operation_id.clone(), (token.clone(), operation_info));

        info!(
            operation_id = %operation_id,
            operation_type = ?operation_type,
            workspace_id = ?workspace_id,
            "Registered async operation"
        );

        token
    }

    /// 注册搜索操作
    #[instrument(skip(self))]
    pub async fn register_search_operation(
        &self,
        workspace_id: String,
        query: String,
    ) -> (String, CancellationToken) {
        let operation_id = format!("search_{}_{}", workspace_id, uuid::Uuid::new_v4());
        let token = self
            .register_operation(
                operation_id.clone(),
                OperationType::Search,
                Some(workspace_id),
            )
            .await;

        debug!(
            operation_id = %operation_id,
            query = %query,
            "Registered search operation"
        );

        (operation_id, token)
    }

    /// 取消指定的操作
    #[instrument(skip(self))]
    pub async fn cancel_operation(&self, operation_id: &str) -> Result<()> {
        let mut operations = self.active_operations.lock().await;
        if let Some((token, mut operation_info)) = operations.remove(operation_id) {
            token.cancel();
            operation_info.cancelled = true;

            let duration = operation_info.started_at.elapsed();
            info!(
                operation_id = %operation_id,
                operation_type = ?operation_info.operation_type,
                duration_ms = duration.as_millis(),
                "Operation cancelled"
            );
        } else {
            warn!(operation_id = %operation_id, "Operation not found for cancellation");
        }
        Ok(())
    }

    /// 取消工作区的所有操作
    #[instrument(skip(self))]
    pub async fn cancel_workspace_operations(&self, workspace_id: &str) -> Result<usize> {
        let mut operations = self.active_operations.lock().await;
        let mut cancelled_count = 0;

        let to_cancel: Vec<String> = operations
            .iter()
            .filter(|(_, (_, info))| info.workspace_id.as_ref() == Some(&workspace_id.to_string()))
            .map(|(id, _)| id.clone())
            .collect();

        for operation_id in to_cancel {
            if let Some((token, mut operation_info)) = operations.remove(&operation_id) {
                token.cancel();
                operation_info.cancelled = true;
                cancelled_count += 1;

                debug!(
                    operation_id = %operation_id,
                    operation_type = ?operation_info.operation_type,
                    "Cancelled workspace operation"
                );
            }
        }

        info!(
            workspace_id = %workspace_id,
            cancelled_count = cancelled_count,
            "Cancelled workspace operations"
        );

        Ok(cancelled_count)
    }

    /// 取消所有操作
    #[instrument(skip(self))]
    pub async fn cancel_all_operations(&self) -> Result<()> {
        let operations_count = {
            let operations = self.active_operations.lock().await;
            operations.len()
        };

        self.global_cancellation.cancel();

        let mut operations = self.active_operations.lock().await;
        operations.clear();

        info!(
            cancelled_count = operations_count,
            "All operations cancelled"
        );

        Ok(())
    }

    /// 优雅关闭 - 等待操作完成或强制取消
    #[instrument(skip(self))]
    pub async fn graceful_shutdown(&self, timeout: Duration) -> Result<()> {
        info!("Starting graceful shutdown");

        let start_time = Instant::now();
        let operations_count = self.active_operations_count().await;

        if operations_count == 0 {
            info!("No active operations, shutdown complete");
            return Ok(());
        }

        info!(
            operations_count = operations_count,
            timeout_ms = timeout.as_millis(),
            "Waiting for operations to complete"
        );

        // 等待操作自然完成
        let wait_result = tokio::time::timeout(timeout, async {
            while self.active_operations_count().await > 0 {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
        .await;

        match wait_result {
            Ok(_) => {
                let elapsed = start_time.elapsed();
                info!(
                    elapsed_ms = elapsed.as_millis(),
                    "All operations completed gracefully"
                );
            }
            Err(_) => {
                warn!("Timeout reached, forcing cancellation of remaining operations");
                self.cancel_all_operations().await?;
            }
        }

        // 如果有同步资源管理器，也进行清理
        if let Some(sync_manager) = &self.sync_resource_manager {
            sync_manager.cleanup_all()?;
        }

        info!("Graceful shutdown completed");
        Ok(())
    }

    /// 注册资源
    pub async fn register_resource(
        &self,
        resource_id: String,
        resource_path: String,
    ) -> Result<()> {
        let mut resources = self.resources.write().await;
        resources.insert(resource_id, resource_path);
        Ok(())
    }

    /// 获取资源路径
    pub async fn get_resource(&self, resource_id: &str) -> Option<String> {
        let resources = self.resources.read().await;
        resources.get(resource_id).cloned()
    }

    /// 清理资源
    pub async fn cleanup_resource(&self, resource_id: &str) -> Result<()> {
        let mut resources = self.resources.write().await;
        if let Some(path) = resources.remove(resource_id) {
            tracing::info!(resource_id = %resource_id, path = %path, "Resource cleaned up");
        }
        Ok(())
    }

    /// 等待操作完成或取消
    #[instrument(skip(self))]
    pub async fn wait_for_completion_or_cancellation(
        &self,
        operation_id: &str,
        timeout: Duration,
    ) -> Result<bool> {
        let token_and_info = {
            let operations = self.active_operations.lock().await;
            operations.get(operation_id).cloned()
        };

        if let Some((token, operation_info)) = token_and_info {
            tokio::select! {
                _ = token.cancelled() => {
                    info!(
                        operation_id = %operation_id,
                        operation_type = ?operation_info.operation_type,
                        "Operation was cancelled"
                    );
                    Ok(false)
                }
                _ = tokio::time::sleep(timeout) => {
                    warn!(
                        operation_id = %operation_id,
                        operation_type = ?operation_info.operation_type,
                        timeout_ms = timeout.as_millis(),
                        "Operation timed out"
                    );
                    // 超时后自动取消操作
                    self.cancel_operation(operation_id).await?;
                    Ok(false)
                }
            }
        } else {
            debug!(operation_id = %operation_id, "Operation already completed");
            Ok(true) // Operation already completed
        }
    }

    /// 创建可取消的异步任务
    pub async fn spawn_cancellable_task<F, T>(
        &self,
        operation_type: OperationType,
        workspace_id: Option<String>,
        task: F,
    ) -> Result<(String, tokio::task::JoinHandle<Result<T>>)>
    where
        F: FnOnce(
                CancellationToken,
            )
                -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T>> + Send>>
            + Send
            + 'static,
        T: Send + 'static,
    {
        let operation_id = uuid::Uuid::new_v4().to_string();
        let token = self
            .register_operation(operation_id.clone(), operation_type, workspace_id)
            .await;

        let handle = tokio::spawn(async move {
            let result = task(token.clone()).await;
            result
        });

        Ok((operation_id, handle))
    }

    /// 获取活跃操作数量
    pub async fn active_operations_count(&self) -> usize {
        let operations = self.active_operations.lock().await;
        operations.len()
    }

    /// 获取资源数量
    pub async fn resources_count(&self) -> usize {
        let resources = self.resources.read().await;
        resources.len()
    }

    /// 获取操作信息
    pub async fn get_operation_info(&self, operation_id: &str) -> Option<OperationInfo> {
        let operations = self.active_operations.lock().await;
        operations.get(operation_id).map(|(_, info)| info.clone())
    }

    /// 列出所有活跃操作
    pub async fn list_active_operations(&self) -> Vec<OperationInfo> {
        let operations = self.active_operations.lock().await;
        operations.values().map(|(_, info)| info.clone()).collect()
    }

    /// 获取按类型分组的操作统计
    pub async fn get_operation_stats(&self) -> HashMap<OperationType, usize> {
        let operations = self.active_operations.lock().await;
        let mut stats = HashMap::new();

        for (_, (_, info)) in operations.iter() {
            *stats.entry(info.operation_type.clone()).or_insert(0) += 1;
        }

        stats
    }

    /// 检查操作是否被取消
    pub async fn is_operation_cancelled(&self, operation_id: &str) -> bool {
        let operations = self.active_operations.lock().await;
        if let Some((token, _)) = operations.get(operation_id) {
            token.is_cancelled()
        } else {
            true // 如果操作不存在，认为已被取消
        }
    }
}

impl Default for AsyncResourceManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_operation_registration_and_cancellation() {
        let manager = AsyncResourceManager::new();

        // 注册操作
        let token = manager
            .register_operation(
                "test_op".to_string(),
                OperationType::Search,
                Some("workspace1".to_string()),
            )
            .await;
        assert_eq!(manager.active_operations_count().await, 1);

        // 取消操作
        manager.cancel_operation("test_op").await.unwrap();
        assert_eq!(manager.active_operations_count().await, 0);
        assert!(token.is_cancelled());
    }

    #[tokio::test]
    async fn test_resource_management() {
        let manager = AsyncResourceManager::new();

        // 注册资源
        manager
            .register_resource("test_resource".to_string(), "/path/to/resource".to_string())
            .await
            .unwrap();
        assert_eq!(manager.resources_count().await, 1);

        // 获取资源
        let path = manager.get_resource("test_resource").await;
        assert_eq!(path, Some("/path/to/resource".to_string()));

        // 清理资源
        manager.cleanup_resource("test_resource").await.unwrap();
        assert_eq!(manager.resources_count().await, 0);
    }

    #[tokio::test]
    async fn test_global_cancellation() {
        let manager = AsyncResourceManager::new();

        // 注册多个操作
        let token1 = manager
            .register_operation(
                "op1".to_string(),
                OperationType::Search,
                Some("workspace1".to_string()),
            )
            .await;
        let token2 = manager
            .register_operation(
                "op2".to_string(),
                OperationType::FileWatch,
                Some("workspace2".to_string()),
            )
            .await;
        assert_eq!(manager.active_operations_count().await, 2);

        // 全局取消
        manager.cancel_all_operations().await.unwrap();
        assert_eq!(manager.active_operations_count().await, 0);
        assert!(token1.is_cancelled());
        assert!(token2.is_cancelled());
    }

    #[tokio::test]
    async fn test_workspace_operations_cancellation() {
        let manager = AsyncResourceManager::new();

        // 注册不同工作区的操作
        let _token1 = manager
            .register_operation(
                "op1".to_string(),
                OperationType::Search,
                Some("workspace1".to_string()),
            )
            .await;
        let _token2 = manager
            .register_operation(
                "op2".to_string(),
                OperationType::Search,
                Some("workspace1".to_string()),
            )
            .await;
        let _token3 = manager
            .register_operation(
                "op3".to_string(),
                OperationType::Search,
                Some("workspace2".to_string()),
            )
            .await;

        assert_eq!(manager.active_operations_count().await, 3);

        // 取消 workspace1 的操作
        let cancelled_count = manager
            .cancel_workspace_operations("workspace1")
            .await
            .unwrap();
        assert_eq!(cancelled_count, 2);
        assert_eq!(manager.active_operations_count().await, 1);
    }

    #[tokio::test]
    async fn test_search_operation_registration() {
        let manager = AsyncResourceManager::new();

        let (operation_id, token) = manager
            .register_search_operation("test_workspace".to_string(), "test query".to_string())
            .await;

        assert!(operation_id.starts_with("search_test_workspace_"));
        assert!(!token.is_cancelled());
        assert_eq!(manager.active_operations_count().await, 1);

        let operation_info = manager.get_operation_info(&operation_id).await.unwrap();
        assert_eq!(operation_info.operation_type, OperationType::Search);
        assert_eq!(
            operation_info.workspace_id,
            Some("test_workspace".to_string())
        );
    }
}
