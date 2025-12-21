//! Security Detector for Archive Extraction
//!
//! Provides zip bomb detection, compression ratio analysis, and risk scoring
//! to protect against malicious archives.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// Security policy configuration with thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicy {
    /// Maximum allowed compression ratio (default: 100.0)
    pub max_compression_ratio: f64,
    /// Maximum cumulative extracted size per archive (default: 10GB)
    pub max_cumulative_size: u64,
    /// Maximum cumulative extracted size per workspace (default: 50GB)
    pub max_workspace_size: u64,
    /// Exponential backoff threshold for risk score (default: 1,000,000.0)
    pub exponential_backoff_threshold: f64,
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            max_compression_ratio: 100.0,
            max_cumulative_size: 10 * 1024 * 1024 * 1024, // 10GB
            max_workspace_size: 50 * 1024 * 1024 * 1024,  // 50GB
            exponential_backoff_threshold: 1_000_000.0,
        }
    }
}

/// Compression metrics for a file or archive
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionMetrics {
    /// Compressed size in bytes
    pub compressed_size: u64,
    /// Uncompressed size in bytes
    pub uncompressed_size: u64,
    /// Compression ratio (uncompressed / compressed)
    pub compression_ratio: f64,
    /// Current nesting depth
    pub nesting_depth: usize,
    /// Calculated risk score
    pub risk_score: f64,
}

/// Security violation detected during extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityViolation {
    /// Type of violation
    pub violation_type: ViolationType,
    /// Severity level
    pub severity: Severity,
    /// Detailed message
    pub message: String,
    /// File path that triggered the violation
    pub file_path: Option<PathBuf>,
    /// Associated metrics
    pub metrics: Option<CompressionMetrics>,
}

/// Types of security violations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViolationType {
    /// Compression ratio exceeds threshold
    ExcessiveCompressionRatio,
    /// Cumulative size exceeds limit
    CumulativeSizeExceeded,
    /// Risk score exceeds threshold
    RiskScoreExceeded,
    /// Suspicious pattern detected
    SuspiciousPattern,
}

/// Severity levels for security events
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

/// Security warning (non-blocking)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityWarning {
    /// Warning message
    pub message: String,
    /// File path
    pub file_path: Option<PathBuf>,
    /// Associated metrics
    pub metrics: Option<CompressionMetrics>,
}

/// Archive entry information for pre-extraction analysis
#[derive(Debug, Clone)]
pub struct ArchiveEntry {
    /// Entry path within archive
    pub path: PathBuf,
    /// Compressed size
    pub compressed_size: u64,
    /// Uncompressed size
    pub uncompressed_size: u64,
    /// Whether this is a directory
    pub is_directory: bool,
}

/// Security detector for archive extraction
pub struct SecurityDetector {
    policy: SecurityPolicy,
}

impl SecurityDetector {
    /// Create a new security detector with the given policy
    pub fn new(policy: SecurityPolicy) -> Self {
        info!(
            "Initializing SecurityDetector with policy: max_ratio={}, max_size={}, threshold={}",
            policy.max_compression_ratio,
            policy.max_cumulative_size,
            policy.exponential_backoff_threshold
        );
        Self { policy }
    }

    /// Create a security detector with default policy
    pub fn default() -> Self {
        Self::new(SecurityPolicy::default())
    }

    /// Calculate compression ratio for a file
    ///
    /// Handles edge cases:
    /// - Zero compressed size: returns f64::INFINITY
    /// - Zero uncompressed size: returns 0.0
    /// - Both zero: returns 0.0
    ///
    /// # Arguments
    /// * `compressed_size` - Size of compressed data in bytes
    /// * `uncompressed_size` - Size of uncompressed data in bytes
    ///
    /// # Returns
    /// Compression ratio as uncompressed_size / compressed_size
    pub fn calculate_compression_ratio(&self, compressed_size: u64, uncompressed_size: u64) -> f64 {
        if compressed_size == 0 {
            if uncompressed_size == 0 {
                // Both zero - no compression, ratio is 0
                0.0
            } else {
                // Zero compressed but non-zero uncompressed - infinite compression
                f64::INFINITY
            }
        } else if uncompressed_size == 0 {
            // Non-zero compressed but zero uncompressed - no data
            0.0
        } else {
            // Normal case
            uncompressed_size as f64 / compressed_size as f64
        }
    }

