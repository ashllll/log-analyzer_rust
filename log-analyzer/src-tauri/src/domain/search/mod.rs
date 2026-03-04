//! 搜索领域模块
//!
//! 定义搜索相关的领域概念：
//! - 搜索策略 (SearchStrategy)
//! - 搜索结果 (SearchResult)
//! - 搜索聚合器 (SearchAggregator)
//! - 搜索仓储接口 (SearchRepository)

#[allow(dead_code)]
pub mod entities;
#[allow(dead_code)]
pub mod repositories;
#[allow(dead_code)]
pub mod services;
#[allow(dead_code)]
pub mod value_objects;

pub use entities::{SearchResult, SearchSession};
pub use repositories::SearchRepository;
pub use services::{SearchAggregator, SearchStrategyEvaluator};
pub use value_objects::{SearchMode, SearchPriority, SearchQuery};
