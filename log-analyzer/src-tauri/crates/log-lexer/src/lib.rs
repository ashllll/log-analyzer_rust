//! LogLexer - 高性能日志词法分析器
//!
//! 提供编译期优化的日志解析能力：
//! - 通过过程宏声明日志格式
//! - 编译期单态化为 SIMD 机器码
//! - 输出紧凑的 Binary Token
//!
//! # 示例
//!
//! ```ignore
//! use log_lexer::{LogLexer, Token, TokenType};
//! use log_lexer_derive::LogLexer;
//!
//! #[derive(LogLexer)]
//! #[log_format(pattern = "{timestamp} [{level}] {message}")]
//! struct MyLogParser;
//!
//! let parser = MyLogParser;
//! let tokens = parser.tokenize("2024-01-15 10:30:45 [INFO] Application started");
//! ```

#![forbid(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

use std::fmt;

/// Token 类型枚举
///
/// 定义日志中可能出现的各种词法单元类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TokenType {
    /// 时间戳（如：2024-01-15 10:30:45）
    Timestamp = 0,
    /// 日志级别（DEBUG, INFO, WARN, ERROR, FATAL）
    Level = 1,
    /// 日志消息主体
    Message = 2,
    /// 键值对键名
    Key = 3,
    /// 键值对值
    Value = 4,
    /// 数字（整数或浮点数）
    Number = 5,
    /// JSON 字段
    JsonField = 6,
    /// 线程 ID 或名称
    ThreadId = 7,
    /// 类名或模块名
    ClassName = 8,
    /// 方法名或函数名
    MethodName = 9,
    /// 异常堆栈
    StackTrace = 10,
    /// 分隔符（空格、制表符等）
    Separator = 11,
    /// 括号（圆括号、方括号、花括号）
    Bracket = 12,
    /// 引号内的字符串
    QuotedString = 13,
    /// URL 或路径
    Url = 14,
    /// IP 地址
    IpAddress = 15,
    /// UUID
    Uuid = 16,
    /// 自定义类型
    Custom = 255,
}

impl TokenType {
    /// 从 u8 转换为 TokenType
    #[inline]
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(TokenType::Timestamp),
            1 => Some(TokenType::Level),
            2 => Some(TokenType::Message),
            3 => Some(TokenType::Key),
            4 => Some(TokenType::Value),
            5 => Some(TokenType::Number),
            6 => Some(TokenType::JsonField),
            7 => Some(TokenType::ThreadId),
            8 => Some(TokenType::ClassName),
            9 => Some(TokenType::MethodName),
            10 => Some(TokenType::StackTrace),
            11 => Some(TokenType::Separator),
            12 => Some(TokenType::Bracket),
            13 => Some(TokenType::QuotedString),
            14 => Some(TokenType::Url),
            15 => Some(TokenType::IpAddress),
            16 => Some(TokenType::Uuid),
            255 => Some(TokenType::Custom),
            _ => None,
        }
    }

    /// 转换为 u8
    #[inline]
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

impl fmt::Display for TokenType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenType::Timestamp => write!(f, "TIMESTAMP"),
            TokenType::Level => write!(f, "LEVEL"),
            TokenType::Message => write!(f, "MESSAGE"),
            TokenType::Key => write!(f, "KEY"),
            TokenType::Value => write!(f, "VALUE"),
            TokenType::Number => write!(f, "NUMBER"),
            TokenType::JsonField => write!(f, "JSON_FIELD"),
            TokenType::ThreadId => write!(f, "THREAD_ID"),
            TokenType::ClassName => write!(f, "CLASS_NAME"),
            TokenType::MethodName => write!(f, "METHOD_NAME"),
            TokenType::StackTrace => write!(f, "STACK_TRACE"),
            TokenType::Separator => write!(f, "SEPARATOR"),
            TokenType::Bracket => write!(f, "BRACKET"),
            TokenType::QuotedString => write!(f, "QUOTED_STRING"),
            TokenType::Url => write!(f, "URL"),
            TokenType::IpAddress => write!(f, "IP_ADDRESS"),
            TokenType::Uuid => write!(f, "UUID"),
            TokenType::Custom => write!(f, "CUSTOM"),
        }
    }
}

/// 高亮 Token（用于 FFI 边界传递）
///
/// 这是经过 rkyv 优化的零拷贝结构，
/// 可直接传递给 Flutter 前端进行渲染。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "rkyv", derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize))]
pub struct HighlightToken {
    /// Token 类型
    pub token_type: u8,
    /// 在行内的起始偏移量
    pub start_offset: u16,
    /// Token 长度
    pub length: u16,
}