    /// Calculate risk score using exponential backoff formula
    ///
    /// Formula: risk_score = compression_ratio ^ nesting_depth
    ///
    /// # Arguments
    /// * `compression_ratio` - Compression ratio of the file
    /// * `nesting_depth` - Current nesting depth (0 for top-level)
    ///
    /// # Returns
    /// Risk score as ratio^depth
    pub fn calculate_risk_score(&self, compression_ratio: f64, nesting_depth: usize) -> f64 {
        if compression_ratio.is_infinite() || compression_ratio.is_nan() {
            // Infinite or NaN compression ratio is maximum risk
            f64::INFINITY
        } else if compression_ratio <= 0.0 {
            // Zero or negative ratio has no risk
            0.0
        } else if nesting_depth == 0 {
            // At depth 0, risk score equals compression ratio
            compression_ratio
        } else {
            // Exponential backoff: ratio^depth
            compression_ratio.powi(nesting_depth as i32)
        }
    }

    /// Check if extraction should be halted based on metrics
    ///
    /// Checks:
    /// 1. Compression ratio exceeds threshold
    /// 2. Risk score exceeds exponential backoff threshold
    /// 3. Cumulative size exceeds limit
    ///
    /// # Arguments
    /// * `compressed_size` - Compressed size of current file
    /// * `uncompressed_size` - Uncompressed size of current file
    /// * `nesting_depth` - Current nesting depth
    /// * `cumulative_size` - Total extracted size so far
    ///
    /// # Returns
    /// (should_halt, optional_violation)
    pub fn should_halt_extraction(
        &self,
        compressed_size: u64,
        uncompressed_size: u64,
        nesting_depth: usize,
        cumulative_size: u64,
    ) -> (bool, Option<SecurityViolation>) {
        // Calculate metrics
        let compression_ratio =
            self.calculate_compression_ratio(compressed_size, uncompressed_size);
        let risk_score = self.calculate_risk_score(compression_ratio, nesting_depth);

        let metrics = CompressionMetrics {
            compressed_size,
            uncompressed_size,
            compression_ratio,
            nesting_depth,
            risk_score,
        };

        // Check compression ratio threshold
        if compression_ratio > self.policy.max_compression_ratio {
            warn!(
                "Excessive compression ratio detected: {} (threshold: {})",
                compression_ratio, self.policy.max_compression_ratio
            );
            return (
                true,
                Some(SecurityViolation {
                    violation_type: ViolationType::ExcessiveCompressionRatio,
                    severity: Severity::High,
                    message: format!(
                        "Compression ratio {} exceeds threshold {}",
                        compression_ratio, self.policy.max_compression_ratio
                    ),
                    file_path: None,
                    metrics: Some(metrics),
                }),
            );
        }

        // Check risk score threshold
        if risk_score > self.policy.exponential_backoff_threshold {
            warn!(
                "Risk score exceeded: {} (threshold: {})",
                risk_score, self.policy.exponential_backoff_threshold
            );
            return (
                true,
                Some(SecurityViolation {
                    violation_type: ViolationType::RiskScoreExceeded,
                    severity: Severity::Critical,
                    message: format!(
                        "Risk score {} exceeds threshold {} (ratio: {}, depth: {})",
                        risk_score,
                        self.policy.exponential_backoff_threshold,
                        compression_ratio,
                        nesting_depth
                    ),
                    file_path: None,
                    metrics: Some(metrics),
                }),
            );
        }

        // Check cumulative size limit
        let new_cumulative_size = cumulative_size.saturating_add(uncompressed_size);
        if new_cumulative_size > self.policy.max_cumulative_size {
            warn!(
                "Cumulative size limit exceeded: {} bytes (limit: {})",
                new_cumulative_size, self.policy.max_cumulative_size
            );
            return (
                true,
                Some(SecurityViolation {
                    violation_type: ViolationType::CumulativeSizeExceeded,
                    severity: Severity::Critical,
                    message: format!(
                        "Cumulative extracted size {} exceeds limit {}",
                        new_cumulative_size, self.policy.max_cumulative_size
                    ),
                    file_path: None,
                    metrics: Some(metrics),
                }),
            );
        }

        // No violations detected
        (false, None)
    }

