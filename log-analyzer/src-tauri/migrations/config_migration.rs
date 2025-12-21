//! Configuration migration tool
//!
//! Converts old configuration format to new TOML-based format

use eyre::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use toml;
use tracing::{info, warn};

/// Old configuration format (JSON-based)
#[derive(Debug, Deserialize)]
pub struct OldConfig {
    pub max_file_size: Option<u64>,
    pub max_total_size: Option<u64>,
    pub max_file_count: Option<usize>,
    pub allowed_extensions: Option<Vec<String>>,
    pub forbidden_extensions: Option<Vec<String>>,
    pub enable_security_checks: Option<bool>,
}

/// New configuration format (TOML-based)
#[derive(Debug, Serialize, Deserialize)]
pub struct NewConfig {
    pub extraction: ExtractionConfig,
    pub security: SecurityConfig,
    pub paths: PathsConfig,
    pub performance: PerformanceConfig,
    pub audit: AuditConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExtractionConfig {
    pub max_depth: usize,
    pub max_file_size: u64,
    pub max_total_size: u64,
    pub max_workspace_size: u64,
    pub concurrent_extractions: usize,
    pub buffer_size: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub compression_ratio_threshold: f64,
    pub exponential_backoff_threshold: f64,
    pub enable_zip_bomb_detection: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PathsConfig {
    pub enable_long_paths: bool,
    pub shortening_threshold: f32,
    pub hash_algorithm: String,
    pub hash_length: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PerformanceConfig {
    pub temp_dir_ttl_hours: u64,
    pub log_retention_days: usize,
    pub enable_streaming: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuditConfig {
    pub enable_audit_logging: bool,
    pub log_format: String,
    pub log_level: String,
}

impl Default for NewConfig {
    fn default() -> Self {
        Self {
            extraction: ExtractionConfig {
                max_depth: 10,
                max_file_size: 104_857_600,      // 100MB
                max_total_size: 10_737_418_240,  // 10GB
                max_workspace_size: 53_687_091_200, // 50GB
                concurrent_extractions: num_cpus::get() / 2,
                buffer_size: 65536, // 64KB
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
            },
            audit: AuditConfig {
                enable_audit_logging: true,
                log_format: "json".to_string(),
                log_level: "info".to_string(),
            },
        }
    }
}

/// Configuration migration tool
pub struct ConfigMigration {
    old_config_path: PathBuf,
    new_config_path: PathBuf,
}

impl ConfigMigration {
    /// Create a new configuration migration tool
    pub fn new(old_config_path: PathBuf, new_config_path: PathBuf) -> Self {
        Self {
            old_config_path,
            new_config_path,
        }
    }

    /// Execute the configuration migration
    pub async fn migrate(&self) -> Result<NewConfig> {
        info!("Starting configuration migration");
        info!("Old config: {:?}", self.old_config_path);
        info!("New config: {:?}", self.new_config_path);

        // Load old configuration
        let old_config = self.load_old_config().await?;

        // Convert to new format
        let new_config = self.convert_config(old_config)?;

        // Validate new configuration
        self.validate_config(&new_config)?;

        // Save new configuration
        self.save_new_config(&new_config).await?;

        info!("Configuration migration completed successfully");
        Ok(new_config)
    }

    /// Load old configuration from JSON file
    async fn load_old_config(&self) -> Result<OldConfig> {
        if !self.old_config_path.exists() {
            warn!("Old configuration file not found, using defaults");
            return Ok(OldConfig {
                max_file_size: None,
                max_total_size: None,
                max_file_count: None,
                allowed_extensions: None,
                forbidden_extensions: None,
                enable_security_checks: None,
            });
        }

        let content = tokio::fs::read_to_string(&self.old_config_path)
            .await
            .context("Failed to read old configuration file")?;

        let old_config: OldConfig = serde_json::from_str(&content)
            .context("Failed to parse old configuration")?;

        Ok(old_config)
    }

    /// Convert old configuration to new format
    fn convert_config(&self, old: OldConfig) -> Result<NewConfig> {
        let mut new_config = NewConfig::default();

        // Map old values to new structure
        if let Some(max_file_size) = old.max_file_size {
            new_config.extraction.max_file_size = max_file_size;
        }

        if let Some(max_total_size) = old.max_total_size {
            new_config.extraction.max_total_size = max_total_size;
        }

        if let Some(enable_security) = old.enable_security_checks {
            new_config.security.enable_zip_bomb_detection = enable_security;
        }

        // Log any unmapped fields
        if old.allowed_extensions.is_some() {
            warn!("allowed_extensions field is not directly mapped in new config");
        }
        if old.forbidden_extensions.is_some() {
            warn!("forbidden_extensions field is not directly mapped in new config");
        }

        Ok(new_config)
    }

    /// Validate new configuration
    fn validate_config(&self, config: &NewConfig) -> Result<()> {
        // Validate extraction config
        if config.extraction.max_depth < 1 || config.extraction.max_depth > 20 {
            return Err(eyre::eyre!("max_depth must be between 1 and 20"));
        }

        if config.extraction.max_file_size == 0 {
            return Err(eyre::eyre!("max_file_size must be positive"));
        }

        if config.extraction.max_total_size == 0 {
            return Err(eyre::eyre!("max_total_size must be positive"));
        }

        // Validate security config
        if config.security.compression_ratio_threshold <= 0.0 {
            return Err(eyre::eyre!("compression_ratio_threshold must be positive"));
        }

        // Validate paths config
        if config.paths.shortening_threshold <= 0.0 || config.paths.shortening_threshold > 1.0 {
            return Err(eyre::eyre!("shortening_threshold must be between 0 and 1"));
        }

        if config.paths.hash_length < 8 || config.paths.hash_length > 64 {
            return Err(eyre::eyre!("hash_length must be between 8 and 64"));
        }

        // Validate performance config
        if config.performance.temp_dir_ttl_hours == 0 {
            return Err(eyre::eyre!("temp_dir_ttl_hours must be positive"));
        }

        // Validate audit config
        let valid_log_formats = ["json", "text", "pretty"];
        if !valid_log_formats.contains(&config.audit.log_format.as_str()) {
            return Err(eyre::eyre!("log_format must be one of: json, text, pretty"));
        }

        let valid_log_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_log_levels.contains(&config.audit.log_level.as_str()) {
            return Err(eyre::eyre!("log_level must be one of: trace, debug, info, warn, error"));
        }

        Ok(())
    }

    /// Save new configuration to TOML file
    async fn save_new_config(&self, config: &NewConfig) -> Result<()> {
        let toml_string = toml::to_string_pretty(config)
            .context("Failed to serialize configuration to TOML")?;

        // Create parent directory if it doesn't exist
        if let Some(parent) = self.new_config_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .context("Failed to create config directory")?;
        }

        tokio::fs::write(&self.new_config_path, toml_string)
            .await
            .context("Failed to write new configuration file")?;

        info!("New configuration saved to: {:?}", self.new_config_path);
        Ok(())
    }

    /// Create a backup of the old configuration
    pub async fn backup_old_config(&self) -> Result<PathBuf> {
        if !self.old_config_path.exists() {
            return Ok(PathBuf::new());
        }

        let backup_path = self.old_config_path.with_extension("json.backup");
        tokio::fs::copy(&self.old_config_path, &backup_path)
            .await
            .context("Failed to create backup")?;

        info!("Old configuration backed up to: {:?}", backup_path);
        Ok(backup_path)
    }
}

/// CLI entry point for configuration migration
#[cfg(feature = "cli")]
pub async fn run_config_migration_cli() -> Result<()> {
    use clap::Parser;

    #[derive(Parser)]
    #[command(name = "config-migration")]
    #[command(about = "Migrate archive configuration to new TOML format")]
    struct Cli {
        /// Path to old JSON configuration
        #[arg(long, default_value = "config/archive.json")]
        old_config: PathBuf,

        /// Path to new TOML configuration
        #[arg(long, default_value = "config/extraction_policy.toml")]
        new_config: PathBuf,

        /// Create backup of old configuration
        #[arg(long)]
        backup: bool,
    }

    let cli = Cli::parse();

    let migration = ConfigMigration::new(cli.old_config, cli.new_config);

    if cli.backup {
        migration.backup_old_config().await?;
    }

    let new_config = migration.migrate().await?;

    println!("\n=== Configuration Migration Complete ===");
    println!("New configuration:");
    println!("{}", toml::to_string_pretty(&new_config)?);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_config_migration() {
        let temp_dir = TempDir::new().unwrap();
        let old_config_path = temp_dir.path().join("old.json");
        let new_config_path = temp_dir.path().join("new.toml");

        // Create old config
        let old_config = OldConfig {
            max_file_size: Some(50_000_000),
            max_total_size: Some(500_000_000),
            max_file_count: Some(500),
            allowed_extensions: None,
            forbidden_extensions: None,
            enable_security_checks: Some(true),
        };

        let old_json = serde_json::to_string_pretty(&old_config).unwrap();
        tokio::fs::write(&old_config_path, old_json).await.unwrap();

        // Run migration
        let migration = ConfigMigration::new(old_config_path, new_config_path.clone());
        let result = migration.migrate().await;

        assert!(result.is_ok());
        assert!(new_config_path.exists());

        // Verify new config
        let new_config = result.unwrap();
        assert_eq!(new_config.extraction.max_file_size, 50_000_000);
        assert_eq!(new_config.extraction.max_total_size, 500_000_000);
        assert!(new_config.security.enable_zip_bomb_detection);
    }

    #[test]
    fn test_validate_config() {
        let migration = ConfigMigration::new(PathBuf::new(), PathBuf::new());
        
        // Valid config
        let valid_config = NewConfig::default();
        assert!(migration.validate_config(&valid_config).is_ok());

        // Invalid max_depth
        let mut invalid_config = NewConfig::default();
        invalid_config.extraction.max_depth = 0;
        assert!(migration.validate_config(&invalid_config).is_err());

        // Invalid compression ratio
        invalid_config = NewConfig::default();
        invalid_config.security.compression_ratio_threshold = -1.0;
        assert!(migration.validate_config(&invalid_config).is_err());
    }
}
