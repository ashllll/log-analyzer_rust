//! 配置数据模型
//!
//! 按领域拆分为子模块，通过 re-export 保持向后兼容。

pub mod defaults;
pub mod archive;
pub mod search;
pub mod network;
pub mod app;

pub use archive::*;
pub use search::*;
pub use network::*;
pub use app::*;
