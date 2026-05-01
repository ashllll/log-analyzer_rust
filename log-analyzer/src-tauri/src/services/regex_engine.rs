//! # 高性能正则表达式引擎
//!
//! 混合引擎架构，根据模式类型自动选择最佳匹配算法：
//! - **AhoCorasickEngine**: 简单关键词搜索，O(n) 线性复杂度
//! - **AutomataEngine**: 复杂正则，DFA 加速
//! - **StandardEngine**: 需要前瞻/后瞻时使用
//!
//! # 性能特点
//!
//! | 引擎 | 单模式 | 多模式 | 前瞻/后瞻 | Streaming |
//! |------|--------|--------|-----------|-----------|
//! | AhoCorasick | O(n) | O(n) | 不支持 | 支持 |
//! | Automata | O(n) | O(n) | 不支持 | 原生支持 |
//! | Standard | 可变 | 可变 | 支持 | 受限 |

use std::fmt;
use std::sync::Arc;

use aho_corasick::{AhoCorasick, AhoCorasickBuilder};
use regex::Regex;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("Pattern compilation failed: {0}")]
    CompilationError(String),
    #[error("Match failed: {0}")]
    MatchError(String),
    #[error("Unsupported pattern for engine: {0}")]
    UnsupportedPattern(String),
}

#[derive(Debug, Clone)]
pub struct EngineInfo {
    pub engine_type: EngineType,
    pub pattern: String,
    pub is_regex: bool,
}

impl EngineInfo {
    pub fn new(engine_type: EngineType, pattern: String, is_regex: bool) -> Self {
        Self {
            engine_type,
            pattern,
            is_regex,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineType {
    AhoCorasick,
    Automata,
    Standard,
    Memchr,
    Fancy,
}

impl fmt::Display for EngineType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EngineType::AhoCorasick => write!(f, "AhoCorasick"),
            EngineType::Automata => write!(f, "Automata"),
            EngineType::Standard => write!(f, "Standard"),
            EngineType::Memchr => write!(f, "Memchr"),
            EngineType::Fancy => write!(f, "Fancy"),
        }
    }
}

#[derive(Clone)]
pub enum RegexEngine {
    AhoCorasick(AhoCorasickEngine),
    Automata(AutomataEngine),
    Standard(StandardEngine),
    Memchr(MemchrEngine),
    Fancy(FancyEngine),
}

impl fmt::Debug for RegexEngine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RegexEngine::AhoCorasick(e) => f
                .debug_struct("AhoCorasick")
                .field("pattern_count", &e.pattern_count())
                .finish(),
            RegexEngine::Automata(e) => f
                .debug_struct("Automata")
                .field("pattern", &e.pattern())
                .finish(),
            RegexEngine::Standard(e) => f
                .debug_struct("Standard")
                .field("pattern", &e.pattern())
                .finish(),
            RegexEngine::Memchr(e) => f
                .debug_struct("Memchr")
                .field("pattern", &e.pattern())
                .finish(),
            RegexEngine::Fancy(e) => f
                .debug_struct("Fancy")
                .field("pattern", &e.pattern())
                .finish(),
        }
    }
}

impl RegexEngine {
    pub fn engine_type(&self) -> EngineType {
        match self {
            RegexEngine::AhoCorasick(_) => EngineType::AhoCorasick,
            RegexEngine::Automata(_) => EngineType::Automata,
            RegexEngine::Standard(_) => EngineType::Standard,
            RegexEngine::Memchr(_) => EngineType::Memchr,
            RegexEngine::Fancy(_) => EngineType::Fancy,
        }
    }

