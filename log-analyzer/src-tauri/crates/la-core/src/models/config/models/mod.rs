//! 配置数据模型
//!
//! 按领域拆分为子模块，通过 re-export 保持向后兼容。

pub mod app;
pub mod archive;
pub mod defaults;
pub mod network;
pub mod search;

pub use app::*;
pub use archive::*;
pub use network::*;
pub use search::*;
