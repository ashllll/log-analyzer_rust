use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::FromRow;
use sqlx::SqlitePool;
use std::collections::VecDeque;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use parking_lot::Mutex;
use tracing::info;
use crate::error::{AppError, Result};

/// 搜索历史全局状态类型（使用 parking_lot::Mutex，因为其 Guard 实现 Send）
pub type HistoryState = Arc<Mutex<SearchHistory>>;

/// 搜索历史记录项
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SearchHistoryItem {
    pub id: String,
    pub query: String,
    pub timestamp: i64,              // Unix timestamp (seconds)
    pub result_count: Option<i64>,   // 搜索结果数量 (数据库使用 i64)
    pub workspace_id: String,
}

impl SearchHistoryItem {
    /// 将数据库的 i64 转换为 Rust 的 usize
    pub fn result_count_usize(&self) -> Option<usize> {
        self.result_count.map(|c| c as usize)
    }
}

/// 搜索历史管理器
///
/// 特性：
/// - 保存最近 N 条记录（默认 50）
/// - 自动去重（相同查询只保留最新）
/// - 按时间倒序排列（最新的在前）
/// - 持久化到 SQLite 数据库
pub struct SearchHistory {
    history: VecDeque<SearchHistoryItem>,
    max_size: usize, // 最大保存数量，默认50
    pool: Option<SqlitePool>, // 数据库连接池
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
            pool: None,
        }
    }

    /// 初始化数据库连接池
    ///
    /// # 参数
    /// * `data_dir` - 应用数据目录
    pub async fn init_db(&mut self, data_dir: &Path) -> Result<()> {
        let db_path = data_dir.join("search_history.db");
        let db_url = format!("sqlite://{}?mode=rwc", db_path.display());

        info!(path = %db_path.display(), "Initializing search history database");

        let pool = SqlitePoolOptions::new()
            .min_connections(1)
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(30))
            .idle_timeout(Duration::from_secs(600))
            .max_lifetime(Duration::from_secs(1800))
            .connect(&db_url)
            .await
            .map_err(|e| AppError::database_error(format!("Failed to connect to search history database: {}", e)))?;

        // 初始化数据库表
        Self::init_schema(&pool).await?;

        self.pool = Some(pool);
        Ok(())
    }

    /// 初始化数据库表结构
    async fn init_schema(pool: &SqlitePool) -> Result<()> {
        // 创建搜索历史表
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS search_history (
                id TEXT PRIMARY KEY,
                query TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                result_count INTEGER,
                workspace_id TEXT NOT NULL,
                created_at INTEGER NOT NULL DEFAULT (unixepoch())
            )
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| AppError::database_error(format!("Failed to create search_history table: {}", e)))?;

        // 创建工作区索引
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_search_history_workspace ON search_history(workspace_id)
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| AppError::database_error(format!("Failed to create search_history index: {}", e)))?;

        // 创建查询索引（用于去重）
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_search_history_query_ws ON search_history(query, workspace_id)
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| AppError::database_error(format!("Failed to create search_history query index: {}", e)))?;

        Ok(())
    }

    /// 检查数据库是否已初始化
    pub fn is_db_initialized(&self) -> bool {
        self.pool.is_some()
    }

    /// 添加搜索记录（内存 + 数据库）
    ///
    /// # 行为
    /// - 去重：如果已存在相同的查询，先删除旧的
    /// - 添加到前面（最新的在前）
    /// - 限制数量：超过 max_size 时删除最旧的
    /// - 同步持久化到数据库
    pub async fn add(&mut self, item: SearchHistoryItem) {
        // 去重：如果已存在相同的查询，先删除旧的
        self.history.retain(|x| x.query != item.query);

        // 添加到前面
        self.history.push_front(item.clone());

        // 限制数量
        if self.history.len() > self.max_size {
            self.history.pop_back();
        }

        // 持久化到数据库
        if let Some(ref pool) = self.pool {
            if let Err(e) = self.save_to_db(pool, &item).await {
                tracing::error!(error = %e, "Failed to save search history to database");
            }
        }
    }

    /// 保存单条记录到数据库
    async fn save_to_db(&self, pool: &SqlitePool, item: &SearchHistoryItem) -> Result<()> {
        // 使用 INSERT OR REPLACE 实现去重（相同 query + workspace_id 替换旧记录）
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO search_history (id, query, timestamp, result_count, workspace_id)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(&item.id)
        .bind(&item.query)
        .bind(item.timestamp)
        .bind(item.result_count.map(|c| c as i64))
        .bind(&item.workspace_id)
        .execute(pool)
        .await
        .map_err(|e| AppError::database_error(format!("Failed to insert search history: {}", e)))?;

        Ok(())
    }

    /// 从数据库加载指定工作区的历史记录
    pub async fn load_from_db(&mut self, workspace_id: &str) -> Result<Vec<SearchHistoryItem>> {
        if let Some(ref pool) = self.pool {
            let items = sqlx::query_as::<_, SearchHistoryItem>(
                r#"
                SELECT id, query, timestamp, result_count, workspace_id
                FROM search_history
                WHERE workspace_id = $1
                ORDER BY timestamp DESC
                LIMIT $2
                "#,
            )
            .bind(workspace_id)
            .bind(self.max_size as i64)
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::database_error(format!("Failed to load search history: {}", e)))?;

            // 更新内存状态
            self.history.clear();
            for item in &items {
                self.history.push_back(item.clone());
            }

            Ok(items)
        } else {
            Ok(Vec::new())
        }
    }

    /// 从数据库获取指定工作区的历史记录（不更新内存状态）
    pub async fn get_from_db(&self, workspace_id: &str) -> Result<Vec<SearchHistoryItem>> {
        if let Some(ref pool) = self.pool {
            let items = sqlx::query_as::<_, SearchHistoryItem>(
                r#"
                SELECT id, query, timestamp, result_count, workspace_id
                FROM search_history
                WHERE workspace_id = $1
                ORDER BY timestamp DESC
                LIMIT $2
                "#,
            )
            .bind(workspace_id)
            .bind(self.max_size as i64)
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::database_error(format!("Failed to load search history: {}", e)))?;

            Ok(items)
        } else {
            Ok(Vec::new())
        }
    }

    /// 搜索匹配的历史记录（从数据库，用于自动补全）
    pub async fn search_in_db(&self, prefix: &str, workspace_id: &str) -> Result<Vec<SearchHistoryItem>> {
        if let Some(ref pool) = self.pool {
            let prefix_lower = prefix.to_lowercase();
            let items = sqlx::query_as::<_, SearchHistoryItem>(
                r#"
                SELECT id, query, timestamp, result_count, workspace_id
                FROM search_history
                WHERE workspace_id = $1
                  AND LOWER(query) LIKE $2 || '%'
                ORDER BY timestamp DESC
                LIMIT 20
                "#,
            )
            .bind(workspace_id)
            .bind(prefix_lower)
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::database_error(format!("Failed to search history: {}", e)))?;

            Ok(items)
        } else {
            Ok(Vec::new())
        }
    }

    /// 从数据库删除单条记录
    pub async fn remove_from_db(&self, id: &str) -> Result<()> {
        if let Some(ref pool) = self.pool {
            sqlx::query(
                r#"
                DELETE FROM search_history WHERE id = $1
                "#,
            )
            .bind(id)
            .execute(pool)
            .await
            .map_err(|e| AppError::database_error(format!("Failed to delete search history: {}", e)))?;
        }
        Ok(())
    }

    /// 清空指定工作区的历史记录
    pub async fn clear_workspace_from_db(&self, workspace_id: &str) -> Result<()> {
        if let Some(ref pool) = self.pool {
            sqlx::query(
                r#"
                DELETE FROM search_history WHERE workspace_id = $1
                "#,
            )
            .bind(workspace_id)
            .execute(pool)
            .await
            .map_err(|e| AppError::database_error(format!("Failed to clear search history: {}", e)))?;
        }
        Ok(())
    }

    /// 清空所有历史记录
    pub async fn clear_all_from_db(&self) -> Result<()> {
        if let Some(ref pool) = self.pool {
            sqlx::query(
                r#"
                DELETE FROM search_history
                "#,
            )
            .execute(pool)
            .await
            .map_err(|e| AppError::database_error(format!("Failed to clear all search history: {}", e)))?;
        }
        Ok(())
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

    /// 搜索匹配的历史记录（用于自动补全，内存版本）
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

    /// 同步添加（用于测试，不进行数据库操作）
    #[cfg(test)]
    pub fn add_sync(&mut self, item: SearchHistoryItem) {
        // 去重：如果已存在相同的查询，先删除旧的
        self.history.retain(|x| x.query != item.query);

        // 添加到前面
        self.history.push_front(item);

        // 限制数量
        if self.history.len() > self.max_size {
            self.history.pop_back();
        }
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
            result_count: result_count.map(|c| c as i64),
            workspace_id: workspace_id.to_string(),
        }
    }

    #[test]
    fn test_add_item() {
        let mut history = SearchHistory::new(50);
        assert_eq!(history.len(), 0);

        let item = create_test_item("1", "ERROR", "ws1", Some(10));
        history.add_sync(item);

        assert_eq!(history.len(), 1);
        assert_eq!(history.get_all()[0].query, "ERROR");
    }

    #[test]
    fn test_add_duplicate_query() {
        let mut history = SearchHistory::new(50);

        // 添加相同查询两次
        history.add_sync(create_test_item("1", "ERROR", "ws1", Some(10)));
        history.add_sync(create_test_item("2", "ERROR", "ws1", Some(20)));

        // 应该只保留最新的（id=2）
        assert_eq!(history.len(), 1);
        assert_eq!(history.get_all()[0].id, "2");
        assert_eq!(history.get_all()[0].result_count, Some(20i64));
    }

    #[test]
    fn test_max_size_limit() {
        let mut history = SearchHistory::new(3);

        history.add_sync(create_test_item("1", "query1", "ws1", None));
        history.add_sync(create_test_item("2", "query2", "ws1", None));
        history.add_sync(create_test_item("3", "query3", "ws1", None));
        history.add_sync(create_test_item("4", "query4", "ws1", None));

        // 应该只保留最新的 3 条
        assert_eq!(history.len(), 3);
        assert_eq!(history.get_all()[0].id, "4"); // 最新
        assert_eq!(history.get_all()[2].id, "2"); // 最旧的
    }

    #[test]
    fn test_filter_by_workspace() {
        let mut history = SearchHistory::new(50);

        history.add_sync(create_test_item("1", "ERROR", "ws1", Some(10)));
        history.add_sync(create_test_item("2", "WARN", "ws2", Some(5)));
        history.add_sync(create_test_item("3", "INFO", "ws1", Some(3)));

        let ws1_history = history.get_by_workspace("ws1");
        assert_eq!(ws1_history.len(), 2);
        assert_eq!(ws1_history[0].query, "INFO"); // 最新的在前
        assert_eq!(ws1_history[1].query, "ERROR");
    }

    #[test]
    fn test_search_prefix() {
        let mut history = SearchHistory::new(50);

        history.add_sync(create_test_item("1", "ERROR", "ws1", Some(10)));
        history.add_sync(create_test_item("2", "WARN", "ws1", Some(5)));
        history.add_sync(create_test_item("3", "ERR", "ws1", Some(3)));

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

        history.add_sync(create_test_item("1", "De H", "ws1", Some(10)));
        history.add_sync(create_test_item("2", "DE N", "ws1", Some(5)));

        let results = history.search("de", "ws1");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_remove_item() {
        let mut history = SearchHistory::new(50);

        history.add_sync(create_test_item("1", "ERROR", "ws1", Some(10)));
        history.add_sync(create_test_item("2", "WARN", "ws1", Some(5)));

        history.remove("1");
        assert_eq!(history.len(), 1);
        assert_eq!(history.get_all()[0].id, "2");
    }

    #[test]
    fn test_clear_all() {
        let mut history = SearchHistory::new(50);

        history.add_sync(create_test_item("1", "ERROR", "ws1", Some(10)));
        history.add_sync(create_test_item("2", "WARN", "ws1", Some(5)));

        history.clear();
        assert_eq!(history.len(), 0);
        assert!(history.is_empty());
    }

    #[test]
    fn test_order_by_time_desc() {
        let mut history = SearchHistory::new(50);

        history.add_sync(SearchHistoryItem {
            id: "1".to_string(),
            query: "ERROR".to_string(),
            timestamp: 1000,
            result_count: None,
            workspace_id: "ws1".to_string(),
        });

        history.add_sync(SearchHistoryItem {
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
        history.add_sync(create_test_item("1", "ERROR", "ws1", Some(10)));

        let results = history.search("", "ws1");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_workspace_isolation() {
        let mut history = SearchHistory::new(50);

        history.add_sync(create_test_item("1", "ERROR", "ws1", Some(10)));
        history.add_sync(create_test_item("2", "WARN", "ws2", Some(5)));

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