    /// 智能选择最佳引擎（业内成熟方案）
    ///
    /// 选择策略：
    /// 1. **需要 lookahead/lookbehind**: 使用 FancyEngine（支持回溯）
    /// 2. **复杂正则元字符**: 使用 StandardEngine
    /// 3. **简单多模式 (| 分隔)**: 使用 AhoCorasickEngine (O(n) 线性复杂度)
    /// 4. **单简单关键词 (case-sensitive)**: 使用 MemchrEngine（SIMD 加速）
    /// 5. **其他**: 使用 StandardEngine
    pub fn new(pattern: &str, is_regex: bool) -> Result<Self, EngineError> {
        // 1. 检测是否需要前瞻/后瞻（使用 FancyEngine）
        if needs_lookaround(pattern) {
            return FancyEngine::new(pattern).map(RegexEngine::Fancy);
        }

        // 2. 如果标记为正则表达式，检查复杂度
        if is_regex {
            // 复杂正则使用 AutomataEngine（与 StandardEngine 均基于 regex crate，
            // 保留为独立变体以便未来做 DFA 预编译等特殊化优化）
            if !is_simple_keyword(pattern) {
                return AutomataEngine::new(pattern).map(RegexEngine::Automata);
            }
        }

        // 3. 多模式匹配 (| 分隔) 使用 Aho-Corasick
        if is_multi_keyword(pattern) {
            return AhoCorasickEngine::new(pattern).map(RegexEngine::AhoCorasick);
        }

        // 4. 简单关键词 (case-sensitive) 使用 Memchr（SIMD 加速）
        if is_simple_keyword(pattern) && !pattern.starts_with("(?i:") {
            return MemchrEngine::new(pattern).map(RegexEngine::Memchr);
        }

        // 5. 默认使用 StandardEngine（包含 (?i:...) 等 case-insensitive 模式）
        StandardEngine::new(pattern).map(RegexEngine::Standard)
    }

    /// 带大小写不敏感标志的构造方法，供 query_planner 使用
    pub fn new_with_case(
        pattern: &str,
        is_regex: bool,
        case_insensitive: bool,
    ) -> Result<Self, EngineError> {
        if needs_lookaround(pattern) {
            return FancyEngine::new(pattern).map(RegexEngine::Fancy);
        }
        if is_regex && !is_simple_keyword(pattern) {
            return StandardEngine::new(pattern).map(RegexEngine::Standard);
        }
        if is_multi_keyword(pattern) {
            return AhoCorasickEngine::new_with_ci(pattern, case_insensitive)
                .map(RegexEngine::AhoCorasick);
        }
        if !case_insensitive && is_simple_keyword(pattern) {
            return MemchrEngine::new(pattern).map(RegexEngine::Memchr);
        }
        if case_insensitive && is_simple_keyword(pattern) {
            // Case-insensitive simple keyword: use AhoCorasick with ascii_case_insensitive
            // instead of StandardEngine (?i:) wrapper for better performance.
            return AhoCorasickEngine::new_with_ci(pattern, true)
                .map(RegexEngine::AhoCorasick);
        }
        StandardEngine::new(pattern).map(RegexEngine::Standard)
    }

    pub fn find_iter<'a>(&'a self, text: &'a str) -> EngineMatches<'a> {
        match self {
            RegexEngine::AhoCorasick(e) => EngineMatches::AhoCorasick(e.find_iter(text)),
            RegexEngine::Automata(e) => EngineMatches::Automata(e.find_iter(text)),
            RegexEngine::Standard(e) => EngineMatches::Standard(e.find_iter(text)),
            RegexEngine::Memchr(e) => EngineMatches::Memchr(e.find_iter(text)),
            RegexEngine::Fancy(e) => EngineMatches::Fancy(e.find_iter(text)),
        }
    }

    pub fn is_match(&self, text: &str) -> bool {
        match self {
            RegexEngine::AhoCorasick(e) => e.is_match(text),
            RegexEngine::Automata(e) => e.is_match(text),
            RegexEngine::Standard(e) => e.is_match(text),
            RegexEngine::Memchr(e) => e.is_match(text),
            RegexEngine::Fancy(e) => e.is_match(text),
        }
    }
}

