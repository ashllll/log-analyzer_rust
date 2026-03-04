//! CQRS 查询处理器模块
//!
//! 实现命令查询职责分离(CQRS)模式的查询部分：
//! - 查询接口 (Query trait)
//! - 查询处理器 (QueryHandler trait)
//! - 查询总线 (QueryBus)
//! - 具体查询实现

#[allow(dead_code)]
mod bus;
#[allow(dead_code)]
mod handlers;
#[allow(clippy::module_inception)]
#[allow(dead_code)]
mod queries;

pub use bus::QueryBus;
pub use handlers::{QueryHandler, QueryResult};
pub use queries::{GetKeywordsQuery, GetTaskStatusQuery, GetWorkspaceQuery, SearchLogsQuery};
