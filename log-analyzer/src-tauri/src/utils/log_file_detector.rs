//! 日志文件检测器模块
//!
//! 智能识别日志文件,过滤非日志文件,提升索引效率和搜索质量
//! 支持扩展名白名单、文件名模式匹配、内容特征检测和Android日志特殊识别

use regex::Regex;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// 文件识别结果
#[derive(Debug, Clone, PartialEq)]
pub enum FileTypeResult {
    /// 是日志文件,返回类型
    LogFile(LogFileType),
    /// 不是日志文件,返回原因
    NonLogFile(String),
    /// 无法判断
    Unknown,
}

/// 日志文件类型
#[derive(Debug, Clone, PartialEq)]
pub enum LogFileType {
    /// 通用日志
    Generic,
    /// Android Logcat
    AndroidLogcat,
    /// Android Tombstone
    AndroidTombstone,
    /// Android Kernel日志
    AndroidKernel,
    /// 系统日志(syslog等)
    #[allow(dead_code)]
    SystemLog,
    /// 应用日志
    #[allow(dead_code)]
    ApplicationLog,
}

/// 检测配置
pub struct DetectorConfig {
    /// 扩展名白名单(优先级最高)
    pub extension_whitelist: Vec<String>,
    /// 扩展名黑名单
    pub extension_blacklist: Vec<String>,
    /// 是否启用内容检测
    pub enable_content_detection: bool,
    /// 内容检测采样行数
    pub content_sample_lines: usize,
    /// 是否启用Android增强
    pub enable_android_detection: bool,
}

impl Default for DetectorConfig {
    fn default() -> Self {
        Self {
            extension_whitelist: vec![
                "log".to_string(),
                "txt".to_string(),
                "out".to_string(),
                "trace".to_string(),
                "crash".to_string(),
                "dump".to_string(),
                "tombstone".to_string(),
                "anr".to_string(),
                "err".to_string(),
                "error".to_string(),
                "warn".to_string(),
                "info".to_string(),
                "debug".to_string(),
            ],
            extension_blacklist: vec![
                "json".to_string(),
                "xml".to_string(),
                "yml".to_string(),
                "yaml".to_string(),
                "conf".to_string(),
                "config".to_string(),
                "sh".to_string(),
                "bash".to_string(),
                "py".to_string(),
                "js".to_string(),
                "java".to_string(),
                "md".to_string(),
                "rst".to_string(),
                "csv".to_string(),
                "sql".to_string(),
                "properties".to_string(),
                "ini".to_string(),
            ],
            enable_content_detection: true,
            content_sample_lines: 100,
            enable_android_detection: true,
        }
    }
}

/// 日志文件检测器
#[allow(dead_code)]
pub struct LogFileDetector {
    config: DetectorConfig,
    filename_patterns: Vec<Regex>,
}

impl LogFileDetector {
    /// 创建新的检测器
    #[allow(dead_code)]
    pub fn new(config: DetectorConfig) -> Self {
        // 编译文件名模式正则表达式
        let patterns = vec![
            r"(?i)^.*\.log$",                           // 以.log结尾
            r"(?i)^log[._-].*",                         // 以log开头
            r"(?i)^.*[._-]log[._-].*",                  // 包含log
            r"(?i)^(system|kernel|dmesg).*",            // 系统日志
            r"(?i)^logcat.*",                           // Android logcat
            r"(?i)^tombstone.*",                        // Android tombstone
            r"(?i)^crash.*",                            // 崩溃日志
            r"(?i)^.*\.trace$",                         // trace文件
            r"(?i)^.*[._](err|error|warn|info|debug)$", // 按级别命名
        ];

        let filename_patterns = patterns
            .into_iter()
            .filter_map(|p| Regex::new(p).ok())
            .collect();

        Self {
            config,
            filename_patterns,
        }
    }

