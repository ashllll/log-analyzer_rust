use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;
use uuid::Uuid;

/// 领域事件基础trait
pub trait DomainEvent: Send + Sync {
    fn event_type(&self) -> &'static str;
    fn timestamp(&self) -> chrono::DateTime<chrono::Utc>;
}

/// 日志分析领域事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogAnalysisEvent {
    LogFileDiscovered {
        file_id: Uuid,
        file_path: String,
        file_size: u64,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    LogEntryParsed {
        entry_id: Uuid,
        file_id: Uuid,
        timestamp: chrono::DateTime<chrono::Utc>,
        level: String,
    },
    SearchExecuted {
        search_id: Uuid,
        query: String,
        result_count: usize,
        duration_ms: u64,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    ExportCompleted {
        export_id: Uuid,
        format: String,
        file_count: usize,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
}

impl DomainEvent for LogAnalysisEvent {
    fn event_type(&self) -> &'static str {
        match self {
            LogAnalysisEvent::LogFileDiscovered { .. } => "log.file.discovered",
            LogAnalysisEvent::LogEntryParsed { .. } => "log.entry.parsed",
            LogAnalysisEvent::SearchExecuted { .. } => "search.executed",
            LogAnalysisEvent::ExportCompleted { .. } => "export.completed",
        }
    }

    fn timestamp(&self) -> chrono::DateTime<chrono::Utc> {
        match self {
            LogAnalysisEvent::LogFileDiscovered { timestamp, .. } => *timestamp,
            LogAnalysisEvent::LogEntryParsed { timestamp, .. } => *timestamp,
            LogAnalysisEvent::SearchExecuted { timestamp, .. } => *timestamp,
            LogAnalysisEvent::ExportCompleted { timestamp, .. } => *timestamp,
        }
    }
}

/// 领域事件总线
#[derive(Debug, Clone)]
pub struct DomainEventBus {
    sender: broadcast::Sender<LogAnalysisEvent>,
}

impl DomainEventBus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _receiver) = broadcast::channel(capacity);
        Self { sender }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<LogAnalysisEvent> {
        self.sender.subscribe()
    }

    pub fn publish(
        &self,
        event: LogAnalysisEvent,
    ) -> Result<(), broadcast::error::SendError<LogAnalysisEvent>> {
        self.sender.send(event).map(|_| ())
    }

    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl Default for DomainEventBus {
    fn default() -> Self {
        Self::new(100)
    }
}

/// 事件处理器trait
#[async_trait::async_trait]
pub trait EventHandler: Send + Sync {
    async fn handle(&self, event: &LogAnalysisEvent) -> Result<(), Box<dyn std::error::Error>>;
}

/// 日志事件处理器示例
pub struct LogEventHandler;

#[async_trait::async_trait]
impl EventHandler for LogEventHandler {
    async fn handle(&self, event: &LogAnalysisEvent) -> Result<(), Box<dyn std::error::Error>> {
        match event {
            LogAnalysisEvent::LogFileDiscovered {
                file_path,
                file_size,
                ..
            } => {
                tracing::info!("Discovered log file: {} ({} bytes)", file_path, file_size);
            }
            LogAnalysisEvent::SearchExecuted {
                query,
                result_count,
                duration_ms,
                ..
            } => {
                tracing::info!(
                    "Search executed: '{}' returned {} results in {}ms",
                    query,
                    result_count,
                    duration_ms
                );
            }
            _ => {}
        }
        Ok(())
    }
}
