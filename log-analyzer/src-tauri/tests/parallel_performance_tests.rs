//! Performance tests for parallel processing
//!
//! These tests verify that parallel processing provides performance benefits
//! and that memory usage remains reasonable under load.
//!
//! # Requirements
//!
//! Validates: Requirements 6.1, 6.2

use log_analyzer::archive::{ParallelConfig, ParallelProcessor};
use log_analyzer::AppError;
use std::path::PathBuf;
use std::time::Instant;
use tempfile::TempDir;

/// Helper to create test files
fn create_test_files(dir: &std::path::Path, count: usize) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for i in 0..count {
        let file_path = dir.join(format!("file{}.txt", i));
        std::fs::write(&file_path, format!("content {}", i)).unwrap();
        files.push(file_path);
    }
    files
}

#[test]
fn test_parallel_processing_faster_than_sequential() {
    let temp_dir = TempDir::new().unwrap();
    let processor = ParallelProcessor::new();

    // Create 100 test files
    let file_paths = create_test_files(temp_dir.path(), 100);

    // Sequential processing (simulated by using 1 worker)
    let sequential_config = ParallelConfig {
        max_concurrent_archives: 1,
        db_batch_size: 100,
        worker_threads: 1,
    };
    let sequential_processor = ParallelProcessor::with_config(sequential_config);

    let start = Instant::now();
    let _results = sequential_processor.process_files_parallel(file_paths.clone(), |path| {
        // Simulate some work
        std::thread::sleep(std::time::Duration::from_micros(100));
        if path.exists() {
            Ok(())
        } else {
            Err(AppError::not_found(format!(
                "File not found: {}",
                path.display()
            )))
        }
    });
    let sequential_duration = start.elapsed();

    // Parallel processing (using default config with multiple workers)
    let start = Instant::now();
    let _results = processor.process_files_parallel(file_paths, |path| {
        // Simulate some work
        std::thread::sleep(std::time::Duration::from_micros(100));
        if path.exists() {
            Ok(())
        } else {
            Err(AppError::not_found(format!(
                "File not found: {}",
                path.display()
            )))
        }
    });
    let parallel_duration = start.elapsed();

    println!(
        "Sequential: {:?}, Parallel: {:?}, Speedup: {:.2}x",
        sequential_duration,
        parallel_duration,
        sequential_duration.as_secs_f64() / parallel_duration.as_secs_f64()
    );

    // Parallel should be faster (with some tolerance for test environment variability)
    // We expect at least some speedup, but not necessarily linear due to overhead
    assert!(
        parallel_duration < sequential_duration,
        "Parallel processing should be faster than sequential"
    );
}

#[test]
fn test_batch_size_configuration() {
    let processor = ParallelProcessor::new();

    // Test with different batch sizes
    let batch_sizes = vec![10, 50, 100, 200];

    for batch_size in batch_sizes {
        let config = ParallelConfig {
            max_concurrent_archives: 4,
            db_batch_size: batch_size,
            worker_threads: num_cpus::get(),
        };

        let custom_processor = ParallelProcessor::with_config(config.clone());

        // Verify configuration is applied
        assert_eq!(custom_processor.config.db_batch_size, batch_size);
        assert_eq!(custom_processor.config.max_concurrent_archives, 4);
    }
}

#[test]
fn test_parallel_processing_with_large_file_count() {
    let temp_dir = TempDir::new().unwrap();
    let processor = ParallelProcessor::new();

    // Create 1000 test files (simulating large archive)
    let file_paths = create_test_files(temp_dir.path(), 1000);

    let start = Instant::now();
    let results = processor.process_files_parallel(file_paths, |path| {
        if path.exists() {
            Ok(path.to_path_buf())
        } else {
            Err(AppError::not_found(format!(
                "File not found: {}",
                path.display()
            )))
        }
    });
    let duration = start.elapsed();

    // All files should be processed successfully
    assert_eq!(results.len(), 1000);
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1000);

    println!(
        "Processed 1000 files in {:?} ({:.2} files/sec)",
        duration,
        1000.0 / duration.as_secs_f64()
    );

    // Should complete in reasonable time (< 5 seconds for 1000 files)
    assert!(
        duration.as_secs() < 5,
        "Processing 1000 files should complete in under 5 seconds"
    );
}

