//! Policy Manager for Configuration Management
//!
//! This module provides the PolicyManager for loading, validating, and managing
//! extraction policies with hot-reload support using RwLock for thread-safe access.

use super::extraction_policy::ExtractionPolicy;
use crate::error::{AppError, Result};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

/// Policy manager for configuration management
///
/// Manages extraction policies with support for:
/// - Loading from TOML files
/// - Validation of policy constraints
/// - Hot-reload with thread-safe access
/// - Default secure values
pub struct PolicyManager {
    /// Path to the configuration file
    config_path: PathBuf,

    /// Current active policy (thread-safe with RwLock)
    current_policy: Arc<RwLock<ExtractionPolicy>>,
}

impl PolicyManager {
    /// Create a new PolicyManager with the specified config path
    ///
    /// # Arguments
    ///
    /// * `config_path` - Path to the TOML configuration file
    ///
    /// # Returns
    ///
    /// A new PolicyManager instance with default policy
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::path::PathBuf;
    /// use log_analyzer::models::PolicyManager;
    ///
    /// let manager = PolicyManager::new(PathBuf::from("config/extraction_policy.toml"));
    /// ```
    pub fn new(config_path: PathBuf) -> Self {
        info!(
            "Initializing PolicyManager with config path: {:?}",
            config_path
        );

        Self {
            config_path,
            current_policy: Arc::new(RwLock::new(ExtractionPolicy::default())),
        }
    }

    /// Create a PolicyManager with a custom default policy
    ///
    /// # Arguments
    ///
    /// * `config_path` - Path to the TOML configuration file
    /// * `default_policy` - Default policy to use
    ///
    /// # Returns
    ///
    /// A new PolicyManager instance with the specified default policy
    pub fn with_default_policy(config_path: PathBuf, default_policy: ExtractionPolicy) -> Self {
        info!("Initializing PolicyManager with custom default policy");

        Self {
            config_path,
            current_policy: Arc::new(RwLock::new(default_policy)),
        }
    }

    /// Load policy from the configured TOML file
    ///
    /// Attempts to load and validate the policy from the TOML file.
    /// If loading fails, the current policy remains unchanged.
    ///
    /// # Returns
    ///
    /// The loaded and validated policy
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file cannot be read
    /// - The TOML parsing fails
    /// - The policy validation fails
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use std::path::PathBuf;
    /// # use log_analyzer::models::PolicyManager;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let manager = PolicyManager::new(PathBuf::from("config/extraction_policy.toml"));
    /// let policy = manager.load_policy().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn load_policy(&self) -> Result<ExtractionPolicy> {
        info!("Loading policy from: {:?}", self.config_path);

        // Check if file exists
        if !self.config_path.exists() {
            warn!(
                "Config file not found: {:?}, using default policy",
                self.config_path
            );
            return Ok(ExtractionPolicy::default());
        }

        // Read file content
        let content = tokio::fs::read_to_string(&self.config_path)
            .await
            .map_err(|e| {
                error!("Failed to read config file: {}", e);
                AppError::archive_error(
                    format!("Failed to read config file: {}", e),
                    Some(self.config_path.clone()),
                )
            })?;

        // Parse TOML
        let policy: ExtractionPolicy = toml::from_str(&content).map_err(|e| {
            error!("Failed to parse TOML config: {}", e);
            AppError::validation_error(format!("Failed to parse TOML config: {}", e))
        })?;

        // Validate policy
        self.validate_policy(&policy)?;

