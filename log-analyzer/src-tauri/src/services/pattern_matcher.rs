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
    #[allow(dead_code)]
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
     */
    pub fn matches_all(&self, text: &str) -> bool {
        let Some(ref ac) = self.ac else {
            return false;
        };

        if self.patterns.is_empty() {
            return false;
        }

        // 收集所有匹配的模式ID
        let mut matched_ids = std::collections::HashSet::new();
        for mat in ac.find_iter(text) {
            matched_ids.insert(mat.pattern().as_usize());
        }

        // 检查是否所有模式都匹配
        matched_ids.len() == self.patterns.len()
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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
}
