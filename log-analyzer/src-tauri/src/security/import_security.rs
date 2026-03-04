//! 导入安全检查模块
//!
//! 提供文件导入时的安全检查，防止：
//! - 超大文件导致内存耗尽
//! - 文件炸弹攻击（如 ZIP 炸弹）
//! - 超长行导致的内存问题
//! - 恶意构造的文件数量攻击

use std::path::Path;

use super::line_guard::{GuardedLine, LineGuard, MAX_LINE_LENGTH};

/// 默认最大文件大小 (500MB)
pub const DEFAULT_MAX_FILE_SIZE: u64 = 500 * 1024 * 1024;

/// 默认最大行数 (1亿行)
pub const DEFAULT_MAX_TOTAL_LINES: u64 = 100_000_000;

/// 默认最大目录深度
pub const DEFAULT_MAX_DEPTH: u32 = 20;

/// 安全检查结果
#[derive(Debug, Clone)]
pub struct SecurityCheckResult {
    /// 是否通过检查
    pub is_safe: bool,
    /// 警告信息（不影响导入，但需要用户知晓）
    pub warnings: Vec<String>,
    /// 错误信息（阻止导入）
    pub errors: Vec<String>,
    /// 文件统计信息
    pub stats: FileSecurityStats,
}

impl SecurityCheckResult {
    /// 创建一个通过的安全检查结果
    pub fn safe() -> Self {
        Self {
            is_safe: true,
            warnings: Vec::new(),
            errors: Vec::new(),
            stats: FileSecurityStats::default(),
        }
    }

    /// 添加警告
    pub fn with_warning(mut self, warning: impl Into<String>) -> Self {
        self.warnings.push(warning.into());
        self
    }

    /// 添加错误（标记为不安全）
    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.errors.push(error.into());
        self.is_safe = false;
        self
    }

    /// 合并另一个检查结果
    pub fn merge(&mut self, other: SecurityCheckResult) {
        if !other.is_safe {
            self.is_safe = false;
        }
        self.warnings.extend(other.warnings);
        self.errors.extend(other.errors);
        self.stats.merge(other.stats);
    }
}

/// 文件安全统计信息
#[derive(Debug, Clone, Default)]
pub struct FileSecurityStats {
    /// 检查的文件数
    pub files_checked: u64,
    /// 检查的总字节数
    pub total_bytes: u64,
    /// 被截断的行数
    pub lines_truncated: u64,
    /// 最大单行长度（原始）
    pub max_line_length: usize,
    /// 跳过的文件数（超过大小限制）
    pub files_skipped: u64,
}

impl FileSecurityStats {
    /// 合并统计信息
    pub fn merge(&mut self, other: FileSecurityStats) {
        self.files_checked += other.files_checked;
        self.total_bytes += other.total_bytes;
        self.lines_truncated += other.lines_truncated;
        self.max_line_length = self.max_line_length.max(other.max_line_length);
        self.files_skipped += other.files_skipped;
    }
}

/// 导入安全配置
#[derive(Debug, Clone)]
pub struct ImportSecurityConfig {
    /// 最大文件大小（字节）
    pub max_file_size: u64,
    /// 单行最大长度（字节）
    pub max_line_length: usize,
    /// 最大总行数
    pub max_total_lines: u64,
    /// 最大目录遍历深度
    pub max_depth: u32,
    /// 是否启用行截断
    pub enable_line_truncation: bool,
    /// 是否跳过超大文件（而非报错）
    pub skip_large_files: bool,
}

impl Default for ImportSecurityConfig {
    fn default() -> Self {
        Self {
            max_file_size: DEFAULT_MAX_FILE_SIZE,
            max_line_length: MAX_LINE_LENGTH,
            max_total_lines: DEFAULT_MAX_TOTAL_LINES,
            max_depth: DEFAULT_MAX_DEPTH,
            enable_line_truncation: true,
            skip_large_files: false,
        }
    }
}

impl ImportSecurityConfig {
    /// 创建新的安全配置
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置最大文件大小
    pub fn with_max_file_size(mut self, size: u64) -> Self {
        self.max_file_size = size;
        self
    }

    /// 设置单行最大长度
    pub fn with_max_line_length(mut self, length: usize) -> Self {
        self.max_line_length = length;
        self
    }

    /// 设置最大总行数
    pub fn with_max_total_lines(mut self, lines: u64) -> Self {
        self.max_total_lines = lines;
        self
    }

    /// 启用/禁用超大文件跳过
    pub fn with_skip_large_files(mut self, skip: bool) -> Self {
        self.skip_large_files = skip;
        self
    }
}

/// 导入安全检查器
///
/// 在文件导入时执行安全检查，防止恶意文件攻击。
///
/// # Example
///
/// ```rust
/// use log_analyzer::security::{ImportSecurity, ImportSecurityConfig};
/// use std::path::Path;
///
/// let config = ImportSecurityConfig::default();
/// let security = ImportSecurity::new(config);
///
/// // 检查单个文件
/// let result = security.check_file(Path::new("example.log"));
/// if result.is_safe {
///     println!("File is safe to import");
/// }
/// ```
#[derive(Debug)]
pub struct ImportSecurity {
    config: ImportSecurityConfig,
    line_guard: LineGuard,
}

