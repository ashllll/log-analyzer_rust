//! Self-Tuning Cache Management
//!
//! Implements automatic cache optimization based on performance metrics:
//! - Automatic cache size adjustment based on performance metrics
//! - Dynamic eviction policy selection based on access patterns
//! - Automatic cache warming for predicted access patterns
//! - Cache performance optimization based on hit rate analysis
//!
//! **Feature: performance-optimization, Property 27: Automatic Cache Tuning**
//! **Validates: Requirements 7.3**

use crate::utils::cache_manager::CacheConfig;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tracing::info;

/// Configuration for the cache tuner
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheTunerConfig {
    /// Interval for tuning checks
    pub tuning_interval: Duration,
    /// Target hit rate (0.0 - 1.0)
    pub target_hit_rate: f64,
    /// Minimum acceptable hit rate before triggering adjustments
    pub min_acceptable_hit_rate: f64,
    /// Maximum acceptable eviction rate per minute
    pub max_eviction_rate: f64,
    /// Size adjustment step (percentage)
    pub size_adjustment_step: f64,
    /// Minimum cache size
    pub min_cache_size: u64,
    /// Maximum cache size
    pub max_cache_size: u64,
    /// History window for trend analysis
    pub history_window_size: usize,
    /// Enable automatic warming
    pub enable_auto_warming: bool,
    /// Warming threshold (access count to trigger warming)
    pub warming_threshold: u32,
    /// Cooldown between adjustments
    pub adjustment_cooldown: Duration,
}

impl Default for CacheTunerConfig {
    fn default() -> Self {
        Self {
            tuning_interval: Duration::from_secs(60),
            target_hit_rate: 0.80,
            min_acceptable_hit_rate: 0.60,
            max_eviction_rate: 10.0,
            size_adjustment_step: 10.0, // 10% adjustment
            min_cache_size: 100,
            max_cache_size: 10000,
            history_window_size: 30, // 30 minutes of history
            enable_auto_warming: true,
            warming_threshold: 5,
            adjustment_cooldown: Duration::from_secs(300), // 5 minutes
        }
    }
}

/// Eviction policy types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvictionPolicy {
    /// Least Recently Used
    LRU,
    /// Least Frequently Used
    LFU,
    /// Time-based expiration
    TTL,
    /// Adaptive (switches based on access patterns)
    Adaptive,
}

impl Default for EvictionPolicy {
    fn default() -> Self {
        Self::LRU
    }
}

/// Cache tuning action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuningAction {
    pub action_type: TuningActionType,
    pub description: String,
    pub timestamp: SystemTime,
    pub metrics_before: TuningMetrics,
    pub expected_improvement: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TuningActionType {
    IncreaseCacheSize {
        from: u64,
        to: u64,
    },
    DecreaseCacheSize {
        from: u64,
        to: u64,
    },
    ChangeEvictionPolicy {
        from: EvictionPolicy,
        to: EvictionPolicy,
    },
    TriggerWarming {
        keys_count: usize,
    },
    AdjustTTL {
        from_secs: u64,
        to_secs: u64,
    },
    NoAction,
}

/// Metrics snapshot for tuning decisions
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TuningMetrics {
    pub hit_rate: f64,
    pub eviction_rate: f64,
    pub avg_access_time_ms: f64,
    pub cache_size: u64,
    pub memory_usage_bytes: u64,
    pub hot_keys_count: usize,
}

/// History entry for trend analysis
#[derive(Debug, Clone)]
struct MetricsHistoryEntry {
    #[allow(dead_code)]
    timestamp: Instant,
    metrics: TuningMetrics,
}

/// Cache tuning state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheTuningState {
    pub current_policy: EvictionPolicy,
    pub current_size: u64,
    pub current_ttl_secs: u64,
    pub last_adjustment: Option<SystemTime>,
    pub total_adjustments: u64,
    pub recent_actions: Vec<TuningAction>,
}