impl HighlightToken {
    /// 创建新的 HighlightToken
    #[inline]
    pub fn new(token_type: TokenType, start_offset: u16, length: u16) -> Self {
        Self {
            token_type: token_type.as_u8(),
            start_offset,
            length,
        }
    }

    /// 获取 Token 类型
    #[inline]
    pub fn token_type(&self) -> Option<TokenType> {
        TokenType::from_u8(self.token_type)
    }

    /// 获取结束偏移量
    #[inline]
    pub fn end_offset(&self) -> u16 {
        self.start_offset.saturating_add(self.length)
    }
}

/// 解析后的日志 Token
///
/// 包含 Token 类型和实际的字符串内容
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    /// Token 类型
    pub token_type: TokenType,
    /// Token 在源文本中的起始位置
    pub start: usize,
    /// Token 在源文本中的结束位置
    pub end: usize,
    /// Token 的字符串值
    pub value: String,
}

impl Token {
    /// 创建新的 Token
    #[inline]
    pub fn new(token_type: TokenType, start: usize, end: usize, value: String) -> Self {
        Self {
            token_type,
            start,
            end,
            value,
        }
    }

    /// 获取 Token 长度
    #[inline]
    pub fn len(&self) -> usize {
        self.end - self.start
    }

    /// 检查 Token 是否为空
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// 转换为 HighlightToken（用于 FFI）
    ///
    /// # 注意
    /// 如果偏移量超过 u16 范围，会被截断
    #[inline]
    pub fn to_highlight_token(&self) -> Option<HighlightToken> {
        let start = u16::try_from(self.start).ok()?;
        let length = u16::try_from(self.len()).ok()?;
        Some(HighlightToken::new(self.token_type, start, length))
    }
}

/// 词法分析错误
#[derive(Debug, Clone, thiserror::Error)]
pub enum LexerError {
    /// 无效的日志格式
    #[error("Invalid log format: {0}")]
    InvalidFormat(String),

    /// 未知的 Token 类型
    #[error("Unknown token type: {0}")]
    UnknownTokenType(String),

    /// 解析失败
    #[error("Parse failed at position {position}: {message}")]
    ParseFailed {
        /// 错误位置
        position: usize,
        /// 错误信息
        message: String,
    },

    /// 编码错误
    #[error("Encoding error: {0}")]
    EncodingError(String),
}

/// 词法分析结果
pub type LexerResult<T> = Result<T, LexerError>;

/// LogLexer Trait - 日志词法分析器接口
///
/// 所有日志解析器都需要实现此 Trait。
/// 通过过程宏可以自动生成高效的实现。
///
/// # 设计原则
///
/// 1. **零拷贝优先**: 尽量避免字符串复制
/// 2. **SIMD 友好**: 数据布局适合 SIMD 优化
/// 3. **编译期优化**: 通过泛型单态化消除运行时开销
pub trait LogLexer: Default {
    /// 解析单行日志，返回 Token 列表
    ///
    /// # 参数
    /// - `line`: 日志行内容
    ///
    /// # 返回
    /// - `Ok(Vec<Token>)`: 解析成功，返回 Token 列表
    /// - `Err(LexerError)`: 解析失败
    fn tokenize(&self, line: &str) -> LexerResult<Vec<Token>>;

    /// 解析单行日志，返回 HighlightToken 列表（用于 FFI）
    ///
    /// 此方法用于生成紧凑的二进制 Token，适合传递给前端。
    ///
    /// # 参数
    /// - `line`: 日志行内容
    ///
    /// # 返回
    /// - `Ok(Vec<HighlightToken>)`: 解析成功
    /// - `Err(LexerError)`: 解析失败
    fn tokenize_for_highlight(&self, line: &str) -> LexerResult<Vec<HighlightToken>> {
        let tokens = self.tokenize(line)?;
        Ok(tokens
            .iter()
            .filter_map(|t| t.to_highlight_token())
            .collect())
    }

    /// 获取解析器名称
    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    /// 检查是否可以解析该格式的日志
    ///
    /// # 参数
    /// - `sample`: 样本日志行
    ///
    /// # 返回
    /// - `true`: 可以解析
    /// - `false`: 不适合该解析器
    fn can_parse(&self, sample: &str) -> bool;

    /// 获取支持的日志格式描述
    fn format_description(&self) -> &'static str;
}

/// 通用日志解析器（内置实现）
///
/// 支持常见的日志格式：
/// - 标准格式：`{timestamp} [{level}] {message}`
/// - JSON 格式
/// - syslog 格式
#[derive(Debug, Clone, Copy, Default)]
pub struct GenericLexer;

