// la-core: 共享类型、错误处理、纯数据模型
pub mod error;
pub mod models;
pub mod storage_types;
pub mod traits;
pub mod utils;

pub use error::{AppError, CommandError, CommandResult, ErrorCategory, Result};
pub use storage_types::{ArchiveMetadata, FileMetadata};