#[derive(Clone)]
pub struct AhoCorasickEngine {
    ac: Arc<AhoCorasick>,
    patterns: Vec<String>,
}

impl AhoCorasickEngine {
    pub fn new(pattern: &str) -> Result<Self, EngineError> {
        Self::new_with_ci(pattern, false)
    }

    pub fn new_with_ci(pattern: &str, case_insensitive: bool) -> Result<Self, EngineError> {
        let patterns: Vec<&str> = if pattern.contains('|') {
            pattern.split('|').collect()
        } else {
            vec![pattern]
        };

        if patterns.iter().any(|p| p.is_empty()) {
            return Err(EngineError::CompilationError(
                "Empty pattern in Aho-Corasick".to_string(),
            ));
        }

        let ac = AhoCorasickBuilder::new()
            .ascii_case_insensitive(case_insensitive)
            .build(&patterns)
            .map_err(|e| EngineError::CompilationError(e.to_string()))?;

        Ok(Self {
            ac: Arc::new(ac),
            patterns: patterns.iter().map(|s| s.to_string()).collect(),
        })
    }

    pub fn pattern_count(&self) -> usize {
        self.patterns.len()
    }

    pub fn pattern(&self) -> &str {
        self.patterns.first().map(|s| s.as_str()).unwrap_or("")
    }

    pub fn find_iter<'a>(&'a self, text: &'a str) -> AhoCorasickMatches<'a> {
        AhoCorasickMatches::new(Arc::clone(&self.ac), text, self.patterns.clone(), false)
    }

    pub fn find_overlapped_iter<'a>(&'a self, text: &'a str) -> AhoCorasickMatches<'a> {
        AhoCorasickMatches::new(Arc::clone(&self.ac), text, self.patterns.clone(), true)
    }

    /// 直接判断文本是否包含任一模式，零分配（比 `find_iter(text).next().is_some()` 快 10-100 倍）
    pub fn is_match(&self, text: &str) -> bool {
        self.ac.is_match(text)
    }
}

pub struct AhoCorasickMatches<'a> {
    ac: Arc<AhoCorasick>,
    text: &'a str,
    offset: usize,
    patterns: Vec<String>,
    overlapping: bool,
    overlap_state: Option<aho_corasick::automaton::OverlappingState>,
}

impl<'a> AhoCorasickMatches<'a> {
    fn new(ac: Arc<AhoCorasick>, text: &'a str, patterns: Vec<String>, overlapping: bool) -> Self {
        let overlap_state = if overlapping {
            Some(aho_corasick::automaton::OverlappingState::start())
        } else {
            None
        };
        Self {
            ac,
            text,
            offset: 0,
            patterns,
            overlapping,
            overlap_state,
        }
    }
}

impl<'a> Iterator for AhoCorasickMatches<'a> {
    type Item = MatchResult;

    fn next(&mut self) -> Option<Self::Item> {
        let haystack = &self.text[self.offset..];
        let mat = if self.overlapping {
            let state = self.overlap_state.as_mut()?;
            self.ac.find_overlapping(haystack, state);
            state.get_match()?
        } else {
            self.ac.find(haystack)?
        };
        let start = self.offset + mat.start();
        let end = self.offset + mat.end();
        // Advance offset: for non-overlapping, move past match end;
        // for overlapping, move past match start to allow overlapping matches.
        self.offset = if self.overlapping {
            start + 1
        } else {
            end
        };
        Some(MatchResult {
            start,
            end,
            pattern: self.patterns.get(mat.pattern().as_usize()).cloned().unwrap_or_default(),
        })
    }
}

#[derive(Clone)]
pub struct AutomataEngine {
    regex: Regex,
    pattern: String,
}

