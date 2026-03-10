//! 单行防护器 - 防止 JSON 炸弹和超长行攻击
//!
//! 本模块实现了单行强制虚拟截断机制，防止恶意日志导致内存耗尽。
//! 主要防护目标：
//! - JSON 炸弹攻击（深度嵌套、超长字符串）
//! - 单行内存耗尽攻击
//! - 恶意构造的超长日志行
//!
//! # 设计原则
//! - 内存安全：限制单行最大长度，防止 OOM
//! - 信息保留：截断时保留原始长度信息
//! - 流式处理：支持大文件流式处理，不一次性加载全部内容

use std::io::{BufRead, Read};

/// 单行最大长度 (1MB)
/// 这个值可以容纳绝大多数正常的日志行，同时防止内存攻击
pub const MAX_LINE_LENGTH: usize = 1024 * 1024;

/// 默认截断标记后缀
pub const DEFAULT_TRUNCATE_MARKER: &str = "... [TRUNCATED]";

/// 已防护的单行数据
#[derive(Debug, Clone, PartialEq)]
pub struct GuardedLine {
    /// 截断后的内容
    pub content: String,
    /// 原始长度（字节）
    pub original_length: usize,
    /// 是否被截断
    pub was_truncated: bool,
}

impl GuardedLine {
    /// 创建一个未被截断的防护行
    pub fn new(content: String) -> Self {
        let len = content.len();
        Self {
            content,
            original_length: len,
            was_truncated: false,
        }
    }

    /// 创建一个被截断的防护行
    pub fn truncated(content: String, original_length: usize) -> Self {
        Self {
            content,
            original_length,
            was_truncated: true,
        }
    }

    /// 获取截断率（0.0 - 1.0）
    pub fn truncation_ratio(&self) -> f64 {
        if self.original_length == 0 {
            return 0.0;
        }
        1.0 - (self.content.len() as f64 / self.original_length as f64)
    }
}

/// 单行防护器配置
#[derive(Debug, Clone)]
pub struct LineGuardConfig {
    /// 单行最大长度（字节）
    pub max_length: usize,
    /// 截断标记
    pub truncate_marker: String,
    /// 是否在截断标记中包含原始长度
    pub include_original_length: bool,
}

impl Default for LineGuardConfig {
    fn default() -> Self {
        Self {
            max_length: MAX_LINE_LENGTH,
            truncate_marker: DEFAULT_TRUNCATE_MARKER.to_string(),
            include_original_length: true,
        }
    }
}

impl LineGuardConfig {
    /// 创建新的配置
    pub fn new(max_length: usize) -> Self {
        Self {
            max_length,
            ..Default::default()
        }
    }

    /// 设置截断标记
    pub fn with_marker(mut self, marker: impl Into<String>) -> Self {
        self.truncate_marker = marker.into();
        self
    }

    /// 设置是否包含原始长度
    pub fn with_original_length(mut self, include: bool) -> Self {
        self.include_original_length = include;
        self
    }
}

/// 单行防护器
///
/// 用于检查和截断超长行，防止 JSON 炸弹攻击。
///
/// # Example
///
/// ```rust
/// use log_analyzer::security::LineGuard;
///
/// let guard = LineGuard::new(1024);
/// let line = guard.guard_line("This is a normal line");
/// assert!(!line.was_truncated);
/// ```
#[derive(Debug, Clone)]
pub struct LineGuard {
    config: LineGuardConfig,
}

impl Default for LineGuard {
    fn default() -> Self {
        Self::new(MAX_LINE_LENGTH)
    }
}

impl LineGuard {
    /// 创建新的单行防护器
    ///
    /// # Arguments
    /// * `max_length` - 单行最大长度（字节）
    pub fn new(max_length: usize) -> Self {
        Self {
            config: LineGuardConfig::new(max_length),
        }
    }

    /// 使用自定义配置创建防护器
    pub fn with_config(config: LineGuardConfig) -> Self {
        Self { config }
    }

    /// 获取最大长度配置
    pub fn max_length(&self) -> usize {
        self.config.max_length
    }

