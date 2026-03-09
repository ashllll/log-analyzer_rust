//! 搜索历史记录数据模型
//!
//! 提供搜索历史的数据结构和相关操作

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 搜索历史条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHistoryEntry {
    /// 查询内容
    pub query: String,
    /// 工作区ID
    pub workspace_id: String,
    /// 结果数量
    pub result_count: usize,
    /// 搜索时间
    pub searched_at: DateTime<Utc>,
}

impl SearchHistoryEntry {
    /// 创建新的搜索历史条目
    pub fn new(query: String, workspace_id: String, result_count: usize) -> Self {
        Self {
            query,
            workspace_id,
            result_count,
            searched_at: Utc::now(),
        }
    }
}

/// 搜索历史管理器
///
/// 管理所有工作区的搜索历史，提供增删查功能
#[derive(Debug, Clone)]
pub struct SearchHistoryManager {
    /// 所有搜索历史条目
    entries: Vec<SearchHistoryEntry>,
    /// 最大历史记录数
    max_entries: usize,
    /// 每个工作区的最大历史记录数
    max_entries_per_workspace: usize,
}

impl Default for SearchHistoryManager {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            max_entries: 500,
            max_entries_per_workspace: 100,
        }
    }
}

impl SearchHistoryManager {
    /// 创建新的搜索历史管理器
    pub fn new() -> Self {
        Self::default()
    }

    /// 创建带自定义配置的管理器
    pub fn with_config(max_entries: usize, max_entries_per_workspace: usize) -> Self {
        Self {
            entries: Vec::new(),
            max_entries,
            max_entries_per_workspace,
        }
    }

    /// 获取所有条目（用于 FFI）
    pub fn get_entries(&self) -> &Vec<SearchHistoryEntry> {
        &self.entries
    }

    /// 获取所有条目的可变引用（用于 FFI）
    pub fn get_entries_mut(&mut self) -> &mut Vec<SearchHistoryEntry> {
        &mut self.entries
    }

    /// 添加搜索历史条目
    ///
    /// 如果超过限制，会自动删除最旧的条目
    pub fn add_entry(&mut self, entry: SearchHistoryEntry) {
        // 检查是否已存在相同的查询（去重）
        self.entries
            .retain(|e| !(e.query == entry.query && e.workspace_id == entry.workspace_id));

        // 添加新条目到开头（最新的在前面）
        self.entries.insert(0, entry);

        // 清理超出限制的条目
        self.cleanup_old_entries();
    }

    /// 获取指定工作区的搜索历史
    ///
    /// 返回按时间倒序排列的历史记录
    pub fn get_history(
        &self,
        workspace_id: &str,
        limit: Option<usize>,
    ) -> Vec<&SearchHistoryEntry> {
        let limit = limit.unwrap_or(self.max_entries_per_workspace);
        self.entries
            .iter()
            .filter(|e| e.workspace_id == workspace_id)
            .take(limit)
            .collect()
    }

    /// 获取所有搜索历史
    pub fn get_all_history(&self, limit: Option<usize>) -> Vec<&SearchHistoryEntry> {
        let limit = limit.unwrap_or(self.max_entries);
        self.entries.iter().take(limit).collect()
    }

    /// 清除指定工作区的搜索历史
    pub fn clear_workspace_history(&mut self, workspace_id: &str) -> usize {
        let original_len = self.entries.len();
        self.entries.retain(|e| e.workspace_id != workspace_id);
        original_len - self.entries.len()
    }

    /// 清除所有搜索历史
    pub fn clear_all_history(&mut self) {
        self.entries.clear();
    }

    /// 获取历史记录总数
    pub fn total_count(&self) -> usize {
        self.entries.len()
    }

    /// 获取指定工作区的历史记录数量
    pub fn workspace_count(&self, workspace_id: &str) -> usize {
        self.entries
            .iter()
            .filter(|e| e.workspace_id == workspace_id)
            .count()
    }