        info!("Successfully loaded and validated policy from config file");
        Ok(policy)
    }

    /// Validate policy constraints
    ///
    /// Checks all policy constraints to ensure they are within valid ranges
    /// and meet security requirements.
    ///
    /// # Arguments
    ///
    /// * `policy` - The policy to validate
    ///
    /// # Returns
    ///
    /// Ok(()) if validation succeeds
    ///
    /// # Errors
    ///
    /// Returns an error if any constraint is violated
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use std::path::PathBuf;
    /// # use log_analyzer::models::{PolicyManager, ExtractionPolicy};
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let manager = PolicyManager::new(PathBuf::from("config.toml"));
    /// let policy = ExtractionPolicy::default();
    /// manager.validate_policy(&policy)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn validate_policy(&self, policy: &ExtractionPolicy) -> Result<()> {
        policy.validate().map_err(|e| {
            error!("Policy validation failed: {}", e);
            AppError::validation_error(format!("Policy validation failed: {}", e))
        })
    }

    /// Update the current policy with hot-reload support
    ///
    /// Validates the new policy before applying it. If validation fails,
    /// the current policy remains unchanged.
    ///
    /// # Arguments
    ///
    /// * `policy` - The new policy to apply
    ///
    /// # Returns
    ///
    /// Ok(()) if the policy was successfully updated
    ///
    /// # Errors
    ///
    /// Returns an error if policy validation fails
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use std::path::PathBuf;
    /// # use log_analyzer::models::{PolicyManager, ExtractionPolicy};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let manager = PolicyManager::new(PathBuf::from("config.toml"));
    /// let new_policy = ExtractionPolicy::default();
    /// manager.update_policy(new_policy).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn update_policy(&self, policy: ExtractionPolicy) -> Result<()> {
        info!("Updating policy with hot-reload");

        // Validate before applying
        self.validate_policy(&policy)?;

        // Acquire write lock and update
        let mut current = self.current_policy.write().await;
        *current = policy.clone();

        info!(
            "Policy updated successfully: max_depth={}, max_file_size={}, max_total_size={}",
            policy.extraction.max_depth,
            policy.extraction.max_file_size,
            policy.extraction.max_total_size
        );

        Ok(())
    }

    /// Load policy from file and update current policy
    ///
    /// Convenience method that combines load_policy and update_policy.
    /// If loading or validation fails, the current policy remains unchanged.
    ///
    /// # Returns
    ///
    /// Ok(()) if the policy was successfully loaded and updated
    ///
    /// # Errors
    ///
    /// Returns an error if loading or validation fails
    pub async fn reload_from_file(&self) -> Result<()> {
        let policy = self.load_policy().await?;
        self.update_policy(policy).await
    }

    /// Get a clone of the current active policy (thread-safe)
    ///
    /// Returns a clone of the current policy for read access.
    /// This is thread-safe and can be called from multiple threads.
    ///
    /// # Returns
    ///
    /// A clone of the current policy
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use std::path::PathBuf;
    /// # use log_analyzer::models::PolicyManager;
    /// # async fn example() {
    /// let manager = PolicyManager::new(PathBuf::from("config.toml"));
    /// let policy = manager.get_policy().await;
    /// println!("Max depth: {}", policy.extraction.max_depth);
    /// # }
    /// ```
    pub async fn get_policy(&self) -> ExtractionPolicy {
        let policy = self.current_policy.read().await;
        policy.clone()
    }

    /// Get a reference to the config path
    pub fn config_path(&self) -> &PathBuf {
        &self.config_path
    }

    /// Check if the config file exists
    pub fn config_exists(&self) -> bool {
        self.config_path.exists()
    }
}

