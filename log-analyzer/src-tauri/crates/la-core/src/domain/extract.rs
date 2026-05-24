//! ArchiveExtractor — domain trait for archive extraction.
//!
//! This trait abstracts archive format handling (ZIP, TAR, GZ, RAR, 7Z)
//! behind a simple interface. Infrastructure adapters wrap format-specific
//! libraries and the `la-archive` crate's `ArchiveManager`.

use std::path::Path;

use async_trait::async_trait;

use crate::error::Result;

/// Summary of an extraction operation.
#[derive(Debug, Clone)]
pub struct ExtractionSummary {
    /// Number of files extracted.
    pub files_extracted: usize,
    /// Total bytes extracted.
    pub total_bytes: u64,
    /// Maximum nesting depth reached.
    pub max_depth_reached: usize,
}

/// A single entry within an archive (file or directory).
#[derive(Debug, Clone)]
pub struct ArchiveEntry {
    /// Relative path inside the archive.
    pub path: String,
    /// Uncompressed size in bytes.
    pub size_bytes: u64,
}

/// Extraction policy constraints for validation.
#[derive(Debug, Clone)]
pub struct ExtractionPolicy {
    /// Maximum archive nesting depth (1-20).
    pub max_depth: usize,
    /// Maximum single file size in bytes.
    pub max_file_size: u64,
    /// Maximum total extracted size per archive.
    pub max_total_size: u64,
}

/// Domain trait for archive extraction operations.
///
/// # Implementors
///
/// - `ArchiveManagerAdapter` (production) — wraps `la_archive::ArchiveManager`.
/// - Mock implementations for unit testing `ImportUseCase`.
#[async_trait]
pub trait ArchiveExtractor: Send + Sync {
    /// Extract an archive to a target directory.
    ///
    /// Returns a summary of the extraction including file count, total bytes,
    /// and maximum nesting depth reached.
    async fn extract(&self, source: &Path, target_dir: &Path) -> Result<ExtractionSummary>;

    /// List the contents of an archive without extracting.
    ///
    /// Returns entries with their relative paths and uncompressed sizes.
    /// Useful for preview before extraction.
    fn list_contents(&self, source: &Path) -> Result<Vec<ArchiveEntry>>;

    /// Return the list of supported file extensions.
    ///
    /// Each extension includes the leading dot (e.g. `".zip"`, `".tar.gz"`).
    fn supported_formats(&self) -> Vec<String>;

    /// Validate the source path and extraction policy.
    ///
    /// Checks:
    /// - Path exists and is readable
    /// - File format is supported
    /// - Policy constraints are within safe bounds
    fn validate(&self, path: &Path, policy: &ExtractionPolicy) -> Result<()>;
}
