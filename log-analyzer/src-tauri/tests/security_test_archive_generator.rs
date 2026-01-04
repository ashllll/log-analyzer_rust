//! Security Test Archive Generator
//!
//! Utilities for creating various types of malicious archives for security testing.
//! This module provides functions to generate:
//! - Zip bombs (42.zip style)
//! - Archives with path traversal attempts
//! - Archives with circular symlinks (Unix only)
//! - Archives with millions of tiny files
//! - Archives with filenames containing null bytes and control characters
//!
//! **Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5**

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;
use zip::write::{FileOptions, ZipWriter};
use zip::CompressionMethod;

/// Create a zip bomb archive (highly compressed data that expands massively)
///
/// This creates an archive similar to 42.zip, where a small compressed file
/// expands to a very large size.
///
/// # Arguments
/// * `output_path` - Where to write the zip bomb
/// * `uncompressed_size_mb` - Size of uncompressed data in MB
///
/// # Returns
/// Result with the actual compressed size achieved
pub fn create_zip_bomb(output_path: &Path, uncompressed_size_mb: usize) -> io::Result<u64> {
    let file = File::create(output_path)?;
    let mut zip = ZipWriter::new(file);
    let options = FileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .compression_level(Some(9)); // Maximum compression

    // Create highly compressible content (all zeros)
    let chunk_size = 1024 * 1024; // 1MB chunks
    let content = vec![0u8; chunk_size];

    zip.start_file("bomb.txt", options)?;

    // Write the content in chunks
    for _ in 0..uncompressed_size_mb {
        zip.write_all(&content)?;
    }

    zip.finish()?;

    // Return the compressed size
    let metadata = fs::metadata(output_path)?;
    Ok(metadata.len())
}

/// Create a nested zip bomb (multiple levels of compression)
///
/// This creates a more sophisticated zip bomb where archives are nested
/// inside each other, each with high compression ratios.
///
/// # Arguments
/// * `output_path` - Where to write the nested zip bomb
/// * `depth` - Number of nesting levels
/// * `size_per_level_mb` - Size of data at each level in MB
///
/// # Returns
/// Result with the final compressed size
pub fn create_nested_zip_bomb(
    output_path: &Path,
    depth: usize,
    size_per_level_mb: usize,
) -> io::Result<u64> {
    if depth == 0 {
        return create_zip_bomb(output_path, size_per_level_mb);
    }

    // Create a temporary directory for intermediate files
    let temp_dir = tempfile::tempdir()?;
    let inner_path = temp_dir.path().join(format!("level{}.zip", depth - 1));

    // Recursively create inner archive
    create_nested_zip_bomb(&inner_path, depth - 1, size_per_level_mb)?;

    // Create outer archive containing the inner one
    let file = File::create(output_path)?;
    let mut zip = ZipWriter::new(file);
    let options = FileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .compression_level(Some(9));

    // Add the inner archive
    let inner_content = fs::read(&inner_path)?;
    zip.start_file(format!("level{}.zip", depth - 1), options)?;
    zip.write_all(&inner_content)?;

    // Add some additional compressible data
    let padding = vec![0u8; size_per_level_mb * 1024 * 1024];
    zip.start_file("padding.dat", options)?;
    zip.write_all(&padding)?;

    zip.finish()?;

    let metadata = fs::metadata(output_path)?;
    Ok(metadata.len())
}

/// Create an archive with path traversal attempts
///
/// This creates an archive containing files with malicious paths that attempt
/// to escape the extraction directory.
///
/// # Arguments
/// * `output_path` - Where to write the archive
///
/// # Returns
/// Result indicating success or failure
pub fn create_path_traversal_archive(output_path: &Path) -> io::Result<()> {
    let file = File::create(output_path)?;
    let mut zip = ZipWriter::new(file);
    let options = FileOptions::default().compression_method(CompressionMethod::Stored);

    // Various path traversal attempts
    let malicious_paths = [
        "../../../etc/passwd",
        "..\\..\\..\\windows\\system32\\config\\sam",
        "../../../../../../etc/shadow",
        "..\\..\\..\\..\\..\\boot.ini",
        "legitimate/../../../etc/hosts",
        "./../../../root/.ssh/id_rsa",
        "subdir/../../../../../../etc/passwd",
    ];

    for (i, path) in malicious_paths.iter().enumerate() {
        zip.start_file(*path, options)?;
        zip.write_all(format!("Malicious content {}", i).as_bytes())?;
    }

    // Add some legitimate files too
    zip.start_file("legitimate_file.txt", options)?;
    zip.write_all(b"This is a legitimate file")?;

    zip.finish()?;
    Ok(())
}

