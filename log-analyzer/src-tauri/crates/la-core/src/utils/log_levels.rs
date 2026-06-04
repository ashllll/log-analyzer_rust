//! 日志级别位掩码工具
//!
//! 提供统一的日志级别字符串 → 位掩码映射，供导入、搜索和过滤器模块共享。
//!
//! # 位定义标准
//!
//! - error   => bit 0 (mask 0x01)
//! - warn    => bit 1 (mask 0x02)
//! - info    => bit 2 (mask 0x04)
//! - debug   => bit 3 (mask 0x08)
//! - trace   => bit 4 (mask 0x10)
//!
//! 这种编码允许多个级别通过 `|` 组合为单个 u8 位掩码（如 error | warn = 0x03）。

/// 将日志级别字符串转换为位掩码。
///
/// 大小写不敏感，支持 "error" / "warn" / "warning" / "info" / "debug" / "trace"。
/// 未识别的级别返回 0（不匹配任何已知级别）。
pub fn level_to_mask(level: &str) -> u8 {
    match level.trim().to_ascii_lowercase().as_str() {
        "error" => 1 << 0,
        "warn" | "warning" => 1 << 1,
        "info" => 1 << 2,
        "debug" => 1 << 3,
        "trace" => 1 << 4,
        _ => 0,
    }
}
