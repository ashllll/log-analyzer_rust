//! 基础设施层 - 技术实现
//!
//! 提供持久化、外部服务、框架集成等技术实现
//!
//! ## 模块说明
//!
//! - `config`: 配置管理
//! - `persistence`: 数据持久化（仓储实现）
//! - `messaging`: 消息基础设施（事件总线）
//! - `external`: 外部服务集成（HTTP 客户端）

pub mod config;
pub mod external;
pub mod messaging;
pub mod persistence;

// 重导出常用类型
pub use external::{
    ExternalServiceError, ExternalServiceManager, HealthCheckResult, HealthChecker, RateLimiter,
    RateLimiterConfig,
};
pub use messaging::{
    DomainEventPublisher, EventBusConfig, EventMessage, EventReplayer, InMemoryEventBus,
    Subscription,
};
pub use persistence::{
    JsonFileStorage, KeywordGroupRepositoryImpl, PersistenceConfig, PersistenceFactory,
    SearchHistoryRepositoryImpl, WorkspaceRepositoryImpl,
};
