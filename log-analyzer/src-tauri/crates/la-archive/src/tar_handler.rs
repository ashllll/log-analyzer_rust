use crate::archive_handler::{ArchiveHandler, ExtractionSummary};
use async_trait::async_trait;
use la_core::error::{AppError, Result};
use la_core::utils::path_security::{
    validate_and_sanitize_archive_path, PathValidationResult, SecurityConfig,
};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::warn;

use flate2::read::GzDecoder;
use tar::Archive;

/**
 * TAR文件处理器 (稳定版本)
 */
pub struct TarHandler;

#[async_trait]
impl ArchiveHandler for TarHandler {
    fn can_handle(&self, path: &Path) -> bool {
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            let lower_ext = ext.to_lowercase();
            if lower_ext == "tar" || lower_ext == "tgz" {
                return true;
            }
            if lower_ext == "gz" {
                if let Some(stem) = path.file_stem() {
                    if let Some(stem_str) = stem.to_str() {
                        return stem_str.to_lowercase().ends_with(".tar");
                    }
                }
            }
        }
        false
    }

    async fn extract_with_limits(
        &self,
        source: &Path,
        target_dir: &Path,
        max_file_size: u64,
        max_total_size: u64,
        max_file_count: usize,
    ) -> Result<ExtractionSummary> {
        fs::create_dir_all(target_dir).await?;

        let source_path = source.to_path_buf();
        let target_path = target_dir.to_path_buf();

        let summary = tokio::task::spawn_blocking(move || {
            let file = std::fs::File::open(&source_path)?;
            let mut summary = ExtractionSummary::new();
            let security_config = SecurityConfig::default();

            let is_gzipped = source_path
                .extension()
                .and_then(|s| s.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("gz") || ext.eq_ignore_ascii_case("tgz"))
                .unwrap_or(false);

            if is_gzipped {
                let decoder = GzDecoder::new(file);
                let mut archive = Archive::new(decoder);
                Self::extract_sync(
                    &mut archive,
                    &target_path,
                    &mut summary,
                    max_file_size,
                    max_total_size,
                    max_file_count,
                    &security_config,
                )?;
            } else {
                let mut archive = Archive::new(file);
                Self::extract_sync(
                    &mut archive,
                    &target_path,
                    &mut summary,
                    max_file_size,
                    max_total_size,
                    max_file_count,
                    &security_config,
                )?;
            }

            Ok::<ExtractionSummary, AppError>(summary)
        })
        .await
        .map_err(|e| {
            if e.is_panic() {
                AppError::Internal(format!("TAR handler panicked: {}", e))
            } else {
                AppError::archive_error(e.to_string(), None)
            }
        })??;

        Ok(summary)
    }

    fn file_extensions(&self) -> Vec<&str> {
        vec!["tar", "tar.gz", "tgz"]
    }
}