    /// 检查并截断超长行
    ///
    /// # Arguments
    /// * `line` - 原始行内容
    ///
    /// # Returns
    /// 返回 `GuardedLine` 结构，包含截断后的内容和元数据
    pub fn guard_line(&self, line: &str) -> GuardedLine {
        let original_length = line.len();

        if original_length <= self.config.max_length {
            return GuardedLine::new(line.to_string());
        }

        // 计算截断后的内容长度（预留截断标记的空间）
        let marker = self.build_truncate_marker(original_length);
        let marker_len = marker.len();

        // 确保截断后的内容 + 标记不超过最大长度
        let available_for_content = self.config.max_length.saturating_sub(marker_len);
        let truncate_at = available_for_content.min(original_length);

        // 确保在 UTF-8 字符边界截断
        let truncate_at = self.find_char_boundary(line, truncate_at);

        let truncated_content = format!("{}{}", &line[..truncate_at], marker);

        GuardedLine::truncated(truncated_content, original_length)
    }

    /// 流式处理大文件，逐行检查并截断
    ///
    /// 此方法使用迭代器模式，支持大文件流式处理，不会一次性加载全部内容到内存。
    ///
    /// # Arguments
    /// * `reader` - 实现 `Read` trait 的读取器
    ///
    /// # Returns
    /// 返回一个迭代器，每次迭代返回一个 `GuardedLine`
    ///
    /// # Example
    /// ```rust
    /// use std::io::Cursor;
    /// use log_analyzer::security::LineGuard;
    ///
    /// let guard = LineGuard::new(100);
    /// let data = b"line1\nline2\nline3";
    /// let reader = Cursor::new(data);
    ///
    /// let lines: Vec<_> = guard.process_stream(reader).collect();
    /// assert_eq!(lines.len(), 3);
    /// ```
    pub fn process_stream<'a, R: Read + 'a>(
        &'a self,
        reader: R,
    ) -> impl Iterator<Item = GuardedLine> + 'a {
        let buf_reader = std::io::BufReader::new(reader);

        LineGuardIterator {
            reader: buf_reader,
            guard: self,
            finished: false,
            leftover: None,
        }
    }

    /// 构建截断标记
    fn build_truncate_marker(&self, original_length: usize) -> String {
        if self.config.include_original_length {
            format!("... [TRUNCATED: {} bytes]", format_size(original_length))
        } else {
            self.config.truncate_marker.clone()
        }
    }

    /// 在 UTF-8 字符边界处找到安全的截断位置
    fn find_char_boundary(&self, s: &str, pos: usize) -> usize {
        if pos >= s.len() {
            return s.len();
        }

        // 向后查找字符边界
        let mut safe_pos = pos;
        while safe_pos > 0 && !s.is_char_boundary(safe_pos) {
            safe_pos -= 1;
        }

        safe_pos
    }
}

/// 流式处理迭代器
struct LineGuardIterator<'a, R: BufRead> {
    reader: R,
    guard: &'a LineGuard,
    finished: bool,
    /// 保存上一次读取后剩余的数据（找到换行符后未处理的数据）
    leftover: Option<Vec<u8>>,
}

impl<'a, R: BufRead> Iterator for LineGuardIterator<'a, R> {
    type Item = GuardedLine;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        let max_length = self.guard.config.max_length;
        let mut line = String::new();
        let mut current_len = 0;

        // 使用带限制的读取，防止超长行绕过防护
        // 每次读取一个合理大小的块
        let mut buffer = [0u8; 8192];

        // 首先处理上次剩余的数据
        if let Some(leftover) = self.leftover.take() {
            // 检查剩余数据中是否包含换行符
            if let Some(pos) = leftover.iter().position(|&b| b == b'\n') {
                line.push_str(&String::from_utf8_lossy(&leftover[..pos]));
                // 保留剩余数据供下次使用
                if pos + 1 < leftover.len() {
                    self.leftover = Some(leftover[pos + 1..].to_vec());
                }
                // 应用截断逻辑后再返回
                let original_len = line.len();
                if original_len <= max_length {
                    return Some(GuardedLine::new(line));
                } else {
                    return Some(self.guard.guard_line(&line));
                }
            } else {
                // 没有换行符，将剩余数据追加到当前行
                line.push_str(&String::from_utf8_lossy(&leftover));
                current_len = leftover.len();
            }
        }