impl Default for ImportSecurity {
    fn default() -> Self {
        Self::new(ImportSecurityConfig::default())
    }
}

impl ImportSecurity {
    /// 创建新的导入安全检查器
    pub fn new(config: ImportSecurityConfig) -> Self {
        let line_guard = LineGuard::new(config.max_line_length);
        Self { config, line_guard }
    }

    /// 使用默认配置创建检查器
    pub fn with_defaults() -> Self {
        Self::default()
    }

    /// 获取行防护器引用
    pub fn line_guard(&self) -> &LineGuard {
        &self.line_guard
    }

    /// 获取配置引用
    pub fn config(&self) -> &ImportSecurityConfig {
        &self.config
    }

    /// 检查单个文件的安全性
    ///
    /// # Arguments
    /// * `path` - 文件路径
    ///
    /// # Returns
    /// 返回安全检查结果
    pub fn check_file(&self, path: &Path) -> SecurityCheckResult {
        let mut result = SecurityCheckResult::safe();

        // 检查文件是否存在
        if !path.exists() {
            return result.with_error(format!("File does not exist: {}", path.display()));
        }

        // 检查文件大小
        match std::fs::metadata(path) {
            Ok(metadata) => {
                let file_size = metadata.len();
                result.stats.total_bytes = file_size;
                result.stats.files_checked = 1;

                if file_size > self.config.max_file_size {
                    let size_mb = file_size as f64 / (1024.0 * 1024.0);
                    let max_mb = self.config.max_file_size as f64 / (1024.0 * 1024.0);

                    if self.config.skip_large_files {
                        result.stats.files_skipped = 1;
                        result = result.with_warning(format!(
                            "File too large ({}), skipping: {}",
                            format_size(file_size),
                            path.display()
                        ));
                    } else {
                        result = result.with_error(format!(
                            "File exceeds size limit ({:.2}MB > {:.2}MB): {}",
                            size_mb,
                            max_mb,
                            path.display()
                        ));
                    }
                }
            }
            Err(e) => {
                result = result.with_error(format!(
                    "Cannot read file metadata: {} - {}",
                    path.display(),
                    e
                ));
            }
        }

        result
    }

    /// 检查并处理单行内容
    ///
    /// 此方法检查行的长度并在必要时进行截断。
    ///
    /// # Arguments
    /// * `line` - 原始行内容
    ///
    /// # Returns
    /// 返回处理后的 `GuardedLine`
    pub fn guard_line(&self, line: &str) -> GuardedLine {
        let guarded = self.line_guard.guard_line(line);

        // 更新统计信息
        if guarded.was_truncated {
            tracing::debug!(
                original_length = guarded.original_length,
                truncated_length = guarded.content.len(),
                "Line truncated for security"
            );
        }

        guarded
    }

    /// 检查目录的安全性
    ///
    /// # Arguments
    /// * `path` - 目录路径
    /// * `recursive` - 是否递归检查
    ///
    /// # Returns
    /// 返回安全检查结果
    pub fn check_directory(&self, path: &Path, recursive: bool) -> SecurityCheckResult {
        let mut result = SecurityCheckResult::safe();

        if !path.exists() {
            return result.with_error(format!("Directory does not exist: {}", path.display()));
        }

        if !path.is_dir() {
            return result.with_error(format!("Path is not a directory: {}", path.display()));
        }

        // 检查目录内容
        match std::fs::read_dir(path) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    let entry_path = entry.path();

                    if entry_path.is_file() {
                        let file_result = self.check_file(&entry_path);
                        result.merge(file_result);

                        // 检查累计大小限制
                        if result.stats.total_bytes > self.config.max_file_size * 10 {
                            result = result.with_warning(
                                "Total directory size is very large, consider importing in batches",
                            );
                        }
                    } else if recursive && entry_path.is_dir() {
                        // 递归检查子目录（深度限制）
                        if self.config.max_depth > 0 {
                            let subdir_result = self.check_directory(&entry_path, true);
                            result.merge(subdir_result);
                        }
                    }
                }
            }
            Err(e) => {
                result = result.with_warning(format!(
                    "Cannot read directory: {} - {}",
                    path.display(),
                    e
                ));
            }
        }

        result
    }

    /// 创建安全读取器
    ///
    /// 返回一个实现了 `Iterator` 的读取器，自动进行行截断。
    ///
    /// # Arguments
    /// * `reader` - 实现 `Read` trait 的读取器
    ///
    /// # Returns
    /// 返回一个迭代器，每次迭代返回一个 `GuardedLine`
    pub fn create_safe_reader<'a, R: std::io::Read + 'a>(
        &'a self,
        reader: R,
    ) -> impl Iterator<Item = GuardedLine> + 'a {
        self.line_guard.process_stream(reader)
    }
}