    /// 检测文件是否为日志文件
    #[allow(dead_code)]
    pub fn detect(&self, path: &Path) -> Result<FileTypeResult, String> {
        // 1. 扩展名黑名单检测(最高优先级,直接拒绝)
        if let Some(result) = self.detect_by_blacklist(path) {
            return Ok(result);
        }

        // 2. 扩展名白名单检测
        if let Some(result) = self.detect_by_extension(path) {
            return Ok(result);
        }

        // 3. 文件名模式检测
        if let Some(result) = self.detect_by_filename(path) {
            return Ok(result);
        }

        // 4. 内容特征检测
        if self.config.enable_content_detection {
            return self.detect_by_content(path);
        }

        // 默认:未知
        Ok(FileTypeResult::Unknown)
    }

    /// 基于黑名单检测
    #[allow(dead_code)]
    fn detect_by_blacklist(&self, path: &Path) -> Option<FileTypeResult> {
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            let ext_lower = ext.to_lowercase();
            if self.config.extension_blacklist.contains(&ext_lower) {
                return Some(FileTypeResult::NonLogFile(format!(
                    "扩展名在黑名单中: .{}",
                    ext
                )));
            }
        }
        None
    }

    /// 基于扩展名检测
    #[allow(dead_code)]
    fn detect_by_extension(&self, path: &Path) -> Option<FileTypeResult> {
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            let ext_lower = ext.to_lowercase();
            if self.config.extension_whitelist.contains(&ext_lower) {
                return Some(FileTypeResult::LogFile(LogFileType::Generic));
            }
        }
        None
    }

    /// 基于文件名检测
    #[allow(dead_code)]
    fn detect_by_filename(&self, path: &Path) -> Option<FileTypeResult> {
        let filename = path.file_name()?.to_str()?;

        for pattern in &self.filename_patterns {
            if pattern.is_match(filename) {
                // 检查是否为Android特定日志
                if filename.to_lowercase().contains("logcat") {
                    return Some(FileTypeResult::LogFile(LogFileType::AndroidLogcat));
                }
                if filename.to_lowercase().contains("tombstone") {
                    return Some(FileTypeResult::LogFile(LogFileType::AndroidTombstone));
                }
                if filename.to_lowercase().contains("kernel")
                    || filename.to_lowercase().contains("dmesg")
                {
                    return Some(FileTypeResult::LogFile(LogFileType::AndroidKernel));
                }

                return Some(FileTypeResult::LogFile(LogFileType::Generic));
            }
        }

        None
    }

    /// 基于内容检测
    #[allow(dead_code)]
    fn detect_by_content(&self, path: &Path) -> Result<FileTypeResult, String> {
        // 打开文件并读取前N行
        let file = match File::open(path) {
            Ok(f) => f,
            Err(_) => return Ok(FileTypeResult::Unknown),
        };

        let reader = BufReader::new(file);
        let mut lines: Vec<String> = Vec::new();

        for (i, line_result) in reader.lines().enumerate() {
            if i >= self.config.content_sample_lines {
                break;
            }
            if let Ok(line) = line_result {
                lines.push(line);
            }
        }

        if lines.is_empty() {
            return Ok(FileTypeResult::NonLogFile("文件为空".to_string()));
        }

        // Android日志特征检测
        if self.config.enable_android_detection {
            if let Some(log_type) = self.detect_android_log(&lines) {
                return Ok(FileTypeResult::LogFile(log_type));
            }
        }

        // 通用日志特征检测
        if self.detect_generic_log_features(&lines) {
            return Ok(FileTypeResult::LogFile(LogFileType::Generic));
        }

        Ok(FileTypeResult::NonLogFile(
            "内容不符合日志文件特征".to_string(),
        ))
    }

    /// Android日志特征检测
    #[allow(dead_code)]
    fn detect_android_log(&self, lines: &[String]) -> Option<LogFileType> {
        // Tombstone特征检测
        if self.is_android_tombstone(lines) {
            return Some(LogFileType::AndroidTombstone);
        }

        // Kernel log特征检测
        if self.is_android_kernel_log(lines) {
            return Some(LogFileType::AndroidKernel);
        }

        // Logcat特征检测
        if self.is_android_logcat(lines) {
            return Some(LogFileType::AndroidLogcat);
        }

        None
    }

    /// 检测是否为Android Logcat
    #[allow(dead_code)]
    fn is_android_logcat(&self, lines: &[String]) -> bool {
        let mut logcat_features = 0;

        for line in lines.iter().take(50) {
            // 特征1: MM-DD HH:MM:SS.mmm 格式时间戳
            if line.len() > 18 {
                let time_part = &line[0..18.min(line.len())];
                if time_part.contains('-') && time_part.contains(':') && time_part.contains('.') {
                    logcat_features += 1;
                }
            }

            // 特征2: 日志级别标记 (V/D/I/W/E/F)
            if line.contains(" V/")
                || line.contains(" D/")
                || line.contains(" I/")
                || line.contains(" W/")
                || line.contains(" E/")
                || line.contains(" F/")
            {
                logcat_features += 1;
            }
        }

        // 如果超过30%的行有logcat特征,判定为logcat
        logcat_features as f64 / lines.len().min(50) as f64 > 0.3
    }

    /// 检测是否为Android Tombstone
    #[allow(dead_code)]
    fn is_android_tombstone(&self, lines: &[String]) -> bool {
        for line in lines.iter().take(20) {
            if line.contains("*** *** *** *** *** *** *** *** *** *** *** *** *** *** *** ***")
                || line.contains("Build fingerprint:")
                || (line.contains("signal") && line.contains("fault addr"))
                || line.contains("backtrace:")
            {
                return true;
            }
        }
        false
    }

    /// 检测是否为Android Kernel日志
    #[allow(dead_code)]
    fn is_android_kernel_log(&self, lines: &[String]) -> bool {
        for line in lines.iter().take(20) {
            if line.contains("[    0.000000]")
                || line.contains("Linux version")
                || line.contains("Kernel command line:")
            {
                return true;
            }
        }
        false
    }

    /// 通用日志特征检测
    #[allow(dead_code)]
    fn detect_generic_log_features(&self, lines: &[String]) -> bool {
        let mut timestamp_count = 0;
        let mut log_level_count = 0;

        for line in lines.iter().take(100) {
            // 检测时间戳
            if self.contains_timestamp(line) {
                timestamp_count += 1;
            }

            // 检测日志级别关键词
            if self.contains_log_level(line) {
                log_level_count += 1;
            }
        }

        let sample_size = lines.len().min(100) as f64;

        // 判定标准:
        // - 包含时间戳的行占比>30% 或
        // - 包含日志级别的行占比>20%
        (timestamp_count as f64 / sample_size > 0.3) || (log_level_count as f64 / sample_size > 0.2)
    }

    /// 检测行中是否包含时间戳
    #[allow(dead_code)]
    fn contains_timestamp(&self, line: &str) -> bool {
        // ISO 8601: 2024-01-20T10:30:45 或 2024-01-20 10:30:45
        if (line.contains("2024-") || line.contains("2023-") || line.contains("2025-"))
            && line.contains(':')
        {
            return true;
        }

        // Syslog: Jan 20 10:30:45
        let months = [
            "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
        ];
        for month in &months {
            if line.contains(month) && line.contains(':') {
                return true;
            }
        }

        // 简单的时间格式: HH:MM:SS 或 [HH:MM:SS]
        let parts: Vec<&str> = line.split_whitespace().collect();
        for part in parts {
            let time_part = part.trim_matches(|c| c == '[' || c == ']');
            if time_part.len() == 8 && time_part.chars().filter(|c| *c == ':').count() == 2 {
                // 验证是否为有效时间格式 HH:MM:SS
                let time_components: Vec<&str> = time_part.split(':').collect();
                if time_components.len() == 3
                    && time_components.iter().all(|c| c.parse::<u32>().is_ok())
                {
                    return true;
                }
            }
        }

        false
    }

    /// 检测行中是否包含日志级别
    #[allow(dead_code)]
    fn contains_log_level(&self, line: &str) -> bool {
        let line_upper = line.to_uppercase();
        line_upper.contains("ERROR")
            || line_upper.contains("ERRO")
            || line_upper.contains("ERR")
            || line_upper.contains("WARN")
            || line_upper.contains("WARNING")
            || line_upper.contains("INFO")
            || line_upper.contains("DEBUG")
            || line_upper.contains("DEBU")
            || line_upper.contains("TRACE")
            || line_upper.contains("FATAL")
            || line_upper.contains("CRIT")
    }
}

