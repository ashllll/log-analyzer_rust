//! 导出领域服务
//!
//! 提供导出相关的领域服务

use std::io::Write;
use std::sync::Arc;

use async_trait::async_trait;

use super::value_objects::{ExportFormat, ExportOptions};
use crate::domain::search::entities::SearchResult;

/// 导出策略接口
#[async_trait]
pub trait ExportStrategy: Send + Sync {
    /// 策略名称
    fn name(&self) -> &str;

    /// 支持的导出格式
    fn supported_format(&self) -> ExportFormat;

    /// 执行导出
    async fn export(
        &self,
        results: &[SearchResult],
        options: &ExportOptions,
    ) -> Result<Vec<u8>, ExportStrategyError>;

    /// 估算输出大小
    fn estimate_size(&self, result_count: usize, avg_line_length: usize) -> usize;
}

/// 导出策略错误
#[derive(Debug, Clone, thiserror::Error)]
pub enum ExportStrategyError {
    #[error("导出失败: {0}")]
    ExportFailed(String),

    #[error("格式化错误: {0}")]
    FormattingError(String),

    #[error("IO错误: {0}")]
    IoError(String),

    #[error("不支持的格式: {0}")]
    UnsupportedFormat(String),
}

/// 导出聚合器
///
/// 管理多种导出策略并选择合适的策略执行导出
pub struct ExportAggregator {
    strategies: Vec<Arc<dyn ExportStrategy>>,
}

impl ExportAggregator {
    /// 创建导出聚合器
    pub fn new() -> Self {
        Self {
            strategies: Vec::new(),
        }
    }

    /// 注册策略
    pub fn register(&mut self, strategy: Arc<dyn ExportStrategy>) {
        self.strategies.push(strategy);
    }

    /// 获取指定格式的策略
    pub fn get_strategy(&self, format: ExportFormat) -> Option<Arc<dyn ExportStrategy>> {
        self.strategies
            .iter()
            .find(|s| s.supported_format() == format)
            .cloned()
    }

    /// 执行导出
    pub async fn export(
        &self,
        results: &[SearchResult],
        options: &ExportOptions,
    ) -> Result<Vec<u8>, ExportStrategyError> {
        let strategy = self
            .get_strategy(options.format)
            .ok_or_else(|| ExportStrategyError::UnsupportedFormat(format!("{}", options.format)))?;

        let data = strategy.export(results, options).await?;

        // 如果需要压缩
        if options.compress {
            self.compress_data(&data)
        } else {
            Ok(data)
        }
    }

    /// 压缩数据
    fn compress_data(&self, data: &[u8]) -> Result<Vec<u8>, ExportStrategyError> {
        use flate2::write::GzEncoder;
        use flate2::Compression;

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(data)
            .map_err(|e| ExportStrategyError::IoError(e.to_string()))?;
        encoder
            .finish()
            .map_err(|e| ExportStrategyError::IoError(e.to_string()))
    }

    /// 获取所有支持的格式
    pub fn supported_formats(&self) -> Vec<ExportFormat> {
        self.strategies
            .iter()
            .map(|s| s.supported_format())
            .collect()
    }
}

impl Default for ExportAggregator {
    fn default() -> Self {
        Self::new()
    }
}

// ==================== 基础导出策略实现 ====================

/// JSON 导出策略
pub struct JsonExportStrategy;

impl JsonExportStrategy {
    pub fn new() -> Self {
        Self
    }
}

impl Default for JsonExportStrategy {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ExportStrategy for JsonExportStrategy {
    fn name(&self) -> &str {
        "json"
    }

    fn supported_format(&self) -> ExportFormat {
        ExportFormat::Json
    }

    async fn export(
        &self,
        results: &[SearchResult],
        _options: &ExportOptions,
    ) -> Result<Vec<u8>, ExportStrategyError> {
        let json = serde_json::to_string_pretty(results)
            .map_err(|e| ExportStrategyError::FormattingError(e.to_string()))?;

        Ok(json.into_bytes())
    }

    fn estimate_size(&self, result_count: usize, avg_line_length: usize) -> usize {
        // JSON 格式大约增加 30% 的开销
        (result_count * avg_line_length * 130) / 100
    }
}

/// CSV 导出策略
pub struct CsvExportStrategy {
    delimiter: char,
}

impl CsvExportStrategy {
    pub fn new(delimiter: char) -> Self {
        Self { delimiter }
    }
}

impl Default for CsvExportStrategy {
    fn default() -> Self {
        Self::new(',')
    }
}

#[async_trait]
impl ExportStrategy for CsvExportStrategy {
    fn name(&self) -> &str {
        "csv"
    }

    fn supported_format(&self) -> ExportFormat {
        ExportFormat::Csv
    }