impl Default for CacheTuningState {
    fn default() -> Self {
        Self {
            current_policy: EvictionPolicy::default(),
            current_size: 1000,
            current_ttl_secs: 300,
            last_adjustment: None,
            total_adjustments: 0,
            recent_actions: Vec::new(),
        }
    }
}

/// Self-tuning cache controller
///
/// **Feature: performance-optimization, Property 27: Automatic Cache Tuning**
/// **Validates: Requirements 7.3**
pub struct CacheTuner {
    config: CacheTunerConfig,
    state: Arc<RwLock<CacheTuningState>>,
    metrics_history: Arc<RwLock<VecDeque<MetricsHistoryEntry>>>,
    last_tuning_check: Arc<RwLock<Option<Instant>>>,
}

impl CacheTuner {
    /// Create a new cache tuner
    pub fn new(config: CacheTunerConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(CacheTuningState::default())),
            metrics_history: Arc::new(RwLock::new(VecDeque::new())),
            last_tuning_check: Arc::new(RwLock::new(None)),
        }
    }

    /// Record metrics for trend analysis
    pub fn record_metrics(&self, metrics: TuningMetrics) {
        let entry = MetricsHistoryEntry {
            timestamp: Instant::now(),
            metrics,
        };

        let mut history = self.metrics_history.write();
        history.push_back(entry);

        // Maintain window size
        while history.len() > self.config.history_window_size {
            history.pop_front();
        }
    }

    /// Analyze metrics and determine tuning action
    ///
    /// **Feature: performance-optimization, Property 27: Automatic Cache Tuning**
    pub fn analyze_and_tune(&self, current_metrics: &TuningMetrics) -> TuningAction {
        // Check cooldown
        {
            let state = self.state.read();
            if let Some(last_adj) = state.last_adjustment {
                if let Ok(elapsed) = SystemTime::now().duration_since(last_adj) {
                    if elapsed < self.config.adjustment_cooldown {
                        return TuningAction {
                            action_type: TuningActionType::NoAction,
                            description: "In cooldown period".to_string(),
                            timestamp: SystemTime::now(),
                            metrics_before: current_metrics.clone(),
                            expected_improvement: 0.0,
                        };
                    }
                }
            }
        }

        // Record current metrics
        self.record_metrics(current_metrics.clone());

        // Analyze trends
        let trend = self.analyze_trend();

        // Determine action based on metrics and trends
        let action = self.determine_action(current_metrics, &trend);

        // Apply action if not NoAction
        if action.action_type != TuningActionType::NoAction {
            self.apply_action(&action);
        }

        action
    }

    /// Analyze metrics trend
    fn analyze_trend(&self) -> MetricsTrend {
        let history = self.metrics_history.read();

        if history.len() < 5 {
            return MetricsTrend::default();
        }

        let mid = history.len() / 2;

        // Calculate recent vs older averages
        let recent_hit_rate: f64 = history
            .iter()
            .skip(mid)
            .map(|e| e.metrics.hit_rate)
            .sum::<f64>()
            / (history.len() - mid) as f64;
        let older_hit_rate: f64 = history
            .iter()
            .take(mid)
            .map(|e| e.metrics.hit_rate)
            .sum::<f64>()
            / mid as f64;

        let recent_eviction: f64 = history
            .iter()
            .skip(mid)
            .map(|e| e.metrics.eviction_rate)
            .sum::<f64>()
            / (history.len() - mid) as f64;
        let older_eviction: f64 = history
            .iter()
            .take(mid)
            .map(|e| e.metrics.eviction_rate)
            .sum::<f64>()
            / mid as f64;

        MetricsTrend {
            hit_rate_trend: recent_hit_rate - older_hit_rate,
            eviction_trend: recent_eviction - older_eviction,
            is_improving: recent_hit_rate > older_hit_rate,
            is_stable: (recent_hit_rate - older_hit_rate).abs() < 0.05,
        }
    }

    /// Determine the best tuning action
    fn determine_action(&self, metrics: &TuningMetrics, trend: &MetricsTrend) -> TuningAction {
        let state = self.state.read();

        // Priority 1: Fix critically low hit rate
        if metrics.hit_rate < self.config.min_acceptable_hit_rate {
            let new_size = self.calculate_new_size(state.current_size, true);
            if new_size != state.current_size {
                return TuningAction {
                    action_type: TuningActionType::IncreaseCacheSize {
                        from: state.current_size,
                        to: new_size,
                    },
                    description: format!(
                        "Hit rate ({:.1}%) below minimum ({:.1}%). Increasing cache size.",
                        metrics.hit_rate * 100.0,
                        self.config.min_acceptable_hit_rate * 100.0
                    ),
                    timestamp: SystemTime::now(),
                    metrics_before: metrics.clone(),
                    expected_improvement: 10.0,
                };
            }
        }

        // Priority 2: Fix high eviction rate
        if metrics.eviction_rate > self.config.max_eviction_rate {
            let new_size = self.calculate_new_size(state.current_size, true);
            if new_size != state.current_size {
                return TuningAction {
                    action_type: TuningActionType::IncreaseCacheSize {
                        from: state.current_size,
                        to: new_size,
                    },
                    description: format!(
                        "Eviction rate ({:.1}/min) exceeds maximum ({:.1}/min). Increasing cache size.",
                        metrics.eviction_rate,
                        self.config.max_eviction_rate
                    ),
                    timestamp: SystemTime::now(),
                    metrics_before: metrics.clone(),
                    expected_improvement: 15.0,
                };
            }
        }

        // Priority 3: Optimize based on trends
        if !trend.is_stable {
            if trend.hit_rate_trend < -0.1 {
                // Hit rate declining - increase size
                let new_size = self.calculate_new_size(state.current_size, true);
                if new_size != state.current_size {
                    return TuningAction {
                        action_type: TuningActionType::IncreaseCacheSize {
                            from: state.current_size,
                            to: new_size,
                        },
                        description: format!(
                            "Hit rate declining ({:.1}% trend). Increasing cache size.",
                            trend.hit_rate_trend * 100.0
                        ),
                        timestamp: SystemTime::now(),
                        metrics_before: metrics.clone(),
                        expected_improvement: 5.0,
                    };
                }
            } else if trend.hit_rate_trend > 0.1 && metrics.hit_rate > self.config.target_hit_rate {
                // Hit rate improving and above target - can reduce size
                let new_size = self.calculate_new_size(state.current_size, false);
                if new_size != state.current_size {
                    return TuningAction {
                        action_type: TuningActionType::DecreaseCacheSize {
                            from: state.current_size,
                            to: new_size,
                        },
                        description: format!(
                            "Hit rate ({:.1}%) above target with improving trend. Reducing cache size to save memory.",
                            metrics.hit_rate * 100.0
                        ),
                        timestamp: SystemTime::now(),
                        metrics_before: metrics.clone(),
                        expected_improvement: 0.0, // Memory savings, not performance
                    };
                }
            }
        }

        // Priority 4: Consider eviction policy change
        if metrics.hot_keys_count > 50 && state.current_policy != EvictionPolicy::LFU {
            return TuningAction {
                action_type: TuningActionType::ChangeEvictionPolicy {
                    from: state.current_policy,
                    to: EvictionPolicy::LFU,
                },
                description: format!(
                    "Many hot keys detected ({}). Switching to LFU eviction policy.",
                    metrics.hot_keys_count
                ),
                timestamp: SystemTime::now(),
                metrics_before: metrics.clone(),
                expected_improvement: 5.0,
            };
        }

        // Priority 5: Trigger warming if enabled and beneficial
        if self.config.enable_auto_warming
            && metrics.hot_keys_count > 0
            && metrics.hit_rate < self.config.target_hit_rate
        {
            return TuningAction {
                action_type: TuningActionType::TriggerWarming {
                    keys_count: metrics.hot_keys_count,
                },
                description: format!(
                    "Triggering cache warming for {} hot keys to improve hit rate.",
                    metrics.hot_keys_count
                ),
                timestamp: SystemTime::now(),
                metrics_before: metrics.clone(),
                expected_improvement: 8.0,
            };
        }

        TuningAction {
            action_type: TuningActionType::NoAction,
            description: "Cache performance is within acceptable parameters".to_string(),
            timestamp: SystemTime::now(),
            metrics_before: metrics.clone(),
            expected_improvement: 0.0,
        }
    }

    /// Calculate new cache size
    pub fn calculate_new_size(&self, current: u64, increase: bool) -> u64 {
        let adjustment = (current as f64 * self.config.size_adjustment_step / 100.0) as u64;
        let adjustment = adjustment.max(10); // Minimum adjustment of 10

        if increase {
            (current + adjustment).min(self.config.max_cache_size)
        } else {
            current
                .saturating_sub(adjustment)
                .max(self.config.min_cache_size)
        }
    }

    /// Apply a tuning action
    fn apply_action(&self, action: &TuningAction) {
        let mut state = self.state.write();

        match &action.action_type {
            TuningActionType::IncreaseCacheSize { to, .. }
            | TuningActionType::DecreaseCacheSize { to, .. } => {
                state.current_size = *to;
            }
            TuningActionType::ChangeEvictionPolicy { to, .. } => {
                state.current_policy = *to;
            }
            TuningActionType::AdjustTTL { to_secs, .. } => {
                state.current_ttl_secs = *to_secs;
            }
            _ => {}
        }

        state.last_adjustment = Some(SystemTime::now());
        state.total_adjustments += 1;

        // Keep recent actions history
        state.recent_actions.push(action.clone());
        if state.recent_actions.len() > 20 {
            state.recent_actions.remove(0);
        }

        info!(
            action = ?action.action_type,
            description = %action.description,
            "Applied cache tuning action"
        );
    }

    /// Get current tuning state
    pub fn get_state(&self) -> CacheTuningState {
        self.state.read().clone()
    }

    /// Get recommended cache configuration based on current state
    pub fn get_recommended_config(&self) -> CacheConfig {
        let state = self.state.read();

        CacheConfig {
            max_capacity: state.current_size,
            ttl: Duration::from_secs(state.current_ttl_secs),
            tti: Duration::from_secs(state.current_ttl_secs / 5), // TTI = TTL / 5
            enable_monitoring: true,
            monitoring_interval: Duration::from_secs(60),
            enable_l2_cache: false,
            redis_url: String::new(),
            l2_prefix: "cache:".to_string(),
            compression_threshold: 10 * 1024,
            enable_compression: true,
            access_pattern_window: 1000,
            preload_threshold: self.config.warming_threshold,
        }
    }

    /// Get tuning history
    pub fn get_history(&self) -> Vec<TuningAction> {
        self.state.read().recent_actions.clone()
    }

    /// Reset tuning state
    pub fn reset(&self) {
        *self.state.write() = CacheTuningState::default();
        self.metrics_history.write().clear();
        *self.last_tuning_check.write() = None;
    }
}

