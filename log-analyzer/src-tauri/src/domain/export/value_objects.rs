//! 导出领域值对象
//!
//! 定义导出相关的不可变值对象

use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;
use thiserror::Error;

/// 导出格式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ExportFormat {
    /// JSON 格式
    #[default]
    Json,
    /// CSV 格式
    Csv,
    /// HTML 格式
    Html,
    /// Markdown 格式
    Markdown,
    /// 纯文本格式
    Text,
}

impl ExportFormat {
    /// 获取文件扩展名
    pub fn extension(&self) -> &'static str {
        match self {
            ExportFormat::Json => "json",
            ExportFormat::Csv => "csv",
            ExportFormat::Html => "html",
            ExportFormat::Markdown => "md",
            ExportFormat::Text => "txt",
        }
    }

    /// 获取 MIME 类型
    pub fn mime_type(&self) -> &'static str {
        match self {
            ExportFormat::Json => "application/json",
            ExportFormat::Csv => "text/csv",
            ExportFormat::Html => "text/html",
            ExportFormat::Markdown => "text/markdown",
            ExportFormat::Text => "text/plain",
        }
    }

    /// 是否支持结构化数据
    pub fn supports_structured_data(&self) -> bool {
        matches!(self, ExportFormat::Json | ExportFormat::Csv)
    }

    /// 是否支持富文本
    pub fn supports_rich_text(&self) -> bool {
        matches!(self, ExportFormat::Html | ExportFormat::Markdown)
    }

    /// 获取所有可用格式
    pub fn all() -> &'static [ExportFormat] {
        &[
            ExportFormat::Json,
            ExportFormat::Csv,
            ExportFormat::Html,
            ExportFormat::Markdown,
            ExportFormat::Text,
        ]
    }
}

impl fmt::Display for ExportFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExportFormat::Json => write!(f, "JSON"),
            ExportFormat::Csv => write!(f, "CSV"),
            ExportFormat::Html => write!(f, "HTML"),
            ExportFormat::Markdown => write!(f, "Markdown"),
            ExportFormat::Text => write!(f, "Text"),
        }
    }
}

impl FromStr for ExportFormat {
    type Err = ExportFormatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "json" => Ok(ExportFormat::Json),
            "csv" => Ok(ExportFormat::Csv),
            "html" | "htm" => Ok(ExportFormat::Html),
            "md" | "markdown" => Ok(ExportFormat::Markdown),
            "txt" | "text" => Ok(ExportFormat::Text),
            _ => Err(ExportFormatError::InvalidFormat(s.to_string())),
        }
    }
}

/// 导出格式错误
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum ExportFormatError {
    #[error("无效的导出格式: {0}")]
    InvalidFormat(String),
}

/// 导出选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportOptions {
    /// 导出格式
    pub format: ExportFormat,
    /// 输出路径
    pub output_path: Option<PathBuf>,
    /// 是否包含高亮
    pub include_highlights: bool,
    /// 是否包含元数据
    pub include_metadata: bool,
    /// 是否压缩输出
    pub compress: bool,
    /// 最大结果数
    pub max_results: Option<usize>,
    /// 编码
    pub encoding: String,
    /// CSV 分隔符
    pub csv_delimiter: char,
    /// 日期格式
    pub date_format: String,
}

impl ExportOptions {
    /// 创建默认选项
    pub fn new(format: ExportFormat) -> Self {
        Self {
            format,
            output_path: None,
            include_highlights: true,
            include_metadata: true,
            compress: false,
            max_results: None,
            encoding: "utf-8".to_string(),
            csv_delimiter: ',',
            date_format: "%Y-%m-%d %H:%M:%S".to_string(),
        }
    }

    /// JSON 格式选项
    pub fn json() -> Self {
        Self::new(ExportFormat::Json)
    }

    /// CSV 格式选项
    pub fn csv() -> Self {
        Self::new(ExportFormat::Csv)
    }

    /// HTML 格式选项
    pub fn html() -> Self {
        Self::new(ExportFormat::Html)
    }

    /// 设置输出路径
    pub fn with_output_path(mut self, path: PathBuf) -> Self {
        self.output_path = Some(path);
        self
    }

