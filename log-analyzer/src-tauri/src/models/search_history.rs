use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// 搜索历史记录项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHistoryItem {
    pub id: String,
    pub query: String,
    pub timestamp: i64,              // Unix timestamp (seconds)
    pub result_count: Option<usize>, // 搜索结果数量
    pub workspace_id: String,
}

/// 搜索历史管理器
///
/// 特性：
/// - 保存最近 N 条记录（默认 50）
/// - 自动去重（相同查询只保留最新）
/// - 按时间倒序排列（最新的在前）
pub struct SearchHistory {
    history: VecDeque<SearchHistoryItem>,
    max_size: usize, // 最大保存数量，默认50
}

impl SearchHistory {
    /// 创建新的搜索历史管理器
    ///
    /// # 参数
    /// * `max_size` - 最大保存数量，默认 50
    pub fn new(max_size: usize) -> Self {
        Self {
            history: VecDeque::with_capacity(max_size),
            max_size,
        }
    }

    /// 添加搜索记录
    ///
    /// # 行为
    /// - 去重：如果已存在相同的查询，先删除旧的
    /// - 添加到前面（最新的在前）
    /// - 限制数量：超过 max_size 时删除最旧的
    pub fn add(&mut self, item: SearchHistoryItem) {
        // 去重：如果已存在相同的查询，先删除旧的
        self.history.retain(|x| x.query != item.query);

        // 添加到前面
        self.history.push_front(item);

        // 限制数量
        if self.history.len() > self.max_size {
            self.history.pop_back();
        }
    }

    /// 获取所有历史记录（按时间倒序）
    pub fn get_all(&self) -> Vec<SearchHistoryItem> {
        self.history.iter().cloned().collect()
    }

    /// 根据 workspace 过滤历史记录
    pub fn get_by_workspace(&self, workspace_id: &str) -> Vec<SearchHistoryItem> {
        self.history
            .iter()
            .filter(|x| x.workspace_id == workspace_id)
            .cloned()
            .collect()
    }

    /// 搜索匹配的历史记录（用于自动补全）
    ///
    /// # 参数
    /// * `prefix` - 搜索前缀（大小写不敏感）
    /// * `workspace_id` - 工作区 ID
    ///
    /// # 返回
    /// 匹配的历史记录（按时间倒序）
    pub fn search(&self, prefix: &str, workspace_id: &str) -> Vec<SearchHistoryItem> {
        if prefix.is_empty() {
            return Vec::new();
        }

        let prefix_lower = prefix.to_lowercase();
        self.history
            .iter()
            .filter(|x| {
                x.workspace_id == workspace_id && x.query.to_lowercase().starts_with(&prefix_lower)
            })
            .cloned()
            .collect()
    }

    /// 删除单条记录
    pub fn remove(&mut self, id: &str) {
        self.history.retain(|x| x.id != id);
    }

    /// 清空所有历史
    pub fn clear(&mut self) {
        self.history.clear();
    }

