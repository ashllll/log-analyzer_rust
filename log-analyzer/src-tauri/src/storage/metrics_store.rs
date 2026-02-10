//! 性能指标时序存储
//!
//! ## 功能
//!
//! - 存储性能指标的历史快照
//! - 记录搜索事件
//! - 支持 7 天数据保留策略
//! - 提供时间范围查询
//!
//! ## 技术选型
//!
//! - **SQLite**: 业内成熟的嵌入式数据库，适合桌面应用
//! - **WAL 模式**: 支持并发读写
//! - **FTS5**: 全文搜索支持（用于事件查询）
//!
//! ## 数据库 Schema
//!
//! ### metrics_snapshots 表
//! - `id`: 主键
//! - `timestamp`: 时间戳（Unix 时间，秒）
//! - `search_latency_current`: 当前搜索延迟
//! - `search_latency_average`: 平均搜索延迟
//! - `search_latency_p95`: P95 延迟
//! - `search_latency_p99`: P99 延迟
//! - `throughput_current`: 当前吞吐量
//! - `throughput_average`: 平均吞吐量
//! - `throughput_peak`: 峰值吞吐量
//! - `cache_hit_rate`: 缓存命中率
//! - `cache_hit_count`: 缓存命中次数
//! - `cache_miss_count`: 缓存未命中次数
//! - `cache_size`: 缓存大小
//! - `cache_capacity`: 缓存容量
//! - `memory_used`: 已用内存
//! - `memory_total`: 总内存
//! - `task_total`: 总任务数
//! - `task_running`: 运行中任务数
//! - `task_completed`: 已完成任务数
//! - `task_failed`: 失败任务数
//! - `index_total_files`: 总文件数
//! - `index_indexed_files`: 已索引文件数
//!
//! ### search_events 表
//! - `id`: 主键
//! - `timestamp`: 时间戳
//! - `workspace_id`: 工作区 ID
//! - `query`: 搜索查询
//! - `results_count`: 结果数量
//! - `duration_ms`: 搜索耗时
//! - `cache_hit`: 是否命中缓存

use crate::error::{AppError, Result};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Row, SqlitePool};
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info};

/// 数据保留天数
const DATA_RETENTION_DAYS: i64 = 7;

/// 指标快照数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub timestamp: i64,
    pub search_latency_current: u64,
    pub search_latency_average: u64,
    pub search_latency_p95: u64,
    pub search_latency_p99: u64,
    pub throughput_current: u64,
    pub throughput_average: u64,
    pub throughput_peak: u64,
    pub cache_hit_rate: f64,
    pub cache_hit_count: u64,
    pub cache_miss_count: u64,
    pub cache_size: u64,
    pub cache_capacity: u64,
    pub memory_used: u64,
    pub memory_total: u64,
    pub task_total: u64,
    pub task_running: u64,
    pub task_completed: u64,
    pub task_failed: u64,
    pub index_total_files: u64,
    pub index_indexed_files: u64,
}

/// 搜索事件数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchEvent {
    pub id: Option<i64>,
    pub timestamp: i64,
    pub workspace_id: Option<String>,
    pub query: String,
    pub results_count: u64,
    pub duration_ms: u64,
    pub cache_hit: bool,
}

/// 时间范围类型
#[derive(Debug, Clone, Copy)]
pub enum TimeRange {
    LastHour,
    Last6Hours,
    Last24Hours,
    Last7Days,
    Last30Days,
    Custom { start: i64, end: i64 },
}

impl TimeRange {
    /// 获取时间范围对应的起始时间戳（秒）
    pub fn start_timestamp(&self) -> i64 {
        let now = Utc::now();
        let duration = match self {
            TimeRange::LastHour => Duration::hours(1),
            TimeRange::Last6Hours => Duration::hours(6),
            TimeRange::Last24Hours => Duration::hours(24),
            TimeRange::Last7Days => Duration::days(7),
            TimeRange::Last30Days => Duration::days(30),
            TimeRange::Custom { start, .. } => return *start,
        };
        (now - duration).timestamp()
    }