impl LogLexer for GenericLexer {
    fn tokenize(&self, line: &str) -> LexerResult<Vec<Token>> {
        let mut tokens = Vec::new();
        let chars: Vec<char> = line.chars().collect();
        let len = chars.len();

        if len == 0 {
            return Ok(tokens);
        }

        // 状态机解析
        let mut pos = 0;

        // 尝试解析时间戳（开头到第一个空格或方括号）
        let timestamp_end = Self::parse_timestamp(&chars, pos);
        if timestamp_end > pos {
            let value: String = chars[pos..timestamp_end].iter().collect();
            tokens.push(Token::new(TokenType::Timestamp, pos, timestamp_end, value));
            pos = timestamp_end;
        }

        // 跳过空格
        while pos < len && chars[pos].is_whitespace() {
            pos += 1;
        }

        // 尝试解析日志级别（方括号内）
        if pos < len && chars[pos] == '[' {
            let level_start = pos;
            let mut level_end = pos + 1;
            while level_end < len && chars[level_end] != ']' {
                level_end += 1;
            }
            if level_end < len {
                let value: String = chars[level_start + 1..level_end].iter().collect();
                let level_type = Self::classify_level(&value);
                tokens.push(Token::new(TokenType::Bracket, level_start, level_start + 1, "[".to_string()));
                tokens.push(Token::new(level_type, level_start + 1, level_end, value));
                tokens.push(Token::new(TokenType::Bracket, level_end, level_end + 1, "]".to_string()));
                pos = level_end + 1;
            }
        }

        // 跳过空格
        while pos < len && chars[pos].is_whitespace() {
            pos += 1;
        }

        // 剩余部分作为消息
        if pos < len {
            let value: String = chars[pos..len].iter().collect();
            tokens.push(Token::new(TokenType::Message, pos, len, value));
        }

        Ok(tokens)
    }

    fn can_parse(&self, _sample: &str) -> bool {
        // 通用解析器总是返回 true
        true
    }

    fn format_description(&self) -> &'static str {
        "Generic log format: {timestamp} [{level}] {message}"
    }
}

impl GenericLexer {
    /// 解析时间戳部分
    fn parse_timestamp(chars: &[char], start: usize) -> usize {
        let mut pos = start;
        let len = chars.len();

        // 时间戳通常以数字开头
        if pos >= len || !chars[pos].is_ascii_digit() {
            return start;
        }

        // 扫描直到遇到 '[' 或连续多个空格
        let mut space_count = 0;
        while pos < len {
            if chars[pos] == '[' {
                break;
            }
            if chars[pos].is_whitespace() {
                space_count += 1;
                if space_count >= 2 {
                    break;
                }
            } else {
                space_count = 0;
            }
            pos += 1;
        }

        // 回退到非空格位置
        while pos > start && chars[pos - 1].is_whitespace() {
            pos -= 1;
        }

        pos
    }

    /// 分类日志级别
    fn classify_level(level: &str) -> TokenType {
        match level.to_uppercase().as_str() {
            "DEBUG" | "TRACE" | "VERBOSE" => TokenType::Level,
            "INFO" | "INFORMATION" => TokenType::Level,
            "WARN" | "WARNING" => TokenType::Level,
            "ERROR" | "ERR" => TokenType::Level,
            "FATAL" | "CRITICAL" | "CRIT" => TokenType::Level,
            _ => TokenType::Level,
        }
    }
}

/// JSON 日志解析器
#[derive(Debug, Clone, Copy, Default)]
pub struct JsonLexer;

