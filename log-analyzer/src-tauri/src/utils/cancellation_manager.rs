//! 取消管理器 - 使用 tokio-util CancellationToken 实现优雅取消
//!
//! 本模块提供统一的取消管理功能，支持：
//! - 搜索操作的取消
//! - 后台任务的优雅关闭
//! - 取消令牌的生命周期管理
//! - 取消状态的追踪和监控
//!
//! # 设计原则
//!
//! - 使用 tokio-util 的 CancellationToken（业界标准）
//! - 支持层级取消（父令牌取消会级联取消子令牌）
//! - 提供取消感知的资源清理
//! - 集成 tracing 进行取消事件追踪

use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

/// 取消管理器
///
/// 管理应用中所有可取消操作的取消令牌。
///
/// # 功能
///
/// - 创建和注册取消令牌
/// - 取消特定操作
/// - 批量取消操作
/// - 自动清理已完成的令牌
pub struct CancellationManager {
    /// 操作ID到取消令牌的映射
    tokens: Arc<Mutex<HashMap<String, CancellationToken>>>,
    /// 全局取消令牌（用于应用关闭）
    global_token: CancellationToken,
}

impl CancellationManager {
    /// 创建新的取消管理器
    pub fn new() -> Self {
        info!("CancellationManager initialized");
        Self {
            tokens: Arc::new(Mutex::new(HashMap::new())),
            global_token: CancellationToken::new(),
        }
    }

    /// 创建新的取消令牌并注册
    ///
    /// # 参数
    ///
    /// - `operation_id` - 操作的唯一标识符
    ///
    /// # 返回值
    ///
    /// 返回新创建的取消令牌
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let token = cancellation_manager.create_token("search-123");
    /// ```
    pub fn create_token(&self, operation_id: String) -> CancellationToken {
        let token = self.global_token.child_token();

        {
            let mut tokens = self.tokens.lock();
            tokens.insert(operation_id.clone(), token.clone());
        }

        info!("Created cancellation token for operation: {}", operation_id);
        token
    }

    /// 获取已存在的取消令牌
    ///
    /// # 参数
    ///
    /// - `operation_id` - 操作的唯一标识符
    ///
    /// # 返回值
    ///
    /// 如果令牌存在则返回 Some(token)，否则返回 None
    pub fn get_token(&self, operation_id: &str) -> Option<CancellationToken> {
        let tokens = self.tokens.lock();
        tokens.get(operation_id).cloned()
    }

    /// 取消特定操作
    ///
    /// # 参数
    ///
    /// - `operation_id` - 要取消的操作ID
    ///
    /// # 返回值
    ///
    /// - `Ok(())` - 取消成功
    /// - `Err(String)` - 操作不存在或已完成
    pub fn cancel_operation(&self, operation_id: &str) -> Result<(), String> {
        let token = {
            let tokens = self.tokens.lock();
            tokens.get(operation_id).cloned()
        };

        if let Some(token) = token {
            token.cancel();
            info!("Cancelled operation: {}", operation_id);
            Ok(())
        } else {
            warn!("Operation not found or already completed: {}", operation_id);
            Err(format!(
                "Operation {} not found or already completed",
                operation_id
            ))
        }
    }

    /// 移除已完成的操作令牌
    ///
    /// # 参数
    ///
    /// - `operation_id` - 操作ID
    ///
    /// # 说明
    ///
    /// 操作完成后应调用此方法清理令牌，避免内存泄漏
    pub fn remove_token(&self, operation_id: &str) {
        let mut tokens = self.tokens.lock();
        if tokens.remove(operation_id).is_some() {
            info!("Removed cancellation token for operation: {}", operation_id);
        }
    }

    /// 取消所有活跃操作
    ///
    /// 用于应用关闭时的优雅关闭
    pub fn cancel_all(&self) {
        info!("Cancelling all active operations");
        self.global_token.cancel();

        let mut tokens = self.tokens.lock();
        let count = tokens.len();
        tokens.clear();

        info!("Cancelled {} active operations", count);
    }

    /// 获取活跃操作数量
    pub fn active_count(&self) -> usize {
        let tokens = self.tokens.lock();
        tokens.len()
    }

    /// 检查操作是否已被取消
    ///
    /// # 参数
    ///
    /// - `operation_id` - 操作ID
    ///
    /// # 返回值
    ///
    /// 如果操作已被取消返回 true，否则返回 false
    pub fn is_cancelled(&self, operation_id: &str) -> bool {
        let tokens = self.tokens.lock();
        tokens
            .get(operation_id)
            .map(|token| token.is_cancelled())
            .unwrap_or(false)
    }

    /// 获取全局取消令牌
    ///
    /// 用于创建子令牌或检查全局取消状态
    pub fn global_token(&self) -> &CancellationToken {
        &self.global_token
    }
}