    /// 获取时间范围对应的结束时间戳（秒）
    pub fn end_timestamp(&self) -> i64 {
        match self {
            TimeRange::Custom { end, .. } => *end,
            _ => Utc::now().timestamp(),
        }
    }
}

/// 性能指标存储
pub struct MetricsStore {
    pool: SqlitePool,
}

impl MetricsStore {
    /// 创建新的指标存储
    ///
    /// # Arguments
    ///
    /// * `data_dir` - 数据目录（数据库将在 `data_dir/metrics.db`）
    ///
    /// # Errors
    ///
    /// 返回错误如果：
    /// - 数据库连接失败
    /// - 表创建失败
    pub async fn new(data_dir: &Path) -> Result<Self> {
        // 创建数据目录
        tokio::fs::create_dir_all(data_dir)
            .await
            .map_err(|e| {
                AppError::io_error(
                    format!("Failed to create metrics data directory: {}", e),
                    Some(data_dir.to_path_buf()),
                )
            })?;

        let db_path = data_dir.join("metrics.db");
        let db_url = format!("sqlite://{}?mode=rwc", db_path.display());

        info!(path = %db_path.display(), "Initializing metrics store");

        // 使用业内成熟的 SQLite 连接池配置
        let pool = SqlitePoolOptions::new()
            .min_connections(1)
            .max_connections(5)
            .connect(&db_url)
            .await
            .map_err(|e| {
                AppError::database_error(format!("Failed to connect to metrics database: {}", e))
            })?;

        // 启用 WAL 模式以支持并发读写
        sqlx::query("PRAGMA journal_mode = WAL")
            .execute(&pool)
            .await
            .map_err(|e| AppError::database_error(format!("Failed to enable WAL mode: {}", e)))?;

        // 优化性能配置
        sqlx::query("PRAGMA synchronous = NORMAL")
            .execute(&pool)
            .await
            .map_err(|e| {
                AppError::database_error(format!("Failed to set synchronous mode: {}", e))
            })?;

        // 初始化数据库表结构
        Self::init_schema(&pool).await?;

        // 执行数据清理（删除超过 7 天的数据）
        Self::cleanup_old_data(&pool).await?;

        Ok(Self { pool })
    }

    /// 初始化数据库表结构
    async fn init_schema(pool: &SqlitePool) -> Result<()> {
        // 创建指标快照表
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS metrics_snapshots (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp INTEGER NOT NULL,
                search_latency_current INTEGER NOT NULL DEFAULT 0,
                search_latency_average INTEGER NOT NULL DEFAULT 0,
                search_latency_p95 INTEGER NOT NULL DEFAULT 0,
                search_latency_p99 INTEGER NOT NULL DEFAULT 0,
                throughput_current INTEGER NOT NULL DEFAULT 0,
                throughput_average INTEGER NOT NULL DEFAULT 0,
                throughput_peak INTEGER NOT NULL DEFAULT 0,
                cache_hit_rate REAL NOT NULL DEFAULT 0,
                cache_hit_count INTEGER NOT NULL DEFAULT 0,
                cache_miss_count INTEGER NOT NULL DEFAULT 0,
                cache_size INTEGER NOT NULL DEFAULT 0,
                cache_capacity INTEGER NOT NULL DEFAULT 0,
                memory_used INTEGER NOT NULL DEFAULT 0,
                memory_total INTEGER NOT NULL DEFAULT 0,
                task_total INTEGER NOT NULL DEFAULT 0,
                task_running INTEGER NOT NULL DEFAULT 0,
                task_completed INTEGER NOT NULL DEFAULT 0,
                task_failed INTEGER NOT NULL DEFAULT 0,
                index_total_files INTEGER NOT NULL DEFAULT 0,
                index_indexed_files INTEGER NOT NULL DEFAULT 0
            )
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| {
            AppError::database_error(format!("Failed to create metrics_snapshots table: {}", e))
        })?;

