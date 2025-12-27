//! 搜索过滤器和性能指标数据结构
//!
//! 本模块定义了搜索过滤条件和性能监控相关的数据结构。

use serde::{Deserialize, Serialize};

/// 高级搜索过滤器
///
/// 支持按时间范围、日志级别和文件模式等条件过滤搜索结果。
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct SearchFilters {
    /// 开始时间（ISO 8601 格式）
    pub time_start: Option<String>,
    /// 结束时间（ISO 8601 格式）
    pub time_end: Option<String>,
    /// 允许的日志级别列表
    pub levels: Vec<String>,
    /// 文件路径匹配模式
    pub file_pattern: Option<String>,
}

/// 性能监控指标
///
/// 记录应用运行时的性能数据，用于性能分析和优化。
#[derive(Serialize, Clone, Debug)]
pub struct PerformanceMetrics {
    /// 当前进程内存使用量（MB）
    pub memory_used_mb: f64,
    /// 索引文件数量
    pub indexed_file_count: usize,
    /// 搜索缓存条目数
    pub cache_size: usize,
    /// 最近一次搜索耗时（毫秒）
    pub last_search_duration_ms: u64,
    /// 缓存命中率（0-100）
    pub cache_hit_rate: f64,
    /// 已索引文件数量
    pub indexed_files_count: usize,
    /// 索引文件磁盘大小（MB）
    pub index_file_size_mb: f64,
}
