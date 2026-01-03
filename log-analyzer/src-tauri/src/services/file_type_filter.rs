//! 文件类型过滤器
//!
//! 三层检测策略过滤非日志文件：
//! - 第1层：二进制文件快速检测（魔数 + 空字节比例）
//! - 第2层：文件名 Glob 模式匹配
//! - 第3层：扩展名白名单/黑名单
//!
//! 防御性设计原则：
//! - 失败安全：任何错误都返回 true（允许文件通过）
//! - 零侵入：独立模块，可随时移除
//! - 可观测：详细日志记录每个决策

use std::fs::File;
use std::io::Read;
use std::path::Path;
use crate::models::{FileFilterConfig, FilterMode};

/// 文件类型过滤器（三层检测策略）
pub struct FileTypeFilter {
    config: FileFilterConfig,
}

impl FileTypeFilter {
    /// 创建新的过滤器
    pub fn new(config: FileFilterConfig) -> Self {
        Self { config }
    }

    /// 检查文件是否应该被导入（三层检测）
    /// 返回 true 表示允许导入，false 表示跳过
    pub fn should_import_file(&self, path: &Path) -> bool {
        // 直接调用防御性版本，忽略错误（内部会处理）
        self.should_import_file_safe(path).unwrap_or(true)
    }

    /// 检查文件是否应该被导入（防御性版本，返回 Result）
    ///
    /// 防御性设计：
    /// - 所有 I/O 错误都返回 Err
    /// - 所有 panic 都被 catch
    /// - 调用者决定如何处理错误（建议返回 true）
    pub fn should_import_file_safe(&self, path: &Path) -> Result<bool, String> {
        // 使用 catch_unwind 捕获可能的 panic
        std::panic::catch_unwind(
            std::panic::AssertUnwindSafe(|| {
                // 第1层：二进制文件快速检测（始终启用）
                if self.config.binary_detection_enabled {
                    match self.detect_binary_file(path) {
                        Ok(is_binary) => {
                            if is_binary {
                                tracing::debug!(
                                    file = %path.display(),
                                    "File skipped: detected as binary"
                                );
                                return Ok(false);
                            }
                        }
                        Err(e) => {
                            // 二进制检测失败，记录警告但继续处理
                            tracing::warn!(
                                file = %path.display(),
                                error = %e,
                                "Binary detection failed, proceeding with filter rules"
                            );
                        }
                    }
                }

                // 如果第2层过滤未启用，允许所有文本文件
                if !self.config.enabled {
                    return Ok(true);
                }

                // 第2层：智能过滤规则
                Ok(self.apply_filter_rules(path))
            })
        ).map_err(|e| {
            // 捕获 panic，转换为错误消息
            tracing::error!(
                file = %path.display(),
                panic_info = ?e,
                "File filter logic panicked, allowing file (fail-safe)"
            );
            "File filter logic panicked".to_string()
        })?
    }