/// Metrics trend analysis result
#[derive(Debug, Clone, Default)]
struct MetricsTrend {
    hit_rate_trend: f64,
    #[allow(dead_code)]
    eviction_trend: f64,
    #[allow(dead_code)]
    is_improving: bool,
    is_stable: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_tuner_creation() {
        let config = CacheTunerConfig::default();
        let tuner = CacheTuner::new(config);

        let state = tuner.get_state();
        assert_eq!(state.current_policy, EvictionPolicy::LRU);
        assert_eq!(state.total_adjustments, 0);
    }

    #[test]
    fn test_metrics_recording() {
        let config = CacheTunerConfig {
            history_window_size: 10,
            ..Default::default()
        };
        let tuner = CacheTuner::new(config);

        for i in 0..15 {
            tuner.record_metrics(TuningMetrics {
                hit_rate: 0.5 + (i as f64 * 0.02),
                eviction_rate: 5.0,
                avg_access_time_ms: 1.0,
                cache_size: 1000,
                memory_usage_bytes: 1024 * 1024,
                hot_keys_count: 10,
            });
        }

        let history = tuner.metrics_history.read();
        assert_eq!(history.len(), 10); // Should be capped at window size
    }

    #[test]
    fn test_low_hit_rate_triggers_size_increase() {
        let config = CacheTunerConfig {
            min_acceptable_hit_rate: 0.6,
            adjustment_cooldown: Duration::from_secs(0), // No cooldown for test
            ..Default::default()
        };
        let tuner = CacheTuner::new(config);

        let metrics = TuningMetrics {
            hit_rate: 0.4, // Below minimum
            eviction_rate: 5.0,
            avg_access_time_ms: 1.0,
            cache_size: 1000,
            memory_usage_bytes: 1024 * 1024,
            hot_keys_count: 10,
        };

        let action = tuner.analyze_and_tune(&metrics);

        match action.action_type {
            TuningActionType::IncreaseCacheSize { from, to } => {
                assert!(to > from);
            }
            _ => panic!("Expected IncreaseCacheSize action"),
        }
    }

