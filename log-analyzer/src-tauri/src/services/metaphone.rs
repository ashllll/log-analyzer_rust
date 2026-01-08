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
/// - 处理特殊辅音组合（TH→0, SH→X, PH→F）
/// - 元音只在首字母位置保留
/// - 忽略Y（除非是唯一元音）
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
            // 首字母特殊处理
            _ if i == 0 => {
                // KN 组合时省略 K（开头的 K 在 N 前不发音）
                if c == 'k' && i + 1 < len && chars[i + 1] == 'n' {
                    // 跳过 K
                    i += 1;
                    continue;
                }
                result.push(c);
                i += 1;
            }

            // 元音：仅首字母保留
            'a' | 'e' | 'i' | 'o' | 'u' => {
                // 元音只在开头保留，中间忽略
                i += 1;
            }

            // C: CAE/CE/CI/CY -> S, 否则 -> K
            'c' => {
                if i + 1 < len {
                    let next = chars[i + 1];
                    if next == 'i' || next == 'e' || next == 'y' {
                        result.push('s'); // soft C
                    } else if next == 'h' && i + 2 < len && chars[i + 2] == ' ' {
                        // 特例：-chs -> K
                        result.push('k');
                        i += 1;
                    } else {
                        result.push('k'); // hard C
                    }
                } else {
                    result.push('k');
                }
                i += 1;
            }

            // G: GE/GI/GY -> J, GH (后跟非元音) -> 忽略, 其他 -> K
            'g' => {
                if i + 1 < len {
                    let next = chars[i + 1];
                    if next == 'e' || next == 'i' || next == 'y' {
                        result.push('j'); // soft G
                    } else if next == 'h' {
                        // GH 后跟非元音时，GH 不发音
                        // 跳过 G，H 后面会处理
                        i += 1; // 只增加 i，让下一个循环处理 h
                        continue;
                    } else {
                        result.push('k'); // hard G
                    }
                } else {
                    result.push('k');
                }
                i += 1;
            }

            // H: 只在元音前保留
            'h' => {
                if i + 1 < len {
                    let next = chars[i + 1];
                    // 如果后面是元音，H发音
                    if matches!(next, 'a' | 'e' | 'i' | 'o' | 'u') {
                        result.push('h');
                    }
                }
                i += 1;
            }

            // P: PH -> F
            'p' => {
                if i + 1 < len && chars[i + 1] == 'h' {
                    result.push('f'); // PH -> F
                    i += 2;
                } else {
                    result.push('p');
                    i += 1;
                }
            }

            // S: SH -> X, 其他 -> S
            's' => {
                if i + 1 < len && chars[i + 1] == 'h' {
                    result.push('x'); // SH sound
                    i += 2;
                } else {
                    result.push('s');
                    i += 1;
                }
            }

            // T: TH -> 0 (数字0), 其他 -> T
            't' => {
                if i + 1 < len && chars[i + 1] == 'h' {
                    result.push('0'); // TH sound (use digit 0)
                    i += 2;
                } else {
                    result.push('t');
                    i += 1;
                }
            }

            // W: 只在元音前保留
            'w' => {
                if i + 1 < len {
                    let next = chars[i + 1];
                    if matches!(next, 'a' | 'e' | 'i' | 'o' | 'u') {
                        result.push('w');
                    }
                }
                i += 1;
            }

            // Y: 忽略（在英语中通常不是独立辅音）
            'y' => {
                i += 1;
            }

            // 其他辅音直接保留
            'f' | 'l' | 'm' | 'n' | 'r' | 'b' | 'd' | 'j' | 'k' | 'v' | 'x' | 'z' => {
                result.push(c);
                i += 1;
            }

            // 空格和标点：跳过
            ' ' | '-' | '\'' => {
                i += 1;
            }

            // 其他字符：跳过
            _ => {
                i += 1;
            }
        }
    }

    // 压缩连续相同的字符
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