    /// 第1层：二进制文件检测（仅读取前1KB，防御性版本）
    fn detect_binary_file(&self, path: &Path) -> std::io::Result<bool> {
        const MAGIC_NUMBERS: &[(&[u8], &str)] = &[
            // 图片
            (&[0xFF, 0xD8, 0xFF], "JPEG"),
            (&[0x89, 0x50, 0x4E, 0x47], "PNG"),
            (&[0x47, 0x49, 0x46], "GIF"),
            (&[0x42, 0x4D], "BMP"),
            (&[0x49, 0x49, 0x2A, 0x00], "TIFF"),

            // 视频/音频
            (&[0x00, 0x00, 0x00, 0x18, 0x66, 0x74, 0x79, 0x70], "MP4"),
            (&[0x1A, 0x45, 0xDF, 0xA3], "MKV"),
            (&[0x49, 0x44, 0x33], "MP3"),
            (&[0xFF, 0xFB], "MP3"),
            (&[0x52, 0x49, 0x46, 0x46], "WAV/AVI"),

            // 可执行文件
            (&[0x4D, 0x5A], "EXE"),
            (&[0x7F, 0x45, 0x4C, 0x46], "ELF"),
            (&[0xFE, 0xED, 0xFA, 0xCF], "Mach-O"),

            // 压缩文件（注意：这些会由 is_archive_file 处理）
            (&[0x50, 0x4B, 0x03, 0x04], "ZIP"),
            (&[0x1F, 0x8B], "GZ"),
        ];

        let mut file = File::open(path)?;
        let mut buffer = [0u8; 1024];  // 只读前1KB
        let n = file.read(&mut buffer)?;

        if n == 0 {
            return Ok(false);  // 空文件不算二进制
        }

        // 检查魔数
        for &(magic, name) in MAGIC_NUMBERS {
            if buffer.starts_with(magic) {
                tracing::debug!(
                    file = %path.display(),
                    file_type = name,
                    "Detected binary file by magic number"
                );
                return Ok(true);
            }
        }

        // 检查空字节比例（二进制文件通常包含大量空字节）
        let null_count = buffer[..n].iter().filter(|&&b| b == 0).count();
        let null_ratio = null_count as f64 / n as f64;

        if null_ratio > 0.05 {  // 5% 空字节阈值
            tracing::debug!(
                file = %path.display(),
                null_ratio = null_ratio,
                "Detected binary file by null byte ratio"
            );
            return Ok(true);
        }

        Ok(false)
    }

    /// 第2层：应用智能过滤规则
    fn apply_filter_rules(&self, path: &Path) -> bool {
        // A. 文件名 Glob 模式匹配（支持无后缀日志）
        if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
            for pattern in &self.config.filename_patterns {
                if let Ok(matched) = self.glob_match(pattern, file_name) {
                    if matched {
                        tracing::debug!(
                            file = %file_name,
                            pattern = pattern,
                            "File matched filename pattern"
                        );
                        return true;  // 匹配模式，允许导入
                    }
                }
            }
        }

        // B. 扩展名检查
        if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
            let ext_lower = extension.to_lowercase();

