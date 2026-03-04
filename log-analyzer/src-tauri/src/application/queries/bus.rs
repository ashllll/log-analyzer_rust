//! CQRS 查询总线
//!
//! 负责将查询分发给对应的处理器执行

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;

use super::handlers::{QueryHandler, QueryResult};
use super::queries::Query;

/// 查询处理器工厂类型
type HandlerFactory = Arc<dyn Fn() -> Box<dyn Any + Send + Sync> + Send + Sync>;

/// 查询总线
///
/// 负责查询的分发和执行
pub struct QueryBus {
    handlers: RwLock<HashMap<TypeId, HandlerFactory>>,
}

impl QueryBus {
    /// 创建新的查询总线
    pub fn new() -> Self {
        Self {
            handlers: RwLock::new(HashMap::new()),
        }
    }

    /// 注册查询处理器
    pub fn register<Q, H>(&self, handler: H)
    where
        Q: Query + 'static,
        H: QueryHandler<Q> + 'static,
    {
        let type_id = TypeId::of::<Q>();
        let handler = Arc::new(handler);

        let factory: HandlerFactory = Arc::new(move || {
            let h: Arc<dyn QueryHandler<Q>> = handler.clone();
            Box::new(h) as Box<dyn Any + Send + Sync>
        });

        let mut handlers = self.handlers.write().unwrap();
        handlers.insert(type_id, factory);
    }

    /// 执行查询
    pub async fn dispatch<Q>(&self, query: Q) -> QueryResult<Box<dyn Any + Send>>
    where
        Q: Query + 'static,
    {
        let type_id = TypeId::of::<Q>();

        // 获取处理器工厂（在 await 之前释放锁）
        let factory = {
            let handlers = self.handlers.read().unwrap();
            handlers.get(&type_id).cloned().ok_or_else(|| {
                super::handlers::QueryError::ExecutionFailed(format!(
                    "未找到查询类型的处理器: {}",
                    query.query_type()
                ))
            })?
        };

        let handler_box = factory();
        let handler = handler_box
            .downcast_ref::<Arc<dyn QueryHandler<Q>>>()
            .ok_or_else(|| {
                super::handlers::QueryError::ExecutionFailed("处理器类型转换失败".to_string())
            })?;

        handler.handle(query).await
    }

    /// 检查是否注册了处理器
    pub fn has_handler<Q: Query + 'static>(&self) -> bool {
        let type_id = TypeId::of::<Q>();
        let handlers = self.handlers.read().unwrap();
        handlers.contains_key(&type_id)
    }

    /// 获取已注册的处理器数量
    pub fn handler_count(&self) -> usize {
        self.handlers.read().unwrap().len()
    }
}

impl Default for QueryBus {
    fn default() -> Self {
        Self::new()
    }
}

/// 查询执行器 trait
///
/// 提供更简洁的查询执行接口
#[async_trait]
pub trait QueryExecutor: Send + Sync {
    /// 执行查询并返回类型化结果
    async fn execute<Q>(&self, query: Q) -> QueryResult<Box<dyn Any + Send>>
    where
        Q: Query + 'static;
}

#[async_trait]
impl QueryExecutor for QueryBus {
    async fn execute<Q>(&self, query: Q) -> QueryResult<Box<dyn Any + Send>>
    where
        Q: Query + 'static,
    {
        self.dispatch(query).await
    }
}

// ==================== 单元测试 ====================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::queries::queries::GetWorkspaceQuery;

    #[test]
    fn test_query_bus_creation() {
        let bus = QueryBus::new();
        assert_eq!(bus.handler_count(), 0);
    }

    #[test]
    fn test_query_bus_has_handler() {
        let bus = QueryBus::new();
        assert!(!bus.has_handler::<GetWorkspaceQuery>());
    }

    #[test]
    fn test_query_bus_default() {
        let bus = QueryBus::default();
        assert_eq!(bus.handler_count(), 0);
    }

    #[tokio::test]
    async fn test_query_bus_dispatch_no_handler() {
        let bus = QueryBus::new();
        let query = GetWorkspaceQuery::new("ws-1".to_string());

        let result = bus.dispatch(query).await;
        assert!(result.is_err());
    }
}
