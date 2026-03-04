//! 应用层 - 应用服务
//!
//! 协调领域层和基础设施层，实现用例
//! 采用 CQRS 模式分离命令和查询

pub mod commands;
pub mod handlers;
pub mod plugins;
pub mod queries;
pub mod services;
