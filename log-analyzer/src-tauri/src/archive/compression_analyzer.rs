//! 压缩分析器
//!
//! 分析压缩包的特征，包括：
//! - 压缩比计算
//! - 文件分布分析
//! - 嵌套结构分析
//! - 安全风险评估

use crate::archive::nested_archive_config::NestedArchiveConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::debug;

/// 文件分布统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDistribution {
    /// 文件总数
    pub total_files: usize,

    /// 总大小（字节）
    pub total_size: u64,

    /// 压缩后大小（字节）
    pub compressed_size: u64,

    /// 各扩展名文件数量
    pub extension_counts: HashMap<String, usize>,

    /// 各扩展名文件大小
    pub extension_sizes: HashMap<String, u64>,

    /// 最大文件大小
    pub max_file_size: u64,

    /// 平均文件大小
    pub avg_file_size: f64,
}

impl Default for FileDistribution {
    fn default() -> Self {
        Self {
            total_files: 0,
            total_size: 0,
            compressed_size: 0,
            extension_counts: HashMap::new(),
            extension_sizes: HashMap::new(),
            max_file_size: 0,
            avg_file_size: 0.0,
        }
    }
}

impl FileDistribution {
    /// 添加文件到统计
    pub fn add_file(&mut self, size: u64, extension: Option<&str>) {
        self.total_files += 1;
        self.total_size += size;
        self.max_file_size = self.max_file_size.max(size);

        if let Some(ext) = extension {
            let ext_lower = ext.to_lowercase();
            *self.extension_counts.entry(ext_lower.clone()).or_insert(0) += 1;
            *self.extension_sizes.entry(ext_lower).or_insert(0) += size;
        }

        if self.total_files > 0 {
            self.avg_file_size = self.total_size as f64 / self.total_files as f64;
        }
    }

    /// 计算压缩比
    pub fn compression_ratio(&self) -> f64 {
        if self.compressed_size == 0 {
            return 1.0;
        }
        self.total_size as f64 / self.compressed_size as f64
    }
}

/// 嵌套结构信息
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NestedStructureInfo {
    /// 当前嵌套深度
    pub current_depth: usize,

    /// 子压缩包数量
    pub nested_archive_count: usize,

    /// 各深度文件分布
    pub depth_distribution: HashMap<usize, usize>,

    /// 最深嵌套级别
    pub max_nested_depth: usize,
}

/// 安全风险评估
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityRisk {
    /// 无风险
    None,
    /// 低风险
    Low(String),
    /// 中等风险
    Medium(String),
    /// 高风险
    High(String),
}

impl SecurityRisk {
    /// 风险等级（0-3）
    pub fn level(&self) -> u8 {
        match self {
            SecurityRisk::None => 0,
            SecurityRisk::Low(_) => 1,
            SecurityRisk::Medium(_) => 2,
            SecurityRisk::High(_) => 3,
        }
    }

    /// 风险描述
    pub fn description(&self) -> &str {
        match self {
            SecurityRisk::None => "No risk detected",
            SecurityRisk::Low(msg) => msg,
            SecurityRisk::Medium(msg) => msg,
            SecurityRisk::High(msg) => msg,
        }
    }
}

/// 压缩包分析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveAnalysis {
    /// 文件分布统计
    pub distribution: FileDistribution,

    /// 嵌套结构信息
    pub nested_structure: NestedStructureInfo,

    /// 安全风险评估
    pub security_risk: SecurityRisk,

    /// 压缩比
    pub compression_ratio: f64,

    /// 是否为压缩炸弹
    pub is_zip_bomb: bool,

    /// 建议的最大嵌套深度
    pub recommended_max_depth: usize,
}

impl Default for ArchiveAnalysis {
    fn default() -> Self {
        Self {
            distribution: FileDistribution::default(),
            nested_structure: NestedStructureInfo::default(),
            security_risk: SecurityRisk::None,
            compression_ratio: 1.0,
            is_zip_bomb: false,
            recommended_max_depth: 15,
        }
    }
}

/// 压缩分析器
pub struct CompressionAnalyzer {
    config: NestedArchiveConfig,
}

impl CompressionAnalyzer {
    /// 创建新的分析器
    pub fn new(config: NestedArchiveConfig) -> Self {
        Self { config }
    }

    /// 分析压缩包
    pub fn analyze(
        &self,
        file_distribution: FileDistribution,
        nested_structure: NestedStructureInfo,
        compressed_size: u64,
    ) -> ArchiveAnalysis {
        // 设置压缩后大小
        let mut distribution = file_distribution;
        distribution.compressed_size = compressed_size;

        // 计算压缩比
        let compression_ratio = distribution.compression_ratio();

        // 检查压缩炸弹
        let is_zip_bomb = self
            .config
            .is_potential_zip_bomb(compression_ratio, nested_structure.current_depth);

        // 安全风险评估
        let security_risk =
            self.assess_security_risk(&distribution, &nested_structure, compression_ratio);

        // 计算推荐的最大嵌套深度
        let recommended_max_depth = self
            .config
            .calculate_dynamic_depth_limit(distribution.total_files, distribution.total_size);

        debug!(
            total_files = distribution.total_files,
            total_size = distribution.total_size,
            compressed_size = compressed_size,
            compression_ratio = compression_ratio,
            is_zip_bomb = is_zip_bomb,
            risk_level = security_risk.level(),
            recommended_depth = recommended_max_depth,
            "Archive analysis complete"
        );

        ArchiveAnalysis {
            distribution,
            nested_structure,
            security_risk,
            compression_ratio,
            is_zip_bomb,
            recommended_max_depth,
        }
    }

