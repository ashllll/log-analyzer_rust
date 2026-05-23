//! 统一配置管理系统
//!
//! 使用 `config` crate 实现行业标准的配置管理：
//! - 多层配置：默认值 → 配置文件 → 环境变量
//! - 支持 JSON、TOML、YAML 等格式
//! - 环境变量自动解析和映射
//! - 配置验证
//!
//! # 配置优先级
//!
//! 1. 环境变量（最高优先级，覆盖所有）
//! 2. 用户配置文件（config.json）
//! 3. 默认值（最低优先级）
//!
//! # 子模块
//!
//! - `validator`: ConfigValidator trait、验证错误类型、验证辅助函数
//! - `models`: 所有配置结构体及其验证实现
//! - `loader`: ConfigLoader（AppConfigLoader）配置加载器

pub mod loader;
pub mod models;
pub mod validator;

// Re-export everything for backward compatibility
pub use loader::*;
pub use models::*;
pub use validator::*;

// Legacy alias
pub use loader::ConfigLoader as AppConfigLoader;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
