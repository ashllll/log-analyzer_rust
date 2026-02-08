//! 嵌套压缩包配置
//!
//! 定义嵌套压缩包处理的配置和策略：
//! - 动态深度控制
//! - 压缩炸弹检测
//! - 文件数量和大小阈值

use serde::{Deserialize, Serialize};

/// 嵌套压缩包处理配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NestedArchiveConfig {
    /// 最大嵌套深度
    #[serde(default = "default_max_depth")]
    pub max_depth: usize,

    /// 文件数量阈值（超过此数量则限制深度）
    #[serde(default = "default_file_count_threshold")]
    pub file_count_threshold: usize,

    /// 总大小阈值（字节）
    #[serde(default = "default_total_size_threshold")]
    pub total_size_threshold: u64,

    /// 压缩比阈值
    #[serde(default = "default_compression_ratio")]
    pub compression_ratio_threshold: f64,

    /// 指数退避阈值（ratio^depth）
    #[serde(default = "default_exponential_threshold")]
    pub exponential_backoff_threshold: f64,

    /// 启用压缩炸弹检测
    #[serde(default = "default_true")]
    pub enable_zip_bomb_detection: bool,

    /// 深度递减步长（每超过阈值一次减少的深度）
    #[serde(default = "default_depth_reduction_step")]
    pub depth_reduction_step: usize,

    /// 最小允许深度
    #[serde(default = "default_min_depth")]
    pub min_depth: usize,
}

fn default_max_depth() -> usize {
    15
}

fn default_file_count_threshold() -> usize {
    5000
}

fn default_total_size_threshold() -> u64 {
    20 * 1024 * 1024 * 1024 // 20GB
}

fn default_compression_ratio() -> f64 {
    100.0
}

fn default_exponential_threshold() -> f64 {
    1_000_000.0
}

fn default_true() -> bool {
    true
}

fn default_depth_reduction_step() -> usize {
    1
}

fn default_min_depth() -> usize {
    1
}

impl Default for NestedArchiveConfig {
    fn default() -> Self {
        Self {
            max_depth: 15,
            file_count_threshold: 5000,
            total_size_threshold: 20 * 1024 * 1024 * 1024,
            compression_ratio_threshold: 100.0,
            exponential_backoff_threshold: 1_000_000.0,
            enable_zip_bomb_detection: true,
            depth_reduction_step: 1,
            min_depth: 1,
        }
    }
}

impl NestedArchiveConfig {
    /// 计算动态深度限制
    ///
    /// 根据当前处理的文件数量和总大小，动态调整嵌套深度限制
    pub fn calculate_dynamic_depth_limit(
        &self,
        current_file_count: usize,
        current_total_size: u64,
    ) -> usize {
        let mut depth_limit = self.max_depth;

        // 根据文件数量调整
        if current_file_count > self.file_count_threshold {
            let reduction =
                (current_file_count - self.file_count_threshold) / 1000 * self.depth_reduction_step;
            depth_limit = depth_limit.saturating_sub(reduction.min(5));
        }

        // 根据总大小调整
        if current_total_size > self.total_size_threshold {
            let reduction = ((current_total_size - self.total_size_threshold)
                / (1024 * 1024 * 1024))
                .min(3) as usize
                * self.depth_reduction_step;
            depth_limit = depth_limit.saturating_sub(reduction);
        }

        depth_limit.max(self.min_depth)
    }

    /// 检查是否为潜在的压缩炸弹
    ///
    /// 基于压缩比和嵌套深度判断
    pub fn is_potential_zip_bomb(&self, compression_ratio: f64, depth: usize) -> bool {
        if !self.enable_zip_bomb_detection {
            return false;
        }

        // 检查压缩比
        if compression_ratio > self.compression_ratio_threshold {
            return true;
        }

        // 检查指数退避阈值
        let exponential_factor = compression_ratio.powi(depth as i32);
        if exponential_factor > self.exponential_backoff_threshold {
            return true;
        }

        false
    }

    /// 验证配置有效性
    pub fn validate(&self) -> Result<(), String> {
        if self.max_depth == 0 {
            return Err("max_depth must be positive".to_string());
        }

        if self.max_depth > 30 {
            return Err("max_depth must not exceed 30".to_string());
        }

        if self.min_depth == 0 {
            return Err("min_depth must be positive".to_string());
        }

        if self.min_depth > self.max_depth {
            return Err("min_depth must not exceed max_depth".to_string());
        }

        if self.file_count_threshold == 0 {
            return Err("file_count_threshold must be positive".to_string());
        }

        if self.total_size_threshold == 0 {
            return Err("total_size_threshold must be positive".to_string());
        }

        if self.compression_ratio_threshold <= 0.0 {
            return Err("compression_ratio_threshold must be positive".to_string());
        }

        if self.exponential_backoff_threshold <= 0.0 {
            return Err("exponential_backoff_threshold must be positive".to_string());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = NestedArchiveConfig::default();
        assert_eq!(config.max_depth, 15);
        assert_eq!(config.file_count_threshold, 5000);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_dynamic_depth_limit() {
        let config = NestedArchiveConfig::default();

        // 正常情况
        assert_eq!(
            config.calculate_dynamic_depth_limit(1000, 1024 * 1024 * 1024),
            15
        );

        // 文件数量超过阈值
        let depth = config.calculate_dynamic_depth_limit(6000, 1024 * 1024 * 1024);
        assert!(depth < 15);
        assert!(depth >= 1);

        // 总大小超过阈值
        let depth = config.calculate_dynamic_depth_limit(1000, 21 * 1024 * 1024 * 1024);
        assert!(depth < 15);
        assert!(depth >= 1);

        // 两者都超过
        let depth = config.calculate_dynamic_depth_limit(10000, 30 * 1024 * 1024 * 1024);
        assert!(depth < 10);
        assert!(depth >= 1);
    }

    #[test]
    fn test_zip_bomb_detection() {
        let config = NestedArchiveConfig::default();

        // 正常压缩比
        assert!(!config.is_potential_zip_bomb(10.0, 5));

        // 超过压缩比阈值
        assert!(config.is_potential_zip_bomb(200.0, 5));

        // 指数增长检测
        assert!(config.is_potential_zip_bomb(50.0, 10)); // 50^10 >> 1_000_000

        // 禁用检测
        let mut config = config;
        config.enable_zip_bomb_detection = false;
        assert!(!config.is_potential_zip_bomb(200.0, 5));
    }

    #[test]
    fn test_validation() {
        let config = NestedArchiveConfig::default();
        assert!(config.validate().is_ok());

        // 无效的最大深度
        let mut config = config.clone();
        config.max_depth = 0;
        assert!(config.validate().is_err());

        config.max_depth = 35;
        assert!(config.validate().is_err());

        // min_depth > max_depth
        let mut config = config.clone();
        config.min_depth = 20;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_min_depth_respected() {
        let config = NestedArchiveConfig {
            max_depth: 15,
            min_depth: 3,
            ..Default::default()
        };

        // 即使极端情况，也会受到最小深度保护
        let depth = config.calculate_dynamic_depth_limit(100000, 100 * 1024 * 1024 * 1024);
        assert!(depth >= 3);
    }
}