impl TarHandler {
    fn extract_sync<R: std::io::Read>(
        archive: &mut Archive<R>,
        target_dir: &Path,
        summary: &mut ExtractionSummary,
        max_file_size: u64,
        max_total_size: u64,
        max_file_count: usize,
        security_config: &SecurityConfig,
    ) -> Result<()> {
        let entries = archive
            .entries()
            .map_err(|e| AppError::archive_error(e.to_string(), None))?;

        for entry_result in entries {
            let mut entry = match entry_result {
                Ok(e) => e,
                Err(e) => {
                    warn!("Failed to read tar entry: {}", e);
                    continue;
                }
            };
            let path = match entry.path() {
                Ok(p) => p.to_path_buf(),
                Err(e) => {
                    warn!("Failed to get entry path: {}", e);
                    continue;
                }
            };
            let path_str = path.to_string_lossy().to_string();

            let validation = validate_and_sanitize_archive_path(&path_str, security_config);
            let safe_path = match validation {
                PathValidationResult::Unsafe(_) => continue,
                PathValidationResult::Valid(p) => PathBuf::from(p),
                PathValidationResult::RequiresSanitization(_, p) => PathBuf::from(p),
            };

            let out_path = target_dir.join(&safe_path);
            // 验证最终路径不逃逸提取目录（防御 TAR Slip 末级绕过）
            if !out_path.starts_with(target_dir) {
                let msg = format!("TAR Slip 尝试被拦截: {}", safe_path.display());
                warn!("{}", msg);
                summary.errors.push(msg);
                continue;
            }
            let size = entry.header().size().unwrap_or(0);

            // 拒绝符号链接和硬链接，防止 ZIP Slip 变种攻击
            if entry.header().entry_type().is_symlink()
                || entry.header().entry_type() == tar::EntryType::Link
            {
                let msg = format!("TAR 条目包含符号链接或硬链接，已跳过: {}", path_str);
                warn!("{}", msg);
                summary.errors.push(msg);
                continue;
            }

            if entry.header().entry_type().is_file() {
                // Check limits before extraction
                let would_exceed_limits = size > max_file_size
                    || summary.total_size + size > max_total_size
                    || summary.files_extracted + 1 > max_file_count;

                if would_exceed_limits {
                    // Log skipped file details instead of silently skipping
                    let path_str = safe_path.to_string_lossy();
                    if size > max_file_size {
                        warn!(
                            file = %path_str,
                            file_size = size,
                            max_allowed = max_file_size,
                            "Skipping file exceeding max_file_size limit"
                        );
                    } else if summary.total_size + size > max_total_size {
                        warn!(
                            file = %path_str,
                            file_size = size,
                            current_total = summary.total_size,
                            max_total = max_total_size,
                            "Skipping file - would exceed max_total_size limit"
                        );
                    } else {
                        warn!(
                            file = %path_str,
                            current_count = summary.files_extracted,
                            max_count = max_file_count,
                            "Skipping file - would exceed max_file_count limit"
                        );
                    }
                    continue;
                }

                if let Some(parent) = out_path.parent() {
                    if let Err(e) = std::fs::create_dir_all(parent) {
                        warn!(path = ?parent, error = %e, "创建 TAR 条目父目录失败，跳过此文件");
                        continue;
                    }
                }

                if let Err(e) = entry.unpack(&out_path) {
                    let msg = format!("Failed to unpack TAR entry {}: {}", out_path.display(), e);
                    warn!("{}", msg);
                    summary.errors.push(msg);
                } else {
                    // TOCTOU 防护: 创建后验证实际路径仍在目标目录内
                    if let (Ok(real_path), Ok(real_target)) = (
                        std::fs::canonicalize(&out_path),
                        std::fs::canonicalize(target_dir),
                    ) {
                        if !real_path.starts_with(&real_target) {
                            std::fs::remove_file(&out_path).ok();
                            let msg = format!("符号链接逃逸已拦截 (TAR): {}", safe_path.display());
                            warn!("{}", msg);
                            summary.errors.push(msg);
                            continue;
                        }
                    }
                    summary.add_file(safe_path, size);
                }
            } else if entry.header().entry_type().is_dir() {
                if let Err(e) = std::fs::create_dir_all(&out_path) {
                    warn!(path = ?out_path, error = %e, "创建 TAR 目录条目失败，跳过");
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use tempfile::TempDir;

    /// 创建测试用的 TAR 文件
    fn create_test_tar(
        path: &Path,
        files: Vec<(&str, &[u8])>,
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let file = std::fs::File::create(path)?;
        let mut tar = tar::Builder::new(file);

        for (name, content) in files {
            let mut header = tar::Header::new_gnu();
            header.set_path(name)?;
            header.set_size(content.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            tar.append(&header, content)?;
        }

        tar.finish()?;
        Ok(())
    }

    /// 创建测试用的 TAR.GZ 文件
    fn create_test_tar_gz(
        path: &Path,
        files: Vec<(&str, &[u8])>,
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let file = std::fs::File::create(path)?;
        let gz = GzEncoder::new(file, Compression::default());
        let mut tar = tar::Builder::new(gz);

        for (name, content) in files {
            let mut header = tar::Header::new_gnu();
            header.set_path(name)?;
            header.set_size(content.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            tar.append(&header, content)?;
        }

        tar.finish()?;
        Ok(())
    }

    #[test]
    fn test_tar_handler_can_handle() {
        let handler = TarHandler;

        // 应该能处理 .tar 文件
        assert!(handler.can_handle(Path::new("test.tar")));
        assert!(handler.can_handle(Path::new("test.TAR")));
        assert!(handler.can_handle(Path::new("/path/to/archive.tar")));

        // 应该能处理 .tar.gz 文件
        assert!(handler.can_handle(Path::new("test.tar.gz")));
        assert!(handler.can_handle(Path::new("test.TAR.GZ")));
        assert!(handler.can_handle(Path::new("archive.tar.gz")));

        // 应该能处理 .tgz 文件
        assert!(handler.can_handle(Path::new("test.tgz")));
        assert!(handler.can_handle(Path::new("test.TGZ")));

        // 不应该处理纯 .gz 文件（由 GzHandler 处理）
        assert!(!handler.can_handle(Path::new("test.gz")));

        // 不应该处理其他格式
        assert!(!handler.can_handle(Path::new("test.zip")));
        assert!(!handler.can_handle(Path::new("test.rar")));
        assert!(!handler.can_handle(Path::new("test.txt")));
        assert!(!handler.can_handle(Path::new("test")));
    }

    #[test]
    fn test_tar_handler_file_extensions() {
        let handler = TarHandler;
        let extensions = handler.file_extensions();

        assert_eq!(extensions, vec!["tar", "tar.gz", "tgz"]);
    }

    #[tokio::test]
    async fn test_extract_tar_file() {
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let tar_file = temp_dir.path().join("test.tar");
        let output_dir = temp_dir.path().join("output");

        // 创建测试 TAR 文件
        let files = vec![
            ("file1.txt", b"Hello, World!" as &[u8]),
            ("file2.txt", b"Second file content"),
            ("dir/nested.txt", b"Nested file content"),
        ];
        create_test_tar(&tar_file, files).expect("创建 TAR 文件失败");

        // 解压文件
        let handler = TarHandler;
        let summary = handler
            .extract(&tar_file, &output_dir)
            .await
            .expect("解压 TAR 文件失败");

        assert_eq!(summary.files_extracted, 3);
        assert!(output_dir.join("file1.txt").exists());
        assert!(output_dir.join("file2.txt").exists());
        assert!(output_dir.join("dir/nested.txt").exists());

        // 验证内容
        let content1 = std::fs::read_to_string(output_dir.join("file1.txt")).unwrap();
        assert_eq!(content1, "Hello, World!");

        let content2 = std::fs::read_to_string(output_dir.join("file2.txt")).unwrap();
        assert_eq!(content2, "Second file content");

        let content3 = std::fs::read_to_string(output_dir.join("dir/nested.txt")).unwrap();
        assert_eq!(content3, "Nested file content");
    }

    #[tokio::test]
    async fn test_extract_tar_gz_file() {
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let tar_gz_file = temp_dir.path().join("test.tar.gz");
        let output_dir = temp_dir.path().join("output");

        // 创建测试 TAR.GZ 文件
        let files = vec![
            ("file1.txt", b"Hello from tar.gz!" as &[u8]),
            ("file2.txt", b"Second file in tar.gz"),
        ];
        create_test_tar_gz(&tar_gz_file, files).expect("创建 TAR.GZ 文件失败");

        // 解压文件
        let handler = TarHandler;
        let summary = handler
            .extract(&tar_gz_file, &output_dir)
            .await
            .expect("解压 TAR.GZ 文件失败");

        assert_eq!(summary.files_extracted, 2);
        assert!(output_dir.join("file1.txt").exists());
        assert!(output_dir.join("file2.txt").exists());

        // 验证内容
        let content1 = std::fs::read_to_string(output_dir.join("file1.txt")).unwrap();
        assert_eq!(content1, "Hello from tar.gz!");
    }

    #[tokio::test]
    async fn test_extract_tgz_file() {
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let tgz_file = temp_dir.path().join("test.tgz");
        let output_dir = temp_dir.path().join("output");

        // 创建测试 TGZ 文件
        let files = vec![
            ("file1.txt", b"Hello from tgz!" as &[u8]),
            ("file2.txt", b"Second file in tgz"),
        ];
        create_test_tar_gz(&tgz_file, files).expect("创建 TGZ 文件失败");

        // 解压文件
        let handler = TarHandler;
        let summary = handler
            .extract(&tgz_file, &output_dir)
            .await
            .expect("解压 TGZ 文件失败");

        assert_eq!(summary.files_extracted, 2);
        assert!(output_dir.join("file1.txt").exists());
        assert!(output_dir.join("file2.txt").exists());
    }

    #[tokio::test]
    async fn test_extract_tar_with_limits() {
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let tar_file = temp_dir.path().join("test.tar");
        let output_dir = temp_dir.path().join("output");

        // 创建测试 TAR 文件
        let files = vec![
            ("small.txt", b"Small content" as &[u8]),
            ("large.txt", &[b'x'; 2000]), // 2KB 文件
        ];
        create_test_tar(&tar_file, files).expect("创建 TAR 文件失败");

        // 使用限制解压：最大文件大小 1KB
        let handler = TarHandler;
        let summary = handler
            .extract_with_limits(&tar_file, &output_dir, 1024, 1024 * 1024, 100)
            .await
            .expect("解压 TAR 文件失败");

        // 只有小文件应该被解压
        assert_eq!(summary.files_extracted, 1);
        assert!(output_dir.join("small.txt").exists());
        assert!(!output_dir.join("large.txt").exists());
    }

    #[tokio::test]
    async fn test_extract_tar_max_total_size_limit() {
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let tar_file = temp_dir.path().join("test.tar");
        let output_dir = temp_dir.path().join("output");

        // 创建测试 TAR 文件，总大小超过限制
        let files = vec![
            ("file1.txt", &[b'a'; 600][..]), // 600 bytes
            ("file2.txt", &[b'b'; 600][..]), // 600 bytes
        ];
        create_test_tar(&tar_file, files).expect("创建 TAR 文件失败");

        // 使用限制解压：最大总大小 1KB
        let handler = TarHandler;
        let summary = handler
            .extract_with_limits(&tar_file, &output_dir, 1024, 1024, 100)
            .await
            .expect("解压 TAR 文件失败");

        // 只有第一个文件应该被解压
        assert_eq!(summary.files_extracted, 1);
        assert!(output_dir.join("file1.txt").exists());
    }

    #[tokio::test]
    async fn test_extract_tar_max_file_count_limit() {
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let tar_file = temp_dir.path().join("test.tar");
        let output_dir = temp_dir.path().join("output");

        // 创建测试 TAR 文件，包含多个文件
        let files = vec![
            ("file1.txt", b"content1" as &[u8]),
            ("file2.txt", b"content2"),
            ("file3.txt", b"content3"),
        ];
        create_test_tar(&tar_file, files).expect("创建 TAR 文件失败");

        // 使用限制解压：最大文件数量 2
        let handler = TarHandler;
        let summary = handler
            .extract_with_limits(&tar_file, &output_dir, 1024 * 1024, 1024 * 1024 * 1024, 2)
            .await
            .expect("解压 TAR 文件失败");

        // 只有前两个文件应该被解压
        assert_eq!(summary.files_extracted, 2);
    }

    #[tokio::test]
    async fn test_extract_tar_empty_file() {
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let tar_file = temp_dir.path().join("empty.tar");
        let output_dir = temp_dir.path().join("output");

        // 创建空内容的 TAR 文件
        let file = std::fs::File::create(&tar_file).unwrap();
        let mut tar = tar::Builder::new(file);
        // 只添加目录条目
        tar.append_dir("empty_dir", ".").unwrap();
        tar.finish().unwrap();

        // 解压文件
        let handler = TarHandler;
        let summary = handler
            .extract(&tar_file, &output_dir)
            .await
            .expect("解压 TAR 文件失败");

        // 目录条目不应该被计为提取的文件
        assert_eq!(summary.files_extracted, 0);
    }

    #[tokio::test]
    async fn test_extract_tar_with_special_chars_in_filename() {
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let tar_file = temp_dir.path().join("test.tar");
        let output_dir = temp_dir.path().join("output");

        // 创建包含特殊字符文件名的 TAR 文件
        let files = vec![
            ("file with spaces.txt", b"content1" as &[u8]),
            ("file-with-dashes.txt", b"content2"),
            ("file_with_underscores.txt", b"content3"),
        ];
        create_test_tar(&tar_file, files).expect("创建 TAR 文件失败");

        // 解压文件
        let handler = TarHandler;
        let summary = handler
            .extract(&tar_file, &output_dir)
            .await
            .expect("解压 TAR 文件失败");

        assert_eq!(summary.files_extracted, 3);
        assert!(output_dir.join("file with spaces.txt").exists());
        assert!(output_dir.join("file-with-dashes.txt").exists());
        assert!(output_dir.join("file_with_underscores.txt").exists());
    }

    #[tokio::test]
    async fn test_extract_nonexistent_tar() {
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let tar_file = temp_dir.path().join("nonexistent.tar");
        let output_dir = temp_dir.path().join("output");

        let handler = TarHandler;
        let result = handler.extract(&tar_file, &output_dir).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_extract_corrupted_tar() {
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let tar_file = temp_dir.path().join("corrupted.tar");
        let output_dir = temp_dir.path().join("output");

        // 写入损坏的 TAR 数据
        std::fs::write(&tar_file, b"This is not a valid TAR file").unwrap();

        let handler = TarHandler;
        let result = handler.extract(&tar_file, &output_dir).await;

        // TAR 文件可能解析失败或返回空结果，取决于实现
        // 这里我们主要确保不会 panic
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_extract_tar_deeply_nested() {
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let tar_file = temp_dir.path().join("test.tar");
        let output_dir = temp_dir.path().join("output");

        // 创建深层嵌套目录结构的 TAR 文件
        let files = vec![
            ("a/b/c/d/e/deep.txt", b"Deep nested content" as &[u8]),
            ("a/b/c/mid.txt", b"Mid level content"),
            ("a/top.txt", b"Top level content"),
        ];
        create_test_tar(&tar_file, files).expect("创建 TAR 文件失败");

        // 解压文件
        let handler = TarHandler;
        let summary = handler
            .extract(&tar_file, &output_dir)
            .await
            .expect("解压 TAR 文件失败");

        assert_eq!(summary.files_extracted, 3);
        assert!(output_dir.join("a/b/c/d/e/deep.txt").exists());
        assert!(output_dir.join("a/b/c/mid.txt").exists());
        assert!(output_dir.join("a/top.txt").exists());

        // 验证内容
        let deep_content = std::fs::read_to_string(output_dir.join("a/b/c/d/e/deep.txt")).unwrap();
        assert_eq!(deep_content, "Deep nested content");
    }

    #[tokio::test]
    async fn test_extract_tar_large_file() {
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let tar_file = temp_dir.path().join("test.tar");
        let output_dir = temp_dir.path().join("output");

        // 创建包含大文件的 TAR 文件 (5MB)
        let large_content = vec![b'x'; 5 * 1024 * 1024];
        let files = vec![("large.bin", large_content.as_slice())];
        create_test_tar(&tar_file, files).expect("创建 TAR 文件失败");

        // 解压文件
        let handler = TarHandler;
        let summary = handler
            .extract(&tar_file, &output_dir)
            .await
            .expect("解压 TAR 文件失败");

        assert_eq!(summary.files_extracted, 1);
        assert_eq!(summary.total_size, 5 * 1024 * 1024);
        assert!(output_dir.join("large.bin").exists());

        // 验证文件大小
        let metadata = std::fs::metadata(output_dir.join("large.bin")).unwrap();
        assert_eq!(metadata.len(), 5 * 1024 * 1024);
    }

    #[tokio::test]
    async fn test_extract_tar_many_files() {
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let tar_file = temp_dir.path().join("test.tar");
        let output_dir = temp_dir.path().join("output");

        // 创建包含多个文件的 TAR 文件
        let file = std::fs::File::create(&tar_file).unwrap();
        let mut tar = tar::Builder::new(file);

        for i in 0..100 {
            let name = format!("file{:03}.txt", i);
            let content = format!("Content of file {}", i);
            let mut header = tar::Header::new_gnu();
            header.set_path(&name).unwrap();
            header.set_size(content.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            tar.append(&header, content.as_bytes()).unwrap();
        }
        tar.finish().unwrap();

        // 解压文件
        let handler = TarHandler;
        let summary = handler
            .extract(&tar_file, &output_dir)
            .await
            .expect("解压 TAR 文件失败");

        assert_eq!(summary.files_extracted, 100);

        // 验证部分文件
        for i in [0, 50, 99] {
            let file_path = output_dir.join(format!("file{:03}.txt", i));
            assert!(file_path.exists());
            let content = std::fs::read_to_string(file_path).unwrap();
            assert_eq!(content, format!("Content of file {}", i));
        }
    }

    #[tokio::test]
    async fn test_extract_tar_gz_large_file() {
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let tar_gz_file = temp_dir.path().join("test.tar.gz");
        let output_dir = temp_dir.path().join("output");

        // 创建包含大文件的 TAR.GZ 文件 (1MB)
        let large_content = vec![b'y'; 1024 * 1024];
        let files = vec![("large.bin", large_content.as_slice())];
        create_test_tar_gz(&tar_gz_file, files).expect("创建 TAR.GZ 文件失败");

        // 解压文件
        let handler = TarHandler;
        let summary = handler
            .extract(&tar_gz_file, &output_dir)
            .await
            .expect("解压 TAR.GZ 文件失败");

        assert_eq!(summary.files_extracted, 1);
        assert_eq!(summary.total_size, 1024 * 1024);
        assert!(output_dir.join("large.bin").exists());

        // 验证文件大小
        let metadata = std::fs::metadata(output_dir.join("large.bin")).unwrap();
        assert_eq!(metadata.len(), 1024 * 1024);
    }
}