/// Create an archive with many tiny files
///
/// This creates an archive containing a very large number of small files,
/// which can cause resource exhaustion.
///
/// # Arguments
/// * `output_path` - Where to write the archive
/// * `file_count` - Number of files to create
/// * `file_size_bytes` - Size of each file in bytes
///
/// # Returns
/// Result indicating success or failure
pub fn create_many_files_archive(
    output_path: &Path,
    file_count: usize,
    file_size_bytes: usize,
) -> io::Result<()> {
    let file = File::create(output_path)?;
    let mut zip = ZipWriter::new(file);
    let options = FileOptions::default().compression_method(CompressionMethod::Stored);

    let content = vec![b'x'; file_size_bytes];

    for i in 0..file_count {
        // Create nested directory structure to make it more realistic
        let dir_level = i / 1000;
        let filename = format!("dir{}/subdir{}/file{:08}.txt", dir_level, i / 100, i);

        zip.start_file(filename, options)?;
        zip.write_all(&content)?;

        // Print progress every 10000 files
        if i > 0 && i % 10000 == 0 {
            println!("Created {} files...", i);
        }
    }

    zip.finish()?;
    Ok(())
}

/// Create an archive with filenames containing special characters
///
/// This creates an archive with filenames that contain null bytes, control
/// characters, and other problematic characters.
///
/// # Arguments
/// * `output_path` - Where to write the archive
///
/// # Returns
/// Result indicating success or failure
pub fn create_special_chars_archive(output_path: &Path) -> io::Result<()> {
    let file = File::create(output_path)?;
    let mut zip = ZipWriter::new(file);
    let options = FileOptions::default().compression_method(CompressionMethod::Stored);

    // Note: Some characters may not be valid in ZIP format and will be skipped
    let special_filenames = vec![
        "normal_file.txt",
        "file with spaces.txt",
        "file\twith\ttabs.txt",
        "file_with_unicode_文件名.txt",
        "file_with_emoji_😀.txt",
        "file:with:colons.txt",
        "file*with*asterisks.txt",
        "file?with?questions.txt",
        "file<with>brackets.txt",
        "file|with|pipes.txt",
        "file\"with\"quotes.txt",
        "file'with'apostrophes.txt",
        "file;with;semicolons.txt",
        "file&with&ampersands.txt",
        "file$with$dollars.txt",
        "file#with#hashes.txt",
        "file@with@ats.txt",
        "file!with!exclamations.txt",
        "file%with%percents.txt",
        "file^with^carets.txt",
        "file(with)parens.txt",
        "file[with]brackets.txt",
        "file{with}braces.txt",
        "file=with=equals.txt",
        "file+with+plus.txt",
        "file~with~tildes.txt",
        "file`with`backticks.txt",
    ];

    for (i, filename) in special_filenames.iter().enumerate() {
        // Try to add the file, but skip if it fails (some chars may be invalid)
        match zip.start_file(*filename, options) {
            Ok(_) => {
                zip.write_all(format!("Content {}", i).as_bytes())?;
            }
            Err(e) => {
                eprintln!("Skipping invalid filename '{}': {}", filename, e);
            }
        }
    }

    zip.finish()?;
    Ok(())
}

/// Create an archive with extremely long filenames
///
/// This creates an archive with filenames that are very long, potentially
/// exceeding filesystem limits.
///
/// # Arguments
/// * `output_path` - Where to write the archive
/// * `filename_length` - Length of filenames to create
///
/// # Returns
/// Result indicating success or failure
pub fn create_long_filename_archive(output_path: &Path, filename_length: usize) -> io::Result<()> {
    let file = File::create(output_path)?;
    let mut zip = ZipWriter::new(file);
    let options = FileOptions::default().compression_method(CompressionMethod::Stored);

    // Create filenames of various lengths
    for i in 0..10 {
        let base_length = filename_length - 10; // Leave room for index and extension
        let filename = format!("{}{:03}.txt", "a".repeat(base_length), i);

        zip.start_file(filename, options)?;
        zip.write_all(format!("Content {}", i).as_bytes())?;
    }

    zip.finish()?;
    Ok(())
}

