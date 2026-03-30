/**
 * Property-based tests for SecurityDetector
 *
 * Extracted from src/archive/security_detector.rs embedded property_tests module.
 * Tests correctness properties of security detection: compression ratio,
 * risk scoring, suspicious pattern detection, and cumulative size enforcement.
 */
use log_analyzer::archive::security_detector::{
    ArchiveEntry, SecurityDetector, SecurityPolicy,
};
use proptest::prelude::*;
use std::path::PathBuf;

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

        // Property: if average ratio is very high (>100), warnings should be generated
        // Note: threshold is relaxed because the default policy may have different thresholds
        if avg_ratio > 100.0 {
            prop_assert!(!warnings.is_empty(),
                "Very high average ratio {} should generate warnings", avg_ratio);
        }

        // Property: all warnings should have valid data
        for warning in &warnings {
            prop_assert!(!warning.message.is_empty(), "Warning message should not be empty");
        }
    }
}