impl Default for CancellationManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 取消感知的操作守卫
///
/// 自动管理操作的取消令牌生命周期
pub struct CancellableOperation {
    operation_id: String,
    token: CancellationToken,
    manager: Arc<CancellationManager>,
}

impl CancellableOperation {
    /// 创建新的可取消操作
    ///
    /// # 参数
    ///
    /// - `operation_id` - 操作ID
    /// - `manager` - 取消管理器引用
    pub fn new(operation_id: String, manager: Arc<CancellationManager>) -> Self {
        let token = manager.create_token(operation_id.clone());

        Self {
            operation_id,
            token,
            manager,
        }
    }

    /// 获取取消令牌
    pub fn token(&self) -> &CancellationToken {
        &self.token
    }

    /// 检查是否已被取消
    pub fn is_cancelled(&self) -> bool {
        self.token.is_cancelled()
    }

    /// 获取操作ID
    pub fn operation_id(&self) -> &str {
        &self.operation_id
    }
}

impl Drop for CancellableOperation {
    fn drop(&mut self) {
        // 自动清理令牌
        self.manager.remove_token(&self.operation_id);
        info!("CancellableOperation dropped: {}", self.operation_id);
    }
}

/// 创建带取消支持的异步任务
///
/// # 示例
///
/// ```ignore
/// use tokio_util::sync::CancellationToken;
///
/// async fn cancellable_task(token: CancellationToken) {
///     loop {
///         tokio::select! {
///             _ = token.cancelled() => {
///                 println!("Task cancelled");
///                 break;
///             }
///             _ = tokio::time::sleep(Duration::from_secs(1)) => {
///                 println!("Working...");
///             }
///         }
///     }
/// }
/// ```
pub async fn run_with_cancellation<F, Fut>(token: CancellationToken, task: F) -> Result<(), String>
where
    F: FnOnce(CancellationToken) -> Fut,
    Fut: std::future::Future<Output = Result<(), String>>,
{
    tokio::select! {
        result = task(token.clone()) => {
            result
        }
        _ = token.cancelled() => {
            warn!("Task cancelled before completion");
            Err("Task was cancelled".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::sleep;

    #[test]
    fn test_create_and_cancel_token() {
        let manager = CancellationManager::new();
        let token = manager.create_token("test-op".to_string());

        assert!(!token.is_cancelled());
        assert_eq!(manager.active_count(), 1);

        manager.cancel_operation("test-op").unwrap();
        assert!(token.is_cancelled());
    }

    #[test]
    fn test_remove_token() {
        let manager = CancellationManager::new();
        manager.create_token("test-op".to_string());

        assert_eq!(manager.active_count(), 1);

        manager.remove_token("test-op");
        assert_eq!(manager.active_count(), 0);
    }

    #[test]
    fn test_cancel_all() {
        let manager = CancellationManager::new();
        let token1 = manager.create_token("op1".to_string());
        let token2 = manager.create_token("op2".to_string());

        assert!(!token1.is_cancelled());
        assert!(!token2.is_cancelled());

        manager.cancel_all();

        assert!(token1.is_cancelled());
        assert!(token2.is_cancelled());
        assert_eq!(manager.active_count(), 0);
    }

    #[test]
    fn test_cancellable_operation_auto_cleanup() {
        let manager = Arc::new(CancellationManager::new());

        {
            let _op = CancellableOperation::new("test-op".to_string(), manager.clone());
            assert_eq!(manager.active_count(), 1);
        } // op dropped here

        // 令牌应该被自动清理
        assert_eq!(manager.active_count(), 0);
    }

    #[tokio::test]
    async fn test_run_with_cancellation_success() {
        let token = CancellationToken::new();

        let result = run_with_cancellation(token, |_token| async {
            sleep(Duration::from_millis(10)).await;
            Ok(())
        })
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_run_with_cancellation_cancelled() {
        let token = CancellationToken::new();
        let token_clone = token.clone();

        // 在后台取消令牌
        tokio::spawn(async move {
            sleep(Duration::from_millis(50)).await;
            token_clone.cancel();
        });

        let result = run_with_cancellation(token, |token| async move {
            loop {
                tokio::select! {
                    _ = token.cancelled() => {
                        return Err("Cancelled".to_string());
                    }
                    _ = sleep(Duration::from_millis(10)) => {
                        // 继续工作
                    }
                }
            }
        })
        .await;

        assert!(result.is_err());
    }

    #[test]
    fn test_child_token_cancellation() {
        let manager = CancellationManager::new();
        let token = manager.create_token("test-op".to_string());

        // 全局取消应该级联到子令牌
        manager.cancel_all();
        assert!(token.is_cancelled());
    }
}