    /// Detect suspicious patterns in archive entries before extraction
    ///
    /// Analyzes:
    /// - Overall compression ratios
    /// - Number of files
    /// - Suspicious file patterns
    ///
    /// # Arguments
    /// * `archive_path` - Path to the archive being analyzed
    /// * `entries` - List of entries in the archive
    ///
    /// # Returns
    /// List of security warnings
    pub fn detect_suspicious_patterns(
        &self,
        archive_path: &Path,
        entries: &[ArchiveEntry],
    ) -> Vec<SecurityWarning> {
        let mut warnings = Vec::new();

        if entries.is_empty() {
            return warnings;
        }

        // Calculate overall statistics
        let total_compressed: u64 = entries.iter().map(|e| e.compressed_size).sum();
        let total_uncompressed: u64 = entries.iter().map(|e| e.uncompressed_size).sum();
        let file_count = entries.len();

        // Check overall compression ratio
        let overall_ratio = self.calculate_compression_ratio(total_compressed, total_uncompressed);
        if overall_ratio > self.policy.max_compression_ratio * 0.5 {
            // Warn at 50% of threshold
            warnings.push(SecurityWarning {
                message: format!(
                    "Archive has high overall compression ratio: {:.2} (threshold: {})",
                    overall_ratio, self.policy.max_compression_ratio
                ),
                file_path: Some(archive_path.to_path_buf()),
                metrics: Some(CompressionMetrics {
                    compressed_size: total_compressed,
                    uncompressed_size: total_uncompressed,
                    compression_ratio: overall_ratio,
                    nesting_depth: 0,
                    risk_score: overall_ratio,
                }),
            });
        }

        // Check for excessive file count
        if file_count > 10000 {
            warnings.push(SecurityWarning {
                message: format!(
                    "Archive contains {} files, which may indicate a decompression bomb",
                    file_count
                ),
                file_path: Some(archive_path.to_path_buf()),
                metrics: None,
            });
        }

        // Check for individual files with extreme compression
        for entry in entries {
            if entry.is_directory {
                continue;
            }

            let ratio =
                self.calculate_compression_ratio(entry.compressed_size, entry.uncompressed_size);
            if ratio > self.policy.max_compression_ratio * 0.8 {
                // Warn at 80% of threshold
                warnings.push(SecurityWarning {
                    message: format!("File has very high compression ratio: {:.2}", ratio),
                    file_path: Some(entry.path.clone()),
                    metrics: Some(CompressionMetrics {
                        compressed_size: entry.compressed_size,
                        uncompressed_size: entry.uncompressed_size,
                        compression_ratio: ratio,
                        nesting_depth: 0,
                        risk_score: ratio,
                    }),
                });
            }
        }

        if !warnings.is_empty() {
            info!(
                "Detected {} suspicious patterns in archive: {}",
                warnings.len(),
                archive_path.display()
            );
        }

        warnings
    }

    /// Get the current security policy
    pub fn policy(&self) -> &SecurityPolicy {
        &self.policy
    }

