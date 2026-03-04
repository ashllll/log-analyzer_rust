//! 导出领域模块
//!
//! 定义导出相关的领域概念：
//! - 导出格式 (ExportFormat)
//! - 导出策略 (ExportStrategy)
//! - 导出任务 (ExportTask)
//! - 导出仓储接口 (ExportRepository)

#[allow(dead_code)]
pub mod entities;
#[allow(dead_code)]
pub mod repositories;
#[allow(dead_code)]
pub mod services;
#[allow(dead_code)]
pub mod value_objects;

pub use entities::ExportTask;
pub use repositories::ExportRepository;
pub use services::{ExportAggregator, ExportStrategy};
pub use value_objects::{ExportFormat, ExportOptions};
