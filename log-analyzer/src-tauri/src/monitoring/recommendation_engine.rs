//! 性能优化建议引擎
//!
//! 基于规则引擎的智能优化建议系统
//! 采用业内成熟的专家系统架构，支持：
//! - 多维度性能分析
//! - 优先级排序
//! - 趋势检测
//! - 根因分析

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use super::metrics_collector::{CacheMetricsSnapshot, QueryTimingStats, SystemResourceMetrics};

/// 优化建议
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    /// 建议 ID
    pub id: String,
    /// 建议标题
    pub title: String,
    /// 详细描述
    pub description: String,
    /// 优先级 (1-5, 5 最高)
    pub priority: u8,
    /// 类别
    pub category: RecommendationCategory,
    /// 影响范围
    pub impact: ImpactLevel,
    /// 置信度 (0.0-1.0)
    pub confidence: f64,
    /// 相关指标
    pub metrics: HashMap<String, f64>,
    /// 建议的操作步骤
    pub actions: Vec<String>,
}

/// 建议类别
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum RecommendationCategory {
    /// 查询性能
    QueryPerformance,
    /// 缓存优化
    CacheOptimization,
    /// 内存管理
    MemoryManagement,
    /// CPU 优化
    CpuOptimization,
    /// 索引优化
    IndexOptimization,
    /// 配置调优
    Configuration,
}

/// 影响级别
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ImpactLevel {
    /// 低影响
    Low,
    /// 中等影响
    Medium,
    /// 高影响
    High,
    /// 严重影响
    Critical,
}

/// 性能快照
#[derive(Debug, Clone)]
pub struct PerformanceSnapshot {
    pub query_stats: QueryTimingStats,
    pub cache_metrics: CacheMetricsSnapshot,
    pub system_metrics: Option<SystemResourceMetrics>,
    pub timestamp: std::time::SystemTime,
}

/// 优化建议引擎
pub struct RecommendationEngine {
    /// 规则集
    rules: Vec<Box<dyn RecommendationRule + Send + Sync>>,
    /// 历史快照（用于趋势分析）
    history: Arc<parking_lot::RwLock<Vec<PerformanceSnapshot>>>,
    /// 最大历史记录数
    max_history: usize,
}

/// 建议规则 trait
pub trait RecommendationRule: Send + Sync {
    /// 规则名称
    fn name(&self) -> &str;

    /// 评估规则，返回建议（如果适用）
    fn evaluate(
        &self,
        snapshot: &PerformanceSnapshot,
        history: &[PerformanceSnapshot],
    ) -> Option<Recommendation>;

    /// 规则优先级
    fn priority(&self) -> u8 {
        3
    }
}

impl RecommendationEngine {
    /// 创建新的建议引擎
    pub fn new() -> Self {
        let mut engine = Self {
            rules: Vec::new(),
            history: Arc::new(parking_lot::RwLock::new(Vec::new())),
            max_history: 100, // 保留最近 100 个快照
        };

        // 注册内置规则
        engine.register_builtin_rules();

        engine
    }

    /// 注册内置规则
    fn register_builtin_rules(&mut self) {
        self.add_rule(Box::new(QueryPerformanceRule));
        self.add_rule(Box::new(CacheHitRateRule));
        self.add_rule(Box::new(MemoryUsageRule));
        self.add_rule(Box::new(CpuUsageRule));
        self.add_rule(Box::new(QueryTrendRule));
        self.add_rule(Box::new(CacheEfficiencyRule));
        self.add_rule(Box::new(ResourceBalanceRule));
    }

    /// 添加规则
    pub fn add_rule(&mut self, rule: Box<dyn RecommendationRule + Send + Sync>) {
        self.rules.push(rule);
    }

    /// 记录性能快照
    pub fn record_snapshot(&self, snapshot: PerformanceSnapshot) {
        let mut history = self.history.write();
        history.push(snapshot);

        // 限制历史记录数量
        let len = history.len();
        if len > self.max_history {
            history.drain(0..len - self.max_history);
        }
    }

