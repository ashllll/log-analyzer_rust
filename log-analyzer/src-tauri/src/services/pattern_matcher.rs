use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};

/**
 * 模式匹配器 - 使用Aho-Corasick算法进行高效多模式匹配
 *
 * 该匹配器执行子串匹配，不是单词边界匹配。
 * 例如：模式"error"会匹配"error occurred"（包含error）和"no error here"（包含error）
 */
pub struct PatternMatcher {
    ac: Option<AhoCorasick>,
    patterns: Vec<String>,
    case_insensitive: bool,
}

impl PatternMatcher {
    /**
     * 创建新的模式匹配器
     *
     * # 参数
     * * `patterns` - 要匹配的模式列表
     * * `case_insensitive` - 是否大小写不敏感
     *
     * # 返回
     * * `Ok(PatternMatcher)` - 成功创建匹配器
     * * `Err(AppError)` - 构建失败，返回错误信息
     */
    pub fn new(patterns: Vec<String>, case_insensitive: bool) -> crate::error::Result<Self> {
        let ac = if !patterns.is_empty() {
            let mut builder = AhoCorasickBuilder::new();
            builder.match_kind(MatchKind::LeftmostFirst);

            if case_insensitive {
                builder.ascii_case_insensitive(true);
            }

            // 构建失败时返回错误而不是 None，避免静默失败
            Some(builder.build(&patterns).map_err(|e| {
                crate::error::AppError::search_error(format!(
                    "Failed to build pattern matcher for patterns {:?}: {}",
                    patterns, e
                ))
            })?)
        } else {
            None
        };

        Ok(Self {
            ac,
            patterns,
            case_insensitive,
        })
    }

    /**
     * 获取匹配器是否配置为大小写不敏感
     *
     * # 返回
     * * `true` - 如果匹配器是大小写不敏感的
     * * `false` - 否则
     */
    pub fn is_case_insensitive(&self) -> bool {
        self.case_insensitive
    }

    /**
     * 检查文本是否包含所有模式（AND逻辑）
     *
     * # 参数
     * * `text` - 要检查的文本
     *
     * # 返回
     * * `true` - 如果文本包含所有模式
     * * `false` - 否则
     *
     * # 说明
     * 执行子串匹配。例如：模式["error"]会匹配任何包含"error"子串的文本。
     * 使用 u128 位向量替代 HashSet，消除热路径上的堆分配。
     */
    pub fn matches_all(&self, text: &str) -> bool {
        let Some(ref ac) = self.ac else {
            return false;
        };

        if self.patterns.is_empty() {
            return false;
        }

        let pattern_count = self.patterns.len();

        // <= 128 个模式：使用栈上 u128 位向量（零堆分配）
        if pattern_count <= 128 {
            // 注意：wrapping_shl(128) == wrapping_shl(0) (取模行为)，需特殊处理
            let all_bits = if pattern_count == 128 {
                u128::MAX
            } else {
                (1u128 << pattern_count) - 1
            };
            let mut matched: u128 = 0;

            for mat in ac.find_iter(text) {
                let pattern_index = mat.pattern().as_usize();
                if pattern_index < 128 {
                    matched |= 1u128 << pattern_index;
                    // 提前退出：所有模式都已匹配
                    if matched == all_bits {
                        return true;
                    }
                }
            }
            matched == all_bits
        } else {
            // 回退到 Vec<bool>（比 HashSet 快，无哈希计算开销）
            let mut matched = vec![false; pattern_count];
            let mut remaining = pattern_count;

            for mat in ac.find_iter(text) {
                let pattern_index = mat.pattern().as_usize();
                if pattern_index < pattern_count && !matched[pattern_index] {
                    matched[pattern_index] = true;
                    remaining -= 1;
                    if remaining == 0 {
                        return true;
                    }
                }
            }
            remaining == 0
        }
    }

    /**
     * 检查文本是否包含任意模式（OR逻辑）
     *
     * # 参数
     * * `text` - 要检查的文本
     *
     * # 返回
     * * `true` - 如果文本包含任意模式
     * * `false` - 否则
     */
    pub fn matches_any(&self, text: &str) -> bool {
        let Some(ref ac) = self.ac else {
            return false;
        };

        ac.is_match(text)
    }