    /// 设置是否包含高亮
    pub fn with_highlights(mut self, include: bool) -> Self {
        self.include_highlights = include;
        self
    }

    /// 设置是否包含元数据
    pub fn with_metadata(mut self, include: bool) -> Self {
        self.include_metadata = include;
        self
    }

    /// 设置是否压缩
    pub fn with_compression(mut self, compress: bool) -> Self {
        self.compress = compress;
        self
    }

    /// 设置最大结果数
    pub fn with_max_results(mut self, max: usize) -> Self {
        self.max_results = Some(max);
        self
    }

    /// 设置 CSV 分隔符
    pub fn with_csv_delimiter(mut self, delimiter: char) -> Self {
        self.csv_delimiter = delimiter;
        self
    }

    /// 生成默认文件名
    pub fn generate_filename(&self, base_name: &str) -> String {
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let extension = if self.compress {
            format!("{}.gz", self.format.extension())
        } else {
            self.format.extension().to_string()
        };
        format!("{}_{}.{}", base_name, timestamp, extension)
    }
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self::new(ExportFormat::default())
    }
}

// ==================== 单元测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_format_extensions() {
        assert_eq!(ExportFormat::Json.extension(), "json");
        assert_eq!(ExportFormat::Csv.extension(), "csv");
        assert_eq!(ExportFormat::Html.extension(), "html");
        assert_eq!(ExportFormat::Markdown.extension(), "md");
        assert_eq!(ExportFormat::Text.extension(), "txt");
    }

    #[test]
    fn test_export_format_mime_types() {
        assert_eq!(ExportFormat::Json.mime_type(), "application/json");
        assert_eq!(ExportFormat::Csv.mime_type(), "text/csv");
        assert_eq!(ExportFormat::Html.mime_type(), "text/html");
    }

    #[test]
    fn test_export_format_capabilities() {
        assert!(ExportFormat::Json.supports_structured_data());
        assert!(ExportFormat::Csv.supports_structured_data());
        assert!(!ExportFormat::Html.supports_structured_data());

        assert!(ExportFormat::Html.supports_rich_text());
        assert!(ExportFormat::Markdown.supports_rich_text());
        assert!(!ExportFormat::Json.supports_rich_text());
    }

    #[test]
    fn test_export_format_from_str() {
        assert_eq!(ExportFormat::from_str("json").unwrap(), ExportFormat::Json);
        assert_eq!(ExportFormat::from_str("CSV").unwrap(), ExportFormat::Csv);
        assert_eq!(
            ExportFormat::from_str("md").unwrap(),
            ExportFormat::Markdown
        );
        assert!(ExportFormat::from_str("invalid").is_err());
    }

    #[test]
    fn test_export_format_display() {
        assert_eq!(format!("{}", ExportFormat::Json), "JSON");
        assert_eq!(format!("{}", ExportFormat::Csv), "CSV");
        assert_eq!(format!("{}", ExportFormat::Html), "HTML");
    }

    #[test]
    fn test_export_options_default() {
        let options = ExportOptions::default();
        assert_eq!(options.format, ExportFormat::Json);
        assert!(options.include_highlights);
        assert!(options.include_metadata);
        assert!(!options.compress);
    }

    #[test]
    fn test_export_options_builder() {
        let options = ExportOptions::csv()
            .with_output_path(PathBuf::from("/tmp/export.csv"))
            .with_highlights(false)
            .with_compression(true)
            .with_max_results(100);

        assert_eq!(options.format, ExportFormat::Csv);
        assert!(!options.include_highlights);
        assert!(options.compress);
        assert_eq!(options.max_results, Some(100));
    }

    #[test]
    fn test_export_options_generate_filename() {
        let options = ExportOptions::json();
        let filename = options.generate_filename("search_results");

        assert!(filename.starts_with("search_results_"));
        assert!(filename.ends_with(".json"));
    }

    #[test]
    fn test_export_options_generate_filename_compressed() {
        let options = ExportOptions::json().with_compression(true);
        let filename = options.generate_filename("results");

        assert!(filename.ends_with(".json.gz"));
    }

    #[test]
    fn test_export_format_all() {
        let all = ExportFormat::all();
        assert_eq!(all.len(), 5);
        assert!(all.contains(&ExportFormat::Json));
    }
}
