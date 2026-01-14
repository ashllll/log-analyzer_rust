//! 应用命令

use crate::application::services::{ConfigurationService, LogAnalysisService};
use crate::error::Result;
use serde::{Deserialize, Serialize};

/// 分析日志文件命令
#[derive(Debug, Deserialize)]
pub struct AnalyzeLogFileCommand {
    pub file_path: String,
    pub workspace_id: String,
}

#[derive(Debug, Serialize)]
pub struct AnalyzeLogFileResult {
    pub file_id: String,
    pub entries_count: usize,
    pub processing_time_ms: u64,
}

/// 搜索日志命令
#[derive(Debug, Deserialize)]
pub struct SearchLogsCommand {
    pub query: String,
    pub workspace_id: String,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct SearchLogsResult {
    pub results: Vec<serde_json::Value>,
    pub total_count: usize,
    pub query_time_ms: u64,
}

/// 配置更新命令
#[derive(Debug, Deserialize)]
pub struct UpdateConfigCommand {
    pub config: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct UpdateConfigResult {
    pub success: bool,
    pub message: String,
}

/// 命令处理器
pub struct CommandHandler {
    log_service: LogAnalysisService,
    config_service: ConfigurationService,
}

impl CommandHandler {
    pub fn new(log_service: LogAnalysisService, config_service: ConfigurationService) -> Self {
        Self {
            log_service,
            config_service,
        }
    }

    pub async fn handle_analyze_log_file(
        &self,
        cmd: AnalyzeLogFileCommand,
    ) -> Result<AnalyzeLogFileResult> {
        let start = std::time::Instant::now();
        let entries = self.log_service.analyze_log_file(&cmd.file_path).await?;
        let duration = start.elapsed();

        Ok(AnalyzeLogFileResult {
            file_id: uuid::Uuid::new_v4().to_string(),
            entries_count: entries.len(),
            processing_time_ms: duration.as_millis() as u64,
        })
    }

    pub async fn handle_search_logs(&self, cmd: SearchLogsCommand) -> Result<SearchLogsResult> {
        let start = std::time::Instant::now();
        let results = self.log_service.search_logs(&cmd.query).await?;
        let duration = start.elapsed();

        let limited_results = results
            .into_iter()
            .skip(cmd.offset.unwrap_or(0))
            .take(cmd.limit.unwrap_or(100))
            .map(|entry| {
                serde_json::json!({
                    "id": entry.id.to_string(),
                    "timestamp": entry.timestamp.to_rfc3339(),
                    "level": entry.level.as_str(),
                    "message": entry.message.as_str(),
                    "source_file": entry.source_file,
                    "line_number": entry.line_number,
                })
            })
            .collect::<Vec<_>>();

        let total_count = limited_results.len();

        Ok(SearchLogsResult {
            results: limited_results,
            total_count,
            query_time_ms: duration.as_millis() as u64,
        })
    }

    pub async fn handle_update_config(
        &self,
        cmd: UpdateConfigCommand,
    ) -> Result<UpdateConfigResult> {
        // 这里将实现配置更新逻辑
        Ok(UpdateConfigResult {
            success: true,
            message: "Configuration updated successfully".to_string(),
        })
    }
}
