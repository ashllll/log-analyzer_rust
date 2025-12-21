//! Migration script for enhanced archive handling system
//!
//! This script migrates existing archive extraction data to the new enhanced system.

use eyre::{Context, Result};
use sqlx::{sqlite::SqlitePool, Row};
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// Migration configuration
pub struct MigrationConfig {
    /// Path to the old database (if any)
    pub old_db_path: Option<PathBuf>,
    /// Path to the new database
    pub new_db_path: PathBuf,
    /// Workspace root directory
    pub workspace_root: PathBuf,
    /// Dry run mode (don't actually migrate)
    pub dry_run: bool,
}

/// Migration statistics
#[derive(Debug, Default)]
pub struct MigrationStats {
    pub workspaces_migrated: usize,
    pub path_mappings_created: usize,
    pub archives_processed: usize,
    pub errors: Vec<String>,
}

/// Main migration orchestrator
pub struct ArchiveMigration {
    config: MigrationConfig,
    stats: MigrationStats,
}

impl ArchiveMigration {
    /// Create a new migration instance
    pub fn new(config: MigrationConfig) -> Self {
        Self {
            config,
            stats: MigrationStats::default(),
        }
    }

    /// Execute the migration
    pub async fn execute(&mut self) -> Result<MigrationStats> {
        info!("Starting archive system migration");
        info!("Dry run mode: {}", self.config.dry_run);

        // Step 1: Initialize new database schema
        self.initialize_new_database().await?;

        // Step 2: Migrate existing workspace data
        self.migrate_workspaces().await?;

        // Step 3: Scan and register existing extracted archives
        self.scan_existing_extractions().await?;

        // Step 4: Validate migration
        self.validate_migration().await?;

        info!("Migration completed successfully");
        info!("Statistics: {:?}", self.stats);

        Ok(std::mem::take(&mut self.stats))
    }

    /// Initialize the new database schema
    async fn initialize_new_database(&mut self) -> Result<()> {
        info!("Initializing new database schema");

        if self.config.dry_run {
            info!("[DRY RUN] Would create database at: {:?}", self.config.new_db_path);
            return Ok(());
        }

        let pool = SqlitePool::connect(&format!("sqlite:{}", self.config.new_db_path.display()))
            .await
            .context("Failed to connect to new database")?;

        // Create path_mappings table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS path_mappings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                workspace_id TEXT NOT NULL,
                short_path TEXT NOT NULL,
                original_path TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                access_count INTEGER DEFAULT 0,
                UNIQUE(workspace_id, short_path)
            )
            "#,
        )
        .execute(&pool)
        .await
        .context("Failed to create path_mappings table")?;

        // Create indexes
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_workspace_short ON path_mappings(workspace_id, short_path)")
            .execute(&pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_workspace_original ON path_mappings(workspace_id, original_path)")
            .execute(&pool)
            .await?;

        info!("Database schema initialized successfully");
        Ok(())
    }

    /// Migrate existing workspace data
    async fn migrate_workspaces(&mut self) -> Result<()> {
        info!("Migrating workspace data");

        // If there's an old database, migrate from it
        if let Some(old_db_path) = &self.config.old_db_path {
            if old_db_path.exists() {
                self.migrate_from_old_database(old_db_path).await?;
            } else {
                warn!("Old database not found at: {:?}", old_db_path);
            }
        }

        // Scan workspace directory for existing workspaces
        self.discover_workspaces().await?;

        Ok(())
    }

    /// Migrate data from old database format
    async fn migrate_from_old_database(&mut self, old_db_path: &Path) -> Result<()> {
        info!("Migrating from old database: {:?}", old_db_path);

        if self.config.dry_run {
            info!("[DRY RUN] Would migrate from old database");
            return Ok(());
        }

        let old_pool = SqlitePool::connect(&format!("sqlite:{}", old_db_path.display()))
            .await
            .context("Failed to connect to old database")?;

        let new_pool = SqlitePool::connect(&format!("sqlite:{}", self.config.new_db_path.display()))
            .await
            .context("Failed to connect to new database")?;

        // Check if old database has any relevant tables
        let tables: Vec<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type='table' AND name LIKE '%archive%' OR name LIKE '%extract%'"
        )
        .fetch_all(&old_pool)
        .await?;

        info!("Found {} relevant tables in old database", tables.len());

        // Migrate each table's data (this is a placeholder - actual migration depends on old schema)
        for table in tables {
            info!("Processing table: {}", table);
            // Add specific migration logic based on old schema
        }

        self.stats.workspaces_migrated += 1;
        Ok(())
    }

    /// Discover existing workspaces
    async fn discover_workspaces(&mut self) -> Result<()> {
        info!("Discovering existing workspaces");

        let workspace_root = &self.config.workspace_root;
        if !workspace_root.exists() {
            warn!("Workspace root does not exist: {:?}", workspace_root);
            return Ok(());
        }

        let mut entries = tokio::fs::read_dir(workspace_root).await?;
        while let Some(entry) = entries.next_entry().await? {
            if entry.file_type().await?.is_dir() {
                let workspace_name = entry.file_name().to_string_lossy().to_string();
                info!("Found workspace: {}", workspace_name);
                self.stats.workspaces_migrated += 1;
            }
        }

        Ok(())
    }

    /// Scan existing extracted archives and create path mappings
    async fn scan_existing_extractions(&mut self) -> Result<()> {
        info!("Scanning existing extracted archives");

        let workspace_root = &self.config.workspace_root;
        if !workspace_root.exists() {
            return Ok(());
        }

        // Recursively scan for extracted archive directories
        self.scan_directory(workspace_root, workspace_root).await?;

        Ok(())
    }

    /// Recursively scan a directory for archives
    async fn scan_directory(&mut self, dir: &Path, workspace_root: &Path) -> Result<()> {
        let mut entries = tokio::fs::read_dir(dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            if path.is_dir() {
                // Check if this looks like an extracted archive directory
                if self.is_extracted_archive_dir(&path).await {
                    self.process_extracted_archive(&path, workspace_root).await?;
                }

                // Recurse into subdirectories
                if let Err(e) = self.scan_directory(&path, workspace_root).await {
                    warn!("Error scanning directory {:?}: {}", path, e);
                    self.stats.errors.push(format!("Scan error: {}", e));
                }
            }
        }

        Ok(())
    }

    /// Check if a directory looks like an extracted archive
    async fn is_extracted_archive_dir(&self, dir: &Path) -> bool {
        // Heuristics: contains many files, has archive-like name patterns
        if let Ok(mut entries) = tokio::fs::read_dir(dir).await {
            let mut file_count = 0;
            while let Ok(Some(_)) = entries.next_entry().await {
                file_count += 1;
                if file_count > 5 {
                    return true; // Likely an extracted archive
                }
            }
        }
        false
    }

    /// Process an extracted archive directory
    async fn process_extracted_archive(&mut self, dir: &Path, workspace_root: &Path) -> Result<()> {
        info!("Processing extracted archive: {:?}", dir);

        if self.config.dry_run {
            info!("[DRY RUN] Would process archive directory");
            self.stats.archives_processed += 1;
            return Ok(());
        }

        // Extract workspace ID from path
        let relative_path = dir.strip_prefix(workspace_root).unwrap_or(dir);
        let workspace_id = relative_path
            .components()
            .next()
            .and_then(|c| c.as_os_str().to_str())
            .unwrap_or("default");

        // For each file in the directory, check if it needs a path mapping
        let mut entries = tokio::fs::read_dir(dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if let Some(filename) = path.file_name() {
                let filename_str = filename.to_string_lossy();
                
                // Check if filename looks shortened (contains hash-like patterns)
                if filename_str.len() < 20 && filename_str.chars().all(|c| c.is_alphanumeric() || c == '_') {
                    // This might be a shortened path - we can't recover the original without metadata
                    warn!("Found potentially shortened path without mapping: {:?}", path);
                    self.stats.errors.push(format!("Unmapped shortened path: {}", path.display()));
                }
            }
        }

        self.stats.archives_processed += 1;
        Ok(())
    }

    /// Validate the migration
    async fn validate_migration(&mut self) -> Result<()> {
        info!("Validating migration");

        if self.config.dry_run {
            info!("[DRY RUN] Would validate migration");
            return Ok(());
        }

        let pool = SqlitePool::connect(&format!("sqlite:{}", self.config.new_db_path.display()))
            .await
            .context("Failed to connect to database for validation")?;

        // Check that tables exist
        let table_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='path_mappings'"
        )
        .fetch_one(&pool)
        .await?;

        if table_count == 0 {
            return Err(eyre::eyre!("path_mappings table not found after migration"));
        }

        // Check that indexes exist
        let index_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name LIKE 'idx_workspace%'"
        )
        .fetch_one(&pool)
        .await?;

        if index_count < 2 {
            warn!("Expected 2 indexes, found {}", index_count);
        }

        info!("Migration validation completed successfully");
        Ok(())
    }
}

