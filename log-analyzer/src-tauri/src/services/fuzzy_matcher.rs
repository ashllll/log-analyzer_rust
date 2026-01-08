use crate::services::metaphone::metaphone;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/**
 * Levenshtein 距离（编辑距离）
 *
 * 计算两个字符串之间的最小编辑操作数（插入、删除、替换）
 *
 * # 参数
 * * `s1` - 第一个字符串
 * * `s2` - 第二个字符串
 *
 * # 返回
 * 编辑距离（非负整数）
 *
 * # 复杂度
 * * 时间: O(n*m)，n 和 m 是字符串长度
 * * 空间: O(n*m)
 *
 * # 示例
 * ```
 * assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
 * assert_eq!(levenshtein_distance("ERROR", "ERRO"), 1);
 * ```
 */
#[allow(clippy::needless_range_loop)]
pub fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();
    let len1 = s1_chars.len();
    let len2 = s2_chars.len();

    // 创建动态规划表
    let mut dp = vec![vec![0; len2 + 1]; len1 + 1];

    // 初始化第一行和第一列
    for i in 0..=len1 {
        dp[i][0] = i;
    }
    for j in 0..=len2 {
        dp[0][j] = j;
    }

    // 填充动态规划表
    for (i, c1) in s1_chars.iter().enumerate() {
        for (j, c2) in s2_chars.iter().enumerate() {
            let cost = if c1 == c2 { 0 } else { 1 };
            dp[i + 1][j + 1] = *[
                dp[i][j + 1] + 1, // 删除
                dp[i + 1][j] + 1, // 插入
                dp[i][j] + cost,  // 替换
            ]
            .iter()
            .min()
            .unwrap();
        }
    }

    dp[len1][len2]
}

/**
 * 模糊匹配器
 *
 * 使用 Levenshtein 距离算法实现模糊匹配，并支持 Metaphone 语音相似度
 */
pub struct FuzzyMatcher {
    max_distance: usize, // 最大编辑距离
    // ✅ 新增：Metaphone编码缓存
    metaphone_cache: Arc<RwLock<HashMap<String, String>>>,
}

