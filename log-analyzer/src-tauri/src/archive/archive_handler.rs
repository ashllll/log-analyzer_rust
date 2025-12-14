use crate::error::Result;
use async_trait::async_trait;
use std::path::{Path, PathBuf};

/**
 * 压缩文件处理器trait
 *
 * 定义统一的压缩文件处理接口
 */
#[async_trait]
pub trait ArchiveHandler: Send + Sync {
    /**
     * 检查是否能处理该文件
     *
     * # 参数
     * * `path` - 文件路径
     *
     * # 返回
     * * `true` - 能处理
     * * `false` - 不能处理
     */
    fn can_handle(&self, path: &Path) -> bool;

    /**
     * 提取压缩文件内容（带安全限制）
     *
     * # 参数
     * * `source` - 源文件路径
     * * `target_dir` - 目标目录
     * * `max_file_size` - 单个文件最大大小（字节）
     * * `max_total_size` - 解压后总大小限制（字节）
     * * `max_file_count` - 解压文件数量限制
     *
     * # 返回
     * * `Ok(ExtractionSummary)` - 提取摘要
     * * `Err(AppError)` - 提取失败
     */
    async fn extract_with_limits(
        &self,
        source: &Path,
        target_dir: &Path,
        max_file_size: u64,
        max_total_size: u64,
        max_file_count: usize,
    ) -> Result<ExtractionSummary>;

    /**
     * 提取压缩文件内容（兼容旧版本，默认调用带限制的方法）
     *
     * # 参数
     * * `source` - 源文件路径
     * * `target_dir` - 目标目录
     *
     * # 返回
     * * `Ok(ExtractionSummary)` - 提取摘要
     * * `Err(AppError)` - 提取失败
     */
    #[allow(dead_code)]
    async fn extract(&self, source: &Path, target_dir: &Path) -> Result<ExtractionSummary> {
        // 默认使用安全限制：单个文件100MB，总大小1GB，文件数1000
        self.extract_with_limits(
            source,
            target_dir,
            100 * 1024 * 1024,
            1024 * 1024 * 1024, // 1GB
            1000,
        )
        .await
    }

    /**
     * 获取支持的文件扩展名
     *
     * # 返回
     * * 扩展名列表
     */
    fn file_extensions(&self) -> Vec<&str>;
}

/**
 * 提取摘要
 */
#[derive(Debug, Clone)]
pub struct ExtractionSummary {
    /// 提取的文件数量
    pub files_extracted: usize,
    /// 提取的总大小（字节）
    pub total_size: u64,
    /// 错误信息列表
    pub errors: Vec<String>,
    /// 提取的文件路径列表
    pub extracted_files: Vec<PathBuf>,
}

impl ExtractionSummary {
    /**
     * 创建新的提取摘要
     */
    pub fn new() -> Self {
        Self {
            files_extracted: 0,
            total_size: 0,
            errors: Vec::new(),
            extracted_files: Vec::new(),
        }
    }

    /**
     * 添加成功提取的文件
     */
    pub fn add_file(&mut self, path: PathBuf, size: u64) {
        self.files_extracted += 1;
        self.total_size += size;
        self.extracted_files.push(path);
    }

    /**
     * 添加错误信息
     */
    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }

    /**
     * 检查是否有错误
     */
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /**
     * 获取成功率（0-100）
     */
    #[allow(dead_code)]
    pub fn success_rate(&self) -> f32 {
        if self.files_extracted + self.errors.len() == 0 {
            return 100.0;
        }

        let total = self.files_extracted + self.errors.len();
        (self.files_extracted as f32 / total as f32) * 100.0
    }
}

/**
 * 提取错误
 */
#[allow(dead_code)]
#[derive(Debug)]
pub struct ExtractionError {
    pub message: String,
    pub source: Option<std::io::Error>,
    pub path: Option<PathBuf>,
}

impl ExtractionError {
    /**
     * 创建新的提取错误
     */
    #[allow(dead_code)]
    pub fn new(message: String) -> Self {
        Self {
            message,
            source: None,
            path: None,
        }
    }

    /**
     * 添加源错误
     */
    #[allow(dead_code)]
    pub fn with_source(mut self, source: std::io::Error) -> Self {
        self.source = Some(source);
        self
    }

    /**
     * 添加路径信息
     */
    #[allow(dead_code)]
    pub fn with_path(mut self, path: PathBuf) -> Self {
        self.path = Some(path);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extraction_summary() {
        let mut summary = ExtractionSummary::new();

        assert_eq!(summary.files_extracted, 0);
        assert_eq!(summary.total_size, 0);
        assert!(!summary.has_errors());
        assert_eq!(summary.success_rate(), 100.0);

        // 添加文件
        summary.add_file(PathBuf::from("test1.txt"), 100);
        summary.add_file(PathBuf::from("test2.txt"), 200);

        assert_eq!(summary.files_extracted, 2);
        assert_eq!(summary.total_size, 300);

        // 添加错误
        summary.add_error("Failed to extract test3.txt".to_string());

        assert!(summary.has_errors());
        let rate = summary.success_rate();
        assert!((rate - 66.67).abs() < 0.01); // 使用浮点数精度比较
    }

    #[test]
    fn test_extraction_error() {
        let error = ExtractionError::new("Extract failed".to_string())
            .with_source(std::io::Error::other("IO error"))
            .with_path(PathBuf::from("test.zip"));

        assert_eq!(error.message, "Extract failed");
        assert!(error.source.is_some());
        assert!(error.path.is_some());
    }
}