    async fn export(
        &self,
        results: &[SearchResult],
        _options: &ExportOptions,
    ) -> Result<Vec<u8>, ExportStrategyError> {
        let mut csv = String::new();

        // 表头
        csv.push_str(&format!(
            "line_number{}\tcontent{}\tsource_file{}\tscore\n",
            self.delimiter, self.delimiter, self.delimiter
        ));

        // 数据行
        for result in results {
            let content = result
                .content
                .replace(self.delimiter, &format!("\\{}", self.delimiter));
            csv.push_str(&format!(
                "{}{}{}{}{}{}{:.2}\n",
                result.line_number,
                self.delimiter,
                content,
                self.delimiter,
                result.source_file,
                self.delimiter,
                result.score
            ));
        }

        Ok(csv.into_bytes())
    }

    fn estimate_size(&self, result_count: usize, avg_line_length: usize) -> usize {
        // CSV 格式大约增加 10% 的开销
        (result_count * avg_line_length * 110) / 100
    }
}

/// 文本导出策略
pub struct TextExportStrategy;

impl TextExportStrategy {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TextExportStrategy {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ExportStrategy for TextExportStrategy {
    fn name(&self) -> &str {
        "text"
    }

    fn supported_format(&self) -> ExportFormat {
        ExportFormat::Text
    }

    async fn export(
        &self,
        results: &[SearchResult],
        options: &ExportOptions,
    ) -> Result<Vec<u8>, ExportStrategyError> {
        let mut text = String::new();

        for result in results {
            let content = if options.include_highlights && result.has_highlights() {
                result.highlighted_content()
            } else {
                result.content.clone()
            };

            text.push_str(&format!(
                "[{}:{}] {}\n",
                result.source_file, result.line_number, content
            ));
        }

        Ok(text.into_bytes())
    }

    fn estimate_size(&self, result_count: usize, avg_line_length: usize) -> usize {
        // 纯文本格式最紧凑
        result_count * avg_line_length
    }
}

// ==================== 单元测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_results() -> Vec<SearchResult> {
        vec![
            SearchResult::new(1, "error in line 1".to_string(), "app.log".to_string()),
            SearchResult::new(2, "warning in line 2".to_string(), "app.log".to_string()),
        ]
    }

    #[test]
    fn test_export_aggregator_creation() {
        let aggregator = ExportAggregator::new();
        assert!(aggregator.supported_formats().is_empty());
    }

    #[test]
    fn test_export_aggregator_register() {
        let mut aggregator = ExportAggregator::new();
        aggregator.register(Arc::new(JsonExportStrategy::new()));

        let formats = aggregator.supported_formats();
        assert_eq!(formats.len(), 1);
        assert!(formats.contains(&ExportFormat::Json));
    }

    #[test]
    fn test_export_aggregator_get_strategy() {
        let mut aggregator = ExportAggregator::new();
        aggregator.register(Arc::new(JsonExportStrategy::new()));

        let strategy = aggregator.get_strategy(ExportFormat::Json);
        assert!(strategy.is_some());

        let strategy = aggregator.get_strategy(ExportFormat::Csv);
        assert!(strategy.is_none());
    }

    #[tokio::test]
    async fn test_json_export_strategy() {
        let strategy = JsonExportStrategy::new();
        let results = create_test_results();
        let options = ExportOptions::json();

        let data = strategy.export(&results, &options).await.unwrap();
        let json_str = String::from_utf8(data).unwrap();

        assert!(json_str.contains("error in line 1"));
        assert!(json_str.starts_with("["));
    }

    #[tokio::test]
    async fn test_csv_export_strategy() {
        let strategy = CsvExportStrategy::new(',');
        let results = create_test_results();
        let options = ExportOptions::csv();

        let data = strategy.export(&results, &options).await.unwrap();
        let csv_str = String::from_utf8(data).unwrap();

        assert!(csv_str.contains("line_number"));
        assert!(csv_str.contains("error in line 1"));
    }

    #[tokio::test]
    async fn test_text_export_strategy() {
        let strategy = TextExportStrategy::new();
        let results = create_test_results();
        let options = ExportOptions::default();

        let data = strategy.export(&results, &options).await.unwrap();
        let text = String::from_utf8(data).unwrap();

        assert!(text.contains("[app.log:1]"));
        assert!(text.contains("error in line 1"));
    }

    #[test]
    fn test_export_strategy_estimate_size() {
        let strategy = JsonExportStrategy::new();
        let size = strategy.estimate_size(100, 50);
        assert!(size > 100 * 50);
    }

    #[tokio::test]
    async fn test_export_aggregator_export() {
        let mut aggregator = ExportAggregator::new();
        aggregator.register(Arc::new(JsonExportStrategy::new()));

        let results = create_test_results();
        let options = ExportOptions::json();

        let data = aggregator.export(&results, &options).await.unwrap();
        assert!(!data.is_empty());
    }

    #[tokio::test]
    async fn test_export_aggregator_unsupported_format() {
        let aggregator = ExportAggregator::new();

        let results = create_test_results();
        let options = ExportOptions::json();

        let result = aggregator.export(&results, &options).await;
        assert!(result.is_err());
    }
}
