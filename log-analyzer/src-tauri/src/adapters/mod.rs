//! 适配器模块
//!
//! 将外部框架类型（Tauri、OS 等）桥接到业务层 trait，
//! 遵循依赖倒置原则，避免业务层直接依赖框架类型。

pub mod tauri_config;
