//! 压缩文件处理模块
//!
//! 提供多种压缩格式的解压和递归处理功能，支持：
//! - TAR / TAR.GZ / TGZ
//! - ZIP (支持多编码文件名)
//! - RAR (使用内置 unrar 二进制)
//! - GZ (单文件压缩)
//!
//! # 特性
//! - **跨平台兼容**：Windows UNC 路径、只读文件处理
//! - **安全检查**：防止路径穿越攻击
//! - **错误容忍**：部分失败不中断整体流程
//! - **递归处理**：自动识别嵌套压缩文件
//! - **元数据收集**：支持增量索引更新

pub mod context;
pub mod gz;
pub mod processor;
pub mod rar;
pub mod tar;
pub mod zip;

pub use context::ArchiveContext;
pub use processor::{
    process_path_recursive, process_path_recursive_inner,
    process_path_recursive_inner_with_metadata, process_path_recursive_with_metadata,
};
