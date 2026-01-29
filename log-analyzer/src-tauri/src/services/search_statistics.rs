use std::collections::HashMap;

use crate::models::{log_entry::LogEntry, search_statistics::KeywordStatistics};

/// 计算关键词统计信息
///
/// # 参数
/// * `results` - 搜索结果列表
/// * `keywords` - 原始关键词列表
///
/// # 返回值
/// 返回按匹配数量降序排列的关键词统计信息列表
pub fn calculate_keyword_statistics(
    results: &[LogEntry],
    keywords: &[String],
) -> Vec<KeywordStatistics> {
    // 初始化每个关键词的计数器
    let mut keyword_counts: HashMap<String, usize> = HashMap::new();

    // 初始化所有关键词计数为0
    for keyword in keywords {
        keyword_counts.insert(keyword.clone(), 0);
    }

    // 遍历所有结果，统计每个关键词的匹配次数
    for entry in results {
        if let Some(ref matched_keywords) = entry.matched_keywords {
            for keyword in matched_keywords {
                // 增加关键词计数
                *keyword_counts.get_mut(keyword).unwrap_or(&mut 0) += 1;
            }
        }
    }

    // 转换为KeywordStatistics向量
    let total_matches = results.len();
    let mut stats: Vec<KeywordStatistics> = keyword_counts
        .into_iter()
        .map(|(keyword, count)| KeywordStatistics::new(keyword, count, total_matches))
        .collect();

    // 按匹配数量降序排序
    stats.sort_by(|a, b| b.match_count.cmp(&a.match_count));

    stats
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::MatchDetail;

    #[test]
    fn test_calculate_keyword_statistics_normal() {
        // 创建测试数据，确保不同关键词有不同的匹配数量
        let keywords = vec![
            "error".to_string(),
            "timeout".to_string(),
            "warning".to_string(),
        ];

        // entry1: 匹配 error
        let entry1 = LogEntry {
            id: 1,
            timestamp: "2024-01-01 12:00:00".into(),
            level: "ERROR".into(),
            file: "app.log".into(),
            real_path: "/path/app.log".into(),
            line: 1,
            content: "An error occurred".into(),
            tags: vec![],
            match_details: Some(vec![MatchDetail {
                term_id: "term_1".to_string(),
                term_value: "error".to_string(),
                priority: 1,
                match_position: Some((3, 8)),
            }]),
            matched_keywords: Some(vec!["error".to_string()]),
        };

        // entry2: 匹配 timeout 和 warning
        let entry2 = LogEntry {
            id: 2,
            timestamp: "2024-01-01 12:01:00".into(),
            level: "WARN".into(),
            file: "app.log".into(),
            real_path: "/path/app.log".into(),
            line: 2,
            content: "Timeout and warning".into(),
            tags: vec![],
            match_details: Some(vec![
                MatchDetail {
                    term_id: "term_2".to_string(),
                    term_value: "timeout".to_string(),
                    priority: 1,
                    match_position: Some((0, 7)),
                },
                MatchDetail {
                    term_id: "term_3".to_string(),
                    term_value: "warning".to_string(),
                    priority: 1,
                    match_position: Some((12, 19)),
                },
            ]),
            matched_keywords: Some(vec!["timeout".to_string(), "warning".to_string()]),
        };

        // entry3: 再次匹配 error
        let entry3 = LogEntry {
            id: 3,
            timestamp: "2024-01-01 12:02:00".into(),
            level: "ERROR".into(),
            file: "app.log".into(),
            real_path: "/path/app.log".into(),
            line: 3,
            content: "Another error".into(),
            tags: vec![],
            match_details: Some(vec![MatchDetail {
                term_id: "term_1".to_string(),
                term_value: "error".to_string(),
                priority: 1,
                match_position: Some((8, 13)),
            }]),
            matched_keywords: Some(vec!["error".to_string()]),
        };

        let results = vec![entry1, entry2, entry3];
        let stats = calculate_keyword_statistics(&results, &keywords);

        // 验证结果：应该按照匹配数量降序排列
        // error: 2次 (66.67%)
        // timeout: 1次 (33.33%)
        // warning: 1次 (33.33%)
        assert_eq!(stats.len(), 3);

        // 第一个应该是 error (最多匹配)
        assert_eq!(stats[0].keyword, "error");
        assert_eq!(stats[0].match_count, 2);
        assert!((stats[0].match_percentage - 66.67).abs() < 0.1);

        // 后两个是 timeout 和 warning，顺序不确定，都是1次匹配
        let remaining: Vec<_> = if stats.len() > 1 {
            stats[1..].iter().collect()
        } else {
            Vec::new()
        };
        assert!(remaining
            .iter()
            .any(|s| s.keyword == "timeout" && s.match_count == 1));
        assert!(remaining
            .iter()
            .any(|s| s.keyword == "warning" && s.match_count == 1));

        // 验证百分比
        for stat in remaining {
            assert!((stat.match_percentage - 33.33).abs() < 0.1);
        }
    }

    #[test]
    fn test_calculate_keyword_statistics_empty_results() {
        let keywords = vec!["error".to_string(), "timeout".to_string()];
        let results: Vec<LogEntry> = vec![];
        let stats = calculate_keyword_statistics(&results, &keywords);

        assert_eq!(stats.len(), 2);
        assert_eq!(stats[0].match_count, 0);
        assert_eq!(stats[1].match_count, 0);
    }

    #[test]
    fn test_calculate_keyword_statistics_no_matches() {
        let keywords = vec!["error".to_string()];
        let entry = LogEntry {
            id: 1,
            timestamp: "2024-01-01 12:00:00".into(),
            level: "INFO".into(),
            file: "app.log".into(),
            real_path: "/path/app.log".into(),
            line: 1,
            content: "Information message".into(),
            tags: vec![],
            match_details: None,
            matched_keywords: None,
        };
        let results = vec![entry];
        let stats = calculate_keyword_statistics(&results, &keywords);

        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0].match_count, 0);
    }
}