impl Default for LogFileDetector {
    fn default() -> Self {
        Self::new(DetectorConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_detect_by_extension_whitelist() {
        let detector = LogFileDetector::default();
        let path = Path::new("test.log");
        let result = detector.detect_by_extension(path);
        assert!(matches!(result, Some(FileTypeResult::LogFile(_))));
    }

    #[test]
    fn test_detect_by_extension_blacklist() {
        let detector = LogFileDetector::default();
        let path = Path::new("config.json");
        let result = detector.detect_by_blacklist(path);
        assert!(matches!(result, Some(FileTypeResult::NonLogFile(_))));
    }

    #[test]
    fn test_detect_by_filename_logcat() {
        let detector = LogFileDetector::default();
        let path = Path::new("logcat.txt");
        let result = detector.detect_by_filename(path);
        assert!(matches!(
            result,
            Some(FileTypeResult::LogFile(LogFileType::AndroidLogcat))
        ));
    }

    #[test]
    fn test_detect_by_filename_tombstone() {
        let detector = LogFileDetector::default();
        let path = Path::new("tombstone_00");
        let result = detector.detect_by_filename(path);
        assert!(matches!(
            result,
            Some(FileTypeResult::LogFile(LogFileType::AndroidTombstone))
        ));
    }

    #[test]
    fn test_contains_timestamp() {
        let detector = LogFileDetector::default();
        assert!(detector.contains_timestamp("2024-01-20 10:30:45 INFO Starting application"));
        assert!(detector.contains_timestamp("Jan 20 10:30:45 kernel: message"));
        assert!(detector.contains_timestamp("[10:30:45] DEBUG log message"));
    }

    #[test]
    fn test_contains_log_level() {
        let detector = LogFileDetector::default();
        assert!(detector.contains_log_level("ERROR: Something went wrong"));
        assert!(detector.contains_log_level("[WARN] Warning message"));
        assert!(detector.contains_log_level("INFO Starting service"));
    }

    #[test]
    fn test_detect_android_logcat_content() {
        let detector = LogFileDetector::default();
        let lines = vec![
            "12-07 10:30:45.123  1234  5678 D ActivityManager: Starting activity".to_string(),
            "12-07 10:30:45.124  1234  5678 I System: Boot completed".to_string(),
            "12-07 10:30:45.125  1234  5678 E Crash: Application crashed".to_string(),
        ];
        assert!(detector.is_android_logcat(&lines));
    }

    #[test]
    fn test_detect_android_tombstone_content() {
        let detector = LogFileDetector::default();
        let lines = vec![
            "*** *** *** *** *** *** *** *** *** *** *** *** *** *** *** ***".to_string(),
            "Build fingerprint: 'google/pixel5/redfin:12/SQ3A.220705.003.A1/8672226:user/release-keys'".to_string(),
            "signal 11 (SIGSEGV), code 1 (SEGV_MAPERR), fault addr 0x0".to_string(),
        ];
        assert!(detector.is_android_tombstone(&lines));
    }

    #[test]
    fn test_detect_generic_log() -> Result<(), Box<dyn std::error::Error>> {
        let mut temp_file = NamedTempFile::new()?;
        writeln!(temp_file, "2024-01-20 10:30:45 INFO Application started")?;
        writeln!(temp_file, "2024-01-20 10:30:46 DEBUG Loading configuration")?;
        writeln!(temp_file, "2024-01-20 10:30:47 ERROR Failed to connect")?;

        let detector = LogFileDetector::default();
        let result = detector.detect(temp_file.path())?;
        assert!(matches!(result, FileTypeResult::LogFile(_)));
        Ok(())
    }
}