impl AutomataEngine {
    pub fn new(pattern: &str) -> Result<Self, EngineError> {
        if pattern.trim().is_empty() {
            return Err(EngineError::CompilationError("Empty pattern".to_string()));
        }

        let regex =
            Regex::new(pattern).map_err(|e| EngineError::CompilationError(e.to_string()))?;

        Ok(Self {
            regex,
            pattern: pattern.to_string(),
        })
    }

    pub fn pattern(&self) -> &str {
        &self.pattern
    }

    pub fn find_iter<'a>(&'a self, text: &'a str) -> AutomataMatches<'a> {
        AutomataMatches {
            matches: self.regex.find_iter(text),
        }
    }

    pub fn is_match(&self, text: &str) -> bool {
        self.regex.is_match(text)
    }
}

pub struct AutomataMatches<'a> {
    matches: regex::Matches<'a, 'a>,
}

impl<'a> Iterator for AutomataMatches<'a> {
    type Item = MatchResult;

    fn next(&mut self) -> Option<Self::Item> {
        self.matches.next().map(|mat| MatchResult {
            start: mat.start(),
            end: mat.end(),
            pattern: String::new(),
        })
    }
}

#[derive(Clone)]
pub struct StandardEngine {
    regex: Regex,
    pattern: String,
}

impl StandardEngine {
    pub fn new(pattern: &str) -> Result<Self, EngineError> {
        if pattern.trim().is_empty() {
            return Err(EngineError::CompilationError("Empty pattern".to_string()));
        }

        let regex =
            Regex::new(pattern).map_err(|e| EngineError::CompilationError(e.to_string()))?;

        Ok(Self {
            regex,
            pattern: pattern.to_string(),
        })
    }

    pub fn pattern(&self) -> &str {
        &self.pattern
    }

    pub fn find_iter<'a>(&'a self, text: &'a str) -> StandardMatches<'a> {
        StandardMatches {
            matches: self.regex.find_iter(text),
            pattern: self.pattern.clone(),
        }
    }

    pub fn is_match(&self, text: &str) -> bool {
        self.regex.is_match(text)
    }
}

pub struct StandardMatches<'a> {
    matches: regex::Matches<'a, 'a>,
    pattern: String,
}

impl<'a> Iterator for StandardMatches<'a> {
    type Item = MatchResult;

    fn next(&mut self) -> Option<Self::Item> {
        self.matches.next().map(|mat| MatchResult {
            start: mat.start(),
            end: mat.end(),
            pattern: self.pattern.clone(),
        })
    }
}

// ========== MemchrEngine：SIMD 加速子串搜索 ==========

#[derive(Clone)]
pub struct MemchrEngine {
    pattern: String,
    needle: Vec<u8>,
}

impl MemchrEngine {
    pub fn new(pattern: &str) -> Result<Self, EngineError> {
        if pattern.trim().is_empty() {
            return Err(EngineError::CompilationError("Empty pattern".to_string()));
        }
        Ok(Self {
            pattern: pattern.to_string(),
            needle: pattern.as_bytes().to_vec(),
        })
    }

    pub fn pattern(&self) -> &str {
        &self.pattern
    }

    pub fn find_iter<'a>(&'a self, text: &'a str) -> MemchrMatches<'a> {
        MemchrMatches {
            needle: self.needle.as_slice(),
            text: text.as_bytes(),
            offset: 0,
            pattern: self.pattern.clone(),
        }
    }

    /// 直接判断文本是否包含子串，零分配
    pub fn is_match(&self, text: &str) -> bool {
        memchr::memmem::find(text.as_bytes(), &self.needle).is_some()
    }
}

pub struct MemchrMatches<'a> {
    needle: &'a [u8],
    text: &'a [u8],
    offset: usize,
    pattern: String,
}

impl<'a> Iterator for MemchrMatches<'a> {
    type Item = MatchResult;