/// CLI entry point for migration
#[cfg(feature = "cli")]
pub async fn run_migration_cli() -> Result<()> {
    use clap::Parser;

    #[derive(Parser)]
    #[command(name = "archive-migration")]
    #[command(about = "Migrate to enhanced archive handling system")]
    struct Cli {
        /// Path to old database (optional)
        #[arg(long)]
        old_db: Option<PathBuf>,

        /// Path to new database
        #[arg(long, default_value = "enhanced_archive.db")]
        new_db: PathBuf,

        /// Workspace root directory
        #[arg(long, default_value = "./workspaces")]
        workspace_root: PathBuf,

        /// Dry run mode
        #[arg(long)]
        dry_run: bool,
    }

    let cli = Cli::parse();

    let config = MigrationConfig {
        old_db_path: cli.old_db,
        new_db_path: cli.new_db,
        workspace_root: cli.workspace_root,
        dry_run: cli.dry_run,
    };

    let mut migration = ArchiveMigration::new(config);
    let stats = migration.execute().await?;

    println!("\n=== Migration Complete ===");
    println!("Workspaces migrated: {}", stats.workspaces_migrated);
    println!("Path mappings created: {}", stats.path_mappings_created);
    println!("Archives processed: {}", stats.archives_processed);
    
    if !stats.errors.is_empty() {
        println!("\nErrors encountered: {}", stats.errors.len());
        for error in &stats.errors {
            println!("  - {}", error);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_migration_dry_run() {
        let temp_dir = TempDir::new().unwrap();
        
        let config = MigrationConfig {
            old_db_path: None,
            new_db_path: temp_dir.path().join("new.db"),
            workspace_root: temp_dir.path().to_path_buf(),
            dry_run: true,
        };

        let mut migration = ArchiveMigration::new(config);
        let result = migration.execute().await;
        
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_initialize_new_database() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        
        let config = MigrationConfig {
            old_db_path: None,
            new_db_path: db_path.clone(),
            workspace_root: temp_dir.path().to_path_buf(),
            dry_run: false,
        };

        let mut migration = ArchiveMigration::new(config);
        let result = migration.initialize_new_database().await;
        
        assert!(result.is_ok());
        assert!(db_path.exists());
    }
}