    /// 清理超出限制的旧条目
    fn cleanup_old_entries(&mut self) {
        // 首先检查总限制
        if self.entries.len() > self.max_entries {
            self.entries.truncate(self.max_entries);
        }

        // 然后检查每个工作区的限制
        // 按工作区分组统计
        let mut workspace_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();

        // 从后向前遍历，删除超出限制的旧条目
        let mut to_remove = Vec::new();
        for (idx, entry) in self.entries.iter().enumerate().rev() {
            let count = workspace_counts
                .entry(entry.workspace_id.clone())
                .or_insert(0);
            *count += 1;
            if *count > self.max_entries_per_workspace {
                to_remove.push(idx);
            }
        }

        // 从后向前删除（保持索引有效）
        for idx in to_remove {
            self.entries.remove(idx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_entry() {
        let mut manager = SearchHistoryManager::new();
        let entry = SearchHistoryEntry::new("test query".to_string(), "ws1".to_string(), 10);
        manager.add_entry(entry);

        assert_eq!(manager.total_count(), 1);
    }

    #[test]
    fn test_duplicate_query_deduplication() {
        let mut manager = SearchHistoryManager::new();

        // 添加相同查询两次
        manager.add_entry(SearchHistoryEntry::new(
            "query1".to_string(),
            "ws1".to_string(),
            10,
        ));
        manager.add_entry(SearchHistoryEntry::new(
            "query1".to_string(),
            "ws1".to_string(),
            20,
        ));

        // 应该只有一个条目（后者覆盖前者）
        assert_eq!(manager.total_count(), 1);

        let history = manager.get_history("ws1", None);
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].result_count, 20); // 应该是最新值
    }

    #[test]
    fn test_get_workspace_history() {
        let mut manager = SearchHistoryManager::new();

        manager.add_entry(SearchHistoryEntry::new(
            "query1".to_string(),
            "ws1".to_string(),
            10,
        ));
        manager.add_entry(SearchHistoryEntry::new(
            "query2".to_string(),
            "ws2".to_string(),
            20,
        ));
        manager.add_entry(SearchHistoryEntry::new(
            "query3".to_string(),
            "ws1".to_string(),
            30,
        ));

        let ws1_history = manager.get_history("ws1", None);
        assert_eq!(ws1_history.len(), 2);

        let ws2_history = manager.get_history("ws2", None);
        assert_eq!(ws2_history.len(), 1);
    }

    #[test]
    fn test_clear_workspace_history() {
        let mut manager = SearchHistoryManager::new();

        manager.add_entry(SearchHistoryEntry::new(
            "query1".to_string(),
            "ws1".to_string(),
            10,
        ));
        manager.add_entry(SearchHistoryEntry::new(
            "query2".to_string(),
            "ws2".to_string(),
            20,
        ));

        let removed = manager.clear_workspace_history("ws1");
        assert_eq!(removed, 1);
        assert_eq!(manager.total_count(), 1);
    }

    #[test]
    fn test_history_limit() {
        let manager = SearchHistoryManager::with_config(5, 2);

        // 添加多个条目，验证限制生效
        let mut manager = manager;
        for i in 0..10 {
            manager.add_entry(SearchHistoryEntry::new(
                format!("query{}", i),
                "ws1".to_string(),
                i,
            ));
        }

        // 由于去重和限制，应该不超过 max_entries_per_workspace
        assert!(manager.workspace_count("ws1") <= 2);
    }

    #[test]
    fn test_chronological_order() {
        let mut manager = SearchHistoryManager::new();

        // 添加顺序：query1 -> query2 -> query3
        manager.add_entry(SearchHistoryEntry::new(
            "query1".to_string(),
            "ws1".to_string(),
            1,
        ));
        manager.add_entry(SearchHistoryEntry::new(
            "query2".to_string(),
            "ws1".to_string(),
            2,
        ));
        manager.add_entry(SearchHistoryEntry::new(
            "query3".to_string(),
            "ws1".to_string(),
            3,
        ));

        let history = manager.get_history("ws1", None);

        // 最新的应该在最前面
        assert_eq!(history[0].query, "query3");
        assert_eq!(history[1].query, "query2");
        assert_eq!(history[2].query, "query1");
    }
}