#[test]
fn test_memory_usage_stays_reasonable() {
    let temp_dir = TempDir::new().unwrap();
    let processor = ParallelProcessor::new();

    // Create files with varying sizes
    let mut file_paths = Vec::new();
    for i in 0..100 {
        let file_path = temp_dir.path().join(format!("file{}.txt", i));
        // Create files with different sizes (1KB to 100KB)
        let content = vec![b'x'; (i + 1) * 1024];
        std::fs::write(&file_path, content).unwrap();
        file_paths.push(file_path);
    }

    // Get initial memory usage (approximate)
    let initial_memory = get_approximate_memory_usage();

    // Process files
    let _results = processor.process_files_parallel(file_paths, |path| {
        // Read file to simulate memory usage
        let _content = std::fs::read(path).ok();
        Ok(())
    });

    // Get final memory usage
    let final_memory = get_approximate_memory_usage();

    println!(
        "Memory usage: initial={} MB, final={} MB, delta={} MB",
        initial_memory / 1024 / 1024,
        final_memory / 1024 / 1024,
        (final_memory - initial_memory) / 1024 / 1024
    );

    // Memory increase should be reasonable (< 100MB for this test)
    // This is a rough check since we can't precisely control memory in tests
    let memory_increase = final_memory.saturating_sub(initial_memory);
    assert!(
        memory_increase < 100 * 1024 * 1024,
        "Memory increase should be less than 100MB"
    );
}

#[test]
fn test_parallel_config_scales_with_cpu_count() {
    let config = ParallelConfig::default();
    let cpu_count = num_cpus::get();

    // Worker threads should match CPU count
    assert_eq!(config.worker_threads, cpu_count);

    // Max concurrent archives should be reasonable (capped at 4)
    assert!(config.max_concurrent_archives <= 4);
    assert!(config.max_concurrent_archives > 0);

    println!(
        "CPU count: {}, Worker threads: {}, Max concurrent: {}",
        cpu_count, config.worker_threads, config.max_concurrent_archives
    );
}

#[test]
fn test_error_handling_doesnt_block_parallel_processing() {
    let temp_dir = TempDir::new().unwrap();
    let processor = ParallelProcessor::new();

    // Create mix of existing and non-existing files
    let mut file_paths = Vec::new();
    for i in 0..50 {
        let file_path = temp_dir.path().join(format!("file{}.txt", i));
        if i % 2 == 0 {
            // Create only even-numbered files
            std::fs::write(&file_path, format!("content {}", i)).unwrap();
        }
        file_paths.push(file_path);
    }

    let start = Instant::now();
    let results = processor.process_files_parallel(file_paths, |path| {
        if path.exists() {
            Ok(())
        } else {
            Err(AppError::not_found(format!(
                "File not found: {}",
                path.display()
            )))
        }
    });
    let duration = start.elapsed();

    // Should process all files (some succeed, some fail)
    assert_eq!(results.len(), 50);
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 25); // Even files
    assert_eq!(results.iter().filter(|r| r.is_err()).count(), 25); // Odd files

    println!("Processed 50 files (25 errors) in {:?}", duration);

    // Errors shouldn't significantly slow down processing
    assert!(
        duration.as_secs() < 2,
        "Processing with errors should still be fast"
    );
}

/// Get approximate memory usage (platform-specific, best effort)
fn get_approximate_memory_usage() -> usize {
    #[cfg(target_os = "linux")]
    {
        // On Linux, read from /proc/self/status
        if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
            for line in status.lines() {
                if line.starts_with("VmRSS:") {
                    if let Some(kb_str) = line.split_whitespace().nth(1) {
                        if let Ok(kb) = kb_str.parse::<usize>() {
                            return kb * 1024; // Convert to bytes
                        }
                    }
                }
            }
        }
    }

    // Fallback: return a dummy value
    // In real tests, we'd use a proper memory profiling tool
    0
}

#[test]
fn test_parallel_processor_handles_empty_input() {
    let processor = ParallelProcessor::new();

    let results = processor.process_files_parallel(Vec::new(), |_path| Ok(()));

    assert_eq!(results.len(), 0);
}

#[test]
fn test_parallel_processor_with_single_file() {
    let temp_dir = TempDir::new().unwrap();
    let processor = ParallelProcessor::new();

    let file_path = temp_dir.path().join("single.txt");
    std::fs::write(&file_path, "content").unwrap();

    let results = processor.process_files_parallel(vec![file_path], |path| {
        if path.exists() {
            Ok(())
        } else {
            Err(AppError::not_found(format!(
                "File not found: {}",
                path.display()
            )))
        }
    });

    assert_eq!(results.len(), 1);
    assert!(results[0].is_ok());
}