/// 格式化字节大小
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_safe_result() {
        let result = SecurityCheckResult::safe();
        assert!(result.is_safe);
        assert!(result.warnings.is_empty());
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_result_with_warning() {
        let result = SecurityCheckResult::safe().with_warning("This is a warning");

        assert!(result.is_safe);
        assert_eq!(result.warnings.len(), 1);
    }

    #[test]
    fn test_result_with_error() {
        let result = SecurityCheckResult::safe().with_error("This is an error");

        assert!(!result.is_safe);
        assert_eq!(result.errors.len(), 1);
    }

    #[test]
    fn test_check_nonexistent_file() {
        let security = ImportSecurity::default();
        let result = security.check_file(Path::new("/nonexistent/file.log"));

        assert!(!result.is_safe);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_check_normal_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.log");

        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "Line 1").unwrap();
        writeln!(file, "Line 2").unwrap();

        let security = ImportSecurity::default();
        let result = security.check_file(&file_path);

        assert!(result.is_safe);
        assert_eq!(result.stats.files_checked, 1);
    }

    #[test]
    fn test_check_large_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("large.log");

        // 创建一个配置，设置很小的文件大小限制
        let config = ImportSecurityConfig::new()
            .with_max_file_size(100) // 100 字节限制
            .with_skip_large_files(false);

        let security = ImportSecurity::new(config);

        // 创建一个超过限制的文件
        let mut file = File::create(&file_path).unwrap();
        write!(file, "{}", "x".repeat(200)).unwrap();

        let result = security.check_file(&file_path);

        assert!(!result.is_safe);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_skip_large_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("large.log");

        // 配置为跳过大文件
        let config = ImportSecurityConfig::new()
            .with_max_file_size(100)
            .with_skip_large_files(true);

        let security = ImportSecurity::new(config);

        let mut file = File::create(&file_path).unwrap();
        write!(file, "{}", "x".repeat(200)).unwrap();

        let result = security.check_file(&file_path);

        // 应该通过（但带警告）
        assert!(result.is_safe);
        assert!(!result.warnings.is_empty());
        assert_eq!(result.stats.files_skipped, 1);
    }

    #[test]
    fn test_guard_line_normal() {
        let security = ImportSecurity::default();
        let result = security.guard_line("Normal line");

        assert!(!result.was_truncated);
        assert_eq!(result.content, "Normal line");
    }

    #[test]
    fn test_guard_line_long() {
        let config = ImportSecurityConfig::new().with_max_line_length(100);
        let security = ImportSecurity::new(config);

        let long_line = "x".repeat(200);
        let result = security.guard_line(&long_line);

        assert!(result.was_truncated);
        assert!(result.content.len() <= 100);
    }

    #[test]
    fn test_check_directory() {
        let temp_dir = TempDir::new().unwrap();

        // 创建一些文件
        File::create(temp_dir.path().join("file1.log")).unwrap();
        File::create(temp_dir.path().join("file2.log")).unwrap();

        let security = ImportSecurity::default();
        let result = security.check_directory(temp_dir.path(), false);

        assert!(result.is_safe);
        assert_eq!(result.stats.files_checked, 2);
    }

    #[test]
    fn test_check_nonexistent_directory() {
        let security = ImportSecurity::default();
        let result = security.check_directory(Path::new("/nonexistent/dir"), false);

        assert!(!result.is_safe);
    }

    #[test]
    fn test_create_safe_reader() {
        let security = ImportSecurity::default();
        let data = b"line1\nline2\nline3";
        let cursor = std::io::Cursor::new(data);

        let lines: Vec<_> = security.create_safe_reader(cursor).collect();

        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].content, "line1");
        assert_eq!(lines[1].content, "line2");
        assert_eq!(lines[2].content, "line3");
    }

    #[test]
    fn test_stats_merge() {
        let mut stats1 = FileSecurityStats {
            files_checked: 10,
            total_bytes: 1000,
            lines_truncated: 5,
            max_line_length: 100,
            files_skipped: 1,
        };

        let stats2 = FileSecurityStats {
            files_checked: 5,
            total_bytes: 500,
            lines_truncated: 3,
            max_line_length: 150,
            files_skipped: 2,
        };

        stats1.merge(stats2);

        assert_eq!(stats1.files_checked, 15);
        assert_eq!(stats1.total_bytes, 1500);
        assert_eq!(stats1.lines_truncated, 8);
        assert_eq!(stats1.max_line_length, 150);
        assert_eq!(stats1.files_skipped, 3);
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(500), "500 B");
        assert_eq!(format_size(1024), "1.00 KB");
        assert_eq!(format_size(1024 * 1024), "1.00 MB");
        assert_eq!(format_size(1024 * 1024 * 1024), "1.00 GB");
    }
}