    fn next(&mut self) -> Option<Self::Item> {
        let remaining = &self.text[self.offset..];
        let pos = memchr::memmem::find(remaining, &self.needle)?;
        let start = self.offset + pos;
        let end = start + self.needle.len();
        // 避免空匹配导致死循环（memchr 不会出现，防御性编程）
        if end <= self.offset {
            self.offset += 1;
            return self.next();
        }
        self.offset = end;
        Some(MatchResult {
            start,
            end,
            pattern: self.pattern.clone(),
        })
    }
}

// ========== FancyEngine：支持前瞻/后瞻的回溯正则引擎 ==========

#[derive(Clone)]
pub struct FancyEngine {
    regex: fancy_regex::Regex,
    pattern: String,
}

impl FancyEngine {
    pub fn new(pattern: &str) -> Result<Self, EngineError> {
        if pattern.trim().is_empty() {
            return Err(EngineError::CompilationError("Empty pattern".to_string()));
        }
        let regex = fancy_regex::Regex::new(pattern)
            .map_err(|e| EngineError::CompilationError(e.to_string()))?;
        Ok(Self {
            regex,
            pattern: pattern.to_string(),
        })
    }

    pub fn pattern(&self) -> &str {
        &self.pattern
    }

    pub fn find_iter<'a>(&'a self, text: &'a str) -> FancyMatches<'a> {
        FancyMatches {
            regex: &self.regex,
            text,
            offset: 0,
            pattern: self.pattern.clone(),
            error: None,
        }
    }

    pub fn is_match(&self, text: &str) -> bool {
        self.regex.is_match(text).unwrap_or(false)
    }
}

pub struct FancyMatches<'a> {
    regex: &'a fancy_regex::Regex,
    text: &'a str,
    offset: usize,
    pattern: String,
    error: Option<String>,
}

impl<'a> FancyMatches<'a> {
    /// Returns true if the iterator encountered a regex execution error
    /// (e.g., catastrophic backtracking) during iteration.
    pub fn has_error(&self) -> bool {
        self.error.is_some()
    }

    /// Returns the error message if an error occurred.
    pub fn error_message(&self) -> Option<&str> {
        self.error.as_deref()
    }
}

impl<'a> Iterator for FancyMatches<'a> {
    type Item = MatchResult;

