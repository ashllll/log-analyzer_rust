use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Complete extraction policy configuration loaded from TOML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionPolicy {
    pub extraction: ExtractionConfig,
    pub security: SecurityConfig,
    pub paths: PathsConfig,
    pub performance: PerformanceConfig,
    pub audit: AuditConfig,
}

/// Extraction operation parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionConfig {
    /// Maximum nesting depth (1-20)
    pub max_depth: usize,

    /// Maximum file size in bytes
    pub max_file_size: u64,

    /// Maximum total size per archive in bytes
    pub max_total_size: u64,

    /// Maximum total size per workspace in bytes
    pub max_workspace_size: u64,

    /// Number of concurrent extractions (0 = auto-detect)
    pub concurrent_extractions: usize,

    /// Buffer size for streaming in bytes
    pub buffer_size: usize,

    /// Use enhanced extraction system (default: false for backward compatibility)
    pub use_enhanced_extraction: bool,
}

/// Security and zip bomb detection parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Compression ratio threshold for flagging suspicious files
    pub compression_ratio_threshold: f64,

    /// Exponential backoff threshold (ratio^depth)
    pub exponential_backoff_threshold: f64,

    /// Enable zip bomb detection
    pub enable_zip_bomb_detection: bool,
}

/// Path management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathsConfig {
    /// Enable Windows long path support (UNC prefix)
    pub enable_long_paths: bool,

    /// Path shortening threshold (0.0-1.0)
    pub shortening_threshold: f32,

    /// Hash algorithm for path shortening
    pub hash_algorithm: String,

    /// Length of hash for shortened paths
    pub hash_length: usize,
}

/// Performance optimization parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Temporary directory TTL in hours
    pub temp_dir_ttl_hours: u64,

    /// Log retention in days
    pub log_retention_days: usize,

    /// Enable streaming extraction
    pub enable_streaming: bool,

    /// Directory creation batch size
    pub directory_batch_size: usize,

    /// Parallel files per archive
    pub parallel_files_per_archive: usize,
}

/// Audit logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    /// Enable audit logging
    pub enable_audit_logging: bool,

    /// Log format: "json" or "text"
    pub log_format: String,

    /// Log level
    pub log_level: String,

    /// Enable security event logging
    pub log_security_events: bool,
}

impl Default for ExtractionPolicy {
    fn default() -> Self {
        Self {
            extraction: ExtractionConfig {
                max_depth: 10,
                max_file_size: 104_857_600,         // 100MB
                max_total_size: 10_737_418_240,     // 10GB
                max_workspace_size: 53_687_091_200, // 50GB
                concurrent_extractions: 0,          // Auto-detect
                buffer_size: 65_536,                // 64KB
                use_enhanced_extraction: false,     // Default to false for backward compatibility
            },
            security: SecurityConfig {
                compression_ratio_threshold: 100.0,
                exponential_backoff_threshold: 1_000_000.0,
                enable_zip_bomb_detection: true,
            },
            paths: PathsConfig {
                enable_long_paths: true,
                shortening_threshold: 0.8,
                hash_algorithm: "SHA256".to_string(),
                hash_length: 16,
            },
            performance: PerformanceConfig {
                temp_dir_ttl_hours: 24,
                log_retention_days: 90,
                enable_streaming: true,
                directory_batch_size: 10,
                parallel_files_per_archive: 4,
            },
            audit: AuditConfig {
                enable_audit_logging: true,
                log_format: "json".to_string(),
                log_level: "info".to_string(),
                log_security_events: true,
            },
        }
    }
}

impl ExtractionPolicy {
    /// Load policy from TOML file
    pub fn from_file(path: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let policy: ExtractionPolicy = toml::from_str(&content)?;
        Ok(policy)
    }

    /// Load policy from TOML string
    pub fn from_str(content: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let policy: ExtractionPolicy = toml::from_str(content)?;
        Ok(policy)
    }

