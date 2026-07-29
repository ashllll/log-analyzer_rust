//! Shared runtime policy for archive extraction.
//!
//! This type lives in `la-core` so application and archive crates use one
//! policy contract. Persisted user configuration remains a separate model in
//! `models::extraction_policy`.

use crate::error::{AppError, Result};

/// Runtime limits and tuning parameters for one archive extraction.
#[derive(Debug, Clone)]
pub struct ExtractionPolicy {
    /// Maximum archive nesting depth (default: 10, range: 1-20).
    pub max_depth: usize,
    /// Maximum single-file size in bytes (default: 100 MiB).
    pub max_file_size: u64,
    /// Maximum total extracted size per archive (default: 500 MiB).
    pub max_total_size: u64,
    /// Streaming buffer size in bytes (default: 1 MiB).
    pub buffer_size: usize,
    /// Directory creation batch size (default: 10).
    pub dir_batch_size: usize,
    /// Maximum concurrent per-file post-processing operations within an archive (default: 4).
    pub max_parallel_files: usize,
    /// Maximum number of files per archive (default: 10,000).
    pub max_file_count: usize,
}

impl Default for ExtractionPolicy {
    fn default() -> Self {
        Self {
            max_depth: 10,
            max_file_size: 100 * 1024 * 1024,
            max_total_size: 500 * 1024 * 1024,
            buffer_size: 1024 * 1024,
            dir_batch_size: 10,
            max_parallel_files: 4,
            max_file_count: 10_000,
        }
    }
}

impl ExtractionPolicy {
    /// Validate all runtime limits before extraction starts.
    pub fn validate(&self) -> Result<()> {
        if !(1..=20).contains(&self.max_depth) {
            return Err(AppError::validation_error(format!(
                "max_depth must be between 1 and 20, got {}",
                self.max_depth
            )));
        }
        if self.max_file_size == 0 {
            return Err(AppError::validation_error("max_file_size must be positive"));
        }
        if self.max_total_size == 0 {
            return Err(AppError::validation_error(
                "max_total_size must be positive",
            ));
        }
        if self.buffer_size == 0 {
            return Err(AppError::validation_error("buffer_size must be positive"));
        }
        if self.dir_batch_size == 0 {
            return Err(AppError::validation_error(
                "dir_batch_size must be positive",
            ));
        }
        if self.max_parallel_files == 0 {
            return Err(AppError::validation_error(
                "max_parallel_files must be positive",
            ));
        }
        if self.max_file_count == 0 {
            return Err(AppError::validation_error(
                "max_file_count must be positive",
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_policy_is_valid() {
        assert!(ExtractionPolicy::default().validate().is_ok());
    }

    #[test]
    fn rejects_zero_file_count() {
        let policy = ExtractionPolicy {
            max_file_count: 0,
            ..ExtractionPolicy::default()
        };

        let error = policy.validate().expect_err("zero file count must fail");
        assert!(error.to_string().contains("max_file_count"));
    }
}