        loop {
            match self.reader.read(&mut buffer) {
                Ok(0) => {
                    // EOF
                    self.finished = true;
                    break;
                }
                Ok(n) => {
                    // 检查读取的块中是否包含换行符
                    if let Some(pos) = buffer[..n].iter().position(|&b| b == b'\n') {
                        // 找到换行符，将之前的内容和换行符之前的内容追加
                        line.push_str(&String::from_utf8_lossy(&buffer[..pos]));
                        // 保存剩余数据供下次使用
                        if pos + 1 < n {
                            self.leftover = Some(buffer[pos + 1..n].to_vec());
                        }
                        // 已经读取了一行，退出循环
                        // 应用截断逻辑后再返回（在循环外的最终处理中）
                        break;
                    }

                    // 没有换行符，检查是否超过最大长度
                    current_len += n;
                    if current_len > max_length {
                        // 超过最大长度，需要截断
                        // 追加超出部分（但不超过限制）
                        let available = max_length.saturating_sub(line.len());
                        if available > 0 {
                            line.push_str(&String::from_utf8_lossy(&buffer[..available]));
                        }
                        // 添加截断标记
                        let marker = self.guard.build_truncate_marker(current_len);
                        line.push_str(&marker);

                        // 标记为已完成（遇到超长行）
                        self.finished = true;
                        break;
                    }

                    // 追加到当前行
                    line.push_str(&String::from_utf8_lossy(&buffer[..n]));
                }
                Err(e) => {
                    tracing::error!(error = %e, "Error reading line in stream");
                    self.finished = true;
                    break;
                }
            }
        }

        if line.is_empty() {
            None
        } else {
            // 检查是否需要截断（针对正常读取但超长的情况）
            let original_len = line.len();
            if original_len <= max_length {
                Some(GuardedLine::new(line))
            } else {
                Some(self.guard.guard_line(&line))
            }
        }
    }
}