    /// Validate policy constraints
    pub fn validate(&self) -> Result<(), String> {
        // Validate max_depth range
        if self.extraction.max_depth < 1 || self.extraction.max_depth > 20 {
            return Err(format!(
                "max_depth must be between 1 and 20, got {}",
                self.extraction.max_depth
            ));
        }

        // Validate positive sizes
        if self.extraction.max_file_size == 0 {
            return Err("max_file_size must be positive".to_string());
        }

        if self.extraction.max_total_size == 0 {
            return Err("max_total_size must be positive".to_string());
        }

        if self.extraction.max_workspace_size == 0 {
            return Err("max_workspace_size must be positive".to_string());
        }

        // Validate buffer size
        if self.extraction.buffer_size == 0 {
            return Err("buffer_size must be positive".to_string());
        }

        // Validate shortening threshold
        if self.paths.shortening_threshold <= 0.0 || self.paths.shortening_threshold > 1.0 {
            return Err(format!(
                "shortening_threshold must be between 0.0 and 1.0, got {}",
                self.paths.shortening_threshold
            ));
        }

        // Validate hash length
        if self.paths.hash_length < 8 || self.paths.hash_length > 32 {
            return Err(format!(
                "hash_length must be between 8 and 32, got {}",
                self.paths.hash_length
            ));
        }

        // Validate hash algorithm
        if self.paths.hash_algorithm != "SHA256" && self.paths.hash_algorithm != "SHA512" {
            return Err(format!(
                "hash_algorithm must be SHA256 or SHA512, got {}",
                self.paths.hash_algorithm
            ));
        }

        // Validate log format
        if self.audit.log_format != "json" && self.audit.log_format != "text" {
            return Err(format!(
                "log_format must be 'json' or 'text', got {}",
                self.audit.log_format
            ));
        }

        // Validate log level
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&self.audit.log_level.as_str()) {
            return Err(format!(
                "log_level must be one of {:?}, got {}",
                valid_levels, self.audit.log_level
            ));
        }

        // Validate compression ratio threshold
        if self.security.compression_ratio_threshold <= 0.0 {
            return Err("compression_ratio_threshold must be positive".to_string());
        }

        // Validate exponential backoff threshold
        if self.security.exponential_backoff_threshold <= 0.0 {
            return Err("exponential_backoff_threshold must be positive".to_string());
        }

        // Validate performance parameters
        if self.performance.directory_batch_size == 0 {
            return Err("directory_batch_size must be positive".to_string());
        }

        if self.performance.parallel_files_per_archive == 0
            || self.performance.parallel_files_per_archive > 8
        {
            return Err(format!(
                "parallel_files_per_archive must be between 1 and 8, got {}",
                self.performance.parallel_files_per_archive
            ));
        }

        Ok(())
    }

    /// Get temp directory TTL as Duration
    pub fn temp_dir_ttl(&self) -> Duration {
        Duration::from_secs(self.performance.temp_dir_ttl_hours * 3600)
    }

    /// Get concurrent extractions count (auto-detect if 0)
    pub fn concurrent_extractions(&self) -> usize {
        if self.extraction.concurrent_extractions == 0 {
            num_cpus::get() / 2
        } else {
            self.extraction.concurrent_extractions
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_policy_is_valid() {
        let policy = ExtractionPolicy::default();
        assert!(policy.validate().is_ok());
    }

    #[test]
    fn test_invalid_max_depth() {
        let mut policy = ExtractionPolicy::default();
        policy.extraction.max_depth = 0;
        assert!(policy.validate().is_err());

        policy.extraction.max_depth = 21;
        assert!(policy.validate().is_err());
    }

    #[test]
    fn test_invalid_shortening_threshold() {
        let mut policy = ExtractionPolicy::default();
        policy.paths.shortening_threshold = 0.0;
        assert!(policy.validate().is_err());

        policy.paths.shortening_threshold = 1.5;
        assert!(policy.validate().is_err());
    }

    #[test]
    fn test_invalid_hash_length() {
        let mut policy = ExtractionPolicy::default();
        policy.paths.hash_length = 7;
        assert!(policy.validate().is_err());

        policy.paths.hash_length = 33;
        assert!(policy.validate().is_err());
    }

    #[test]
    fn test_parse_toml_config() {
        let toml_str = r#"
            [extraction]
            max_depth = 15
            max_file_size = 104857600
            max_total_size = 10737418240
            max_workspace_size = 53687091200
            concurrent_extractions = 4
            buffer_size = 65536
            use_enhanced_extraction = false
            
            [security]
            compression_ratio_threshold = 100.0
            exponential_backoff_threshold = 1000000.0
            enable_zip_bomb_detection = true
            
            [paths]
            enable_long_paths = true
            shortening_threshold = 0.8
            hash_algorithm = "SHA256"
            hash_length = 16
            
            [performance]
            temp_dir_ttl_hours = 24
            log_retention_days = 90
            enable_streaming = true
            directory_batch_size = 10
            parallel_files_per_archive = 4
            
            [audit]
            enable_audit_logging = true
            log_format = "json"
            log_level = "info"
            log_security_events = true
        "#;

        let policy = ExtractionPolicy::from_str(toml_str).unwrap();
        assert_eq!(policy.extraction.max_depth, 15);
        assert!(policy.validate().is_ok());
    }
}