    /// Update the security policy
    pub fn update_policy(&mut self, policy: SecurityPolicy) {
        info!("Updating security policy");
        self.policy = policy;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_policy() {
        let policy = SecurityPolicy::default();
        assert_eq!(policy.max_compression_ratio, 100.0);
        assert_eq!(policy.max_cumulative_size, 10 * 1024 * 1024 * 1024);
        assert_eq!(policy.max_workspace_size, 50 * 1024 * 1024 * 1024);
        assert_eq!(policy.exponential_backoff_threshold, 1_000_000.0);
    }

    #[test]
    fn test_compression_ratio_normal() {
        let detector = SecurityDetector::default();
        let ratio = detector.calculate_compression_ratio(100, 1000);
        assert_eq!(ratio, 10.0);
    }

    #[test]
    fn test_compression_ratio_zero_compressed() {
        let detector = SecurityDetector::default();
        let ratio = detector.calculate_compression_ratio(0, 1000);
        assert!(ratio.is_infinite());
    }

    #[test]
    fn test_compression_ratio_zero_uncompressed() {
        let detector = SecurityDetector::default();
        let ratio = detector.calculate_compression_ratio(100, 0);
        assert_eq!(ratio, 0.0);
    }

    #[test]
    fn test_compression_ratio_both_zero() {
        let detector = SecurityDetector::default();
        let ratio = detector.calculate_compression_ratio(0, 0);
        assert_eq!(ratio, 0.0);
    }

    #[test]
    fn test_risk_score_depth_zero() {
        let detector = SecurityDetector::default();
        let score = detector.calculate_risk_score(10.0, 0);
        assert_eq!(score, 10.0);
    }

    #[test]
    fn test_risk_score_exponential() {
        let detector = SecurityDetector::default();
        let score = detector.calculate_risk_score(10.0, 2);
        assert_eq!(score, 100.0); // 10^2
    }

    #[test]
    fn test_risk_score_infinite_ratio() {
        let detector = SecurityDetector::default();
        let score = detector.calculate_risk_score(f64::INFINITY, 1);
        assert!(score.is_infinite());
    }

    #[test]
    fn test_risk_score_zero_ratio() {
        let detector = SecurityDetector::default();
        let score = detector.calculate_risk_score(0.0, 5);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_should_halt_normal_file() {
        let detector = SecurityDetector::default();
        let (should_halt, violation) = detector.should_halt_extraction(
            1000,  // 1KB compressed
            10000, // 10KB uncompressed (ratio: 10)
            0, 0,
        );
        assert!(!should_halt);
        assert!(violation.is_none());
    }

    #[test]
    fn test_should_halt_excessive_ratio() {
        let detector = SecurityDetector::default();
        let (should_halt, violation) = detector.should_halt_extraction(
            1000,   // 1KB compressed
            200000, // 200KB uncompressed (ratio: 200 > 100)
            0, 0,
        );
        assert!(should_halt);
        assert!(violation.is_some());
        let v = violation.unwrap();
        assert_eq!(v.violation_type, ViolationType::ExcessiveCompressionRatio);
        assert_eq!(v.severity, Severity::High);
    }

    #[test]
    fn test_should_halt_risk_score() {
        let detector = SecurityDetector::default();
        // ratio: 50, depth: 4 -> score: 50^4 = 6,250,000 > 1,000,000
        let (should_halt, violation) = detector.should_halt_extraction(
            1000,  // 1KB compressed
            50000, // 50KB uncompressed (ratio: 50)
            4,     // depth 4
            0,
        );
        assert!(should_halt);
        assert!(violation.is_some());
        let v = violation.unwrap();
        assert_eq!(v.violation_type, ViolationType::RiskScoreExceeded);
        assert_eq!(v.severity, Severity::Critical);
    }

    #[test]
    fn test_should_halt_cumulative_size() {
        let detector = SecurityDetector::default();
        let max_size = 10 * 1024 * 1024 * 1024; // 10GB
        let (should_halt, violation) = detector.should_halt_extraction(
            1000,
            1000,
            0,
            max_size + 1, // Already exceeded
        );
        assert!(should_halt);
        assert!(violation.is_some());
        let v = violation.unwrap();
        assert_eq!(v.violation_type, ViolationType::CumulativeSizeExceeded);
        assert_eq!(v.severity, Severity::Critical);
    }

    #[test]
    fn test_detect_suspicious_patterns_empty() {
        let detector = SecurityDetector::default();
        let warnings = detector.detect_suspicious_patterns(Path::new("test.zip"), &[]);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_detect_suspicious_patterns_high_ratio() {
        let detector = SecurityDetector::default();
        let entries = vec![ArchiveEntry {
            path: PathBuf::from("test.txt"),
            compressed_size: 1000,
            uncompressed_size: 60000, // ratio: 60 (> 50% of 100)
            is_directory: false,
        }];
        let warnings = detector.detect_suspicious_patterns(Path::new("test.zip"), &entries);
        assert!(!warnings.is_empty());
    }

    #[test]
    fn test_detect_suspicious_patterns_many_files() {
        let detector = SecurityDetector::default();
        let entries: Vec<ArchiveEntry> = (0..15000)
            .map(|i| ArchiveEntry {
                path: PathBuf::from(format!("file{}.txt", i)),
                compressed_size: 100,
                uncompressed_size: 100,
                is_directory: false,
            })
            .collect();
        let warnings = detector.detect_suspicious_patterns(Path::new("test.zip"), &entries);
        assert!(warnings.iter().any(|w| w.message.contains("15000 files")));
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    /// **Feature: enhanced-archive-handling, Property 10: Compression ratio calculation**
    /// **Validates: Requirements 3.1**
    ///
    /// Property: For any compressed and uncompressed sizes, the compression ratio
    /// should equal uncompressed_size / compressed_size, with proper handling of
    /// edge cases (zero values, infinity).
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_compression_ratio_calculation(
            compressed_size in 1u64..1_000_000_000u64,
            uncompressed_size in 1u64..1_000_000_000u64,
        ) {
            let detector = SecurityDetector::default();
            let ratio = detector.calculate_compression_ratio(compressed_size, uncompressed_size);

            // Property: ratio should equal uncompressed / compressed
            let expected_ratio = uncompressed_size as f64 / compressed_size as f64;
            prop_assert!((ratio - expected_ratio).abs() < 0.0001,
                "Compression ratio mismatch: got {}, expected {}", ratio, expected_ratio);

            // Property: ratio should be positive for positive inputs
            prop_assert!(ratio > 0.0, "Ratio should be positive for positive inputs");

            // Property: ratio should be finite for non-zero compressed size
            prop_assert!(ratio.is_finite(), "Ratio should be finite for non-zero compressed size");
        }

        #[test]
        fn prop_compression_ratio_zero_compressed(
            uncompressed_size in 0u64..1_000_000_000u64,
        ) {
            let detector = SecurityDetector::default();
            let ratio = detector.calculate_compression_ratio(0, uncompressed_size);

            if uncompressed_size == 0 {
                // Property: zero/zero should be 0.0
                prop_assert_eq!(ratio, 0.0, "Zero/zero should be 0.0");
            } else {
                // Property: non-zero/zero should be infinity
                prop_assert!(ratio.is_infinite(), "Non-zero/zero should be infinite");
                prop_assert!(ratio > 0.0, "Infinity should be positive");
            }
        }

        #[test]
        fn prop_compression_ratio_zero_uncompressed(
            compressed_size in 1u64..1_000_000_000u64,
        ) {
            let detector = SecurityDetector::default();
            let ratio = detector.calculate_compression_ratio(compressed_size, 0);

            // Property: zero uncompressed size should give ratio of 0.0
            prop_assert_eq!(ratio, 0.0, "Zero uncompressed should give 0.0 ratio");
        }

        #[test]
        fn prop_compression_ratio_symmetry(
            size in 1u64..1_000_000u64,
        ) {
            let detector = SecurityDetector::default();

            // Property: ratio(A, B) * ratio(B, A) should equal 1.0 (reciprocal relationship)
            let ratio1 = detector.calculate_compression_ratio(size, size * 10);
            let ratio2 = detector.calculate_compression_ratio(size * 10, size);

            let product = ratio1 * ratio2;
            prop_assert!((product - 1.0).abs() < 0.0001,
                "Reciprocal ratios should multiply to 1.0: {} * {} = {}", ratio1, ratio2, product);
        }

        #[test]
        fn prop_compression_ratio_monotonic(
            compressed_size in 1u64..1_000_000u64,
            uncompressed_size in 1u64..1_000_000u64,
        ) {
            let detector = SecurityDetector::default();

            // Property: increasing uncompressed size should increase ratio
            let ratio1 = detector.calculate_compression_ratio(compressed_size, uncompressed_size);
            let ratio2 = detector.calculate_compression_ratio(compressed_size, uncompressed_size * 2);

            prop_assert!(ratio2 >= ratio1,
                "Doubling uncompressed size should increase or maintain ratio: {} vs {}",
                ratio1, ratio2);
        }
    }

    /// **Feature: enhanced-archive-handling, Property 13: Exponential backoff scoring**
    /// **Validates: Requirements 3.4**
    ///
    /// Property: For any compression ratio and nesting depth, the risk score
    /// should equal ratio^depth, with proper handling of edge cases.
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_exponential_backoff_scoring(
            compression_ratio in 1.0f64..100.0f64,
            nesting_depth in 1usize..10usize, // Start from 1, not 0
        ) {
            let detector = SecurityDetector::default();
            let risk_score = detector.calculate_risk_score(compression_ratio, nesting_depth);

            // Property: risk score should equal ratio^depth
            let expected_score = compression_ratio.powi(nesting_depth as i32);
            prop_assert!((risk_score - expected_score).abs() < 0.01,
                "Risk score mismatch: got {}, expected {}", risk_score, expected_score);

            // Property: risk score should be positive for positive ratio
            prop_assert!(risk_score > 0.0, "Risk score should be positive");

            // Property: risk score should be finite for finite ratio
            prop_assert!(risk_score.is_finite(), "Risk score should be finite");
        }

        #[test]
        fn prop_exponential_backoff_depth_zero(
            compression_ratio in 1.0f64..1000.0f64,
        ) {
            let detector = SecurityDetector::default();
            let risk_score = detector.calculate_risk_score(compression_ratio, 0);

            // Property: at depth 0, risk score equals compression ratio
            prop_assert!((risk_score - compression_ratio).abs() < 0.0001,
                "At depth 0, risk score should equal ratio: {} vs {}", risk_score, compression_ratio);
        }

        #[test]
        fn prop_exponential_backoff_monotonic_depth(
            compression_ratio in 2.0f64..10.0f64,
            depth1 in 0usize..5usize,
        ) {
            let detector = SecurityDetector::default();
            let depth2 = depth1 + 1;

            // Property: increasing depth should increase risk score (for ratio > 1)
            let score1 = detector.calculate_risk_score(compression_ratio, depth1);
            let score2 = detector.calculate_risk_score(compression_ratio, depth2);

            prop_assert!(score2 >= score1,
                "Increasing depth should increase risk score: {} (depth {}) vs {} (depth {})",
                score1, depth1, score2, depth2);
        }

        #[test]
        fn prop_exponential_backoff_monotonic_ratio(
            ratio1 in 1.0f64..50.0f64,
            nesting_depth in 1usize..5usize,
        ) {
            let detector = SecurityDetector::default();
            let ratio2 = ratio1 * 1.5; // 50% increase

            // Property: increasing ratio should increase risk score (for depth > 0)
            let score1 = detector.calculate_risk_score(ratio1, nesting_depth);
            let score2 = detector.calculate_risk_score(ratio2, nesting_depth);

            prop_assert!(score2 >= score1,
                "Increasing ratio should increase risk score: {} (ratio {}) vs {} (ratio {})",
                score1, ratio1, score2, ratio2);
        }

        #[test]
        fn prop_exponential_backoff_zero_ratio(
            nesting_depth in 0usize..10usize,
        ) {
            let detector = SecurityDetector::default();
            let risk_score = detector.calculate_risk_score(0.0, nesting_depth);

            // Property: zero ratio should give zero risk score
            prop_assert_eq!(risk_score, 0.0, "Zero ratio should give zero risk score");
        }
    }

    /// **Feature: enhanced-archive-handling, Property 11: Suspicious file flagging**
    /// **Validates: Requirements 3.2**
    ///
    /// Property: For any file with compression ratio exceeding the threshold,
    /// the system should flag it as suspicious.
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_suspicious_file_flagging(
            compressed_size in 1u64..1_000_000u64,
            ratio_multiplier in 1.1f64..5.0f64, // Multiplier above threshold
        ) {
            let policy = SecurityPolicy {
                max_compression_ratio: 100.0,
                ..Default::default()
            };
            let detector = SecurityDetector::new(policy.clone());

            // Create a file that exceeds the threshold
            let uncompressed_size = (compressed_size as f64 * policy.max_compression_ratio * ratio_multiplier) as u64;

            let (should_halt, violation) = detector.should_halt_extraction(
                compressed_size,
                uncompressed_size,
                0,
                0,
            );

            // Property: files exceeding threshold should be flagged
            prop_assert!(should_halt, "File with ratio {} should be flagged",
                uncompressed_size as f64 / compressed_size as f64);
            prop_assert!(violation.is_some(), "Violation should be reported");

            if let Some(v) = violation {
                prop_assert!(v.metrics.is_some(), "Metrics should be included");
                if let Some(metrics) = v.metrics {
                    prop_assert!(metrics.compression_ratio > policy.max_compression_ratio,
                        "Reported ratio {} should exceed threshold {}",
                        metrics.compression_ratio, policy.max_compression_ratio);
                }
            }
        }

        #[test]
        fn prop_normal_files_not_flagged(
            compressed_size in 1000u64..1_000_000u64,
            ratio_multiplier in 0.1f64..0.9f64, // Multiplier below threshold
        ) {
            let policy = SecurityPolicy {
                max_compression_ratio: 100.0,
                ..Default::default()
            };
            let detector = SecurityDetector::new(policy.clone());

            // Create a file that's below the threshold
            let uncompressed_size = (compressed_size as f64 * policy.max_compression_ratio * ratio_multiplier) as u64;

            let (should_halt, violation) = detector.should_halt_extraction(
                compressed_size,
                uncompressed_size,
                0,
                0,
            );

            // Property: files below threshold should not be flagged
            prop_assert!(!should_halt, "File with ratio {} should not be flagged",
                uncompressed_size as f64 / compressed_size as f64);
            prop_assert!(violation.is_none(), "No violation should be reported for normal files");
        }

        #[test]
        fn prop_cumulative_size_enforcement(
            file_size in 1_000_000u64..10_000_000u64,
        ) {
            let policy = SecurityPolicy {
                max_cumulative_size: 100_000_000, // 100MB
                ..Default::default()
            };
            let detector = SecurityDetector::new(policy.clone());

            // Simulate extracting files until we exceed the limit
            let mut cumulative = 0u64;
            let mut should_halt_eventually = false;

            for _ in 0..20 {
                let (should_halt, _) = detector.should_halt_extraction(
                    file_size / 10, // Low compression ratio
                    file_size,
                    0,
                    cumulative,
                );

                if should_halt {
                    should_halt_eventually = true;
                    break;
                }

                cumulative = cumulative.saturating_add(file_size);
                if cumulative > policy.max_cumulative_size {
                    break;
                }
            }

            // Property: extraction should halt when cumulative size exceeds limit
            prop_assert!(should_halt_eventually || cumulative <= policy.max_cumulative_size,
                "Should halt when cumulative size {} exceeds limit {}",
                cumulative, policy.max_cumulative_size);
        }

        #[test]
        fn prop_detect_suspicious_patterns_consistency(
            num_files in 1usize..100usize,
            avg_ratio in 1.0f64..200.0f64,
        ) {
            let detector = SecurityDetector::default();

            // Create entries with varying compression ratios around the average
            let entries: Vec<ArchiveEntry> = (0..num_files)
                .map(|i| {
                    let compressed = 1000u64;
                    let ratio_variation = 0.8 + (i as f64 / num_files as f64) * 0.4; // 0.8 to 1.2
                    let uncompressed = (compressed as f64 * avg_ratio * ratio_variation) as u64;
                    ArchiveEntry {
                        path: PathBuf::from(format!("file{}.txt", i)),
                        compressed_size: compressed,
                        uncompressed_size: uncompressed,
                        is_directory: false,
                    }
                })
                .collect();

            let warnings = detector.detect_suspicious_patterns(
                &PathBuf::from("test.zip"),
                &entries,
            );

            // Property: if average ratio is high, warnings should be generated
            if avg_ratio > 50.0 {
                prop_assert!(!warnings.is_empty(),
                    "High average ratio {} should generate warnings", avg_ratio);
            }

            // Property: all warnings should have valid data
            for warning in &warnings {
                prop_assert!(!warning.message.is_empty(), "Warning message should not be empty");
            }
        }
    }
}
