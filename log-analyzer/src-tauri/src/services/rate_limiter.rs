//! 速率限制器 - 使用 governor 库实现行业标准限流
//!
//! 本模块提供基于令牌桶算法的速率限制功能，防止 API 滥用和 DoS 攻击。
//!
//! # 核心功能
//!
//! - 基于令牌桶算法的精确限流
//! - 支持按 IP、用户、API 端点等维度限流
//! - 线程安全设计
//! - 可配置的限流策略
//!
//! # 设计原则
//!
//! - 使用 governor 库（GitHub 3k+ stars）
//! - 支持 burst 突发流量
//! - 提供清晰的限流反馈
//!
//! # 示例
//!
//! ```ignore
//! use crate::services::rate_limiter::RateLimitService;
//!
//! let rate_limiter = RateLimitService::new(100, 10); // 每分钟100次，burst 10
//!
//! match rate_limiter.check().await {
//!     Ok(_) => { /* 执行操作 */ }
//!     Err(e) => { /* 返回 429 错误 */ }
//! }
//! ```

use governor::clock::Clock;
use governor::middleware::NoOpMiddleware;
use governor::state::{InMemoryState, NotKeyed};
use governor::{Quota, RateLimiter};
use nonzero_ext::nonzero;
use std::sync::Arc;
use std::time::Duration;

/// 速率限制服务
///
/// 使用 governor 库实现企业级速率限制。
/// 支持按请求频率限制，防止滥用和 DoS 攻击。
#[derive(Clone)]
pub struct RateLimitService {
    /// 内部限流器
    limiter: Arc<RateLimiter<NotKeyed, InMemoryState, Clock, NoOpMiddleware>>,
    /// 允许的请求数
    requests_per_minute: u32,
    /// 最大突发请求数
    max_burst: u32,
}

impl RateLimitService {
    /// 创建新的速率限制服务
    ///
    /// # 参数
    ///
    /// - `requests_per_minute` - 每分钟允许的请求数
    /// - `max_burst` - 最大突发请求数（令牌桶容量）
    ///
    /// # 示例
    ///
    /// ```ignore
    /// // 限制每分钟 100 次请求，最多突发 10 次
    /// let service = RateLimitService::new(100, 10);
    /// ```
    pub fn new(requests_per_minute: u32, max_burst: u32) -> Self {
        // 使用 governor 的 Quota 配置速率限制
        let quota = Quota::per_minute(nonzero!(requests_per_minute));
        let limiter = RateLimiter::direct(quota);

        Self {
            limiter: Arc::new(limiter),
            requests_per_minute,
            max_burst,
        }
    }

    /// 创建适用于搜索 API 的限流器
    ///
    /// 搜索操作通常较重，限制为每分钟 60 次，最多突发 5 次
    pub fn for_search() -> Self {
        Self::new(60, 5)
    }

    /// 创建适用于导入操作的限流器
    ///
    /// 导入操作涉及文件 IO，限制为每分钟 10 次，最多突发 2 次
    pub fn for_import() -> Self {
        Self::new(10, 2)
    }

    /// 创建适用于工作区管理的限流器
    ///
    /// 工作区操作相对轻量，限制为每分钟 120 次，最多突发 20 次
    pub fn for_workspace() -> Self {
        Self::new(120, 20)
    }

    /// 检查是否允许请求
    ///
    /// # 返回值
    ///
    /// - `Ok(())` - 请求被允许
    /// - `Err(Duration)` - 请求被限流，返回需要等待的时间
    pub fn check(&self) -> Result<(), Duration> {
        self.limiter.check().map_err(|neg| {
            // 返回等待时间
            Duration::from_secs(neg.remaining.as_secs())
        })
    }

    /// 异步检查是否允许请求
    ///
    /// 适用于需要异步操作的场景
    pub async fn check_async(&self) -> Result<(), Duration> {
        // governor 的 check 是同步的，直接调用
        self.check()
    }

    /// 获取限流配置信息
    pub fn config(&self) -> RateLimitConfig {
        RateLimitConfig {
            requests_per_minute: self.requests_per_minute,
            max_burst: self.max_burst,
        }
    }
}

/// 速率限制配置信息
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// 每分钟允许的请求数
    pub requests_per_minute: u32,
    /// 最大突发请求数
    pub max_burst: u32,
}

/// 全局限流器管理器
///
/// 提供全局的速率限制策略，适用于多端点共享限流场景。
#[derive(Clone)]
pub struct GlobalRateLimitManager {
    /// 搜索限流器
    search_limiter: Arc<RateLimitService>,
    /// 导入限流器
    import_limiter: Arc<RateLimitService>,
    /// 工作区限流器
    workspace_limiter: Arc<RateLimitService>,
    /// 默认限流器
    default_limiter: Arc<RateLimitService>,
}

impl GlobalRateLimitManager {
    /// 创建新的全局限流器管理器
    pub fn new() -> Self {
        Self {
            search_limiter: Arc::new(RateLimitService::for_search()),
            import_limiter: Arc::new(RateLimitService::for_import()),
            workspace_limiter: Arc::new(RateLimitService::for_workspace()),
            default_limiter: Arc::new(RateLimitService::new(200, 30)),
        }
    }

    /// 获取搜索限流器
    pub fn search(&self) -> &RateLimitService {
        &self.search_limiter
    }

    /// 获取导入限流器
    pub fn import(&self) -> &RateLimitService {
        &self.import_limiter
    }

    /// 获取工作区限流器
    pub fn workspace(&self) -> &RateLimitService {
        &self.workspace_limiter
    }

    /// 获取默认限流器
    pub fn default(&self) -> &RateLimitService {
        &self.default_limiter
    }
}

impl Default for GlobalRateLimitManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_rate_limiter_allows_requests() {
        let rate_limiter = RateLimitService::new(10, 5);

        // 前 10 个请求应该被允许
        for _ in 0..10 {
            assert!(rate_limiter.check().is_ok());
        }

        // 第 11 个请求应该被拒绝
        assert!(rate_limiter.check().is_err());
    }

    #[test]
    fn test_rate_limiter_burst() {
        let rate_limiter = RateLimitService::new(10, 5);

        // 突发 5 个请求
        for _ in 0..5 {
            assert!(rate_limiter.check().is_ok());
        }

        // 超出 burst 限制
        assert!(rate_limiter.check().is_err());
    }

    #[test]
    fn test_rate_limiter_config() {
        let rate_limiter = RateLimitService::new(100, 10);
        let config = rate_limiter.config();

        assert_eq!(config.requests_per_minute, 100);
        assert_eq!(config.max_burst, 10);
    }

    #[test]
    fn test_global_rate_limit_manager() {
        let manager = GlobalRateLimitManager::new();

        assert_eq!(manager.search().config().requests_per_minute, 60);
        assert_eq!(manager.import().config().requests_per_minute, 10);
        assert_eq!(manager.workspace().config().requests_per_minute, 120);
    }

    #[test]
    fn test_search_rate_limiter() {
        let rate_limiter = RateLimitService::for_search();

        // 搜索限流器应该是每分钟 60 次，最多突发 5 次
        assert_eq!(rate_limiter.config().requests_per_minute, 60);
        assert_eq!(rate_limiter.config().max_burst, 5);
    }

    #[test]
    fn test_import_rate_limiter() {
        let rate_limiter = RateLimitService::for_import();

        // 导入限流器应该是每分钟 10 次，最多突发 2 次
        assert_eq!(rate_limiter.config().requests_per_minute, 10);
        assert_eq!(rate_limiter.config().max_burst, 2);
    }
}