    #[test]
    fn test_high_eviction_rate_triggers_size_increase() {
        let config = CacheTunerConfig {
            max_eviction_rate: 5.0,
            adjustment_cooldown: Duration::from_secs(0),
            ..Default::default()
        };
        let tuner = CacheTuner::new(config);

        let metrics = TuningMetrics {
            hit_rate: 0.8,
            eviction_rate: 15.0, // Above maximum
            avg_access_time_ms: 1.0,
            cache_size: 1000,
            memory_usage_bytes: 1024 * 1024,
            hot_keys_count: 10,
        };

        let action = tuner.analyze_and_tune(&metrics);

        match action.action_type {
            TuningActionType::IncreaseCacheSize { from, to } => {
                assert!(to > from);
            }
            _ => panic!("Expected IncreaseCacheSize action"),
        }
    }

    #[test]
    fn test_cooldown_prevents_rapid_adjustments() {
        let config = CacheTunerConfig {
            adjustment_cooldown: Duration::from_secs(300),
            min_acceptable_hit_rate: 0.6,
            ..Default::default()
        };
        let tuner = CacheTuner::new(config);

        let metrics = TuningMetrics {
            hit_rate: 0.4,
            eviction_rate: 5.0,
            avg_access_time_ms: 1.0,
            cache_size: 1000,
            memory_usage_bytes: 1024 * 1024,
            hot_keys_count: 10,
        };

        // First action should succeed
        let action1 = tuner.analyze_and_tune(&metrics);
        assert_ne!(action1.action_type, TuningActionType::NoAction);

        // Second action should be blocked by cooldown
        let action2 = tuner.analyze_and_tune(&metrics);
        assert_eq!(action2.action_type, TuningActionType::NoAction);
    }

    #[test]
    fn test_size_calculation_bounds() {
        let config = CacheTunerConfig {
            min_cache_size: 100,
            max_cache_size: 5000,
            size_adjustment_step: 10.0,
            ..Default::default()
        };
        let tuner = CacheTuner::new(config.clone());

        // Test increase doesn't exceed max
        let new_size = tuner.calculate_new_size(4900, true);
        assert!(new_size <= config.max_cache_size);

        // Test decrease doesn't go below min
        let new_size = tuner.calculate_new_size(110, false);
        assert!(new_size >= config.min_cache_size);
    }

    #[test]
    fn test_recommended_config_generation() {
        let config = CacheTunerConfig::default();
        let tuner = CacheTuner::new(config);

        let recommended = tuner.get_recommended_config();
        assert!(recommended.max_capacity > 0);
        assert!(recommended.ttl.as_secs() > 0);
    }
}