            match self.config.mode {
                FilterMode::Whitelist => {
                    // 白名单模式：必须在允许列表中
                    if self.config.allowed_extensions.is_empty() {
                        return true;  // 空白名单 = 允许所有
                    }
                    return self.config.allowed_extensions.iter()
                        .any(|allowed| allowed.eq_ignore_ascii_case(&ext_lower));
                }
                FilterMode::Blacklist => {
                    // 黑名单模式：不能在禁止列表中
                    return !self.config.forbidden_extensions.iter()
                        .any(|forbidden| forbidden.eq_ignore_ascii_case(&ext_lower));
                }
            }
        }

        // 无扩展名且未匹配文件名模式
        match self.config.mode {
            FilterMode::Whitelist => false,  // 白名单拒绝
            FilterMode::Blacklist => true,   // 黑名单允许
        }
    }

    /// 简单的 glob 模式匹配（支持 * 和 ?）
    fn glob_match(&self, pattern: &str, text: &str) -> Result<bool, regex::Error> {
        // 转换为小写进行不区分大小写匹配
        let pattern_lower = pattern.to_lowercase();
        let text_lower = text.to_lowercase();

        // 简单实现：将 * 替换为正则表达式 .*
        let regex_pattern = pattern_lower
            .replace('.', r"\.")  // 转义点号
            .replace('*', ".*")
            .replace('?', ".");

        let re = regex::Regex::new(&regex_pattern)?;
        Ok(re.is_match(&text_lower))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disabled_filter() {
        let config = FileFilterConfig {
            enabled: false,
            binary_detection_enabled: false,
            ..Default::default()
        };

        let filter = FileTypeFilter::new(config);
        assert!(filter.should_import_file(Path::new("test.anything")));
    }

    #[test]
    fn test_glob_match() {
        let config = FileFilterConfig {
            enabled: true,
            binary_detection_enabled: false,
            mode: FilterMode::Whitelist,
            filename_patterns: vec![
                "syslog".to_string(),
                "*log*".to_string(),
            ],
            allowed_extensions: vec![],
            ..Default::default()
        };

        let filter = FileTypeFilter::new(config);

        // 测试文件名模式匹配
        assert!(filter.glob_match("syslog", "syslog").unwrap());
        assert!(filter.glob_match("*log*", "app.log").unwrap());
        assert!(filter.glob_match("*log*", "mylog.txt").unwrap());
        assert!(!filter.glob_match("*log*", "data.csv").unwrap());
    }

    #[test]
    fn test_case_insensitive_glob() {
        let config = FileFilterConfig {
            enabled: true,
            binary_detection_enabled: false,
            mode: FilterMode::Whitelist,
            filename_patterns: vec!["*Log*".to_string()],
            allowed_extensions: vec![],
            ..Default::default()
        };

        let filter = FileTypeFilter::new(config);

        // 测试大小写不敏感
        assert!(filter.glob_match("*Log*", "app.log").unwrap());
        assert!(filter.glob_match("*Log*", "app.LOG").unwrap());
        assert!(filter.glob_match("*Log*", "ERROR.Log").unwrap());
    }

    #[test]
    fn test_whitelist_mode() {
        let config = FileFilterConfig {
            enabled: true,
            binary_detection_enabled: false,
            mode: FilterMode::Whitelist,
            allowed_extensions: vec!["log".to_string(), "txt".to_string()],
            ..Default::default()
        };

        let filter = FileTypeFilter::new(config);

        assert!(filter.should_import_file(Path::new("test.log")));
        assert!(filter.should_import_file(Path::new("test.txt")));
        assert!(!filter.should_import_file(Path::new("test.exe")));
        assert!(!filter.should_import_file(Path::new("test.pdf")));
    }

    #[test]
    fn test_blacklist_mode() {
        let config = FileFilterConfig {
            enabled: true,
            binary_detection_enabled: false,
            mode: FilterMode::Blacklist,
            forbidden_extensions: vec!["exe".to_string(), "pdf".to_string()],
            ..Default::default()
        };

        let filter = FileTypeFilter::new(config);

        assert!(filter.should_import_file(Path::new("test.log")));
        assert!(filter.should_import_file(Path::new("test.txt")));
        assert!(!filter.should_import_file(Path::new("test.exe")));
        assert!(!filter.should_import_file(Path::new("test.pdf")));
    }

    #[test]
    fn test_empty_whitelist_allows_all() {
        let config = FileFilterConfig {
            enabled: true,
            binary_detection_enabled: false,
            mode: FilterMode::Whitelist,
            allowed_extensions: vec![],  // 空白名单
            ..Default::default()
        };

        let filter = FileTypeFilter::new(config);

        // 空白名单应该允许所有文件
        assert!(filter.should_import_file(Path::new("test.log")));
        assert!(filter.should_import_file(Path::new("test.exe")));
        assert!(filter.should_import_file(Path::new("test.any")));
    }

    #[test]
    fn test_filename_patterns_with_extensions() {
        let config = FileFilterConfig {
            enabled: true,
            binary_detection_enabled: false,
            mode: FilterMode::Whitelist,
            filename_patterns: vec!["*log*".to_string()],
            allowed_extensions: vec!["txt".to_string()],
            ..Default::default()
        };

        let filter = FileTypeFilter::new(config);

        // 文件名模式匹配优先
        assert!(filter.should_import_file(Path::new("app.log")));      // 匹配 *log* 模式
        assert!(filter.should_import_file(Path::new("test.txt")));     // 匹配扩展名
        assert!(!filter.should_import_file(Path::new("data.csv")));     // 都不匹配
    }
}