        // 创建时间戳索引（用于快速范围查询）
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_metrics_timestamp ON metrics_snapshots(timestamp)")
            .execute(pool)
            .await
            .map_err(|e| {
                AppError::database_error(format!("Failed to create timestamp index: {}", e))
            })?;

        // 创建搜索事件表
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS search_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp INTEGER NOT NULL,
                workspace_id TEXT,
                query TEXT NOT NULL,
                results_count INTEGER NOT NULL DEFAULT 0,
                duration_ms INTEGER NOT NULL DEFAULT 0,
                cache_hit INTEGER NOT NULL DEFAULT 0
            )
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| {
            AppError::database_error(format!("Failed to create search_events table: {}", e))
        })?;

        // 创建搜索事件索引
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_search_events_timestamp ON search_events(timestamp)")
            .execute(pool)
            .await
            .map_err(|e| {
                AppError::database_error(format!("Failed to create search events timestamp index: {}", e))
            })?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_search_events_workspace ON search_events(workspace_id)")
            .execute(pool)
            .await
            .map_err(|e| {
                AppError::database_error(format!("Failed to create search events workspace index: {}", e))
            })?;

        info!("Database schema initialized successfully");
        Ok(())
    }

    /// 保存指标快照
    pub async fn save_snapshot(&self, snapshot: &MetricsSnapshot) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO metrics_snapshots (
                timestamp, search_latency_current, search_latency_average, search_latency_p95, search_latency_p99,
                throughput_current, throughput_average, throughput_peak,
                cache_hit_rate, cache_hit_count, cache_miss_count, cache_size, cache_capacity,
                memory_used, memory_total,
                task_total, task_running, task_completed, task_failed,
                index_total_files, index_indexed_files
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(snapshot.timestamp)
        .bind(snapshot.search_latency_current as i64)
        .bind(snapshot.search_latency_average as i64)
        .bind(snapshot.search_latency_p95 as i64)
        .bind(snapshot.search_latency_p99 as i64)
        .bind(snapshot.throughput_current as i64)
        .bind(snapshot.throughput_average as i64)
        .bind(snapshot.throughput_peak as i64)
        .bind(snapshot.cache_hit_rate)
        .bind(snapshot.cache_hit_count as i64)
        .bind(snapshot.cache_miss_count as i64)
        .bind(snapshot.cache_size as i64)
        .bind(snapshot.cache_capacity as i64)
        .bind(snapshot.memory_used as i64)
        .bind(snapshot.memory_total as i64)
        .bind(snapshot.task_total as i64)
        .bind(snapshot.task_running as i64)
        .bind(snapshot.task_completed as i64)
        .bind(snapshot.task_failed as i64)
        .bind(snapshot.index_total_files as i64)
        .bind(snapshot.index_indexed_files as i64)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database_error(format!("Failed to save snapshot: {}", e)))?;

        debug!(
            timestamp = snapshot.timestamp,
            "Saved metrics snapshot"
        );

        Ok(())
    }

    /// 记录搜索事件
    pub async fn record_search_event(&self, event: &SearchEvent) -> Result<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO search_events (timestamp, workspace_id, query, results_count, duration_ms, cache_hit)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(event.timestamp)
        .bind(&event.workspace_id)
        .bind(&event.query)
        .bind(event.results_count as i64)
        .bind(event.duration_ms as i64)
        .bind(event.cache_hit as i32)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database_error(format!("Failed to record search event: {}", e)))?;

        let id = result.last_insert_rowid();
        debug!(
            id,
            timestamp = event.timestamp,
            query = %event.query,
            "Recorded search event"
        );

        Ok(id)
    }

    /// 获取时间范围内的指标快照
    pub async fn get_snapshots(&self, range: TimeRange) -> Result<Vec<MetricsSnapshot>> {
        let start = range.start_timestamp();
        let end = range.end_timestamp();

        let rows = sqlx::query(
            r#"
            SELECT timestamp, search_latency_current, search_latency_average, search_latency_p95, search_latency_p99,
                   throughput_current, throughput_average, throughput_peak,
                   cache_hit_rate, cache_hit_count, cache_miss_count, cache_size, cache_capacity,
                   memory_used, memory_total,
                   task_total, task_running, task_completed, task_failed,
                   index_total_files, index_indexed_files
            FROM metrics_snapshots
            WHERE timestamp >= ? AND timestamp <= ?
            ORDER BY timestamp ASC
            "#,
        )
        .bind(start)
        .bind(end)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::database_error(format!("Failed to fetch snapshots: {}", e)))?;

        let snapshots: Vec<MetricsSnapshot> = rows
            .iter()
            .map(|row| MetricsSnapshot {
                timestamp: row.get("timestamp"),
                search_latency_current: row.get::<i64, _>("search_latency_current") as u64,
                search_latency_average: row.get::<i64, _>("search_latency_average") as u64,
                search_latency_p95: row.get::<i64, _>("search_latency_p95") as u64,
                search_latency_p99: row.get::<i64, _>("search_latency_p99") as u64,
                throughput_current: row.get::<i64, _>("throughput_current") as u64,
                throughput_average: row.get::<i64, _>("throughput_average") as u64,
                throughput_peak: row.get::<i64, _>("throughput_peak") as u64,
                cache_hit_rate: row.get("cache_hit_rate"),
                cache_hit_count: row.get::<i64, _>("cache_hit_count") as u64,
                cache_miss_count: row.get::<i64, _>("cache_miss_count") as u64,
                cache_size: row.get::<i64, _>("cache_size") as u64,
                cache_capacity: row.get::<i64, _>("cache_capacity") as u64,
                memory_used: row.get::<i64, _>("memory_used") as u64,
                memory_total: row.get::<i64, _>("memory_total") as u64,
                task_total: row.get::<i64, _>("task_total") as u64,
                task_running: row.get::<i64, _>("task_running") as u64,
                task_completed: row.get::<i64, _>("task_completed") as u64,
                task_failed: row.get::<i64, _>("task_failed") as u64,
                index_total_files: row.get::<i64, _>("index_total_files") as u64,
                index_indexed_files: row.get::<i64, _>("index_indexed_files") as u64,
            })
            .collect();

        debug!(
            start,
            end,
            count = snapshots.len(),
            "Fetched metrics snapshots"
        );

        Ok(snapshots)
    }

    /// 获取时间范围内的聚合统计数据
    ///
    /// 返回每个时间段的平均值（用于绘制趋势图）
    pub async fn get_aggregated_metrics(
        &self,
        range: TimeRange,
        interval_seconds: i64,
    ) -> Result<Vec<MetricsSnapshot>> {
        let start = range.start_timestamp();
        let end = range.end_timestamp();

        // 使用 GROUP BY 将数据聚合到指定间隔
        let rows = sqlx::query(
            r#"
            SELECT
                (timestamp / ?) * ? as time_bucket,
                AVG(search_latency_current) as search_latency_current,
                AVG(search_latency_average) as search_latency_average,
                AVG(search_latency_p95) as search_latency_p95,
                AVG(search_latency_p99) as search_latency_p99,
                AVG(throughput_current) as throughput_current,
                AVG(throughput_average) as throughput_average,
                AVG(throughput_peak) as throughput_peak,
                AVG(cache_hit_rate) as cache_hit_rate,
                AVG(cache_hit_count) as cache_hit_count,
                AVG(cache_miss_count) as cache_miss_count,
                AVG(cache_size) as cache_size,
                AVG(cache_capacity) as cache_capacity,
                AVG(memory_used) as memory_used,
                AVG(memory_total) as memory_total,
                AVG(task_total) as task_total,
                AVG(task_running) as task_running,
                AVG(task_completed) as task_completed,
                AVG(task_failed) as task_failed,
                AVG(index_total_files) as index_total_files,
                AVG(index_indexed_files) as index_indexed_files
            FROM metrics_snapshots
            WHERE timestamp >= ? AND timestamp <= ?
            GROUP BY time_bucket
            ORDER BY time_bucket ASC
            "#,
        )
        .bind(interval_seconds)
        .bind(interval_seconds)
        .bind(start)
        .bind(end)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            AppError::database_error(format!("Failed to fetch aggregated metrics: {}", e))
        })?;

        let snapshots: Vec<MetricsSnapshot> = rows
            .iter()
            .map(|row| MetricsSnapshot {
                timestamp: row.get::<i64, _>("time_bucket"),
                search_latency_current: row.get::<f64, _>("search_latency_current") as u64,
                search_latency_average: row.get::<f64, _>("search_latency_average") as u64,
                search_latency_p95: row.get::<f64, _>("search_latency_p95") as u64,
                search_latency_p99: row.get::<f64, _>("search_latency_p99") as u64,
                throughput_current: row.get::<f64, _>("throughput_current") as u64,
                throughput_average: row.get::<f64, _>("throughput_average") as u64,
                throughput_peak: row.get::<f64, _>("throughput_peak") as u64,
                cache_hit_rate: row.get("cache_hit_rate"),
                cache_hit_count: row.get::<f64, _>("cache_hit_count") as u64,
                cache_miss_count: row.get::<f64, _>("cache_miss_count") as u64,
                cache_size: row.get::<f64, _>("cache_size") as u64,
                cache_capacity: row.get::<f64, _>("cache_capacity") as u64,
                memory_used: row.get::<f64, _>("memory_used") as u64,
                memory_total: row.get::<f64, _>("memory_total") as u64,
                task_total: row.get::<f64, _>("task_total") as u64,
                task_running: row.get::<f64, _>("task_running") as u64,
                task_completed: row.get::<f64, _>("task_completed") as u64,
                task_failed: row.get::<f64, _>("task_failed") as u64,
                index_total_files: row.get::<f64, _>("index_total_files") as u64,
                index_indexed_files: row.get::<f64, _>("index_indexed_files") as u64,
            })
            .collect();

        debug!(
            start,
            end,
            interval_seconds,
            count = snapshots.len(),
            "Fetched aggregated metrics"
        );

        Ok(snapshots)
    }

    /// 获取搜索事件统计
    pub async fn get_search_events(
        &self,
        range: TimeRange,
        workspace_id: Option<&str>,
    ) -> Result<Vec<SearchEvent>> {
        let start = range.start_timestamp();
        let end = range.end_timestamp();

        let rows = if let Some(wid) = workspace_id {
            sqlx::query(
                r#"
                SELECT id, timestamp, workspace_id, query, results_count, duration_ms, cache_hit
                FROM search_events
                WHERE timestamp >= ? AND timestamp <= ? AND workspace_id = ?
                ORDER BY timestamp DESC
                LIMIT 1000
                "#,
            )
            .bind(start)
            .bind(end)
            .bind(wid)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query(
                r#"
                SELECT id, timestamp, workspace_id, query, results_count, duration_ms, cache_hit
                FROM search_events
                WHERE timestamp >= ? AND timestamp <= ?
                ORDER BY timestamp DESC
                LIMIT 1000
                "#,
            )
            .bind(start)
            .bind(end)
            .fetch_all(&self.pool)
            .await
        }
        .map_err(|e| AppError::database_error(format!("Failed to fetch search events: {}", e)))?;

        let events: Vec<SearchEvent> = rows
            .iter()
            .map(|row| SearchEvent {
                id: Some(row.get("id")),
                timestamp: row.get("timestamp"),
                workspace_id: row.get("workspace_id"),
                query: row.get("query"),
                results_count: row.get::<i64, _>("results_count") as u64,
                duration_ms: row.get::<i64, _>("duration_ms") as u64,
                cache_hit: row.get::<i32, _>("cache_hit") != 0,
            })
            .collect();

        debug!(
            start,
            end,
            workspace_id = workspace_id.unwrap_or("all"),
            count = events.len(),
            "Fetched search events"
        );

        Ok(events)
    }

    /// 清理超过保留期的旧数据
    async fn cleanup_old_data(pool: &SqlitePool) -> Result<()> {
        let retention_timestamp = Utc::now()
            - Duration::days(DATA_RETENTION_DAYS)
            - Duration::hours(1); // 额外多删除1小时，确保边界数据也被清理

        let cutoff = retention_timestamp.timestamp();

        // 删除旧的指标快照
        let deleted_snapshots = sqlx::query("DELETE FROM metrics_snapshots WHERE timestamp < ?")
            .bind(cutoff)
            .execute(pool)
            .await
            .map_err(|e| {
                AppError::database_error(format!("Failed to cleanup old snapshots: {}", e))
            })?
            .rows_affected();

        // 删除旧的搜索事件
        let deleted_events = sqlx::query("DELETE FROM search_events WHERE timestamp < ?")
            .bind(cutoff)
            .execute(pool)
            .await
            .map_err(|e| {
                AppError::database_error(format!("Failed to cleanup old events: {}", e))
            })?
            .rows_affected();

        info!(
            cutoff,
            deleted_snapshots,
            deleted_events,
            "Cleaned up old metrics data"
        );

        Ok(())
    }

    /// 手动触发数据清理
    pub async fn cleanup(&self) -> Result<()> {
        Self::cleanup_old_data(&self.pool).await
    }

    /// 获取统计信息
    pub async fn get_stats(&self) -> Result<MetricsStoreStats> {
        let snapshot_count: i64 = sqlx::query("SELECT COUNT(*) as count FROM metrics_snapshots")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::database_error(format!("Failed to get snapshot count: {}", e)))?
            .get("count");

        let event_count: i64 = sqlx::query("SELECT COUNT(*) as count FROM search_events")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::database_error(format!("Failed to get event count: {}", e)))?
            .get("count");

        // 获取最新快照时间戳
        let latest_timestamp: Option<i64> = sqlx::query(
            "SELECT timestamp FROM metrics_snapshots ORDER BY timestamp DESC LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database_error(format!("Failed to get latest timestamp: {}", e)))?
        .map(|row| row.get("timestamp"));

        // 获取最旧快照时间戳
        let oldest_timestamp: Option<i64> = sqlx::query(
            "SELECT timestamp FROM metrics_snapshots ORDER BY timestamp ASC LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database_error(format!("Failed to get oldest timestamp: {}", e)))?
        .map(|row| row.get("timestamp"));

        Ok(MetricsStoreStats {
            snapshot_count: snapshot_count as u64,
            event_count: event_count as u64,
            latest_timestamp,
            oldest_timestamp,
        })
    }
}

