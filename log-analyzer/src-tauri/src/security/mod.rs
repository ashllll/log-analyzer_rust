//! 安全模块
//!
//! 提供日志分析器的安全防护功能，防止恶意输入攻击。
//!
//! # 子模块
//!
//! - [`line_guard`] - 单行防护器，防止 JSON 炸弹和超长行攻击
//! - [`import_security`] - 导入安全检查，防止文件炸弹攻击
//!
//! # 主要功能
//!
//! - **单行截断**: 限制单行最大长度，防止内存耗尽
//! - **流式处理**: 支持大文件流式处理，不一次性加载全部内容
//! - **文件大小检查**: 限制导入文件的最大大小
//! - **目录深度检查**: 限制目录遍历深度，防止目录遍历攻击
//!
//! # Example
//!
//! ```rust
//! use log_analyzer::security::{LineGuard, ImportSecurity, GuardedLine};
//!
//! // 使用单行防护器
//! let guard = LineGuard::new(1024);
//! let result = guard.guard_line("some log line");
//!
//! if result.was_truncated {
//!     println!("Line was truncated from {} bytes", result.original_length);
//! }
//!
//! // 使用导入安全检查器
//! let security = ImportSecurity::default();
//! let check_result = security.check_file(std::path::Path::new("example.log"));
//!
//! if check_result.is_safe {
//!     println!("File is safe to import");
//! }
//! ```

pub mod import_security;
pub mod line_guard;

// 重导出主要类型，方便使用
pub use import_security::{
    FileSecurityStats, ImportSecurity, ImportSecurityConfig, SecurityCheckResult,
    DEFAULT_MAX_DEPTH, DEFAULT_MAX_FILE_SIZE, DEFAULT_MAX_TOTAL_LINES,
};
pub use line_guard::{
    GuardedLine, LineGuard, LineGuardConfig, DEFAULT_TRUNCATE_MARKER, MAX_LINE_LENGTH,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// 集成测试：完整的导入安全流程
    #[test]
    fn test_full_security_workflow() {
        // 1. 创建安全检查器
        let config = ImportSecurityConfig::new()
            .with_max_file_size(1024 * 1024) // 1MB
            .with_max_line_length(100);
        let security = ImportSecurity::new(config);

        // 2. 模拟一个大文件内容
        let normal_line = "This is a normal log line";
        let long_line = "x".repeat(200);

        // 3. 检查行安全性
        let normal_result = security.guard_line(normal_line);
        assert!(!normal_result.was_truncated);

        let long_result = security.guard_line(&long_line);
        assert!(long_result.was_truncated);
        assert!(long_result.content.contains("[TRUNCATED"));
    }

    /// 测试流式处理大量数据
    #[test]
    fn test_streaming_large_data() {
        let guard = LineGuard::new(100);

        // 模拟 1000 行数据
        let mut data = String::new();
        for i in 0..1000 {
            if i % 100 == 0 {
                // 每 100 行有一个超长行
                data.push_str(&"x".repeat(200));
            } else {
                data.push_str(&format!("Normal line {}", i));
            }
            data.push('\n');
        }

        let reader = Cursor::new(data.as_bytes());
        let lines: Vec<_> = guard.process_stream(reader).collect();

        assert_eq!(lines.len(), 1000);

        // 检查被截断的行数
        let truncated_count = lines.iter().filter(|l| l.was_truncated).count();
        assert_eq!(truncated_count, 10); // 1000 / 100 = 10 个超长行
    }

    /// 测试 JSON 炸弹防护
    #[test]
    fn test_json_bomb_protection() {
        let guard = LineGuard::new(1024);

        // 构造一个 JSON 炸弹
        let mut json_bomb = String::new();
        for _ in 0..10000 {
            json_bomb.push_str("{\"data\":\"");
        }
        json_bomb.push_str("payload");
        for _ in 0..10000 {
            json_bomb.push_str("\"}");
        }

        let result = guard.guard_line(&json_bomb);

        assert!(result.was_truncated);
        assert!(result.content.len() <= 1024);
        assert!(result.original_length > 50000);
    }

    /// 测试 Unicode 安全截断
    #[test]
    fn test_unicode_safe_truncation() {
        let guard = LineGuard::new(20);

        // 包含多字节 UTF-8 字符的字符串
        let unicode_content = "你好世界Hello世界你好世界Hello世界";
        let result = guard.guard_line(unicode_content);

        // 确保截断后的内容是有效的 UTF-8
        assert!(result
            .content
            .chars()
            .all(|c| c != std::char::REPLACEMENT_CHARACTER));

        // 确保没有截断在多字节字符中间
        let chars: Vec<char> = result.content.chars().collect();
        assert!(!chars.is_empty());
    }

    /// 测试空输入处理
    #[test]
    fn test_empty_input() {
        let guard = LineGuard::new(100);

        let result = guard.guard_line("");
        assert!(!result.was_truncated);
        assert!(result.content.is_empty());
    }

    /// 测试边界条件
    #[test]
    fn test_boundary_conditions() {
        let guard = LineGuard::new(10);

        // 正好 10 字节
        let exact = "0123456789";
        let result = guard.guard_line(exact);
        assert!(!result.was_truncated);

        // 11 字节
        let over = "01234567890";
        let result = guard.guard_line(over);
        assert!(result.was_truncated);
    }
}