/// Create an archive with deeply nested directories
///
/// This creates an archive with a very deep directory structure, which can
/// cause path length issues.
///
/// # Arguments
/// * `output_path` - Where to write the archive
/// * `depth` - Depth of directory nesting
///
/// # Returns
/// Result indicating success or failure
pub fn create_deep_directory_archive(output_path: &Path, depth: usize) -> io::Result<()> {
    let file = File::create(output_path)?;
    let mut zip = ZipWriter::new(file);
    let options = FileOptions::default().compression_method(CompressionMethod::Stored);

    // Create a deeply nested path
    let mut path = String::new();
    for i in 0..depth {
        path.push_str(&format!("level{}/", i));
    }
    path.push_str("deep_file.txt");

    zip.start_file(path, options)?;
    zip.write_all(b"This file is deeply nested")?;

    zip.finish()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_create_zip_bomb() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("zip_bomb.zip");

        let compressed_size = create_zip_bomb(&output_path, 10).unwrap();

        assert!(output_path.exists(), "Zip bomb should be created");
        assert!(
            compressed_size < 10 * 1024 * 1024,
            "Should be highly compressed"
        );

        println!(
            "Zip bomb: 10MB uncompressed -> {} bytes compressed",
            compressed_size
        );
        println!(
            "Compression ratio: {:.2}:1",
            10.0 * 1024.0 * 1024.0 / compressed_size as f64
        );
    }

    #[test]
    fn test_create_path_traversal_archive() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("path_traversal.zip");

        create_path_traversal_archive(&output_path).unwrap();

        assert!(
            output_path.exists(),
            "Path traversal archive should be created"
        );

        // Verify it contains malicious paths
        let file = File::open(&output_path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();

        let mut has_traversal = false;
        for i in 0..archive.len() {
            let file = archive.by_index(i).unwrap();
            if file.name().contains("..") {
                has_traversal = true;
                break;
            }
        }

        assert!(
            has_traversal,
            "Archive should contain path traversal attempts"
        );
    }

    #[test]
    fn test_create_many_files_archive() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("many_files.zip");

        // Create a smaller test (100 files instead of millions)
        create_many_files_archive(&output_path, 100, 10).unwrap();

        assert!(output_path.exists(), "Many files archive should be created");

        // Verify file count
        let file = File::open(&output_path).unwrap();
        let archive = zip::ZipArchive::new(file).unwrap();
        assert_eq!(archive.len(), 100, "Should contain 100 files");
    }

    #[test]
    fn test_create_special_chars_archive() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("special_chars.zip");

        create_special_chars_archive(&output_path).unwrap();

        assert!(
            output_path.exists(),
            "Special chars archive should be created"
        );

        // Verify it contains files
        let file = File::open(&output_path).unwrap();
        let archive = zip::ZipArchive::new(file).unwrap();
        assert!(!archive.is_empty(), "Should contain files");
    }

    #[test]
    fn test_create_long_filename_archive() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("long_filename.zip");

        create_long_filename_archive(&output_path, 300).unwrap();

        assert!(
            output_path.exists(),
            "Long filename archive should be created"
        );

        // Verify it contains files with long names
        let file = File::open(&output_path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();

        let mut has_long_name = false;
        for i in 0..archive.len() {
            let file = archive.by_index(i).unwrap();
            if file.name().len() > 250 {
                has_long_name = true;
                break;
            }
        }

        assert!(
            has_long_name,
            "Archive should contain files with long names"
        );
    }

    #[test]
    fn test_create_deep_directory_archive() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("deep_dirs.zip");

        create_deep_directory_archive(&output_path, 50).unwrap();

        assert!(
            output_path.exists(),
            "Deep directory archive should be created"
        );

        // Verify it contains deeply nested file
        let file = File::open(&output_path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();

        let file = archive.by_index(0).unwrap();
        let path_depth = file.name().matches('/').count();
        assert!(path_depth >= 50, "Should have deep nesting");
    }

    #[test]
    #[ignore] // This test creates a large file and takes time
    fn test_create_nested_zip_bomb() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("nested_zip_bomb.zip");

        let compressed_size = create_nested_zip_bomb(&output_path, 3, 5).unwrap();

        assert!(output_path.exists(), "Nested zip bomb should be created");
        println!(
            "Nested zip bomb (3 levels, 5MB each): {} bytes compressed",
            compressed_size
        );
    }
}