    /// 评估安全风险
    fn assess_security_risk(
        &self,
        distribution: &FileDistribution,
        nested_structure: &NestedStructureInfo,
        compression_ratio: f64,
    ) -> SecurityRisk {
        // 检查压缩炸弹
        if self
            .config
            .is_potential_zip_bomb(compression_ratio, nested_structure.current_depth)
        {
            return SecurityRisk::High(format!(
                "Potential zip bomb detected (compression ratio: {:.2}, depth: {})",
                compression_ratio, nested_structure.current_depth
            ));
        }

        // 检查过度嵌套
        if nested_structure.current_depth > self.config.max_depth {
            return SecurityRisk::High(format!(
                "Excessive nesting depth: {} (max: {})",
                nested_structure.current_depth, self.config.max_depth
            ));
        }

        // 检查文件数量过多
        if distribution.total_files > self.config.file_count_threshold * 10 {
            return SecurityRisk::Medium(format!(
                "Large file count: {} (threshold: {})",
                distribution.total_files, self.config.file_count_threshold
            ));
        }

        // 检查总大小过大
        if distribution.total_size > self.config.total_size_threshold * 2 {
            return SecurityRisk::Medium(format!(
                "Large total size: {} GB (threshold: {} GB)",
                distribution.total_size / (1024 * 1024 * 1024),
                self.config.total_size_threshold / (1024 * 1024 * 1024)
            ));
        }

        // 检查高压缩比
        if compression_ratio > self.config.compression_ratio_threshold * 0.8 {
            return SecurityRisk::Low(format!(
                "High compression ratio: {:.2} (threshold: {:.2})",
                compression_ratio, self.config.compression_ratio_threshold
            ));
        }

        SecurityRisk::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> NestedArchiveConfig {
        NestedArchiveConfig {
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

    #[test]
    fn test_file_distribution() {
        let mut dist = FileDistribution {
            compressed_size: 1000,
            avg_file_size: 0.0,
            ..Default::default()
        };

        dist.add_file(100, Some("txt"));
        dist.add_file(200, Some("txt"));
        dist.add_file(300, Some("log"));

        assert_eq!(dist.total_files, 3);
        assert_eq!(dist.total_size, 600);
        assert_eq!(dist.max_file_size, 300);
        assert_eq!(dist.avg_file_size, 200.0);
        assert_eq!(*dist.extension_counts.get("txt").unwrap(), 2);
        assert_eq!(*dist.extension_counts.get("log").unwrap(), 1);
    }

    #[test]
    fn test_compression_ratio() {
        let dist = FileDistribution {
            compressed_size: 100,
            total_size: 1000,
            ..Default::default()
        };

        assert_eq!(dist.compression_ratio(), 10.0);
    }

    #[test]
    fn test_archive_analysis_normal() {
        let analyzer = CompressionAnalyzer::new(create_test_config());

        let mut distribution = FileDistribution {
            compressed_size: 500,
            avg_file_size: 0.0,
            ..Default::default()
        };
        distribution.add_file(1000, Some("log"));
        distribution.add_file(2000, Some("txt"));

        let nested_structure = NestedStructureInfo {
            current_depth: 1,
            ..Default::default()
        };

        let analysis = analyzer.analyze(distribution, nested_structure, 500);

        assert_eq!(analysis.distribution.total_files, 2);
        assert_eq!(analysis.compression_ratio, 6.0);
        assert!(!analysis.is_zip_bomb);
        assert_eq!(analysis.security_risk.level(), 0);
    }

    #[test]
    fn test_archive_analysis_zip_bomb() {
        let analyzer = CompressionAnalyzer::new(create_test_config());

        let mut distribution = FileDistribution {
            compressed_size: 1000,
            avg_file_size: 0.0,
            ..Default::default()
        };
        for _ in 0..100 {
            distribution.add_file(1_000_000, Some("log")); // 1MB each
        }

        let nested_structure = NestedStructureInfo {
            current_depth: 5,
            ..Default::default()
        };

        let analysis = analyzer.analyze(distribution, nested_structure, 1000);

        assert!(analysis.is_zip_bomb);
        assert_eq!(analysis.security_risk.level(), 3); // High risk
    }

    #[test]
    fn test_security_risk_assessment() {
        let analyzer = CompressionAnalyzer::new(create_test_config());

        // 正常情况
        let distribution = FileDistribution {
            compressed_size: 500,
            avg_file_size: 0.0,
            ..Default::default()
        };
        let mut distribution = distribution;
        distribution.add_file(1000, Some("log"));

        let nested_structure = NestedStructureInfo::default();

        let risk = analyzer.assess_security_risk(&distribution, &nested_structure, 2.0);
        assert_eq!(risk.level(), 0);

        // 高压缩比
        let risk = analyzer.assess_security_risk(&distribution, &nested_structure, 90.0);
        assert_eq!(risk.level(), 1); // Low risk
    }

    #[test]
    fn test_recommended_depth_calculation() {
        let analyzer = CompressionAnalyzer::new(create_test_config());

        let distribution = FileDistribution {
            total_files: 6000,
            total_size: 25 * 1024 * 1024 * 1024,
            compressed_size: 0,
            extension_counts: HashMap::new(),
            extension_sizes: HashMap::new(),
            max_file_size: 0,
            avg_file_size: 0.0,
        };

        let nested_structure = NestedStructureInfo::default();

        let analysis = analyzer.analyze(distribution, nested_structure, 0);

        // 应该降低推荐深度
        assert!(analysis.recommended_max_depth < 15);
        assert!(analysis.recommended_max_depth >= 1);
    }
}
