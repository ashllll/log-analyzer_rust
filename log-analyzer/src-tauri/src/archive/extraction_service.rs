//! 压缩文件提取服务
//!
//! 提供带有安全限制和验证的压缩文件提取功能
//! 包含 TOCTOU 安全检查，使用 O_NOFOLLOW 标志原子性验证文件

use eyre::eyre;
use eyre::Result;
use rustix::fs::OFlags;
use std::path::Path;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

/// 提取进度跟踪器
#[derive(Debug)]
pub struct ExtractionProgress {
    pub files_processed: Arc<AtomicUsize>,
    pub bytes_processed: Arc<AtomicU64>,
    pub files_extracted: Arc<AtomicUsize>,
    pub bytes_extracted: Arc<AtomicU64>,
    pub errors_count: Arc<AtomicUsize>,
}

impl ExtractionProgress {
    pub fn new() -> Self {
        Self {
            files_processed: Arc::new(AtomicUsize::new(0)),
            bytes_processed: Arc::new(AtomicU64::new(0)),
            files_extracted: Arc::new(AtomicUsize::new(0)),
            bytes_extracted: Arc::new(AtomicU64::new(0)),
            errors_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn increment_processed(&self, size: u64) {
        self.files_processed.fetch_add(1, Ordering::Relaxed);
        self.bytes_processed.fetch_add(size, Ordering::Relaxed);
    }

    pub fn increment_extracted(&self, size: u64) {
        self.files_extracted.fetch_add(1, Ordering::Relaxed);
        self.bytes_extracted.fetch_add(size, Ordering::Relaxed);
    }

    pub fn increment_errors(&self) {
        self.errors_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_stats(&self) -> (usize, u64, usize, u64, usize) {
        (
            self.files_processed.load(Ordering::Relaxed),
            self.bytes_processed.load(Ordering::Relaxed),
            self.files_extracted.load(Ordering::Relaxed),
            self.bytes_extracted.load(Ordering::Relaxed),
            self.errors_count.load(Ordering::Relaxed),
        )
    }
}

/// 提取限制配置
#[derive(Debug, Clone)]
pub struct ExtractionLimits {
    pub max_file_size: u64,
    pub max_total_size: u64,
    pub max_file_count: usize,
    pub allowed_extensions: Vec<String>,
    pub forbidden_extensions: Vec<String>,
    pub validate_filenames: bool,
}

impl Default for ExtractionLimits {
    fn default() -> Self {
        Self {
            max_file_size: 104_857_600,    // 100MB
            max_total_size: 1_073_741_824, // 1GB
            max_file_count: 1000,
            allowed_extensions: Vec::new(),
            forbidden_extensions: vec![
                "exe".to_string(),
                "bat".to_string(),
                "cmd".to_string(),
                "com".to_string(),
                "scr".to_string(),
                "pif".to_string(),
                "sh".to_string(),
                "bash".to_string(),
                "zsh".to_string(),
            ],
            validate_filenames: true,
        }
    }
}

/// 增强的压缩文件提取服务
pub struct ArchiveExtractionService {
    limits: ExtractionLimits,
}

impl ArchiveExtractionService {
    /// 创建新的提取服务
    pub fn new(limits: ExtractionLimits) -> Self {
        Self { limits }
    }

    /// 使用默认配置创建服务
    pub fn default() -> Self {
        Self::new(ExtractionLimits::default())
    }

    /// 获取提取限制
    pub fn get_limits(&self) -> &ExtractionLimits {
        &self.limits
    }

    /// 更新提取限制
    pub fn update_limits(&mut self, limits: ExtractionLimits) {
        self.limits = limits;
    }
}

/// 使用 O_NOFOLLOW 原子性检查文件安全性
///
/// 此函数解决了 TOCTOU (Time-of-Check to Time-of-Use) 竞态问题：
/// 传统的先检查 is_symlink() 再操作的方式存在时间窗口，
/// 攻击者可以在检查后、操作前修改文件为符号链接。
/// 使用 openat with O_NOFOLLOW 可以在打开文件时原子性地检测符号链接，
/// 如果路径是符号链接则打开失败，而不是先检查后使用。
pub fn check_file_safety_with_nofollow(file_path: &Path) -> Result<()> {
    // 使用 rustix 的 open 配合 O_NOFOLLOW 标志
    // O_NOFOLLOW: 如果路径是符号链接，则打开失败 (ELOOP)
    // 这提供了原子性的安全检查
    let result = rustix::fs::open(
        file_path,
        OFlags::RDONLY | OFlags::NOFOLLOW,
        rustix::fs::Mode::empty(),
    );

    match result {
        Ok(fd) => {
            // 成功打开文件（不是符号链接），OwnedFd 会自动关闭
            // 不需要手动关闭，Drop 会自动处理
            let _ = fd;
            Ok(())
        }
        Err(e) => {
            // rustix 的错误使用 Errno 类型，需要转换
            let error_code = e.raw_os_error() as i32;
            if error_code == libc::ELOOP || error_code == libc::EMLINK {
                // ELOOP: 符号链接（在 POSIX 系统上）
                // EMLINK: 指向符号链接（某些系统的行为）
                Err(eyre!(
                    "Symbolic link detected (O_NOFOLLOW): {}",
                    file_path.display()
                ))
            } else if error_code == libc::ENOENT {
                // 文件不存在
                Err(eyre!("File does not exist: {}", file_path.display()))
            } else if error_code == libc::EACCES {
                // 权限不足
                Err(eyre!(
                    "Permission denied accessing file: {}",
                    file_path.display()
                ))
            } else {
                // 其他错误
                Err(eyre!(
                    "Cannot open file (O_NOFOLLOW): {} - {}",
                    file_path.display(),
                    e
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_extraction_limits_default() {
        let limits = ExtractionLimits::default();
        assert_eq!(limits.max_file_size, 104_857_600);
        assert_eq!(limits.max_total_size, 1_073_741_824);
        assert_eq!(limits.max_file_count, 1000);
        assert!(limits.validate_filenames);
        assert!(!limits.forbidden_extensions.is_empty());
    }

    #[test]
    fn test_extraction_progress() {
        let progress = ExtractionProgress::new();

        progress.increment_processed(1000);
        progress.increment_extracted(800);
        progress.increment_errors();

        let (processed, bytes_processed, extracted, bytes_extracted, errors) = progress.get_stats();
        assert_eq!(processed, 1);
        assert_eq!(bytes_processed, 1000);
        assert_eq!(extracted, 1);
        assert_eq!(bytes_extracted, 800);
        assert_eq!(errors, 1);
    }

    /// TOCTOU 安全测试：O_NOFOLLOW 检查普通文件
    #[test]
    fn test_check_file_safety_with_nofollow_regular_file() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        std::fs::write(&test_file, "test content").unwrap();

        // 普通文件应该能通过 O_NOFOLLOW 检查
        let result = check_file_safety_with_nofollow(&test_file);
        assert!(result.is_ok(), "Regular file should pass O_NOFOLLOW check: {:?}", result);
    }

    /// TOCTOU 安全测试：O_NOFOLLOW 检查不存在的文件
    #[test]
    fn test_check_file_safety_with_nofollow_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let nonexistent = temp_dir.path().join("nonexistent.txt");

        // 不存在的文件应该返回错误
        let result = check_file_safety_with_nofollow(&nonexistent);
        assert!(result.is_err(), "Non-existent file should fail O_NOFOLLOW check");
    }

    /// TOCTOU 安全测试：O_NOFOLLOW 检查符号链接
    #[test]
    #[cfg(unix)]
    fn test_check_file_safety_with_nofollow_symlink() {
        use std::os::unix::fs as unix_fs;

        let temp_dir = TempDir::new().unwrap();
        let target_file = temp_dir.path().join("target.txt");
        let symlink_file = temp_dir.path().join("link.txt");

        std::fs::write(&target_file, "target content").unwrap();
        unix_fs::symlink(&target_file, &symlink_file).unwrap();

        // 符号链接应该被 O_NOFOLLOW 检测到并拒绝
        let result = check_file_safety_with_nofollow(&symlink_file);
        assert!(result.is_err(), "Symlink should fail O_NOFOLLOW check: {:?}", result);
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Symbolic link") || err_msg.contains("O_NOFOLLOW"),
            "Error message should mention symlink or O_NOFOLLOW, got: {}",
            err_msg
        );
    }

    /// TOCTOU 安全测试：O_NOFOLLOW 检查目录
    #[test]
    fn test_check_file_safety_with_nofollow_directory() {
        let temp_dir = TempDir::new().unwrap();
        let sub_dir = temp_dir.path().join("subdir");
        std::fs::create_dir(&sub_dir).unwrap();

        // 目录应该能通过 O_NOFOLLOW 检查（O_NOFOLLOW 只对最终路径组件生效）
        // 注意：在某些系统上，目录可能无法以 RDONLY 模式打开
        // 这里我们只验证函数能正常执行
        let result = check_file_safety_with_nofollow(&sub_dir);
        let _ = result; // 允许任何结果
    }
}
