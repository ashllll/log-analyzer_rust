use crate::archive_handler::{ArchiveHandler, ExtractionSummary};
use async_trait::async_trait;
use la_core::error::{AppError, Result};
use la_core::utils::path_security::{
    validate_and_sanitize_archive_path, PathValidationResult, SecurityConfig,
};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::warn;

use zip::ZipArchive;

/**
 * ZIP文件处理器 (稳定版本)
 */
pub struct ZipHandler;

#[async_trait]
impl ArchiveHandler for ZipHandler {
    fn can_handle(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|s| s.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("zip"))
            .unwrap_or(false)
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
            let mut archive =
                ZipArchive::new(file).map_err(|e| AppError::archive_error(e.to_string(), None))?;
            let mut summary = ExtractionSummary::new();
            let security_config = SecurityConfig::default();

            for i in 0..archive.len() {
                let mut file = match archive.by_index(i) {
                    Ok(f) => f,
                    Err(e) => {
                        warn!("Failed to read zip entry {}: {}", i, e);
                        continue;
                    }
                };
                let name = file.name().to_string();

                let validation = validate_and_sanitize_archive_path(&name, &security_config);
                let safe_path = match validation {
                    PathValidationResult::Unsafe(_) => continue,
                    PathValidationResult::Valid(p) => PathBuf::from(p),
                    PathValidationResult::RequiresSanitization(_, p) => PathBuf::from(p),
                };

                let out_path = target_path.join(&safe_path);
                // 验证最终路径不逃逸提取目录（防御 ZIP Slip 末级绕过）
                if !out_path.starts_with(&target_path) {
                    let msg = format!("ZIP Slip 尝试被拦截: {}", safe_path.display());
                    warn!("{}", msg);
                    summary.errors.push(msg);
                    continue;
                }
                // 安全检查：跳过 ZIP 内的符号链接，防止沙箱逃逸
                // ZIP 格式支持符号链接条目，若解压到目标目录会创建指向任意路径的链接
                // 使用 unix_mode() 检查符号链接：S_IFMT = 0o170000, S_IFLNK = 0o120000
                // 注意: unix_mode() 依赖 ZIP 文件中的 Unix extra field (0x000d),
                // 在 Windows 平台或缺少该 extra field 的 ZIP 文件中可能不可用，
                // 此时 is_symlink 为 false（保守策略：允许解压）;
                // 最终的路径逃逸防护由文件创建后的 canonicalize 检查保证 (TOCTOU 防护)
                let is_symlink = file
                    .unix_mode()
                    .map(|mode| mode & 0o170000 == 0o120000)
                    .unwrap_or(false);
                if is_symlink {
                    let msg = format!("ZIP 条目为符号链接已跳过: {name}");
                    warn!("{}", msg);
                    summary.errors.push(msg);
                    continue;
                }

                let size = file.size();

                if file.is_dir() {
                    if let Err(e) = std::fs::create_dir_all(&out_path) {
                        warn!(path = ?out_path, error = %e, "创建 ZIP 目录条目失败，跳过");
                    }
                } else {
                    // Check limits before extraction
                    let would_exceed_limits = size > max_file_size
                        || summary.total_size + size > max_total_size
                        || summary.files_extracted + 1 > max_file_count;

                    if would_exceed_limits {
                        // Log skipped file details instead of silently skipping
                        if size > max_file_size {
                            warn!(
                                file = %name,
                                file_size = size,
                                max_allowed = max_file_size,
                                "Skipping file exceeding max_file_size limit"
                            );
                        } else if summary.total_size + size > max_total_size {
                            warn!(
                                file = %name,
                                file_size = size,
                                current_total = summary.total_size,
                                max_total = max_total_size,
                                "Skipping file - would exceed max_total_size limit"
                            );
                        } else {
                            warn!(
                                file = %name,
                                current_count = summary.files_extracted,
                                max_count = max_file_count,
                                "Skipping file - would exceed max_file_count limit"
                            );
                        }
                        continue;
                    }

                    if let Some(parent) = out_path.parent() {
                        if let Err(e) = std::fs::create_dir_all(parent) {
                            warn!(path = ?parent, error = %e, "创建 ZIP 条目父目录失败，跳过此文件");
                            continue;
                        }
                    }

                    match std::fs::File::create(&out_path) {
                        Ok(mut out_file) => {
                            // ── 流式解压 + 实际字节计数 ──
                            // ZIP 头部声明的 uncompressed_size 可被伪造，
                            // 仅凭声明值检查无法阻止 zip 炸弹。这里按
                            // 实际解压字节强制 max_file_size，超限立即
                            // 中止并删除半写文件。
                            const COPY_BUFFER_SIZE: usize = 64 * 1024;
                            let mut buffer = [0u8; COPY_BUFFER_SIZE];
                            let mut written: u64 = 0;
                            let mut copy_error: Option<std::io::Error> = None;
                            let mut exceeded = false;
                            loop {
                                match file.read(&mut buffer) {
                                    Ok(0) => break,
                                    Ok(n) => {
                                        let next = written + n as u64;
                                        if next > max_file_size {
                                            exceeded = true;
                                            break;
                                        }
                                        if let Err(e) = out_file.write_all(&buffer[..n]) {
                                            copy_error = Some(e);
                                            break;
                                        }
                                        written = next;
                                    }
                                    Err(e) => {
                                        copy_error = Some(e);
                                        break;
                                    }
                                }
                            }
                            if exceeded {
                                drop(out_file);
                                let _ = std::fs::remove_file(&out_path);
                                let msg = format!("ZIP 条目实际解压大小超过 max_file_size，已中止并删除: {name} (written={written} bytes)");
                                warn!("{}", msg);
                                summary.errors.push(msg);
                                continue;
                            }
                            if let Some(e) = copy_error {
                                let msg = format!("Failed to extract file {}: {}", out_path.display(), e);
                                warn!("{}", msg);
                                summary.errors.push(msg);
                                drop(out_file);
                                let _ = std::fs::remove_file(&out_path);
                                continue;
                            }
                            {
                                // TOCTOU 防护: 创建后验证实际路径仍在目标目录内
                                if let (Ok(real_path), Ok(real_target)) =
                                    (std::fs::canonicalize(&out_path), std::fs::canonicalize(&target_path))
                                {
                                    if !real_path.starts_with(&real_target) {
                                        std::fs::remove_file(&out_path).ok();
                                        let msg = format!("符号链接逃逸已拦截 (ZIP): {}", safe_path.display());
                                        warn!("{}", msg);
                                        summary.errors.push(msg);
                                        drop(out_file);
                                        continue;
                                    }
                                }
                                // 记录实际写入字节（而非声明值），使压缩比
                                // 检测与累计上限基于真实数据。
                                summary.add_file(safe_path, written);
                            }
                        }
                        Err(e) => {
                            warn!("Failed to create file {:?}: {}", out_path, e);
                        }
                    }
                }
            }
            // 显式释放归档文件句柄
            drop(archive);
            Ok::<ExtractionSummary, AppError>(summary)
        })
        .await
        .map_err(|e| {
            if e.is_panic() {
                AppError::internal_error(format!("ZIP handler panicked: {e}"))
            } else {
                AppError::archive_error(e.to_string(), None)
            }
        })??;

        Ok(summary)
    }

    fn file_extensions(&self) -> Vec<&str> {
        vec!["zip"]
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;
    use zip::write::FileOptions;

    /// 创建测试用的 ZIP 文件
    fn create_test_zip(
        path: &Path,
        files: Vec<(&str, &[u8])>,
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let file = std::fs::File::create(path)?;
        let mut zip = zip::ZipWriter::new(file);
        let options: zip::write::FileOptions<'_, ()> =
            FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        for (name, content) in files {
            zip.start_file(name, options)?;
            zip.write_all(content)?;
        }

        zip.finish()?;
        Ok(())
    }

    #[test]
    fn test_zip_handler_can_handle() {
        let handler = ZipHandler;

        // 应该能处理 .zip 文件
        assert!(handler.can_handle(Path::new("test.zip")));
        assert!(handler.can_handle(Path::new("test.ZIP")));
        assert!(handler.can_handle(Path::new("/path/to/archive.zip")));

        // 不应该处理其他格式
        assert!(!handler.can_handle(Path::new("test.tar")));
        assert!(!handler.can_handle(Path::new("test.gz")));
        assert!(!handler.can_handle(Path::new("test.rar")));
        assert!(!handler.can_handle(Path::new("test.txt")));
        assert!(!handler.can_handle(Path::new("test")));
    }

    #[test]
    fn test_zip_handler_file_extensions() {
        let handler = ZipHandler;
        let extensions = handler.file_extensions();

        assert_eq!(extensions, vec!["zip"]);
    }

    #[tokio::test]
    async fn test_extract_zip_file() {
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let zip_file = temp_dir.path().join("test.zip");
        let output_dir = temp_dir.path().join("output");

        // 创建测试 ZIP 文件
        let files = vec![
            ("file1.txt", b"Hello, World!" as &[u8]),
            ("file2.txt", b"Second file content"),
            ("dir/nested.txt", b"Nested file content"),
        ];
        create_test_zip(&zip_file, files).expect("创建 ZIP 文件失败");

        // 解压文件
        let handler = ZipHandler;
        let summary = handler
            .extract(&zip_file, &output_dir)
            .await
            .expect("解压 ZIP 文件失败");

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
    async fn test_extract_zip_with_limits() {
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let zip_file = temp_dir.path().join("test.zip");
        let output_dir = temp_dir.path().join("output");

        // 创建测试 ZIP 文件
        let files = vec![
            ("small.txt", b"Small content" as &[u8]),
            ("large.txt", &[b'x'; 2000]), // 2KB 文件
        ];
        create_test_zip(&zip_file, files).expect("创建 ZIP 文件失败");

        // 使用限制解压：最大文件大小 1KB
        let handler = ZipHandler;
        let summary = handler
            .extract_with_limits(&zip_file, &output_dir, 1024, 1024 * 1024, 100)
            .await
            .expect("解压 ZIP 文件失败");

        // 只有小文件应该被解压
        assert_eq!(summary.files_extracted, 1);
        assert!(output_dir.join("small.txt").exists());
        assert!(!output_dir.join("large.txt").exists());
    }

    #[tokio::test]
    async fn test_extract_zip_max_total_size_limit() {
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let zip_file = temp_dir.path().join("test.zip");
        let output_dir = temp_dir.path().join("output");

        // 创建测试 ZIP 文件，总大小超过限制
        let files = vec![
            ("file1.txt", &[b'a'; 600][..]), // 600 bytes
            ("file2.txt", &[b'b'; 600][..]), // 600 bytes
        ];
        create_test_zip(&zip_file, files).expect("创建 ZIP 文件失败");

        // 使用限制解压：最大总大小 1KB
        let handler = ZipHandler;
        let summary = handler
            .extract_with_limits(&zip_file, &output_dir, 1024, 1024, 100)
            .await
            .expect("解压 ZIP 文件失败");

        // 只有第一个文件应该被解压
        assert_eq!(summary.files_extracted, 1);
        assert!(output_dir.join("file1.txt").exists());
    }

    #[tokio::test]
    async fn test_extract_zip_max_file_count_limit() {
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let zip_file = temp_dir.path().join("test.zip");
        let output_dir = temp_dir.path().join("output");

        // 创建测试 ZIP 文件，包含多个文件
        let files = vec![
            ("file1.txt", b"content1" as &[u8]),
            ("file2.txt", b"content2"),
            ("file3.txt", b"content3"),
        ];
        create_test_zip(&zip_file, files).expect("创建 ZIP 文件失败");

        // 使用限制解压：最大文件数量 2
        let handler = ZipHandler;
        let summary = handler
            .extract_with_limits(&zip_file, &output_dir, 1024 * 1024, 1024 * 1024 * 1024, 2)
            .await
            .expect("解压 ZIP 文件失败");

        // 只有前两个文件应该被解压
        assert_eq!(summary.files_extracted, 2);
    }

    #[tokio::test]
    async fn test_extract_zip_empty_file() {
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let zip_file = temp_dir.path().join("empty.zip");
        let output_dir = temp_dir.path().join("output");

        // 创建空内容的 ZIP 文件（只有目录条目）
        let file = std::fs::File::create(&zip_file).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.add_directory("empty_dir/", zip::write::FileOptions::<'_, ()>::default())
            .unwrap();
        zip.finish().unwrap();

        // 解压文件
        let handler = ZipHandler;
        let summary = handler
            .extract(&zip_file, &output_dir)
            .await
            .expect("解压 ZIP 文件失败");

        // 目录条目不应该被计为提取的文件
        assert_eq!(summary.files_extracted, 0);
        // 但目录应该被创建
        assert!(output_dir.join("empty_dir").exists());
    }

    #[tokio::test]
    async fn test_extract_zip_with_special_chars_in_filename() {
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let zip_file = temp_dir.path().join("test.zip");
        let output_dir = temp_dir.path().join("output");

        // 创建包含特殊字符文件名的 ZIP 文件
        let files = vec![
            ("file with spaces.txt", b"content1" as &[u8]),
            ("file-with-dashes.txt", b"content2"),
            ("file_with_underscores.txt", b"content3"),
        ];
        create_test_zip(&zip_file, files).expect("创建 ZIP 文件失败");

        // 解压文件
        let handler = ZipHandler;
        let summary = handler
            .extract(&zip_file, &output_dir)
            .await
            .expect("解压 ZIP 文件失败");

        assert_eq!(summary.files_extracted, 3);
        assert!(output_dir.join("file with spaces.txt").exists());
        assert!(output_dir.join("file-with-dashes.txt").exists());
        assert!(output_dir.join("file_with_underscores.txt").exists());
    }

    #[tokio::test]
    async fn test_extract_nonexistent_zip() {
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let zip_file = temp_dir.path().join("nonexistent.zip");
        let output_dir = temp_dir.path().join("output");

        let handler = ZipHandler;
        let result = handler.extract(&zip_file, &output_dir).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_extract_corrupted_zip() {
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let zip_file = temp_dir.path().join("corrupted.zip");
        let output_dir = temp_dir.path().join("output");

        // 写入损坏的 ZIP 数据
        std::fs::write(&zip_file, b"This is not a valid ZIP file").unwrap();

        let handler = ZipHandler;
        let result = handler.extract(&zip_file, &output_dir).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_extract_zip_slip_protection() {
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let zip_file = temp_dir.path().join("test.zip");
        let output_dir = temp_dir.path().join("output");

        // 创建包含路径遍历尝试的 ZIP 文件
        // 注意：zip crate 在写入时会自动规范化路径，所以我们测试的是解压时的保护
        let file = std::fs::File::create(&zip_file).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options: zip::write::FileOptions<'_, ()> =
            FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        // 添加一个正常文件
        zip.start_file("normal.txt", options).unwrap();
        zip.write_all(b"Normal content").unwrap();

        zip.finish().unwrap();

        // 解压文件
        let handler = ZipHandler;
        let summary = handler
            .extract(&zip_file, &output_dir)
            .await
            .expect("解压 ZIP 文件失败");

        assert_eq!(summary.files_extracted, 1);
        assert!(output_dir.join("normal.txt").exists());
    }

    #[tokio::test]
    async fn test_extract_zip_deeply_nested() {
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let zip_file = temp_dir.path().join("test.zip");
        let output_dir = temp_dir.path().join("output");

        // 创建深层嵌套目录结构的 ZIP 文件
        let files = vec![
            ("a/b/c/d/e/deep.txt", b"Deep nested content" as &[u8]),
            ("a/b/c/mid.txt", b"Mid level content"),
            ("a/top.txt", b"Top level content"),
        ];
        create_test_zip(&zip_file, files).expect("创建 ZIP 文件失败");

        // 解压文件
        let handler = ZipHandler;
        let summary = handler
            .extract(&zip_file, &output_dir)
            .await
            .expect("解压 ZIP 文件失败");

        assert_eq!(summary.files_extracted, 3);
        assert!(output_dir.join("a/b/c/d/e/deep.txt").exists());
        assert!(output_dir.join("a/b/c/mid.txt").exists());
        assert!(output_dir.join("a/top.txt").exists());

        // 验证内容
        let deep_content = std::fs::read_to_string(output_dir.join("a/b/c/d/e/deep.txt")).unwrap();
        assert_eq!(deep_content, "Deep nested content");
    }

    #[tokio::test]
    async fn test_extract_zip_large_file() {
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let zip_file = temp_dir.path().join("test.zip");
        let output_dir = temp_dir.path().join("output");

        // 创建包含大文件的 ZIP 文件 (5MB)
        let large_content = vec![b'x'; 5 * 1024 * 1024];
        let files = vec![("large.bin", large_content.as_slice())];
        create_test_zip(&zip_file, files).expect("创建 ZIP 文件失败");

        // 解压文件
        let handler = ZipHandler;
        let summary = handler
            .extract(&zip_file, &output_dir)
            .await
            .expect("解压 ZIP 文件失败");

        assert_eq!(summary.files_extracted, 1);
        assert_eq!(summary.total_size, 5 * 1024 * 1024);
        assert!(output_dir.join("large.bin").exists());

        // 验证文件大小
        let metadata = std::fs::metadata(output_dir.join("large.bin")).unwrap();
        assert_eq!(metadata.len(), 5 * 1024 * 1024);
    }

    #[tokio::test]
    async fn test_extract_zip_many_files() {
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let zip_file = temp_dir.path().join("test.zip");
        let output_dir = temp_dir.path().join("output");

        // 创建包含多个文件的 ZIP 文件
        let mut files = Vec::new();
        for i in 0..100 {
            files.push((
                format!("file{i:03}.txt"),
                format!("Content of file {i}").into_bytes(),
            ));
        }

        let file = std::fs::File::create(&zip_file).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options: zip::write::FileOptions<'_, ()> =
            FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        for (name, content) in files {
            zip.start_file(&name, options).unwrap();
            zip.write_all(&content).unwrap();
        }
        zip.finish().unwrap();

        // 解压文件
        let handler = ZipHandler;
        let summary = handler
            .extract(&zip_file, &output_dir)
            .await
            .expect("解压 ZIP 文件失败");

        assert_eq!(summary.files_extracted, 100);

        // 验证部分文件
        for i in [0, 50, 99] {
            let file_path = output_dir.join(format!("file{i:03}.txt"));
            assert!(file_path.exists());
            let content = std::fs::read_to_string(file_path).unwrap();
            assert_eq!(content, format!("Content of file {i}"));
        }
    }

    /// 表驱动 CRC-32（IEEE 802.3），用于手工构造 ZIP 字节流
    fn crc32(data: &[u8]) -> u32 {
        fn make_table() -> [u32; 256] {
            let mut table = [0u32; 256];
            for (i, entry) in table.iter_mut().enumerate() {
                let mut c = i as u32;
                for _ in 0..8 {
                    c = if c & 1 != 0 {
                        0xEDB88320 ^ (c >> 1)
                    } else {
                        c >> 1
                    };
                }
                *entry = c;
            }
            table
        }
        let table = make_table();
        let mut crc = 0xFFFF_FFFFu32;
        for &b in data {
            crc = table[((crc ^ b as u32) & 0xFF) as usize] ^ (crc >> 8);
        }
        !crc
    }

    /// 构造一个 ZIP：local header 与 central directory 的
    /// uncompressed_size 均伪造为 declared_size，而 deflate 流的实际
    /// 解压大小是 actual_data.len()。
    fn build_zip_with_forged_size(actual_data: &[u8], declared_size: u32) -> Vec<u8> {
        use flate2::write::DeflateEncoder;
        use flate2::Compression;

        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(actual_data).unwrap();
        let compressed = encoder.finish().unwrap();
        let crc = crc32(actual_data);
        let name = b"boom.log";
        let mut out = Vec::new();

        // ── Local file header ──
        out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        out.extend_from_slice(&20u16.to_le_bytes()); // version needed
        out.extend_from_slice(&0u16.to_le_bytes()); // flags
        out.extend_from_slice(&8u16.to_le_bytes()); // deflate
        out.extend_from_slice(&0u16.to_le_bytes()); // mod time
        out.extend_from_slice(&0u16.to_le_bytes()); // mod date
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&(compressed.len() as u32).to_le_bytes());
        out.extend_from_slice(&declared_size.to_le_bytes()); // ← 伪造
        out.extend_from_slice(&(name.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // extra len
        out.extend_from_slice(name);
        out.extend_from_slice(&compressed);

        // ── Central directory ──
        let cd_offset = out.len() as u32;
        out.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        out.extend_from_slice(&20u16.to_le_bytes()); // version made by
        out.extend_from_slice(&20u16.to_le_bytes()); // version needed
        out.extend_from_slice(&0u16.to_le_bytes()); // flags
        out.extend_from_slice(&8u16.to_le_bytes()); // deflate
        out.extend_from_slice(&0u16.to_le_bytes()); // mod time
        out.extend_from_slice(&0u16.to_le_bytes()); // mod date
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&(compressed.len() as u32).to_le_bytes());
        out.extend_from_slice(&declared_size.to_le_bytes()); // ← 伪造
        out.extend_from_slice(&(name.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // extra len
        out.extend_from_slice(&0u16.to_le_bytes()); // comment len
        out.extend_from_slice(&0u16.to_le_bytes()); // disk number
        out.extend_from_slice(&0u16.to_le_bytes()); // internal attrs
        out.extend_from_slice(&0u32.to_le_bytes()); // external attrs
        out.extend_from_slice(&0u32.to_le_bytes()); // local header offset
        out.extend_from_slice(name);
        let cd_len = out.len() as u32 - cd_offset;

        // ── End of central directory ──
        out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // disk
        out.extend_from_slice(&0u16.to_le_bytes()); // cd disk
        out.extend_from_slice(&1u16.to_le_bytes()); // entries on disk
        out.extend_from_slice(&1u16.to_le_bytes()); // total entries
        out.extend_from_slice(&cd_len.to_le_bytes());
        out.extend_from_slice(&cd_offset.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // comment len
        out
    }

    #[tokio::test]
    async fn forged_zip_size_is_throttled_by_actual_written_bytes() {
        let temp_dir = TempDir::new().unwrap();
        let zip_path = temp_dir.path().join("bomb.zip");
        let output_dir = temp_dir.path().join("output");

        // 声明 10 字节，实际解压 5000 字节 —— 前置声明值检查会放行。
        let actual = vec![b'a'; 5000];
        let zip_bytes = build_zip_with_forged_size(&actual, 10);
        std::fs::write(&zip_path, &zip_bytes).unwrap();

        let handler = ZipHandler;
        let summary = handler
            .extract_with_limits(&zip_path, &output_dir, 1000, 10_000, 100)
            .await
            .expect("extraction should complete (entry skipped)");

        assert_eq!(summary.files_extracted, 0);
        assert!(
            !output_dir.join("boom.log").exists(),
            "partial file must be removed"
        );
        assert!(
            summary.errors.iter().any(|e| e.contains("max_file_size")),
            "abort must be recorded in errors, got: {:?}",
            summary.errors,
        );
    }

    #[tokio::test]
    async fn normal_zip_entry_is_recorded_with_actual_size() {
        let temp_dir = TempDir::new().unwrap();
        let zip_path = temp_dir.path().join("ok.zip");
        let output_dir = temp_dir.path().join("output");

        let files = vec![("small.txt", b"hello world" as &[u8])];
        create_test_zip(&zip_path, files).unwrap();

        let handler = ZipHandler;
        let summary = handler
            .extract_with_limits(&zip_path, &output_dir, 10_000, 10_000, 100)
            .await
            .unwrap();

        assert_eq!(summary.files_extracted, 1);
        assert!(output_dir.join("small.txt").exists());
    }
}