impl FuzzyMatcher {
    /// 创建新的模糊匹配器
    ///
    /// # 参数
    /// * `max_distance` - 最大编辑距离，默认为 2
    pub fn new(max_distance: usize) -> Self {
        Self {
            max_distance,
            metaphone_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /**
     * 检查两个字符串是否足够相似
     *
     * 使用动态阈值：
     * - 短词（≤4字符）: 最多 1 个差异
     * - 中等词（5-8字符）: 最多 2 个差异
     * - 长词（>8字符）: 最多 3 个差异
     *
     * # 参数
     * * `s1` - 第一个字符串
     * * `s2` - 第二个字符串
     *
     * # 返回
     * * `true` - 足够相似
     * * `false` - 不够相似
     */
    pub fn is_similar(&self, s1: &str, s2: &str) -> bool {
        let distance = levenshtein_distance(s1, s2);

        // 动态阈值：短词允许更小的距离
        let threshold = if s1.len() <= 4 {
            1 // 短词（≤4字符）最多1个差异
        } else if s1.len() <= 8 {
            2 // 中等词（5-8字符）最多2个差异
        } else {
            3 // 长词（>8字符）最多3个差异
        };

        distance <= threshold && distance <= self.max_distance
    }

    /**
     * 在日志行中查找模糊匹配的单词
     *
     * 改进版本：支持完整单词匹配 + 子串匹配（UTF-8安全）
     *
     * # 参数
     * * `query` - 搜索查询
     * * `line` - 日志行
     *
     * # 返回
     * * `true` - 找到相似单词
     * * `false` - 没有找到
     */
    pub fn find_similar_words(&self, query: &str, line: &str) -> bool {
        // 分割日志行为单词
        let words: Vec<&str> = line.split_whitespace().collect();

        // 快速路径：先检查完整单词
        if words.iter().any(|word| self.is_similar(query, word)) {
            return true;
        }

        // 改进：检查单词的子串（覆盖更多场景）
        // 例如：搜索 "ERRO" 可以匹配 "ERROR" 中的 "ERRO" 子串
        for word in words {
            if word.len() > query.len() {
                // 提取所有可能的长度相近的子串
                let min_len = query.len().saturating_sub(2);
                let max_len = (query.len() + 2).min(word.len());

                // ✅ 修复：使用 char_indices() 获取有效的UTF-8字符边界
                let char_indices: Vec<usize> = word
                    .char_indices()
                    .map(|(idx, _)| idx)
                    .chain(std::iter::once(word.len()))
                    .collect();

                // 在有效的字符边界上生成子串
                for (i, &start) in char_indices.iter().enumerate() {
                    for &end in char_indices.iter().skip(i + 1) {
                        let sub_len = end - start;
                        if sub_len >= min_len && sub_len <= max_len {
                            // ✅ 使用 get() 安全访问，避免panic
                            if let Some(substring) = word.get(start..end) {
                                if self.is_similar(query, substring) {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }

        false
    }

    /**
     * 获取最佳匹配（如果存在）
     *
     * 返回最相似的单词及其距离
     *
     * # 参数
     * * `query` - 搜索查询
     * * `line` - 日志行
     *
     * # 返回
     * * `Some((word, distance))` - 最佳匹配
     * * `None` - 没有找到相似单词
     */
    pub fn find_best_match(&self, query: &str, line: &str) -> Option<(String, usize)> {
        let words: Vec<&str> = line.split_whitespace().collect();

        words
            .iter()
            .filter_map(|word| {
                let distance = levenshtein_distance(query, word);
                if self.is_similar(query, word) {
                    Some((word.to_string(), distance))
                } else {
                    None
                }
            })
            .min_by_key(|(_, distance)| *distance)
    }

    // ✅ 新增：Metaphone相关方法

    /// ✅ 新增：获取缓存的Metaphone编码（同步版本）
    fn get_metaphone_cached(&self, word: &str) -> String {
        // 检查缓存
        {
            let cache = self.metaphone_cache.read().unwrap();
            if let Some(cached) = cache.get(word) {
                return cached.clone();
            }
        }

        // 计算编码
        let encoded = metaphone(word);

        // 更新缓存
        {
            let mut cache = self.metaphone_cache.write().unwrap();
            cache.insert(word.to_string(), encoded.clone());
        }

        encoded
    }

    /// ✅ 新增：语音相似度检查
    ///
    /// 使用Metaphone算法检查两个单词是否语音相似
    ///
    /// # 参数
    /// * `s1` - 第一个单词
    /// * `s2` - 第二个单词
    ///
    /// # 返回
    /// * `true` - 语音相似
    /// * `false` - 不相似
    pub fn is_phonetically_similar(&self, s1: &str, s2: &str) -> bool {
        let m1 = self.get_metaphone_cached(s1);
        let m2 = self.get_metaphone_cached(s2);
        m1 == m2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_levenshtein_distance_identical() {
        assert_eq!(levenshtein_distance("test", "test"), 0);
    }

    #[test]
    fn test_levenshtein_distance_one_insertion() {
        assert_eq!(levenshtein_distance("test", "tests"), 1);
    }

    #[test]
    fn test_levenshtein_distance_one_deletion() {
        assert_eq!(levenshtein_distance("ERROR", "ERRO"), 1);
    }

    #[test]
    fn test_levenshtein_distance_one_substitution() {
        assert_eq!(levenshtein_distance("cat", "bat"), 1);
    }

    #[test]
    fn test_levenshtein_distance_multiple() {
        assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
    }

    #[test]
    fn test_levenshtein_distance_empty() {
        assert_eq!(levenshtein_distance("", "test"), 4);
        assert_eq!(levenshtein_distance("test", ""), 4);
        assert_eq!(levenshtein_distance("", ""), 0);
    }

    #[test]
    fn test_fuzzy_match_exact() {
        let matcher = FuzzyMatcher::new(2);
        assert!(matcher.is_similar("ERROR", "ERROR"));
    }

    #[test]
    fn test_fuzzy_match_one_char_diff() {
        let matcher = FuzzyMatcher::new(2);
        assert!(matcher.is_similar("ERROR", "ERRO"));
        assert!(matcher.is_similar("connection", "connetion"));
        assert!(matcher.is_similar("database", "databse"));
    }

    #[test]
    fn test_fuzzy_match_short_word_strict() {
        let matcher = FuzzyMatcher::new(2);
        // 短词（≤4字符）最多1个差异
        assert!(matcher.is_similar("test", "tst"));
        assert!(matcher.is_similar("code", "cod"));

        // 超过阈值
        assert!(!matcher.is_similar("test", "ts"));
        assert!(!matcher.is_similar("code", "cd"));
    }

    #[test]
    fn test_fuzzy_match_medium_word() {
        let matcher = FuzzyMatcher::new(2);
        // 中等词（5-8字符）最多2个差异
        assert!(matcher.is_similar("connect", "conect"));
        assert!(matcher.is_similar("timeout", "timout"));

        // cnect 缺少两个字符，距离为2，阈值也是2，应该匹配
        assert!(matcher.is_similar("connect", "cnect"));
    }

    #[test]
    fn test_fuzzy_match_long_word() {
        let matcher = FuzzyMatcher::new(3);
        // 长词（>8字符）最多3个差异
        assert!(matcher.is_similar("connection", "conecton"));
        assert!(matcher.is_similar("database", "databas"));
    }

    #[test]
    fn test_fuzzy_match_respects_max_distance() {
        let matcher = FuzzyMatcher::new(1); // 限制为1个差异

        assert!(matcher.is_similar("ERROR", "ERRO"));
        assert!(matcher.is_similar("connection", "connetion")); // 1个差异，未超过限制
    }

    #[test]
    fn test_find_similar_words_in_line() {
        let matcher = FuzzyMatcher::new(2);

        // 精确匹配
        assert!(matcher.find_similar_words("ERROR", "2024-01-01 ERROR Database connection failed"));

        // 模糊匹配
        assert!(matcher.find_similar_words("ERRO", "2024-01-01 ERROR Database connection failed"));

        // 不匹配
        assert!(!matcher.find_similar_words("WARN", "2024-01-01 ERROR Database connection failed"));
    }

    #[test]
    fn test_find_similar_words_substring_match() {
        let matcher = FuzzyMatcher::new(2);

        // 子串匹配：搜索 "ERRO" 应该匹配 "ERROR" 中的子串
        assert!(matcher.find_similar_words("ERRO", "2024-01-01 ERROR: Database connection failed"));

        // 子串匹配：搜索 "conne" 应该匹配 "connection" 中的子串
        assert!(matcher.find_similar_words("conne", "2024-01-01 Failed to establish connection"));

        // 子串匹配：搜索 "datab" 应该匹配 "database" 中的子串
        assert!(matcher.find_similar_words("datab", "2024-01-01 Database query timeout"));
    }

    #[test]
    fn test_find_best_match() {
        let matcher = FuzzyMatcher::new(2);

        let line = "ERRO ERROR WARN";
        let result = matcher.find_best_match("ERRO", line);

        assert!(result.is_some());
        let (_word, distance) = result.unwrap();
        assert_eq!(distance, 0);
    }

    #[test]
    fn test_find_best_match_fuzzy() {
        let matcher = FuzzyMatcher::new(2);

        let line = "ERROE WARN INFO"; // 移除精确匹配，只保留模糊匹配
        let result = matcher.find_best_match("ERROR", line);

        assert!(result.is_some());
        let (_word, distance) = result.unwrap();
        assert_eq!(distance, 1); // ERROE vs ERROR = 1个差异
    }

    #[test]
    fn test_find_best_match_no_match() {
        let matcher = FuzzyMatcher::new(2);

        let line = "WARN INFO DEBUG";
        let result = matcher.find_best_match("ERROR", line);

        assert!(result.is_none());
    }

    #[test]
    fn test_unicode_support() {
        // 测试 Unicode 字符
        assert_eq!(levenshtein_distance("café", "cafe"), 1);
        assert_eq!(levenshtein_distance("测试", "试测"), 2);
    }

    #[test]
    fn test_case_sensitive() {
        // 默认区分大小写
        assert_eq!(levenshtein_distance("ERROR", "error"), 5);
        assert!(!FuzzyMatcher::new(2).is_similar("ERROR", "error"));
    }

    #[test]
    fn test_performance_small_strings() {
        // 性能测试：小字符串
        let matcher = FuzzyMatcher::new(2);
        for _ in 0..1000 {
            matcher.is_similar("test", "tst");
        }
        // 如果没有超时，测试通过
    }

    #[test]
    fn test_common_typos() {
        let matcher = FuzzyMatcher::new(2);

        // 常见拼写错误
        assert!(matcher.is_similar("recieve", "receive"));
        assert!(matcher.is_similar("occured", "occurred"));
        assert!(matcher.is_similar("seperate", "separate"));
    }

    // ✅ 新增：UTF-8安全性测试
    #[test]
    fn test_unicode_substring_matching() {
        let matcher = FuzzyMatcher::new(2);

        // 中文日志行
        assert!(matcher.find_similar_words("ERRO", "2024-01-01 错误 ERROR: 数据库连接失败"));

        // Emoji测试
        assert!(matcher.find_similar_words("ERR", "2024-01-01 🚨 ERROR: 系统告警"));
    }

    #[test]
    fn test_multibyte_character_boundaries() {
        let matcher = FuzzyMatcher::new(2);

        // 包含多字节UTF-8字符的日志
        let log_line = "2024-01-01 ÄÖÜ ERROR: Verbindung fehlgeschlagen";
        assert!(matcher.find_similar_words("ERRO", log_line));

        // 德语特殊字符
        let log_line2 = "2024-01-01 Übertragungsfehler ERROR";
        assert!(matcher.find_similar_words("ERR", log_line2));
    }

    #[test]
    fn test_emoji_in_logs() {
        let matcher = FuzzyMatcher::new(2);

        // 包含emoji的日志
        let log_line = "2024-01-01 🚨 ❌ ERROR ⚠️ WARN";
        assert!(matcher.find_similar_words("ERRO", log_line));
    }

    // ✅ 新增：Metaphone测试
    #[test]
    fn test_metaphone_integration() {
        let matcher = FuzzyMatcher::new(2);

        // 语音相似度测试
        assert!(matcher.is_phonetically_similar("Smith", "Smyth"));
        assert!(matcher.is_phonetically_similar("Knight", "Nite"));
        assert!(matcher.is_phonetically_similar("through", "thru"));

        // 不相似
        assert!(!matcher.is_phonetically_similar("hello", "world"));
    }

    #[test]
    fn test_metaphone_cache() {
        let matcher = FuzzyMatcher::new(2);

        // 第一次计算
        let result1 = matcher.get_metaphone_cached("Smith");

        // 第二次应该从缓存读取
        let result2 = matcher.get_metaphone_cached("Smith");

        assert_eq!(result1, result2);
    }

    #[test]
    fn test_combined_fuzzy_and_phonetic() {
        let matcher = FuzzyMatcher::new(2);

        // Levenshtein距离匹配
        assert!(matcher.is_similar("ERROR", "ERRO"));

        // Metaphone语音匹配
        assert!(matcher.is_phonetically_similar("Smith", "Smyth"));
    }
}
