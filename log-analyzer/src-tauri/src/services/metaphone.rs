/**
 * Metaphone语音编码算法
 *
 * 将单词编码为语音表示，用于匹配发音相似的单词
 *
 * # 示例
 * ```
 * assert_eq!(metaphone("Smith"), "sm0");
 * assert_eq!(metaphone("Smyth"), "sm0");
 * assert!(is_phonetically_similar("Smith", "Smyth"));
 * ```
 */
/// Metaphone算法实现（简化版）
///
/// # 规则
/// - 转换为小写
/// - 保留首字母
/// - 移除无声字母
/// - 转换相似的辅音为同一编码
/// - 压缩连续相同的字母
///
/// # 参数
/// * `word` - 输入单词
///
/// # 返回
/// Metaphone编码字符串
pub fn metaphone(word: &str) -> String {
    let word = word.to_lowercase();
    let mut result = String::new();
    let chars: Vec<char> = word.chars().collect();
    let len = chars.len();

    if len == 0 {
        return result;
    }

    let mut i = 0;

    while i < len {
        let c = chars[i];

        match c {
            // 元音：仅首字母保留
            'a' | 'e' | 'i' | 'o' | 'u' => {
                if result.is_empty() {
                    result.push(c);
                }
                i += 1;
            }

            // KN开头的K省略
            'k' => {
                if i + 1 < len && chars[i + 1] == 'n' {
                    // KN组合，忽略K
                    i += 1;
                } else {
                    result.push('k');
                    i += 1;
                }
            }

            // 辅音转换规则
            'c' => {
                if i + 1 < len {
                    let next = chars[i + 1];
                    if next == 'i' || next == 'e' || next == 'y' {
                        result.push('s'); // soft C
                    } else {
                        result.push('k'); // hard C
                    }
                } else {
                    result.push('k');
                }
                i += 1;
            }

            'g' => {
                if i + 1 < len {
                    let next = chars[i + 1];
                    if next == 'e' || next == 'i' || next == 'y' {
                        result.push('j'); // soft G
                    } else if next == 'h' {
                        // GH组合，忽略G
                        i += 1;
                        continue;
                    } else {
                        result.push('k'); // hard G
                    }
                } else {
                    result.push('k');
                }
                i += 1;
            }

            // H的处理已集成到GH组合中
            'h' => {
                // H仅在词首保留
                if result.is_empty() {
                    result.push('h');
                }
                i += 1;
            }

            'p' => {
                if i + 1 < len && chars[i + 1] == 'h' {
                    result.push('f'); // PH -> F
                    i += 2;
                } else {
                    result.push('p');
                    i += 1;
                }
            }

            's' => {
                if i + 1 < len && chars[i + 1] == 'h' {
                    result.push('x'); // SH sound
                    i += 2;
                } else {
                    result.push('s');
                    i += 1;
                }
            }

            't' => {
                if i + 1 < len && chars[i + 1] == 'h' {
                    result.push('0'); // TH sound (use digit 0)
                    i += 2;
                } else {
                    result.push('t');
                    i += 1;
                }
            }

            // 其他辅音直接保留（y在特定位置作为元音，应被忽略）
            'f' | 'l' | 'm' | 'n' | 'r' | 'b' | 'd' | 'j' | 'v' | 'w' | 'x' | 'z' => {
                result.push(c);
                i += 1;
            }

            _ => {
                i += 1;
            }
        }
    }

    // 压缩连续相同的字符（使用标准库实现）
    let compressed: String = result.chars().fold(String::new(), |mut acc, c| {
        if !acc.ends_with(c) {
            acc.push(c);
        }
        acc
    });

    compressed
}

/// 检查两个单词是否语音相似
///
/// # 参数
/// * `s1` - 第一个单词
/// * `s2` - 第二个单词
///
/// # 返回
/// * `true` - 语音相似（Metaphone编码相同）
/// * `false` - 不相似
pub fn is_phonetically_similar(s1: &str, s2: &str) -> bool {
    metaphone(s1) == metaphone(s2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metaphone_basic() {
        // Smith 和 Smyth 应该有相同的编码
        let code1 = metaphone("Smith");
        let code2 = metaphone("Smyth");
        assert_eq!(code1, code2);

        // Knight 和 Nite 应该相似
        assert_eq!(metaphone("Knight"), metaphone("Nite"));

        // through 和 thru 应该相似
        assert_eq!(metaphone("through"), metaphone("thru"));
    }

    #[test]
    fn test_phonetic_similarity() {
        assert!(is_phonetically_similar("Smith", "Smyth"));
        assert!(is_phonetically_similar("Knight", "Nite"));
        assert!(is_phonetically_similar("through", "thru"));

        // 不相似的单词
        assert!(!is_phonetically_similar("hello", "world"));
    }

    #[test]
    fn test_metaphone_case_insensitive() {
        assert_eq!(metaphone("Smith"), metaphone("SMITH"));
        assert_eq!(metaphone("Smith"), metaphone("smith"));
    }

    #[test]
    fn test_metaphone_empty() {
        assert_eq!(metaphone(""), "");
    }

    #[test]
    fn test_common_misspellings() {
        // 常见拼写错误
        assert!(is_phonetically_similar("recieve", "receive"));
        assert!(is_phonetically_similar("occured", "occurred"));
        assert!(is_phonetically_similar("seperate", "separate"));
    }
}