impl Default for PolicyManager {
    /// Create a PolicyManager with default config path
    ///
    /// Uses "config/extraction_policy.toml" as the default path
    fn default() -> Self {
        Self::new(PathBuf::from("config/extraction_policy.toml"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs;

    #[tokio::test]
    async fn test_policy_manager_creation() {
        let manager = PolicyManager::new(PathBuf::from("test_config.toml"));
        let policy = manager.get_policy().await;

        // Should have default policy
        assert_eq!(policy.extraction.max_depth, 10);
        assert_eq!(policy.extraction.max_file_size, 104_857_600);
    }

    #[tokio::test]
    async fn test_policy_manager_with_custom_default() {
        let mut custom_policy = ExtractionPolicy::default();
        custom_policy.extraction.max_depth = 15;

        let manager =
            PolicyManager::with_default_policy(PathBuf::from("test_config.toml"), custom_policy);

        let policy = manager.get_policy().await;
        assert_eq!(policy.extraction.max_depth, 15);
    }

    #[tokio::test]
    async fn test_load_policy_file_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("nonexistent.toml");

        let manager = PolicyManager::new(config_path);
        let policy = manager.load_policy().await.unwrap();

        // Should return default policy when file not found
        assert_eq!(policy.extraction.max_depth, 10);
    }

    #[tokio::test]
    async fn test_load_valid_policy_from_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");

        // Write valid TOML config
        let toml_content = r#"
            [extraction]
            max_depth = 15
            max_file_size = 104857600
            max_total_size = 10737418240
            max_workspace_size = 53687091200
            concurrent_extractions = 4
            buffer_size = 65536
            
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

        fs::write(&config_path, toml_content).await.unwrap();

        let manager = PolicyManager::new(config_path);
        let policy = manager.load_policy().await.unwrap();

        assert_eq!(policy.extraction.max_depth, 15);
        assert_eq!(policy.extraction.concurrent_extractions, 4);
    }

    #[tokio::test]
    async fn test_load_invalid_toml() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("invalid.toml");

        // Write invalid TOML
        fs::write(&config_path, "this is not valid toml {{{")
            .await
            .unwrap();

        let manager = PolicyManager::new(config_path);
        let result = manager.load_policy().await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_policy_success() {
        let manager = PolicyManager::default();
        let policy = ExtractionPolicy::default();

        assert!(manager.validate_policy(&policy).is_ok());
    }

    #[tokio::test]
    async fn test_validate_policy_invalid_max_depth() {
        let manager = PolicyManager::default();
        let mut policy = ExtractionPolicy::default();

        // Invalid: max_depth = 0
        policy.extraction.max_depth = 0;
        assert!(manager.validate_policy(&policy).is_err());

        // Invalid: max_depth = 21
        policy.extraction.max_depth = 21;
        assert!(manager.validate_policy(&policy).is_err());

        // Valid: max_depth = 10
        policy.extraction.max_depth = 10;
        assert!(manager.validate_policy(&policy).is_ok());
    }

    #[tokio::test]
    async fn test_update_policy_success() {
        let manager = PolicyManager::default();

        let mut new_policy = ExtractionPolicy::default();
        new_policy.extraction.max_depth = 15;

        manager.update_policy(new_policy).await.unwrap();

        let current = manager.get_policy().await;
        assert_eq!(current.extraction.max_depth, 15);
    }

    #[tokio::test]
    async fn test_update_policy_validation_failure() {
        let manager = PolicyManager::default();

        // Get initial policy
        let initial = manager.get_policy().await;
        let initial_depth = initial.extraction.max_depth;

        // Try to update with invalid policy
        let mut invalid_policy = ExtractionPolicy::default();
        invalid_policy.extraction.max_depth = 0;

        let result = manager.update_policy(invalid_policy).await;
        assert!(result.is_err());

        // Current policy should remain unchanged
        let current = manager.get_policy().await;
        assert_eq!(current.extraction.max_depth, initial_depth);
    }

    #[tokio::test]
    async fn test_reload_from_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("reload_test.toml");

        // Write initial config
        let toml_content = r#"
            [extraction]
            max_depth = 12
            max_file_size = 104857600
            max_total_size = 10737418240
            max_workspace_size = 53687091200
            concurrent_extractions = 4
            buffer_size = 65536
            
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

        fs::write(&config_path, toml_content).await.unwrap();

        let manager = PolicyManager::new(config_path);
        manager.reload_from_file().await.unwrap();

        let policy = manager.get_policy().await;
        assert_eq!(policy.extraction.max_depth, 12);
    }

    #[tokio::test]
    async fn test_concurrent_access() {
        let manager = Arc::new(PolicyManager::default());

        // Spawn multiple tasks reading the policy
        let mut handles = vec![];

        for _ in 0..10 {
            let manager_clone = Arc::clone(&manager);
            let handle = tokio::spawn(async move {
                let policy = manager_clone.get_policy().await;
                assert_eq!(policy.extraction.max_depth, 10);
            });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        for handle in handles {
            handle.await.unwrap();
        }
    }

    #[tokio::test]
    async fn test_config_path_methods() {
        let config_path = PathBuf::from("test_config.toml");
        let manager = PolicyManager::new(config_path.clone());

        assert_eq!(manager.config_path(), &config_path);
        assert!(!manager.config_exists());
    }
}