    /// 生成优化建议
    pub fn generate_recommendations(&self, current: &PerformanceSnapshot) -> Vec<Recommendation> {
        let history = self.history.read();
        let mut recommendations = Vec::new();

        // 评估所有规则
        for rule in &self.rules {
            if let Some(rec) = rule.evaluate(current, &history) {
                recommendations.push(rec);
            }
        }

        // 按优先级和置信度排序
        recommendations.sort_by(|a, b| {
            let a_score = (a.priority as f64) * a.confidence;
            let b_score = (b.priority as f64) * b.confidence;
            b_score
                .partial_cmp(&a_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        recommendations
    }
}

impl Default for RecommendationEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 内置规则实现
// ============================================================================

/// 查询性能规则
struct QueryPerformanceRule;

impl RecommendationRule for QueryPerformanceRule {
    fn name(&self) -> &str {
        "query_performance"
    }

    fn evaluate(
        &self,
        snapshot: &PerformanceSnapshot,
        _history: &[PerformanceSnapshot],
    ) -> Option<Recommendation> {
        let avg_ms = snapshot.query_stats.avg_total_ms;

        if avg_ms > 500.0 {
            let mut metrics = HashMap::new();
            metrics.insert("avg_query_time_ms".to_string(), avg_ms);

            Some(Recommendation {
                id: "query_perf_slow".to_string(),
                title: "查询响应时间过慢".to_string(),
                description: format!(
                    "当前平均查询时间为 {:.2}ms，远超推荐值 200ms。这可能导致用户体验下降。",
                    avg_ms
                ),
                priority: 5,
                category: RecommendationCategory::QueryPerformance,
                impact: ImpactLevel::Critical,
                confidence: 0.95,
                metrics,
                actions: vec![
                    "检查索引是否正确构建".to_string(),
                    "考虑增加查询缓存大小".to_string(),
                    "优化正则表达式查询模式".to_string(),
                    "减少单次查询的数据量".to_string(),
                ],
            })
        } else if avg_ms > 200.0 {
            let mut metrics = HashMap::new();
            metrics.insert("avg_query_time_ms".to_string(), avg_ms);

            Some(Recommendation {
                id: "query_perf_moderate".to_string(),
                title: "查询性能有优化空间".to_string(),
                description: format!(
                    "当前平均查询时间为 {:.2}ms，超过推荐值 200ms。建议进行优化。",
                    avg_ms
                ),
                priority: 3,
                category: RecommendationCategory::QueryPerformance,
                impact: ImpactLevel::Medium,
                confidence: 0.85,
                metrics,
                actions: vec![
                    "检查查询模式是否可以优化".to_string(),
                    "考虑启用查询结果缓存".to_string(),
                ],
            })
        } else {
            None
        }
    }

    fn priority(&self) -> u8 {
        5
    }
}

/// 缓存命中率规则
struct CacheHitRateRule;

impl RecommendationRule for CacheHitRateRule {
    fn name(&self) -> &str {
        "cache_hit_rate"
    }

    fn evaluate(
        &self,
        snapshot: &PerformanceSnapshot,
        _history: &[PerformanceSnapshot],
    ) -> Option<Recommendation> {
        let hit_rate = snapshot.cache_metrics.l1_hit_rate;

        if hit_rate < 0.5 {
            let mut metrics = HashMap::new();
            metrics.insert("cache_hit_rate".to_string(), hit_rate);

            Some(Recommendation {
                id: "cache_hit_low".to_string(),
                title: "缓存命中率过低".to_string(),
                description: format!(
                    "当前缓存命中率为 {:.1}%，远低于推荐值 70%。这会导致大量磁盘 I/O 操作。",
                    hit_rate * 100.0
                ),
                priority: 4,
                category: RecommendationCategory::CacheOptimization,
                impact: ImpactLevel::High,
                confidence: 0.9,
                metrics,
                actions: vec![
                    "增加 L1 缓存大小（当前可能过小）".to_string(),
                    "检查查询模式是否导致缓存失效".to_string(),
                    "考虑启用预加载策略".to_string(),
                    "优化缓存淘汰算法（LRU -> LFU）".to_string(),
                ],
            })
        } else if hit_rate < 0.7 {
            let mut metrics = HashMap::new();
            metrics.insert("cache_hit_rate".to_string(), hit_rate);

            Some(Recommendation {
                id: "cache_hit_moderate".to_string(),
                title: "缓存命中率可以提升".to_string(),
                description: format!(
                    "当前缓存命中率为 {:.1}%，低于推荐值 70%。提升缓存命中率可显著改善性能。",
                    hit_rate * 100.0
                ),
                priority: 3,
                category: RecommendationCategory::CacheOptimization,
                impact: ImpactLevel::Medium,
                confidence: 0.8,
                metrics,
                actions: vec![
                    "适当增加缓存大小".to_string(),
                    "分析热点数据访问模式".to_string(),
                ],
            })
        } else {
            None
        }
    }

    fn priority(&self) -> u8 {
        4
    }
}

/// 内存使用规则
struct MemoryUsageRule;

impl RecommendationRule for MemoryUsageRule {
    fn name(&self) -> &str {
        "memory_usage"
    }

    fn evaluate(
        &self,
        snapshot: &PerformanceSnapshot,
        _history: &[PerformanceSnapshot],
    ) -> Option<Recommendation> {
        let sys_metrics = snapshot.system_metrics.as_ref()?;
        let mem_usage = sys_metrics.memory_usage_percent;

        if mem_usage > 90.0 {
            let mut metrics = HashMap::new();
            metrics.insert("memory_usage_percent".to_string(), mem_usage);

            Some(Recommendation {
                id: "memory_critical".to_string(),
                title: "内存使用率严重过高".to_string(),
                description: format!(
                    "当前内存使用率为 {:.1}%，已接近系统极限。可能导致系统不稳定或崩溃。",
                    mem_usage
                ),
                priority: 5,
                category: RecommendationCategory::MemoryManagement,
                impact: ImpactLevel::Critical,
                confidence: 0.95,
                metrics,
                actions: vec![
                    "立即释放不必要的缓存".to_string(),
                    "关闭其他占用内存的应用程序".to_string(),
                    "考虑增加系统内存".to_string(),
                    "检查是否存在内存泄漏".to_string(),
                ],
            })
        } else if mem_usage > 80.0 {
            let mut metrics = HashMap::new();
            metrics.insert("memory_usage_percent".to_string(), mem_usage);

            Some(Recommendation {
                id: "memory_high".to_string(),
                title: "内存使用率偏高".to_string(),
                description: format!(
                    "当前内存使用率为 {:.1}%，建议采取措施降低内存占用。",
                    mem_usage
                ),
                priority: 4,
                category: RecommendationCategory::MemoryManagement,
                impact: ImpactLevel::High,
                confidence: 0.85,
                metrics,
                actions: vec![
                    "减小缓存大小".to_string(),
                    "清理临时文件".to_string(),
                    "优化数据结构占用".to_string(),
                ],
            })
        } else {
            None
        }
    }

    fn priority(&self) -> u8 {
        5
    }
}

/// CPU 使用规则
struct CpuUsageRule;

impl RecommendationRule for CpuUsageRule {
    fn name(&self) -> &str {
        "cpu_usage"
    }

    fn evaluate(
        &self,
        snapshot: &PerformanceSnapshot,
        _history: &[PerformanceSnapshot],
    ) -> Option<Recommendation> {
        let sys_metrics = snapshot.system_metrics.as_ref()?;
        let cpu_usage = sys_metrics.cpu_usage_percent;

        if cpu_usage > 85.0 {
            let mut metrics = HashMap::new();
            metrics.insert("cpu_usage_percent".to_string(), cpu_usage);

            Some(Recommendation {
                id: "cpu_high".to_string(),
                title: "CPU 使用率过高".to_string(),
                description: format!(
                    "当前 CPU 使用率为 {:.1}%，可能影响系统响应速度。",
                    cpu_usage
                ),
                priority: 4,
                category: RecommendationCategory::CpuOptimization,
                impact: ImpactLevel::High,
                confidence: 0.85,
                metrics,
                actions: vec![
                    "减少并发查询数量".to_string(),
                    "优化正则表达式匹配算法".to_string(),
                    "考虑使用增量索引".to_string(),
                ],
            })
        } else {
            None
        }
    }

    fn priority(&self) -> u8 {
        4
    }
}

/// 查询趋势规则（基于历史数据）
struct QueryTrendRule;

impl RecommendationRule for QueryTrendRule {
    fn name(&self) -> &str {
        "query_trend"
    }

    fn evaluate(
        &self,
        snapshot: &PerformanceSnapshot,
        history: &[PerformanceSnapshot],
    ) -> Option<Recommendation> {
        if history.len() < 10 {
            return None; // 数据不足
        }

        // 计算最近 10 次的平均查询时间
        let recent_avg: f64 = history
            .iter()
            .rev()
            .take(10)
            .map(|s| s.query_stats.avg_total_ms)
            .sum::<f64>()
            / 10.0;

        let current_avg = snapshot.query_stats.avg_total_ms;

        // 检测性能下降趋势
        if current_avg > recent_avg * 1.5 {
            let mut metrics = HashMap::new();
            metrics.insert("current_avg_ms".to_string(), current_avg);
            metrics.insert("recent_avg_ms".to_string(), recent_avg);
            metrics.insert("degradation_ratio".to_string(), current_avg / recent_avg);

            Some(Recommendation {
                id: "query_trend_degradation".to_string(),
                title: "检测到查询性能下降趋势".to_string(),
                description: format!(
                    "当前查询时间 {:.2}ms 比最近平均值 {:.2}ms 高出 {:.1}%，性能呈下降趋势。",
                    current_avg,
                    recent_avg,
                    ((current_avg / recent_avg - 1.0) * 100.0)
                ),
                priority: 4,
                category: RecommendationCategory::QueryPerformance,
                impact: ImpactLevel::High,
                confidence: 0.8,
                metrics,
                actions: vec![
                    "检查是否有新的大文件导入".to_string(),
                    "考虑重建索引".to_string(),
                    "检查磁盘空间是否充足".to_string(),
                    "分析是否有资源竞争".to_string(),
                ],
            })
        } else {
            None
        }
    }

    fn priority(&self) -> u8 {
        4
    }
}

/// 缓存效率规则
struct CacheEfficiencyRule;

impl RecommendationRule for CacheEfficiencyRule {
    fn name(&self) -> &str {
        "cache_efficiency"
    }

    fn evaluate(
        &self,
        snapshot: &PerformanceSnapshot,
        _history: &[PerformanceSnapshot],
    ) -> Option<Recommendation> {
        let cache = &snapshot.cache_metrics;

        // 检查缓存命中率和请求量的关系
        // 如果请求量大但命中率低，说明缓存策略可能需要调整
        if cache.total_requests > 1000 && cache.l1_hit_rate < 0.5 {
            let mut metrics = HashMap::new();
            metrics.insert("total_requests".to_string(), cache.total_requests as f64);
            metrics.insert("hit_rate".to_string(), cache.l1_hit_rate);
            metrics.insert("eviction_rate".to_string(), cache.eviction_rate_per_minute);

            Some(Recommendation {
                id: "cache_efficiency_low".to_string(),
                title: "缓存效率不佳".to_string(),
                description: format!(
                    "处理了 {} 次请求但命中率仅 {:.1}%，缓存策略可能需要调整。",
                    cache.total_requests,
                    cache.l1_hit_rate * 100.0
                ),
                priority: 3,
                category: RecommendationCategory::CacheOptimization,
                impact: ImpactLevel::Medium,
                confidence: 0.75,
                metrics,
                actions: vec![
                    "调整缓存淘汰策略".to_string(),
                    "分析缓存访问模式".to_string(),
                    "考虑使用分层缓存".to_string(),
                    format!("当前淘汰率: {:.2}/分钟", cache.eviction_rate_per_minute),
                ],
            })
        } else {
            None
        }
    }

    fn priority(&self) -> u8 {
        3
    }
}

/// 资源平衡规则
struct ResourceBalanceRule;

impl RecommendationRule for ResourceBalanceRule {
    fn name(&self) -> &str {
        "resource_balance"
    }

    fn evaluate(
        &self,
        snapshot: &PerformanceSnapshot,
        _history: &[PerformanceSnapshot],
    ) -> Option<Recommendation> {
        let sys_metrics = snapshot.system_metrics.as_ref()?;

        // 检测资源不平衡：CPU 高但内存低，或反之
        let cpu = sys_metrics.cpu_usage_percent;
        let mem = sys_metrics.memory_usage_percent;

        if cpu > 80.0 && mem < 50.0 {
            let mut metrics = HashMap::new();
            metrics.insert("cpu_usage".to_string(), cpu);
            metrics.insert("memory_usage".to_string(), mem);

            Some(Recommendation {
                id: "resource_imbalance_cpu".to_string(),
                title: "资源使用不平衡（CPU 密集）".to_string(),
                description: format!(
                    "CPU 使用率 {:.1}% 但内存使用率仅 {:.1}%，可以通过增加缓存来减轻 CPU 负担。",
                    cpu, mem
                ),
                priority: 3,
                category: RecommendationCategory::Configuration,
                impact: ImpactLevel::Medium,
                confidence: 0.7,
                metrics,
                actions: vec![
                    "增加查询结果缓存".to_string(),
                    "启用预计算索引".to_string(),
                    "考虑使用更多内存换取 CPU 性能".to_string(),
                ],
            })
        } else if mem > 80.0 && cpu < 30.0 {
            let mut metrics = HashMap::new();
            metrics.insert("cpu_usage".to_string(), cpu);
            metrics.insert("memory_usage".to_string(), mem);

            Some(Recommendation {
                id: "resource_imbalance_memory".to_string(),
                title: "资源使用不平衡（内存密集）".to_string(),
                description: format!(
                    "内存使用率 {:.1}% 但 CPU 使用率仅 {:.1}%，可以减小缓存大小。",
                    mem, cpu
                ),
                priority: 3,
                category: RecommendationCategory::Configuration,
                impact: ImpactLevel::Medium,
                confidence: 0.7,
                metrics,
                actions: vec![
                    "减小缓存大小".to_string(),
                    "清理不必要的内存占用".to_string(),
                    "优化数据结构".to_string(),
                ],
            })
        } else {
            None
        }
    }

    fn priority(&self) -> u8 {
        3
    }
}