    /// 获取历史记录数量
    pub fn len(&self) -> usize {
        self.history.len()
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.history.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_item(
        id: &str,
        query: &str,
        workspace_id: &str,
        result_count: Option<usize>,
    ) -> SearchHistoryItem {
        SearchHistoryItem {
            id: id.to_string(),
            query: query.to_string(),
            timestamp: 1000,
            result_count,
            workspace_id: workspace_id.to_string(),
        }
    }

    #[test]
    fn test_add_item() {
        let mut history = SearchHistory::new(50);
        assert_eq!(history.len(), 0);

        let item = create_test_item("1", "ERROR", "ws1", Some(10));
        history.add(item);

        assert_eq!(history.len(), 1);
        assert_eq!(history.get_all()[0].query, "ERROR");
    }

    #[test]
    fn test_add_duplicate_query() {
        let mut history = SearchHistory::new(50);

        // 添加相同查询两次
        history.add(create_test_item("1", "ERROR", "ws1", Some(10)));
        history.add(create_test_item("2", "ERROR", "ws1", Some(20)));

        // 应该只保留最新的（id=2）
        assert_eq!(history.len(), 1);
        assert_eq!(history.get_all()[0].id, "2");
        assert_eq!(history.get_all()[0].result_count, Some(20));
    }

    #[test]
    fn test_max_size_limit() {
        let mut history = SearchHistory::new(3);

        history.add(create_test_item("1", "query1", "ws1", None));
        history.add(create_test_item("2", "query2", "ws1", None));
        history.add(create_test_item("3", "query3", "ws1", None));
        history.add(create_test_item("4", "query4", "ws1", None));

        // 应该只保留最新的 3 条
        assert_eq!(history.len(), 3);
        assert_eq!(history.get_all()[0].id, "4"); // 最新
        assert_eq!(history.get_all()[2].id, "2"); // 最旧的
    }

    #[test]
    fn test_filter_by_workspace() {
        let mut history = SearchHistory::new(50);

        history.add(create_test_item("1", "ERROR", "ws1", Some(10)));
        history.add(create_test_item("2", "WARN", "ws2", Some(5)));
        history.add(create_test_item("3", "INFO", "ws1", Some(3)));

        let ws1_history = history.get_by_workspace("ws1");
        assert_eq!(ws1_history.len(), 2);
        assert_eq!(ws1_history[0].query, "INFO"); // 最新的在前
        assert_eq!(ws1_history[1].query, "ERROR");
    }

    #[test]
    fn test_search_prefix() {
        let mut history = SearchHistory::new(50);

        history.add(create_test_item("1", "ERROR", "ws1", Some(10)));
        history.add(create_test_item("2", "WARN", "ws1", Some(5)));
        history.add(create_test_item("3", "ERR", "ws1", Some(3)));

        // 搜索 "ERR" 前缀
        let results = history.search("ERR", "ws1");
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].query, "ERR"); // 最新的在前
        assert_eq!(results[1].query, "ERROR");

        // 大小写不敏感
        let results = history.search("err", "ws1");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_search_case_insensitive() {
        let mut history = SearchHistory::new(50);

        history.add(create_test_item("1", "De H", "ws1", Some(10)));
        history.add(create_test_item("2", "DE N", "ws1", Some(5)));

        let results = history.search("de", "ws1");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_remove_item() {
        let mut history = SearchHistory::new(50);

        history.add(create_test_item("1", "ERROR", "ws1", Some(10)));
        history.add(create_test_item("2", "WARN", "ws1", Some(5)));

        history.remove("1");
        assert_eq!(history.len(), 1);
        assert_eq!(history.get_all()[0].id, "2");
    }

    #[test]
    fn test_clear_all() {
        let mut history = SearchHistory::new(50);

        history.add(create_test_item("1", "ERROR", "ws1", Some(10)));
        history.add(create_test_item("2", "WARN", "ws1", Some(5)));

        history.clear();
        assert_eq!(history.len(), 0);
        assert!(history.is_empty());
    }

    #[test]
    fn test_order_by_time_desc() {
        let mut history = SearchHistory::new(50);

        history.add(SearchHistoryItem {
            id: "1".to_string(),
            query: "ERROR".to_string(),
            timestamp: 1000,
            result_count: None,
            workspace_id: "ws1".to_string(),
        });

        history.add(SearchHistoryItem {
            id: "2".to_string(),
            query: "WARN".to_string(),
            timestamp: 2000, // 更新的时间戳
            result_count: None,
            workspace_id: "ws1".to_string(),
        });

        let all = history.get_all();
        assert_eq!(all[0].id, "2"); // 最新的在前
        assert_eq!(all[1].id, "1");
    }

    #[test]
    fn test_empty_search_returns_nothing() {
        let mut history = SearchHistory::new(50);
        history.add(create_test_item("1", "ERROR", "ws1", Some(10)));

        let results = history.search("", "ws1");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_workspace_isolation() {
        let mut history = SearchHistory::new(50);

        history.add(create_test_item("1", "ERROR", "ws1", Some(10)));
        history.add(create_test_item("2", "WARN", "ws2", Some(5)));

        let ws1_results = history.search("", "ws1");
        let ws2_results = history.search("", "ws2");

        assert_eq!(ws1_results.len(), 0); // 空搜索
        assert_eq!(ws2_results.len(), 0); // 空搜索

        let ws1_all = history.get_by_workspace("ws1");
        let ws2_all = history.get_by_workspace("ws2");

        assert_eq!(ws1_all.len(), 1);
        assert_eq!(ws2_all.len(), 1);
    }
}