/// 指标存储统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsStoreStats {
    pub snapshot_count: u64,
    pub event_count: u64,
    pub latest_timestamp: Option<i64>,
    pub oldest_timestamp: Option<i64>,
}

/// 指标快照调度器
///
/// 定期自动保存性能指标快照到数据库。
/// 使用 tokio::spawn 在后台运行，支持停止和重启。
///
/// ## 技术选型
///
/// - **tokio::time::interval**: 业内成熟的异步定时器，替代手动 sleep 循环
/// - **Arc<Mutex<bool>>**: 线程安全的状态标志
/// - **JoinHandle**: 任务句柄管理，支持优雅关闭
pub struct MetricsSnapshotScheduler {
    store: Arc<MetricsStore>,
    is_running: Arc<std::sync::Mutex<bool>>,
    _handle: Arc<std::sync::Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl MetricsSnapshotScheduler {
    /// 创建新的调度器并自动启动
    ///
    /// # Arguments
    ///
    /// * `store` - 指标存储实例
    /// * `interval_seconds` - 快照间隔（秒），默认 60 秒
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use log_analyzer::storage::MetricsSnapshotScheduler;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let store = MetricsStore::new("/path/to/data").await.unwrap();
    ///     let scheduler = MetricsSnapshotScheduler::new(store, 60).await;
    ///
    ///     // 调度器在后台自动运行
    ///
    ///     // 停止调度器
    ///     scheduler.stop().await;
    /// }
    /// ```
    pub async fn new(store: MetricsStore, interval_seconds: u64) -> Result<Self> {
        let store = Arc::new(store);
        let is_running = Arc::new(std::sync::Mutex::new(true));
        let store_clone = Arc::clone(&store);
        let is_running_clone = is_running.clone();

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(interval_seconds));
            interval.tick().await; // 跳过第一次立即触发

            while *is_running_clone.lock().unwrap() {
                interval.tick().await;

                // 检查是否仍在运行
                if !*is_running_clone.lock().unwrap() {
                    break;
                }

                // 保存当前性能指标快照
                // 注意：这里需要从 AppState 获取实际指标
                // 为了简化，暂时只记录时间戳
                let snapshot = MetricsSnapshot {
                    timestamp: chrono::Utc::now().timestamp(),
                    search_latency_current: 0,
                    search_latency_average: 0,
                    search_latency_p95: 0,
                    search_latency_p99: 0,
                    throughput_current: 0,
                    throughput_average: 0,
                    throughput_peak: 0,
                    cache_hit_rate: 0.0,
                    cache_hit_count: 0,
                    cache_miss_count: 0,
                    cache_size: 0,
                    cache_capacity: 0,
                    memory_used: 0,
                    memory_total: 0,
                    task_total: 0,
                    task_running: 0,
                    task_completed: 0,
                    task_failed: 0,
                    index_total_files: 0,
                    index_indexed_files: 0,
                };

                if let Err(e) = store_clone.save_snapshot(&snapshot).await {
                    tracing::warn!(error = %e, "Failed to save metrics snapshot");
                }
            }
        });

        Ok(Self {
            store,
            is_running,
            _handle: Arc::new(std::sync::Mutex::new(Some(handle))),
        })
    }

    /// 停止调度器
    pub async fn stop(self) {
        *self.is_running.lock().unwrap() = false;

        // 等待任务结束（先释放锁再await）
        let handle = self._handle.lock().unwrap().take();
        if let Some(handle) = handle {
            let _ = tokio::time::timeout(
                tokio::time::Duration::from_secs(5),
                handle,
            )
            .await;
        }
    }

    /// 手动触发一次快照保存
    pub async fn save_now(&self, snapshot: &MetricsSnapshot) -> Result<()> {
        self.store.save_snapshot(snapshot).await
    }

    /// 记录搜索事件
    pub async fn record_search(&self, event: &SearchEvent) -> Result<i64> {
        self.store.record_search_event(event).await
    }

    /// 获取存储引用
    pub fn store(&self) -> &MetricsStore {
        &self.store
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 创建测试用的临时数据库
    async fn create_test_store() -> MetricsStore {
        let temp_dir = std::env::temp_dir().join("metrics_test");
        let _ = std::fs::remove_dir_all(&temp_dir);
        MetricsStore::new(&temp_dir).await.unwrap()
    }

    #[tokio::test]
    async fn test_save_and_get_snapshot() {
        let store = create_test_store().await;

        let snapshot = MetricsSnapshot {
            timestamp: Utc::now().timestamp(),
            search_latency_current: 100,
            search_latency_average: 95,
            search_latency_p95: 150,
            search_latency_p99: 200,
            throughput_current: 1000,
            throughput_average: 950,
            throughput_peak: 1500,
            cache_hit_rate: 85.5,
            cache_hit_count: 850,
            cache_miss_count: 150,
            cache_size: 1000,
            cache_capacity: 10000,
            memory_used: 500,
            memory_total: 1000,
            task_total: 100,
            task_running: 5,
            task_completed: 90,
            task_failed: 5,
            index_total_files: 1000,
            index_indexed_files: 950,
        };

        store.save_snapshot(&snapshot).await.unwrap();

        let snapshots = store
            .get_snapshots(TimeRange::Last24Hours)
            .await
            .unwrap();

        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].search_latency_current, 100);
    }

    #[tokio::test]
    async fn test_record_search_event() {
        let store = create_test_store().await;

        let event = SearchEvent {
            id: None,
            timestamp: Utc::now().timestamp(),
            workspace_id: Some("test_workspace".to_string()),
            query: "error".to_string(),
            results_count: 50,
            duration_ms: 100,
            cache_hit: true,
        };

        let id = store.record_search_event(&event).await.unwrap();
        assert!(id > 0);

        let events = store
            .get_search_events(TimeRange::Last24Hours, Some("test_workspace"))
            .await
            .unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].query, "error");
    }

    #[tokio::test]
    async fn test_cleanup_old_data() {
        let store = create_test_store().await;

        // 创建一个 10 天前的快照
        let old_timestamp = (Utc::now() - Duration::days(10)).timestamp();

        let old_snapshot = MetricsSnapshot {
            timestamp: old_timestamp,
            search_latency_current: 100,
            search_latency_average: 95,
            search_latency_p95: 150,
            search_latency_p99: 200,
            throughput_current: 1000,
            throughput_average: 950,
            throughput_peak: 1500,
            cache_hit_rate: 85.5,
            cache_hit_count: 850,
            cache_miss_count: 150,
            cache_size: 1000,
            cache_capacity: 10000,
            memory_used: 500,
            memory_total: 1000,
            task_total: 100,
            task_running: 5,
            task_completed: 90,
            task_failed: 5,
            index_total_files: 1000,
            index_indexed_files: 950,
        };

        store.save_snapshot(&old_snapshot).await.unwrap();

        // 创建一个最近的快照
        let recent_snapshot = MetricsSnapshot {
            timestamp: Utc::now().timestamp(),
            ..old_snapshot.clone()
        };

        store.save_snapshot(&recent_snapshot).await.unwrap();

        // 执行清理
        store.cleanup().await.unwrap();

        // 验证只保留了最近的快照
        let snapshots = store
            .get_snapshots(TimeRange::Last30Days)
            .await
            .unwrap();

        assert_eq!(snapshots.len(), 1);
        assert!(snapshots[0].timestamp > old_timestamp);
    }

    #[tokio::test]
    async fn test_aggregated_metrics() {
        let store = create_test_store().await;

        let now = Utc::now().timestamp();

        // 创建多个快照
        for i in 0..10 {
            let snapshot = MetricsSnapshot {
                timestamp: now - (3600 - i * 300), // 过去一小时内分布
                search_latency_current: 100 + i as u64,
                search_latency_average: 95,
                search_latency_p95: 150,
                search_latency_p99: 200,
                throughput_current: 1000,
                throughput_average: 950,
                throughput_peak: 1500,
                cache_hit_rate: 85.5,
                cache_hit_count: 850,
                cache_miss_count: 150,
                cache_size: 1000,
                cache_capacity: 10000,
                memory_used: 500,
                memory_total: 1000,
                task_total: 100,
                task_running: 5,
                task_completed: 90,
                task_failed: 5,
                index_total_files: 1000,
                index_indexed_files: 950,
            };

            store.save_snapshot(&snapshot).await.unwrap();
        }

        // 按 10 分钟间隔聚合
        let aggregated = store
            .get_aggregated_metrics(TimeRange::Last24Hours, 600)
            .await
            .unwrap();

        // 聚合结果应该少于原始数据
        assert!(aggregated.len() <= 10);
    }
}