impl LogLexer for JsonLexer {
    fn tokenize(&self, line: &str) -> LexerResult<Vec<Token>> {
        let mut tokens = Vec::new();
        let trimmed = line.trim();

        if !trimmed.starts_with('{') {
            return Err(LexerError::InvalidFormat(
                "JSON log must start with '{'".to_string(),
            ));
        }

        // 简化的 JSON 词法分析
        let chars: Vec<char> = trimmed.chars().collect();
        let len = chars.len();
        let mut pos = 0;

        while pos < len {
            match chars[pos] {
                '{' | '}' | '[' | ']' | ':' | ',' => {
                    tokens.push(Token::new(
                        TokenType::Bracket,
                        pos,
                        pos + 1,
                        chars[pos].to_string(),
                    ));
                    pos += 1;
                }
                '"' => {
                    // 解析字符串
                    let start = pos;
                    pos += 1;
                    while pos < len && chars[pos] != '"' {
                        if chars[pos] == '\\' && pos + 1 < len {
                            pos += 2; // 跳过转义字符
                        } else {
                            pos += 1;
                        }
                    }
                    if pos < len {
                        pos += 1; // 包含结束引号
                    }
                    let value: String = chars[start..pos].iter().collect();
                    tokens.push(Token::new(TokenType::QuotedString, start, pos, value));
                }
                ' ' | '\t' | '\n' | '\r' => {
                    pos += 1;
                }
                _ => {
                    if chars[pos].is_ascii_digit() || chars[pos] == '-' {
                        // 解析数字
                        let start = pos;
                        while pos < len
                            && (chars[pos].is_ascii_digit()
                                || chars[pos] == '.'
                                || chars[pos] == '-'
                                || chars[pos] == 'e'
                                || chars[pos] == 'E'
                                || chars[pos] == '+')
                        {
                            pos += 1;
                        }
                        let value: String = chars[start..pos].iter().collect();
                        tokens.push(Token::new(TokenType::Number, start, pos, value));
                    } else if chars[pos].is_alphabetic() {
                        // 解析关键字（true, false, null）
                        let start = pos;
                        while pos < len && (chars[pos].is_alphanumeric() || chars[pos] == '_') {
                            pos += 1;
                        }
                        let value: String = chars[start..pos].iter().collect();
                        tokens.push(Token::new(TokenType::Value, start, pos, value));
                    } else {
                        pos += 1;
                    }
                }
            }
        }

        Ok(tokens)
    }

    fn can_parse(&self, sample: &str) -> bool {
        let trimmed = sample.trim();
        trimmed.starts_with('{') && trimmed.ends_with('}')
    }

    fn format_description(&self) -> &'static str {
        "JSON log format: {\"timestamp\": \"...\", \"level\": \"...\", \"message\": \"...\"}"
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_type_roundtrip() {
        for i in 0u8..=16 {
            if let Some(tt) = TokenType::from_u8(i) {
                assert_eq!(tt.as_u8(), i);
            }
        }
        assert_eq!(TokenType::Custom.as_u8(), 255);
        assert_eq!(TokenType::from_u8(255), Some(TokenType::Custom));
    }

    #[test]
    fn test_highlight_token_new() {
        let token = HighlightToken::new(TokenType::Timestamp, 0, 25);
        assert_eq!(token.token_type, 0);
        assert_eq!(token.start_offset, 0);
        assert_eq!(token.length, 25);
        assert_eq!(token.end_offset(), 25);
    }

    #[test]
    fn test_generic_lexer_tokenize() {
        let lexer = GenericLexer;
        let line = "2024-01-15 10:30:45 [INFO] Application started";
        let tokens = lexer.tokenize(line).unwrap();

        assert!(!tokens.is_empty());
        // 应该包含时间戳
        assert!(tokens.iter().any(|t| t.token_type == TokenType::Timestamp));
        // 应该包含级别
        assert!(tokens.iter().any(|t| t.token_type == TokenType::Level));
        // 应该包含消息
        assert!(tokens.iter().any(|t| t.token_type == TokenType::Message));
    }

    #[test]
    fn test_generic_lexer_empty_line() {
        let lexer = GenericLexer;
        let tokens = lexer.tokenize("").unwrap();
        assert!(tokens.is_empty());
    }

    #[test]
    fn test_json_lexer_tokenize() {
        let lexer = JsonLexer;
        let line = r#"{"timestamp": "2024-01-15", "level": "INFO", "message": "Hello"}"#;
        let tokens = lexer.tokenize(line).unwrap();

        assert!(!tokens.is_empty());
        // 应该包含引号字符串
        assert!(tokens.iter().any(|t| t.token_type == TokenType::QuotedString));
    }

    #[test]
    fn test_json_lexer_invalid() {
        let lexer = JsonLexer;
        let result = lexer.tokenize("not a json");
        assert!(result.is_err());
    }

    #[test]
    fn test_json_lexer_can_parse() {
        let lexer = JsonLexer;
        assert!(lexer.can_parse(r#"{"key": "value"}"#));
        assert!(!lexer.can_parse("plain text log"));
    }

    #[test]
    fn test_token_to_highlight() {
        let token = Token::new(TokenType::Level, 10, 14, "INFO".to_string());
        let highlight = token.to_highlight_token().unwrap();
        assert_eq!(highlight.start_offset, 10);
        assert_eq!(highlight.length, 4);
    }

    #[test]
    fn test_token_to_highlight_overflow() {
        // 超过 u16 范围的偏移量应该返回 None
        let token = Token::new(TokenType::Level, 100000, 100004, "INFO".to_string());
        assert!(token.to_highlight_token().is_none());
    }
}
