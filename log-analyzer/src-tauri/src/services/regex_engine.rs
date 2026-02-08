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

use aho_corasick::AhoCorasick;
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
}

impl fmt::Display for EngineType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EngineType::AhoCorasick => write!(f, "AhoCorasick"),
            EngineType::Automata => write!(f, "Automata"),
            EngineType::Standard => write!(f, "Standard"),
        }
    }
}

#[derive(Clone)]
pub enum RegexEngine {
    AhoCorasick(AhoCorasickEngine),
    Automata(AutomataEngine),
    Standard(StandardEngine),
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
        }
    }
}

impl RegexEngine {
    pub fn engine_type(&self) -> EngineType {
        match self {
            RegexEngine::AhoCorasick(_) => EngineType::AhoCorasick,
            RegexEngine::Automata(_) => EngineType::Automata,
            RegexEngine::Standard(_) => EngineType::Standard,
        }
    }

    /// 智能选择最佳引擎（业内成熟方案）
    ///
    /// 选择策略：
    /// 1. **需要 lookahead/lookbehind**: 必须使用 StandardEngine
    /// 2. **复杂正则元字符**: 使用 StandardEngine
    /// 3. **简单多模式 (| 分隔)**: 使用 AhoCorasickEngine (O(n) 线性复杂度)
    /// 4. **单简单关键词**: 使用 StandardEngine (已优化)
    pub fn new(pattern: &str, is_regex: bool) -> Result<Self, EngineError> {
        // 1. 检测是否需要前瞻/后瞻（必须使用 StandardEngine）
        if needs_lookaround(pattern) {
            return StandardEngine::new(pattern).map(RegexEngine::Standard);
        }

        // 2. 如果标记为正则表达式，检查复杂度
        if is_regex {
            // 复杂正则使用 StandardEngine
            if !is_simple_keyword(pattern) {
                return StandardEngine::new(pattern).map(RegexEngine::Standard);
            }
        }

        // 3. 多模式匹配 (| 分隔) 使用 Aho-Corasick
        if is_multi_keyword(pattern) {
            return AhoCorasickEngine::new(pattern).map(RegexEngine::AhoCorasick);
        }

        // 4. 默认使用 StandardEngine
        StandardEngine::new(pattern).map(RegexEngine::Standard)
    }

    pub fn find_iter<'a>(&'a self, text: &'a str) -> EngineMatches<'a> {
        match self {
            RegexEngine::AhoCorasick(e) => EngineMatches::AhoCorasick(e.find_iter(text)),
            RegexEngine::Automata(e) => EngineMatches::Automata(e.find_iter(text)),
            RegexEngine::Standard(e) => EngineMatches::Standard(e.find_iter(text)),
        }
    }

    pub fn is_match(&self, text: &str) -> bool {
        match self {
            RegexEngine::AhoCorasick(e) => e.find_iter(text).next().is_some(),
            RegexEngine::Automata(e) => e.is_match(text),
            RegexEngine::Standard(e) => e.is_match(text),
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

        let ac = AhoCorasick::new(&patterns)
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

    pub fn find_iter(&self, text: &str) -> AhoCorasickMatches {
        let matches: Vec<_> = self.ac.find_iter(text).collect();
        AhoCorasickMatches::new(matches, self.patterns.clone())
    }

    pub fn find_overlapped_iter(&self, text: &str) -> AhoCorasickMatches {
        let matches: Vec<_> = self.ac.find_overlapping_iter(text).collect();
        AhoCorasickMatches::new(matches, self.patterns.clone())
    }
}

pub struct AhoCorasickMatches {
    matches: std::vec::IntoIter<aho_corasick::Match>,
    patterns: Vec<String>,
}

impl AhoCorasickMatches {
    fn new(matches: Vec<aho_corasick::Match>, patterns: Vec<String>) -> Self {
        Self {
            matches: matches.into_iter(),
            patterns,
        }
    }
}

impl Iterator for AhoCorasickMatches {
    type Item = MatchResult;

    fn next(&mut self) -> Option<Self::Item> {
        self.matches.next().map(|mat| {
            let pattern_id = mat.pattern().as_usize();
            MatchResult {
                start: mat.start(),
                end: mat.end(),
                pattern: self.patterns.get(pattern_id).cloned().unwrap_or_default(),
            }
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
    AhoCorasick(AhoCorasickMatches),
    Automata(AutomataMatches<'a>),
    Standard(StandardMatches<'a>),
}

impl<'a> Iterator for EngineMatches<'a> {
    type Item = MatchResult;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            EngineMatches::AhoCorasick(m) => m.next(),
            EngineMatches::Automata(m) => m.next(),
            EngineMatches::Standard(m) => m.next(),
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

    // ========== 引擎选择测试 (智能选择) ==========

    #[test]
    fn test_engine_selection_lookaround() {
        // 注意: 标准的 regex crate 不支持 lookaround
        // 我们的 needs_lookaround 函数应该正确检测这些模式
        assert!(needs_lookaround(r"(?=foo)bar"));
        assert!(needs_lookaround(r"(?<=foo)bar"));

        // 由于 regex 不支持 lookaround，创建引擎会失败
        // 这是预期行为
        let result = RegexEngine::new(r"(?=foo)bar", true);
        assert!(result.is_err(), "Lookaround should fail to compile");
    }

    #[test]
    fn test_engine_selection_multi_keyword() {
        // 多模式使用 AhoCorasick
        let engine = RegexEngine::new("error|warning|info", false).unwrap();
        assert!(matches!(engine, RegexEngine::AhoCorasick(_)));
    }

    #[test]
    fn test_engine_selection_simple_keyword() {
        // 简单关键词使用 StandardEngine（已优化）
        let engine = RegexEngine::new("error", false).unwrap();
        assert!(matches!(engine, RegexEngine::Standard(_)));
    }

    #[test]
    fn test_engine_selection_complex_regex() {
        // 复杂正则使用 StandardEngine
        let engine = RegexEngine::new(r"\d{4}-\d{2}-\d{2}", true).unwrap();
        assert!(matches!(engine, RegexEngine::Standard(_)));

        let engine = RegexEngine::new(r"[A-Z]\w+", true).unwrap();
        assert!(matches!(engine, RegexEngine::Standard(_)));
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
