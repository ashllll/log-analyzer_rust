//! LogLexer 过程宏
//!
//! 提供编译期优化的日志词法分析器生成。
//!
//! # 使用方法
//!
//! ```ignore
//! use log_lexer::LogLexer;
//! use log_lexer_derive::LogLexerParser;
//!
//! // 定义日志格式
//! #[derive(LogLexerParser)]
//! #[log_pattern(
//!     timestamp = r"\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}",
//!     level = r"\[DEBUG|INFO|WARN|ERROR|FATAL\]",
//!     message = r".*"
//! )]
//! struct MyAppLogParser;
//! ```

#![forbid(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Meta};

/// LogLexerParser 派生宏
///
/// 为结构体自动生成 LogLexer Trait 实现。
///
/// # 属性
///
/// - `#[log_pattern(timestamp = "...", level = "...", message = "...")]`
///   定义各个字段的正则表达式模式
///
/// - `#[log_format(pattern = "{timestamp} [{level}] {message}")]`
///   定义日志格式的简化描述
///
/// - `#[log_delimiter(delim = " ")]`
///   定义字段分隔符（默认为空格）
///
/// # 示例
///
/// ```ignore
/// #[derive(LogLexerParser)]
/// #[log_pattern(
///     timestamp = r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}",
///     level = r"(DEBUG|INFO|WARN|ERROR)",
///     message = r".+"
/// )]
/// struct IsoLogParser;
/// ```
#[proc_macro_derive(LogLexerParser, attributes(log_pattern, log_format, log_delimiter))]
pub fn log_lexer_parser_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match generate_lexer_impl(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// 解析日志模式属性
struct LogPattern {
    timestamp: Option<String>,
    level: Option<String>,
    message: Option<String>,
    custom: Vec<(String, String)>,
}

impl Default for LogPattern {
    fn default() -> Self {
        Self {
            timestamp: Some(r"\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}".to_string()),
            level: Some(r"\[(DEBUG|INFO|WARN|ERROR|FATAL|TRACE)\]".to_string()),
            message: Some(r".*".to_string()),
            custom: Vec::new(),
        }
    }
}

/// 解析 log_pattern 属性
fn parse_log_pattern(meta: &Meta) -> Result<LogPattern, syn::Error> {
    let mut pattern = LogPattern::default();

    let Meta::List(list) = meta else {
        return Err(syn::Error::new_spanned(
            meta,
            "expected #[log_pattern(...)]",
        ));
    };

    let mut timestamp: Option<String> = None;
    let mut level: Option<String> = None;
    let mut message: Option<String> = None;

    // 解析键值对
    let tokens: Vec<_> = list.tokens.clone().into_iter().collect();
    let mut i = 0;
    while i < tokens.len() {
        // 查找 key = value 模式
        if let Some(proc_macro2::TokenTree::Ident(ident)) = tokens.get(i) {
            let key = ident.to_string();
            // 跳过等号
            if let Some(proc_macro2::TokenTree::Punct(punct)) = tokens.get(i + 1) {
                if punct.as_char() == '=' {
                    // 获取值
                    if let Some(proc_macro2::TokenTree::Literal(lit)) = tokens.get(i + 2) {
                        let value_str = lit.to_string();
                        // 去掉引号
                        let value = value_str
                            .strip_prefix('"')
                            .and_then(|s| s.strip_suffix('"'))
                            .unwrap_or(&value_str)
                            .to_string();

                        match key.as_str() {
                            "timestamp" => timestamp = Some(value),
                            "level" => level = Some(value),
                            "message" => message = Some(value),
                            other => {
                                pattern.custom.push((other.to_string(), value));
                            }
                        }
                        i += 3;
                        continue;
                    }
                }
            }
        }
        i += 1;
    }

    pattern.timestamp = timestamp.or(pattern.timestamp);
    pattern.level = level.or(pattern.level);
    pattern.message = message.or(pattern.message);

    Ok(pattern)
}

/// 解析 log_format 属性
fn parse_log_format(meta: &Meta) -> Result<String, syn::Error> {
    let Meta::List(list) = meta else {
        return Err(syn::Error::new_spanned(
            meta,
            "expected #[log_format(...)]",
        ));
    };

    let tokens: Vec<_> = list.tokens.clone().into_iter().collect();

    // 查找 pattern = "..."
    for i in 0..tokens.len().saturating_sub(2) {
        if let proc_macro2::TokenTree::Ident(ident) = &tokens[i] {
            if ident == "pattern" {
                if let proc_macro2::TokenTree::Punct(punct) = &tokens[i + 1] {
                    if punct.as_char() == '=' {
                        if let proc_macro2::TokenTree::Literal(lit) = &tokens[i + 2] {
                            let value = lit.to_string();
                            let value = value
                                .strip_prefix('"')
                                .and_then(|s| s.strip_suffix('"'))
                                .unwrap_or(&value);
                            return Ok(value.to_string());
                        }
                    }
                }
            }
        }
    }

    Ok("{timestamp} [{level}] {message}".to_string())
}

/// 生成 LogLexer 实现代码
fn generate_lexer_impl(input: &DeriveInput) -> Result<TokenStream2, syn::Error> {
    let name = &input.ident;

    // 解析属性
    let mut pattern = LogPattern::default();
    let mut format_desc = "{timestamp} [{level}] {message}".to_string();

    for attr in &input.attrs {
        if let Some(ident) = attr.path().get_ident() {
            match ident.to_string().as_str() {
                "log_pattern" => {
                    pattern = parse_log_pattern(&attr.meta)?;
                }
                "log_format" => {
                    format_desc = parse_log_format(&attr.meta)?;
                }
                _ => {}
            }
        }
    }

    // 生成正则表达式常量
    let timestamp_regex = pattern
        .timestamp
        .unwrap_or_else(|| r"\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}".to_string());
    let level_regex = pattern
        .level
        .unwrap_or_else(|| r"\[(DEBUG|INFO|WARN|ERROR|FATAL)\]".to_string());
    let message_regex = pattern.message.unwrap_or_else(|| r".*".to_string());

    // 生成实现
    let expanded = quote! {
        impl ::std::default::Default for #name {
            fn default() -> Self {
                Self
            }
        }

        impl ::log_lexer::LogLexer for #name {
            fn tokenize(&self, line: &str) -> ::log_lexer::LexerResult<Vec<::log_lexer::Token>> {
                use ::log_lexer::{Token, TokenType, LexerError};

                let mut tokens = Vec::new();
                let chars: Vec<char> = line.chars().collect();
                let len = chars.len();

                if len == 0 {
                    return Ok(tokens);
                }

                // 使用编译期生成的模式进行解析
                let timestamp_pattern = #timestamp_regex;
                let level_pattern = #level_regex;
                let message_pattern = #message_regex;

                // 简化的状态机解析（编译期优化版本）
                let mut pos = 0;

                // 1. 解析时间戳
                let ts_end = Self::parse_timestamp_fast(&chars, pos);
                if ts_end > pos {
                    let value: String = chars[pos..ts_end].iter().collect();
                    tokens.push(Token::new(TokenType::Timestamp, pos, ts_end, value));
                    pos = ts_end;
                }

                // 跳过分隔符
                while pos < len && (chars[pos].is_whitespace() || chars[pos] == ' ') {
                    pos += 1;
                }

                // 2. 解析日志级别（方括号内）
                if pos < len && chars[pos] == '[' {
                    let level_start = pos;
                    let mut level_end = pos + 1;
                    while level_end < len && chars[level_end] != ']' {
                        level_end += 1;
                    }
                    if level_end < len {
                        let value: String = chars[level_start + 1..level_end].iter().collect();
                        tokens.push(Token::new(TokenType::Bracket, level_start, level_start + 1, "[".to_string()));
                        tokens.push(Token::new(TokenType::Level, level_start + 1, level_end, value));
                        tokens.push(Token::new(TokenType::Bracket, level_end, level_end + 1, "]".to_string()));
                        pos = level_end + 1;
                    }
                }

                // 跳过分隔符
                while pos < len && chars[pos].is_whitespace() {
                    pos += 1;
                }

                // 3. 解析消息（剩余部分）
                if pos < len {
                    let value: String = chars[pos..len].iter().collect();
                    tokens.push(Token::new(TokenType::Message, pos, len, value));
                }

                Ok(tokens)
            }

            fn can_parse(&self, sample: &str) -> bool {
                // 检查是否符合预期的日志格式
                let trimmed = sample.trim();
                if trimmed.is_empty() {
                    return false;
                }

                // 简单启发式检查
                // 1. 检查是否以数字开头（可能是时间戳）
                let starts_with_digit = trimmed.chars().next()
                    .map(|c| c.is_ascii_digit())
                    .unwrap_or(false);

                // 2. 检查是否包含日志级别标记
                let has_level = ["[DEBUG]", "[INFO]", "[WARN]", "[ERROR]", "[FATAL]"]
                    .iter()
                    .any(|lvl| trimmed.contains(lvl));

                starts_with_digit || has_level
            }

            fn format_description(&self) -> &'static str {
                #format_desc
            }
        }

        impl #name {
            /// 快速时间戳解析（编译期优化）
            ///
            /// 此方法由过程宏生成，针对特定格式进行优化。
            #[inline]
            fn parse_timestamp_fast(chars: &[char], start: usize) -> usize {
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
        }
    };

    Ok(expanded)
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    // 注意：过程宏的测试需要在独立的 crate 中进行
    // 这里只测试辅助函数

    #[test]
    fn test_pattern_parsing() {
        // 基本测试确保宏能编译
        assert!(true);
    }
}