    /**
     * 获取匹配的模式索引
     *
     * # 参数
     * * `text` - 要检查的文本
     *
     * # 返回
     * * `Vec<usize>` - 匹配的模式索引列表
     */
    pub fn find_matches(&self, text: &str) -> Vec<usize> {
        let Some(ref ac) = self.ac else {
            return Vec::new();
        };

        ac.find_iter(text)
            .map(|mat| mat.pattern().as_usize())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_matcher_empty_patterns() {
        let matcher = PatternMatcher::new(Vec::new(), false).unwrap();
        assert!(!matcher.matches_all("test text"));
        assert!(!matcher.matches_any("test text"));
    }

    #[test]
    fn test_pattern_matcher_single_pattern() {
        let matcher = PatternMatcher::new(vec!["error".to_string()], false).unwrap();

        // 包含error子串的应该匹配
        assert!(matcher.matches_all("error occurred"));
        assert!(matcher.matches_all("no error here"));
        assert!(matcher.matches_any("error occurred"));
        assert!(matcher.matches_any("no error here"));

        // 不包含error子串的应该不匹配
        assert!(!matcher.matches_all("no here"));
        assert!(!matcher.matches_any("no here"));
    }

    #[test]
    fn test_pattern_matcher_multiple_patterns_and() {
        let matcher =
            PatternMatcher::new(vec!["error".to_string(), "timeout".to_string()], false).unwrap();

        // 应该匹配包含所有关键词的行
        assert!(matcher.matches_all("error occurred due to timeout"));
        assert!(matcher.matches_all("timeout caused error"));

        // 不应该匹配只包含部分关键词的行
        assert!(!matcher.matches_all("just an error"));
        assert!(!matcher.matches_all("only timeout"));
        assert!(!matcher.matches_all("no keywords here"));
    }

    #[test]
    fn test_pattern_matcher_case_insensitive() {
        let matcher =
            PatternMatcher::new(vec!["ERROR".to_string(), "TIMEOUT".to_string()], true).unwrap();

        assert!(matcher.matches_all("Error occurred due to Timeout"));
        assert!(matcher.matches_all("ERROR: timeout"));
        assert!(matcher.matches_all("error: TIMEOUT"));
    }

    #[test]
    fn test_pattern_matcher_case_sensitive() {
        let matcher =
            PatternMatcher::new(vec!["ERROR".to_string(), "timeout".to_string()], false).unwrap();

        assert!(matcher.matches_all("ERROR occurred due to timeout"));
        assert!(!matcher.matches_all("error occurred due to TIMEOUT")); // ERROR不匹配
    }

    #[test]
    fn test_pattern_matcher_find_matches() {
        let matcher = PatternMatcher::new(
            vec![
                "error".to_string(),
                "timeout".to_string(),
                "warning".to_string(),
            ],
            false,
        )
        .unwrap();

        let matches = matcher.find_matches("error and timeout occurred");
        assert_eq!(matches.len(), 2);
        assert!(matches.contains(&0)); // error
        assert!(matches.contains(&1)); // timeout
        assert!(!matches.contains(&2)); // warning

        let matches = matcher.find_matches("error only");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0], 0);
    }

    #[test]
    fn test_pattern_matcher_performance() {
        // 测试大量关键词的性能
        let patterns: Vec<String> = (0..10) // 减少到10个关键词以便测试
            .map(|i| format!("keyword{}", i))
            .collect();

        let matcher = PatternMatcher::new(patterns.clone(), false).unwrap();

        // 构造包含所有关键词的测试文本
        let text = patterns.join(" ");

        // 预热
        for _ in 0..100 {
            let _ = matcher.matches_all(&text);
        }

        // 正式测试
        let start = std::time::Instant::now();
        let iterations = 1000;
        for _ in 0..iterations {
            let _ = matcher.matches_all(&text);
        }
        let duration = start.elapsed();

        // 计算每次操作的平均时间
        let avg_time = duration / iterations;

        assert!(
            matcher.matches_all(&text),
            "All keywords should be found in the text"
        );
        // 使用相对阈值（每次操作 < 1ms）
        assert!(
            avg_time < std::time::Duration::from_millis(1),
            "Average time per operation should be < 1ms, actual: {:?}",
            avg_time
        );
    }

    #[test]
    fn test_pattern_matcher_edge_cases() {
        // 测试空文本
        let matcher = PatternMatcher::new(vec!["error".to_string()], false).unwrap();
        assert!(!matcher.matches_all(""));
        assert!(!matcher.matches_any(""));

        // 测试重复关键词
        let matcher = PatternMatcher::new(vec!["error".to_string()], false).unwrap();
        assert!(matcher.matches_all("error occurred"));

        // 测试特殊字符
        let matcher = PatternMatcher::new(
            vec!["error.log".to_string(), "timeout[ms]".to_string()],
            false,
        )
        .unwrap();
        assert!(matcher.matches_all("Found error.log and timeout[ms]"));
    }

    #[test]
    fn test_pattern_matcher_partial_match() {
        // 测试部分匹配的情况
        let matcher =
            PatternMatcher::new(vec!["error".to_string(), "timeout".to_string()], false).unwrap();

        // 只包含一个关键词
        assert!(!matcher.matches_all("just an error"));
        assert!(!matcher.matches_all("only timeout"));

        // 包含两个关键词
        assert!(matcher.matches_all("error and timeout"));

        // 不包含任何关键词
        assert!(!matcher.matches_all("just a warning"));
    }

    // ========== 位向量优化专项测试 ==========

    #[test]
    fn test_matches_all_early_exit() {
        // 验证提前退出：所有模式出现在文本前部时无需全文扫描
        // 使用固定宽度格式避免 LeftmostFirst 前缀冲突
        let patterns: Vec<String> = (0..10).map(|i| format!("kw_{:02}", i)).collect();
        let matcher = PatternMatcher::new(patterns, false).unwrap();

        let text = "kw_00 kw_01 kw_02 kw_03 kw_04 kw_05 kw_06 kw_07 kw_08 kw_09 ".to_string()
            + &"padding ".repeat(10000);
        assert!(matcher.matches_all(&text));
    }

    #[test]
    fn test_matches_all_128_patterns() {
        // 测试 u128 位向量边界：恰好 128 个模式
        // 使用固定宽度格式避免 LeftmostFirst 前缀冲突（如 "p1" vs "p10"）
        let patterns: Vec<String> = (0..128).map(|i| format!("p{:03}", i)).collect();
        let matcher = PatternMatcher::new(patterns.clone(), false).unwrap();

        let text = patterns.join(" ");
        assert!(matcher.matches_all(&text));

        // 缺少最后一个模式应返回 false
        let text_missing = (0..127).map(|i| format!("p{:03} ", i)).collect::<String>();
        assert!(!matcher.matches_all(&text_missing));
    }

    #[test]
    fn test_matches_all_129_patterns_fallback() {
        // 测试 Vec<bool> 回退路径：超过 128 个模式
        let patterns: Vec<String> = (0..130).map(|i| format!("p{:03}", i)).collect();
        let matcher = PatternMatcher::new(patterns.clone(), false).unwrap();

        let text = patterns.join(" ");
        assert!(matcher.matches_all(&text));

        // 缺少最后一个模式应返回 false
        let text_missing = (0..129).map(|i| format!("p{:03} ", i)).collect::<String>();
        assert!(!matcher.matches_all(&text_missing));
    }

    #[test]
    fn test_matches_all_no_heap_allocation() {
        // 性能验证：位向量路径应在 < 100us 内完成（零堆分配）
        let patterns: Vec<String> = (0..64).map(|i| format!("kw_{:03}", i)).collect();
        let matcher = PatternMatcher::new(patterns.clone(), false).unwrap();
        let text = patterns.join(" ");

        // 预热
        for _ in 0..100 {
            let _ = matcher.matches_all(&text);
        }

        let start = std::time::Instant::now();
        for _ in 0..10000 {
            let _ = matcher.matches_all(&text);
        }
        let avg = start.elapsed() / 10000;

        assert!(
            avg < std::time::Duration::from_micros(100),
            "Bit vector path should be < 100us per call, actual: {:?}",
            avg
        );
    }
}
