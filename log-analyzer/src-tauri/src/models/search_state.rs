//! 搜索状态管理
//!
//! 使用 DashMap 替代 Arc<Mutex<HashMap<...>>> 实现无锁并发访问

use crate::utils::async_resource_manager::AsyncResourceManager;
use dashmap::DashMap;
use la_search::VirtualSearchManager;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

/// 搜索状态 - 管理搜索相关的所有资源
pub struct SearchState {
    /// 搜索取消令牌映射 (search_id -> CancellationToken)
    pub cancellation_tokens: DashMap<String, CancellationToken>,
    /// 虚拟搜索管理器 - 支持服务端虚拟化和分页加载
    pub virtual_search_manager: Arc<VirtualSearchManager>,
    /// 异步资源管理器，支持搜索取消和超时
    pub async_resource_manager: Arc<AsyncResourceManager>,
}

impl SearchState {
    /// 创建新的搜索状态
    pub fn new() -> Self {
        Self {
            cancellation_tokens: DashMap::new(),
            virtual_search_manager: Arc::new(VirtualSearchManager::new(100)),
            async_resource_manager: Arc::new(AsyncResourceManager::new()),
        }
    }

    /// 注册搜索取消令牌
    pub fn register_cancellation_token(&self, search_id: String, token: CancellationToken) {
        self.cancellation_tokens.insert(search_id, token);
    }

    /// 取消搜索并移除令牌
    pub fn cancel_search(&self, search_id: &str) -> bool {
        if let Some((_, token)) = self.cancellation_tokens.remove(search_id) {
            token.cancel();
            true
        } else {
            false
        }
    }

    /// 清理已完成的搜索令牌
    pub fn cleanup_finished_search(&self, search_id: &str) {
        self.cancellation_tokens.remove(search_id);
    }
}

impl Default for SearchState {
    fn default() -> Self {
        Self::new()
    }
}