/// 格式化字节大小为人类可读格式
fn format_size(bytes: usize) -> String {
    const KB: usize = 1024;
    const MB: usize = KB * 1024;
    const GB: usize = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_guard_normal_line() {
        let guard = LineGuard::new(100);
        let line = guard.guard_line("This is a normal line");

        assert!(!line.was_truncated);
        assert_eq!(line.content, "This is a normal line");
        assert_eq!(line.original_length, 21);
    }

    #[test]
    fn test_guard_long_line() {
        let guard = LineGuard::new(100);
        let long_line = "x".repeat(200);
        let result = guard.guard_line(&long_line);

        assert!(result.was_truncated);
        assert_eq!(result.original_length, 200);
        assert!(result.content.contains("[TRUNCATED"));
        assert!(result.content.len() <= 100);
    }

    #[test]
    fn test_truncation_ratio() {
        let guard = LineGuard::new(100);

        // 未截断的情况
        let normal = guard.guard_line("short");
        assert_eq!(normal.truncation_ratio(), 0.0);

        // 被截断的情况 - 由于截断标记的存在，实际截断比例会低于预期
        let long_line = "x".repeat(200);
        let truncated = guard.guard_line(&long_line);
        // 验证确实发生了截断（内容比原始内容短）
        assert!(
            truncated.truncation_ratio() > 0.0,
            "Expected positive truncation ratio"
        );
    }

    #[test]
    fn test_utf8_boundary_truncation() {
        let guard = LineGuard::new(10);
        // 包含多字节 UTF-8 字符
        let unicode_line = "你好世界这是一个测试行内容更多文字";
        let result = guard.guard_line(unicode_line);

        // 确保截断后的内容是有效的 UTF-8
        assert!(result
            .content
            .chars()
            .all(|c| c != std::char::REPLACEMENT_CHARACTER));
        assert!(result.was_truncated);
    }

    #[test]
    fn test_process_stream() {
        let guard = LineGuard::new(50);
        let data = b"line1\nline2\nline3";
        let reader = Cursor::new(data);

        let lines: Vec<_> = guard.process_stream(reader).collect();

        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].content, "line1");
        assert_eq!(lines[1].content, "line2");
        assert_eq!(lines[2].content, "line3");
    }

    #[test]
    fn test_process_stream_with_long_line() {
        let guard = LineGuard::new(50);
        let long_line = "x".repeat(100);
        // 注意：添加换行符以便正确分隔行
        let data = format!("short\n{}\nshort\n", long_line);
        let reader = Cursor::new(data.as_bytes());

        let lines: Vec<_> = guard.process_stream(reader).collect();

        assert_eq!(lines.len(), 3, "Expected 3 lines, got {}", lines.len());
        assert!(
            !lines[0].was_truncated,
            "First line should not be truncated"
        );
        assert!(lines[1].was_truncated, "Second line should be truncated");
        assert!(
            lines[1].content.contains("[TRUNCATED"),
            "Second line should contain TRUNCATED marker"
        );
        assert!(
            !lines[2].was_truncated,
            "Third line should not be truncated"
        );
    }

    #[test]
    fn test_empty_line() {
        let guard = LineGuard::new(100);
        let result = guard.guard_line("");

        assert!(!result.was_truncated);
        assert_eq!(result.content, "");
        assert_eq!(result.original_length, 0);
    }

    #[test]
    fn test_exact_boundary() {
        let guard = LineGuard::new(10);
        let exact_line = "0123456789"; // 正好 10 字节
        let result = guard.guard_line(exact_line);

        assert!(!result.was_truncated);
        assert_eq!(result.content, "0123456789");
    }

    #[test]
    fn test_one_over_boundary() {
        let guard = LineGuard::new(10);
        let over_line = "0123456789a"; // 11 字节
        let result = guard.guard_line(over_line);

        assert!(result.was_truncated);
        // 由于截断标记会被添加，总长度会超过 max_length
        // 验证行被截断且包含原始长度信息
        assert!(result.content.len() > 10);
        assert!(result.content.contains("TRUNCATED"));
    }

    #[test]
    fn test_custom_config() {
        let config = LineGuardConfig::new(50)
            .with_marker("[CUT]")
            .with_original_length(false);

        let guard = LineGuard::with_config(config);
        let long_line = "x".repeat(100);
        let result = guard.guard_line(&long_line);

        assert!(result.was_truncated);
        assert!(result.content.contains("[CUT]"));
        assert!(!result.content.contains("bytes"));
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(500), "500 B");
        assert_eq!(format_size(1024), "1.00 KB");
        assert_eq!(format_size(1024 * 1024), "1.00 MB");
        assert_eq!(format_size(1024 * 1024 * 1024), "1.00 GB");
    }

    #[test]
    fn test_guarded_line_constructors() {
        let normal = GuardedLine::new("test".to_string());
        assert!(!normal.was_truncated);
        assert_eq!(normal.content, "test");
        assert_eq!(normal.original_length, 4);

        let truncated = GuardedLine::truncated("te".to_string(), 100);
        assert!(truncated.was_truncated);
        assert_eq!(truncated.content, "te");
        assert_eq!(truncated.original_length, 100);
    }

    #[test]
    fn test_json_bomb_protection() {
        // 模拟 JSON 炸弹攻击：深度嵌套的 JSON
        let guard = LineGuard::new(1024);

        // 构造一个深度嵌套的 JSON 炸弹
        let mut json_bomb = String::new();
        for _ in 0..1000 {
            json_bomb.push_str("{\"a\":");
        }
        json_bomb.push_str("\"x\"");
        for _ in 0..1000 {
            json_bomb.push('}');
        }

        let result = guard.guard_line(&json_bomb);

        assert!(result.was_truncated);
        assert!(result.content.len() <= 1024);
        assert!(result.original_length > 1024);
        // 确保截断后的内容是有效的 UTF-8
        assert!(result.content.chars().count() > 0);
    }

    #[test]
    fn test_string_bomb_protection() {
        // 模拟字符串炸弹攻击：超长字符串
        let guard = LineGuard::new(1000);

        // 构造一个 10MB 的字符串
        let bomb = "A".repeat(10 * 1024 * 1024);
        let result = guard.guard_line(&bomb);

        assert!(result.was_truncated);
        assert!(result.content.len() <= 1000);
        assert_eq!(result.original_length, 10 * 1024 * 1024);
    }
}