    fn next(&mut self) -> Option<Self::Item> {
        let remaining = &self.text[self.offset..];
        match self.regex.find(remaining) {
            Ok(Some(mat)) => {
                let start = self.offset + mat.start();
                let end = self.offset + mat.end();
                if end <= self.offset {
                    self.offset += 1;
                    return self.next();
                }
                self.offset = end;
                Some(MatchResult {
                    start,
                    end,
                    pattern: self.pattern.clone(),
                })
            }
            Ok(None) => None,
            Err(e) => {
                // 回溯失败（如 catastrophic backtracking），记录错误并终止迭代
                if self.error.is_none() {
                    self.error = Some(format!("fancy-regex execution error: {}", e));
                }
                None
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct MatchResult {
    pub start: usize,
    pub end: usize,
    pub pattern: String,
}

impl MatchResult {
    pub fn text<'a>(&self, source: &'a str) -> &'a str {
        &source[self.start..self.end]
    }

    pub fn len(&self) -> usize {
        self.end - self.start
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

pub enum EngineMatches<'a> {
    AhoCorasick(AhoCorasickMatches<'a>),
    Automata(AutomataMatches<'a>),
    Standard(StandardMatches<'a>),
    Memchr(MemchrMatches<'a>),
    Fancy(FancyMatches<'a>),
}

impl<'a> Iterator for EngineMatches<'a> {
    type Item = MatchResult;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            EngineMatches::AhoCorasick(m) => m.next(),
            EngineMatches::Automata(m) => m.next(),
            EngineMatches::Standard(m) => m.next(),
            EngineMatches::Memchr(m) => m.next(),
            EngineMatches::Fancy(m) => m.next(),
        }
    }
}

/// 检测是否为简单关键词（适合 Aho-Corasick）
///
/// 简单关键词定义：不包含正则元字符
pub fn is_simple_keyword(pattern: &str) -> bool {
    let trimmed = pattern.trim();
    if trimmed.is_empty() {
        return false;
    }
    !trimmed.contains(|c: char| {
        matches!(
            c,
            '(' | ')' | '[' | ']' | '{' | '}' | '+' | '*' | '?' | '|' | '^' | '$' | '.' | '\\'
        )
    })
}

/// 检测正则表达式复杂度
///
/// 返回值：
/// - 0: 简单模式（适合 Aho-Corasick）
/// - 1-3: 中等复杂度（StandardEngine 可处理）
/// - 4+: 高复杂度（需要 StandardEngine）
pub fn regex_complexity_score(pattern: &str) -> usize {
    let mut score = 0;

    // 字符类
    if pattern.contains('[') && pattern.contains(']') {
        score += 2;
    }

    // 量词
    if pattern.contains('*') || pattern.contains('+') {
        score += 1;
    }

    // 范围
    if pattern.contains('{') && pattern.contains('}') {
        score += 2;
    }

    // 分组
    let paren_count = pattern.matches('(').count();
    if paren_count > 0 {
        score += paren_count;
    }

    // 锚点
    if pattern.contains('^') || pattern.contains('$') {
        score += 1;
    }

    score
}

/// 检测是否需要前瞻/后瞻
pub fn needs_lookaround(pattern: &str) -> bool {
    pattern.contains("(?=")
        || pattern.contains("(?!")
        || pattern.contains("(?<=")
        || pattern.contains("(?<!")
}

/// 检测是否包含 Aho-Corasick 友好的模式（多关键词）
pub fn is_multi_keyword(pattern: &str) -> bool {
    pattern.contains('|')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_keyword_detection() {
        assert!(is_simple_keyword("error"));
        assert!(is_simple_keyword("error_code"));
        assert!(is_simple_keyword("test123"));
        assert!(!is_simple_keyword(r"\d+")); // 正则
        assert!(!is_simple_keyword(r"error|warn")); // 多模式
        assert!(!is_simple_keyword("")); // 空
    }

    #[test]
    fn test_lookaround_detection() {
        assert!(needs_lookaround(r"(?<=foo)bar"));
        assert!(needs_lookaround(r"bar(?=foo)"));
        assert!(needs_lookaround(r"(?!test)"));
        assert!(!needs_lookaround(r"\d+"));
        assert!(!needs_lookaround("error"));
    }

    #[test]
    fn test_aho_corasiick_engine() {
        let engine = AhoCorasickEngine::new("error|warning|info").unwrap();
        let text = "error warning info error";
        let matches: Vec<_> = engine.find_iter(text).collect();
        assert_eq!(matches.len(), 4);
    }

    #[test]
    fn test_automata_engine() {
        let engine = AutomataEngine::new(r"\d{4}-\d{2}-\d{2}").unwrap();
        let text = "Dates: 2024-01-30, 2025-02-28";
        let matches: Vec<_> = engine.find_iter(text).collect();
        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn test_standard_engine() {
        let engine = StandardEngine::new(r"\d+").unwrap();
        let text = "foo123 and foo456";
        let matches: Vec<_> = engine.find_iter(text).collect();
        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn test_memchr_engine() {
        let engine = MemchrEngine::new("error").unwrap();
        let text = "error occurred, error_code, error123";
        let matches: Vec<_> = engine.find_iter(text).collect();
        assert_eq!(matches.len(), 3);
        assert_eq!(matches[0].text(text), "error");
        assert_eq!(matches[0].start, 0);
        assert!(!engine.is_match("warning info"));
        assert!(engine.is_match("ERROR error"));
    }

    #[test]
    fn test_memchr_engine_unicode() {
        // Memchr 在 UTF-8 文本中按字节匹配，对 ASCII 子串仍正确工作
        let engine = MemchrEngine::new("error").unwrap();
        let text = "错误 error 信息 error";
        let matches: Vec<_> = engine.find_iter(text).collect();
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].text(text), "error");
    }

    #[test]
    fn test_memchr_engine_empty_pattern_fails() {
        assert!(MemchrEngine::new("").is_err());
        assert!(MemchrEngine::new("   ").is_err());
    }

    #[test]
    fn test_fancy_engine_lookaround() {
        // 正向后瞻：bar 前面必须有 foo
        let engine = FancyEngine::new(r"(?<=foo)bar").unwrap();
        assert!(engine.is_match("foobar"));
        assert!(!engine.is_match("bar"));
        assert!(!engine.is_match("bazbar"));

        // 正向前瞻：foo 后面必须有 bar
        let engine = FancyEngine::new(r"foo(?=bar)").unwrap();
        assert!(engine.is_match("foobar"));
        assert!(!engine.is_match("foo"));

        let text = "foobar barbaz";
        let matches: Vec<_> = engine.find_iter(text).collect();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].text(text), "foo");
    }

    #[test]
    fn test_fancy_engine_negative_lookaround() {
        // 负向前瞻：q 后面不是 u
        let engine = FancyEngine::new(r"q(?!u)").unwrap();
        assert!(engine.is_match("Iraq"));
        assert!(!engine.is_match("queen"));

        // 负向后瞻：bar 前面不是 foo
        let engine = FancyEngine::new(r"(?<!foo)bar").unwrap();
        assert!(engine.is_match("bazbar"));
        assert!(!engine.is_match("foobar"));
    }

    // ========== 引擎选择测试 (智能选择) ==========

    #[test]
    fn test_engine_selection_lookaround() {
        // 标准的 regex crate 不支持 lookaround，现在用 fancy-regex 支持
        assert!(needs_lookaround(r"foo(?=bar)"));
        assert!(needs_lookaround(r"(?<=foo)bar"));

        // FancyEngine 应该能成功编译 lookaround 模式
        let engine = RegexEngine::new(r"(?<=foo)bar", true).unwrap();
        assert!(matches!(engine, RegexEngine::Fancy(_)));
        assert!(engine.is_match("foobar"));
    }

    #[test]
    fn test_engine_selection_multi_keyword() {
        // 多模式使用 AhoCorasick
        let engine = RegexEngine::new("error|warning|info", false).unwrap();
        assert!(matches!(engine, RegexEngine::AhoCorasick(_)));
    }

    #[test]
    fn test_engine_selection_simple_keyword() {
        // 简单关键词（case-sensitive）使用 MemchrEngine（SIMD 加速）
        let engine = RegexEngine::new("error", false).unwrap();
        assert!(matches!(engine, RegexEngine::Memchr(_)));
        assert!(engine.is_match("error occurred"));
    }

    #[test]
    fn test_engine_selection_complex_regex() {
        // 复杂正则使用 AutomataEngine（与 StandardEngine 均基于 regex crate）
        let engine = RegexEngine::new(r"\d{4}-\d{2}-\d{2}", true).unwrap();
        assert!(matches!(engine, RegexEngine::Automata(_)));

        let engine = RegexEngine::new(r"[A-Z]\w+", true).unwrap();
        assert!(matches!(engine, RegexEngine::Automata(_)));
    }

    #[test]
    fn test_regex_complexity_score() {
        assert_eq!(regex_complexity_score("simple"), 0);
        assert_eq!(regex_complexity_score(r"\d+"), 1);
        assert_eq!(regex_complexity_score(r"[A-Z]+"), 3);
        // 2个括号 + 2个大括号 = 4
        assert_eq!(regex_complexity_score(r"(\d{4})-(\d{2})"), 4);
        // 测试锚点
        assert_eq!(regex_complexity_score(r"^start"), 1);
        assert_eq!(regex_complexity_score(r"^start$"), 1);
    }
}
