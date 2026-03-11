//! ArchiveHandler 基类 trait 模块
//!
//! 提供可复用的默认实现，包括路径验证、安全检查、进度追踪和限制检查。
//! 所有具体的归档处理器都应实现 ArchiveHandlerBase trait 以获得这些功能。

use async_trait::async_trait;
use std::path::{Path, PathBuf};
use tracing::{debug, trace, warn};

use crate::archive::archive_handler::{ArchiveHandler, ExtractionSummary};
use crate::archive::extraction_config::{ExtractionConfig, SecurityConfig};
use crate::archive::extraction_error::{ExtractionError, ExtractionResult};

/// 提取统计信息
///
/// 跟踪提取过程中的文件数量、总大小和提取的文件路径
#[derive(Debug, Clone)]
pub struct ExtractionStats {
    /// 提取的文件总数
    pub total_files: usize,
    /// 提取的总字节数
    pub total_bytes: u64,
    /// 提取的文件路径列表
    pub extracted_files: Vec<PathBuf>,
    /// 错误信息列表
    pub errors: Vec<String>,
}

impl Default for ExtractionStats {
    fn default() -> Self {
        Self::new()
    }
}

impl ExtractionStats {
    /// 创建新的统计信息实例
    pub fn new() -> Self {
        Self {
            total_files: 0,
            total_bytes: 0,
            extracted_files: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// 记录成功提取的文件
    pub fn record_file(&mut self, path: PathBuf, size: u64) {
        self.total_files += 1;
        self.total_bytes += size;
        self.extracted_files.push(path);
    }

    /// 记录错误信息
    pub fn record_error(&mut self, error: impl Into<String>) {
        self.errors.push(error.into());
    }

    /// 检查是否有错误
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// 获取成功率（0-100）
    pub fn success_rate(&self) -> f32 {
        let total = self.total_files + self.errors.len();
        if total == 0 {
            return 100.0;
        }
        (self.total_files as f32 / total as f32) * 100.0
    }
}

/// 提取上下文
///
/// 管理提取过程中的状态，包括配置、统计信息、深度追踪等
#[derive(Debug, Clone)]
pub struct ExtractionContext {
    /// 提取配置
    pub config: ExtractionConfig,
    /// 提取统计信息
    pub stats: ExtractionStats,
    /// 当前嵌套深度
    pub depth: u32,
    /// 父路径（用于嵌套归档）
    pub parent_path: Option<PathBuf>,
}

impl ExtractionContext {
    /// 创建新的提取上下文
    ///
    /// # Arguments
    ///
    /// * `config` - 提取配置
    pub fn new(config: ExtractionConfig) -> Self {
        Self {
            config,
            stats: ExtractionStats::new(),
            depth: 0,
            parent_path: None,
        }
    }

    /// 记录提取的文件
    ///
    /// # Arguments
    ///
    /// * `path` - 文件路径
    /// * `size` - 文件大小
    pub fn record_extraction(&mut self, path: &Path, size: u64) {
        self.stats.record_file(path.to_path_buf(), size);
        trace!(
            "记录提取: path={:?}, size={}, depth={}",
            path,
            size,
            self.depth
        );
    }

    /// 记录错误
    ///
    /// # Arguments
    ///
    /// * `error` - 错误信息
    pub fn record_error(&mut self, error: impl Into<String>) {
        let error_msg = error.into();
        warn!("提取错误 (depth={}): {}", self.depth, error_msg);
        self.stats.record_error(error_msg);
    }

    /// 创建子上下文（用于嵌套归档）
    ///
    /// 递增深度并设置父路径
    pub fn child_context(&self) -> Self {
        Self {
            config: self.config.clone(),
            stats: ExtractionStats::new(),
            depth: self.depth + 1,
            parent_path: self.parent_path.clone(),
        }
    }

    /// 检查是否超出深度限制
    pub fn is_depth_exceeded(&self) -> bool {
        self.depth > self.config.limits.max_depth
    }

    /// 转换为提取摘要
    pub fn into_summary(self) -> ExtractionSummary {
        let mut summary = ExtractionSummary::new();
        summary.files_extracted = self.stats.total_files;
        summary.total_size = self.stats.total_bytes;
        summary.errors = self.stats.errors;
        summary.extracted_files = self.stats.extracted_files;
        summary
    }
}

/// ArchiveHandler 基类 trait
///
/// 提供可复用的默认实现，包括：
/// - 路径验证
/// - 安全检查
/// - 进度追踪
/// - 限制检查
///
/// 所有具体的归档处理器都应实现此 trait
#[async_trait]
pub trait ArchiveHandlerBase: ArchiveHandler + Send + Sync {
    /// 处理器名称
    ///
    /// 用于日志和错误信息
    fn handler_name(&self) -> &'static str;

    /// 支持的格式列表
    ///
    /// 返回此处理器支持的所有文件扩展名
    fn supported_formats(&self) -> &[&'static str];

    /// 验证路径安全性
    ///
    /// 检查路径是否包含：
    /// - 符号链接（如果未启用）
    /// - 绝对路径（如果未启用）
    /// - 父目录遍历 (..)（如果未启用）
    /// - 黑名单路径
    ///
    /// # Arguments
    ///
    /// * `path` - 要验证的路径
    /// * `config` - 安全配置
    ///
    /// # Returns
    ///
    /// * `Ok(())` - 路径安全
    /// * `Err(ExtractionError)` - 路径存在安全问题
    fn validate_path(&self, path: &Path, config: &SecurityConfig) -> ExtractionResult<()> {
        let path_str = path.to_string_lossy();

        trace!("验证路径安全性: {:?}", path);

        // 检查符号链接
        #[cfg(unix)]
        if !config.allow_symlinks {
            if let Ok(metadata) = std::fs::symlink_metadata(path) {
                if metadata.file_type().is_symlink() {
                    return Err(ExtractionError::SymlinkNotAllowed {
                        path: path.to_path_buf(),
                    });
                }
            }
        }

        // 检查绝对路径
        if !config.allow_absolute_paths && path.is_absolute() {
            return Err(ExtractionError::AbsolutePathNotAllowed {
                path: path_str.to_string(),
            });
        }

        // 检查父目录遍历
        if !config.allow_parent_traversal {
            let components: Vec<_> = path.components().collect();
            let mut depth = 0i32;
            for component in &components {
                match component {
                    std::path::Component::ParentDir => {
                        depth -= 1;
                        if depth < 0 {
                            return Err(ExtractionError::ParentTraversalNotAllowed {
                                path: path_str.to_string(),
                            });
                        }
                    }
                    std::path::Component::Normal(_) => depth += 1,
                    _ => {}
                }
            }
        }

        // 检查黑名单
        for blacklisted in &config.path_blacklist {
            if path_str.contains(blacklisted) {
                return Err(ExtractionError::PathBlacklisted {
                    path: path_str.to_string(),
                });
            }
        }

        debug!("路径验证通过: {:?}", path);
        Ok(())
    }

    /// 检查限制
    ///
    /// 检查是否超出以下限制：
    /// - 单个文件大小
    /// - 总大小
    /// - 文件数量
    /// - 嵌套深度
    ///
    /// # Arguments
    ///
    /// * `size` - 当前文件大小
    /// * `context` - 提取上下文
    ///
    /// # Returns
    ///
    /// * `Ok(())` - 未超出限制
    /// * `Err(ExtractionError)` - 超出某个限制
    fn check_limits(&self, size: u64, context: &ExtractionContext) -> ExtractionResult<()> {
        let limits = &context.config.limits;

        // 检查深度限制
        if context.is_depth_exceeded() {
            return Err(ExtractionError::depth_exceeded(
                context.depth,
                limits.max_depth,
            ));
        }

        // 检查单个文件大小
        if size > limits.max_file_size {
            return Err(ExtractionError::file_too_large(size, limits.max_file_size));
        }

        // 检查总大小
        let new_total = context.stats.total_bytes.saturating_add(size);
        if new_total > limits.max_total_size {
            return Err(ExtractionError::total_size_exceeded(
                new_total,
                limits.max_total_size,
            ));
        }

        // 检查文件数量
        let new_count = context.stats.total_files + 1;
        if new_count > limits.max_file_count {
            return Err(ExtractionError::file_count_exceeded(
                new_count,
                limits.max_file_count,
            ));
        }

        Ok(())
    }

    /// 安全检查
    ///
    /// 综合路径验证和扩展名检查
    ///
    /// # Arguments
    ///
    /// * `path` - 要检查的路径
    /// * `context` - 提取上下文
    ///
    /// # Returns
    ///
    /// * `Ok(())` - 检查通过
    /// * `Err(ExtractionError)` - 安全检查失败
    fn check_security(&self, path: &Path, context: &ExtractionContext) -> ExtractionResult<()> {
        // 路径验证
        self.validate_path(path, &context.config.security)?;

        // 检查文件扩展名
        if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            let security = &context.config.security;

            // 检查白名单
            if !security.allowed_extensions.is_empty()
                && !security.allowed_extensions.contains(&ext_str)
            {
                return Err(ExtractionError::ExtensionNotAllowed { extension: ext_str });
            }

            // 检查黑名单
            if security.forbidden_extensions.contains(&ext_str) {
                return Err(ExtractionError::ExtensionForbidden { extension: ext_str });
            }
        }

        Ok(())
    }

    /// 验证并记录提取
    ///
    /// 组合安全检查、限制检查和记录功能
    ///
    /// # Arguments
    ///
    /// * `path` - 文件路径
    /// * `size` - 文件大小
    /// * `context` - 提取上下文
    ///
    /// # Returns
    ///
    /// * `Ok(())` - 验证通过并已记录
    /// * `Err(ExtractionError)` - 验证失败
    fn validate_and_record(
        &self,
        path: &Path,
        size: u64,
        context: &mut ExtractionContext,
    ) -> ExtractionResult<()> {
        // 安全检查
        if let Err(e) = self.check_security(path, context) {
            context.record_error(format!("安全验证失败: {}", e));
            return Err(e);
        }

        // 限制检查
        if let Err(e) = self.check_limits(size, context) {
            context.record_error(format!("超出限制: {}", e));
            return Err(e);
        }

        // 记录提取
        context.record_extraction(path, size);

        Ok(())
    }

    /// 进度追踪
    ///
    /// 返回当前提取进度信息
    fn track_progress(&self, context: &ExtractionContext) -> ExtractionProgress {
        ExtractionProgress {
            files_extracted: context.stats.total_files,
            total_bytes: context.stats.total_bytes,
            current_depth: context.depth,
            errors_count: context.stats.errors.len(),
            success_rate: context.stats.success_rate(),
        }
    }

    /// 使用上下文提取
    ///
    /// 新的推荐 API，使用 ExtractionContext 进行提取
    ///
    /// # Arguments
    ///
    /// * `source` - 源归档路径
    /// * `target_dir` - 目标目录
    /// * `context` - 提取上下文
    ///
    /// # Returns
    ///
    /// * `Ok(ExtractionSummary)` - 提取成功
    /// * `Err(ExtractionError)` - 提取失败
    async fn extract_with_context(
        &self,
        source: &Path,
        target_dir: &Path,
        context: &mut ExtractionContext,
    ) -> ExtractionResult<ExtractionSummary>;

    /// 使用限制参数提取（默认实现）
    ///
    /// 此方法提供默认实现，将限制参数转换为 ExtractionContext 并调用 extract_with_context。
    /// 具体的处理器只需实现 extract_with_context 即可。
    ///
    /// # Arguments
    ///
    /// * `source` - 源归档路径
    /// * `target_dir` - 目标目录
    /// * `max_file_size` - 单文件最大大小
    /// * `max_total_size` - 总大小限制
    /// * `max_file_count` - 文件数量限制
    ///
    /// # Returns
    ///
    /// * `Ok(ExtractionSummary)` - 提取成功
    /// * `Err(AppError)` - 提取失败
    #[allow(deprecated)]
    async fn extract_with_limits_default(
        &self,
        source: &Path,
        target_dir: &Path,
        max_file_size: u64,
        max_total_size: u64,
        max_file_count: usize,
    ) -> crate::error::Result<ExtractionSummary> {
        let config = ExtractionConfig {
            limits: crate::archive::extraction_config::ExtractionLimits {
                max_file_size,
                max_total_size,
                max_file_count,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut context = ExtractionContext::new(config);

        match self
            .extract_with_context(source, target_dir, &mut context)
            .await
        {
            Ok(summary) => Ok(summary),
            Err(e) => Err(crate::error::AppError::archive_error(
                e.to_string(),
                Some(source.to_path_buf()),
            )),
        }
    }
}

/// 提取进度信息
#[derive(Debug, Clone)]
pub struct ExtractionProgress {
    /// 已提取文件数
    pub files_extracted: usize,
    /// 已提取总字节数
    pub total_bytes: u64,
    /// 当前深度
    pub current_depth: u32,
    /// 错误数量
    pub errors_count: usize,
    /// 成功率（0-100）
    pub success_rate: f32,
}

impl ExtractionProgress {
    /// 获取格式化的进度字符串
    pub fn format(&self) -> String {
        format!(
            "已提取 {} 个文件 ({} bytes), 深度: {}, 错误: {}, 成功率: {:.1}%",
            self.files_extracted,
            self.total_bytes,
            self.current_depth,
            self.errors_count,
            self.success_rate
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// 测试用的处理器实现
    struct TestHandler;

    #[async_trait]
    impl ArchiveHandler for TestHandler {
        fn can_handle(&self, _path: &Path) -> bool {
            true
        }

        async fn extract_with_limits(
            &self,
            _source: &Path,
            _target_dir: &Path,
            _max_file_size: u64,
            _max_total_size: u64,
            _max_file_count: usize,
        ) -> crate::error::Result<ExtractionSummary> {
            Ok(ExtractionSummary::new())
        }

        fn file_extensions(&self) -> Vec<&str> {
            vec!["test"]
        }

        async fn list_contents(
            &self,
            _path: &Path,
        ) -> crate::error::Result<Vec<crate::archive::ArchiveEntry>> {
            Ok(vec![])
        }

        async fn read_file(&self, _path: &Path, _file_name: &str) -> crate::error::Result<String> {
            Ok(String::new())
        }
    }

    #[async_trait]
    impl ArchiveHandlerBase for TestHandler {
        fn handler_name(&self) -> &'static str {
            "TestHandler"
        }

        fn supported_formats(&self) -> &[&'static str] {
            &["test", "tst"]
        }

        async fn extract_with_context(
            &self,
            _source: &Path,
            _target_dir: &Path,
            _context: &mut ExtractionContext,
        ) -> ExtractionResult<ExtractionSummary> {
            Ok(ExtractionSummary::new())
        }
    }

    #[test]
    fn test_extraction_stats_new() {
        let stats = ExtractionStats::new();
        assert_eq!(stats.total_files, 0);
        assert_eq!(stats.total_bytes, 0);
        assert!(stats.extracted_files.is_empty());
        assert!(stats.errors.is_empty());
    }

    #[test]
    fn test_extraction_stats_record_file() {
        let mut stats = ExtractionStats::new();
        stats.record_file(PathBuf::from("test.txt"), 100);

        assert_eq!(stats.total_files, 1);
        assert_eq!(stats.total_bytes, 100);
        assert_eq!(stats.extracted_files.len(), 1);
    }

    #[test]
    fn test_extraction_stats_record_error() {
        let mut stats = ExtractionStats::new();
        stats.record_error("test error");

        assert!(stats.has_errors());
        assert_eq!(stats.errors.len(), 1);
    }

    #[test]
    fn test_extraction_stats_success_rate() {
        let mut stats = ExtractionStats::new();
        assert_eq!(stats.success_rate(), 100.0);

        stats.record_file(PathBuf::from("test1.txt"), 100);
        stats.record_file(PathBuf::from("test2.txt"), 200);
        stats.record_error("error");

        assert!((stats.success_rate() - 66.67).abs() < 0.1);
    }

    #[test]
    fn test_extraction_context_new() {
        let config = ExtractionConfig::default();
        let ctx = ExtractionContext::new(config);

        assert_eq!(ctx.depth, 0);
        assert!(ctx.parent_path.is_none());
        assert_eq!(ctx.stats.total_files, 0);
    }

    #[test]
    fn test_extraction_context_child() {
        let config = ExtractionConfig::default();
        let parent = ExtractionContext::new(config);
        let child = parent.child_context();

        assert_eq!(child.depth, 1);
        assert_eq!(
            child.config.limits.max_file_size,
            parent.config.limits.max_file_size
        );
    }

    #[test]
    fn test_extraction_context_is_depth_exceeded() {
        let mut config = ExtractionConfig::default();
        config.limits.max_depth = 3;

        let mut ctx = ExtractionContext::new(config);
        assert!(!ctx.is_depth_exceeded());

        ctx.depth = 3;
        assert!(!ctx.is_depth_exceeded());

        ctx.depth = 4;
        assert!(ctx.is_depth_exceeded());
    }

    #[test]
    fn test_archive_handler_base_handler_name() {
        let handler = TestHandler;
        assert_eq!(handler.handler_name(), "TestHandler");
    }

    #[test]
    fn test_archive_handler_base_supported_formats() {
        let handler = TestHandler;
        let formats = handler.supported_formats();
        assert_eq!(formats, &["test", "tst"]);
    }

    #[test]
    fn test_validate_path_with_traversal() {
        let handler = TestHandler;
        let config = SecurityConfig::default();

        // 正常的相对路径应该通过
        let result = handler.validate_path(Path::new("normal/path/file.txt"), &config);
        assert!(result.is_ok());

        // 包含 .. 的路径应该被拒绝
        let result = handler.validate_path(Path::new("../etc/passwd"), &config);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ExtractionError::ParentTraversalNotAllowed { .. }
        ));
    }

    #[test]
    fn test_validate_path_with_absolute() {
        let handler = TestHandler;
        let config = SecurityConfig::default();

        // Unix 绝对路径
        #[cfg(unix)]
        {
            let result = handler.validate_path(Path::new("/absolute/path"), &config);
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                ExtractionError::AbsolutePathNotAllowed { .. }
            ));
        }
    }

    #[test]
    fn test_validate_path_with_blacklist() {
        let handler = TestHandler;
        let config = SecurityConfig::default();

        let result = handler.validate_path(Path::new("some/path/Windows/system32"), &config);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ExtractionError::PathBlacklisted { .. }
        ));
    }

    #[test]
    fn test_check_limits_file_size() {
        let handler = TestHandler;
        let mut config = ExtractionConfig::default();
        config.limits.max_file_size = 100;

        let context = ExtractionContext::new(config);

        // 正常大小应该通过
        let result = handler.check_limits(50, &context);
        assert!(result.is_ok());

        // 超过限制应该失败
        let result = handler.check_limits(150, &context);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ExtractionError::FileTooLarge { .. }
        ));
    }

    #[test]
    fn test_check_limits_total_size() {
        let handler = TestHandler;
        let mut config = ExtractionConfig::default();
        config.limits.max_total_size = 100;

        let mut context = ExtractionContext::new(config);
        context.stats.total_bytes = 80;

        // 超过总大小限制应该失败
        let result = handler.check_limits(30, &context);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ExtractionError::TotalSizeExceeded { .. }
        ));
    }

    #[test]
    fn test_check_limits_file_count() {
        let handler = TestHandler;
        let mut config = ExtractionConfig::default();
        config.limits.max_file_count = 2;

        let mut context = ExtractionContext::new(config);
        context.stats.total_files = 2;

        // 超过文件数量限制应该失败
        let result = handler.check_limits(10, &context);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ExtractionError::FileCountExceeded { .. }
        ));
    }

    #[test]
    fn test_check_limits_depth() {
        let handler = TestHandler;
        let mut config = ExtractionConfig::default();
        config.limits.max_depth = 2;

        let mut context = ExtractionContext::new(config);
        context.depth = 3;

        // 超过深度限制应该失败
        let result = handler.check_limits(10, &context);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ExtractionError::DepthExceeded { .. }
        ));
    }

    #[test]
    fn test_check_security_extension_whitelist() {
        let handler = TestHandler;
        let mut config = ExtractionConfig::default();
        config.security.allowed_extensions = vec!["txt".to_string(), "log".to_string()];

        let context = ExtractionContext::new(config);

        // 允许的扩展名
        let result = handler.check_security(Path::new("test.txt"), &context);
        assert!(result.is_ok());

        // 不允许的扩展名
        let result = handler.check_security(Path::new("test.exe"), &context);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ExtractionError::ExtensionNotAllowed { .. }
        ));
    }

    #[test]
    fn test_check_security_extension_blacklist() {
        let handler = TestHandler;
        let mut config = ExtractionConfig::default();
        config.security.forbidden_extensions = vec!["exe".to_string()];

        let context = ExtractionContext::new(config);

        // 允许的扩展名
        let result = handler.check_security(Path::new("test.txt"), &context);
        assert!(result.is_ok());

        // 禁止的扩展名
        let result = handler.check_security(Path::new("test.exe"), &context);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ExtractionError::ExtensionForbidden { .. }
        ));
    }

    #[test]
    fn test_track_progress() {
        let handler = TestHandler;
        let mut context = ExtractionContext::new(ExtractionConfig::default());

        context.stats.total_files = 10;
        context.stats.total_bytes = 1024;
        context.depth = 2;
        context.stats.record_error("error1");
        context.stats.record_error("error2");

        let progress = handler.track_progress(&context);

        assert_eq!(progress.files_extracted, 10);
        assert_eq!(progress.total_bytes, 1024);
        assert_eq!(progress.current_depth, 2);
        assert_eq!(progress.errors_count, 2);
        assert!((progress.success_rate - 83.33).abs() < 0.1);
    }

    #[test]
    fn test_extraction_progress_format() {
        let progress = ExtractionProgress {
            files_extracted: 5,
            total_bytes: 10240,
            current_depth: 2,
            errors_count: 1,
            success_rate: 83.33,
        };

        let formatted = progress.format();
        assert!(formatted.contains("5 个文件"));
        assert!(formatted.contains("10240 bytes"));
        assert!(formatted.contains("深度: 2"));
        assert!(formatted.contains("错误: 1"));
        assert!(formatted.contains("成功率: 83.3%"));
    }

    #[test]
    fn test_validate_and_record_success() {
        let handler = TestHandler;
        let mut context = ExtractionContext::new(ExtractionConfig::default());

        let result = handler.validate_and_record(Path::new("test.txt"), 100, &mut context);

        assert!(result.is_ok());
        assert_eq!(context.stats.total_files, 1);
        assert_eq!(context.stats.total_bytes, 100);
    }

    #[test]
    fn test_validate_and_record_failure() {
        let handler = TestHandler;
        let mut config = ExtractionConfig::default();
        config.limits.max_file_size = 50;

        let mut context = ExtractionContext::new(config);

        let result = handler.validate_and_record(Path::new("test.txt"), 100, &mut context);

        assert!(result.is_err());
        assert!(!context.stats.errors.is_empty());
    }
}
