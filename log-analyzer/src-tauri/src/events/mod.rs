//! Event system using tokio::sync::broadcast for type-safe event handling

pub mod bridge;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

/// Type alias for broadcast results to avoid large error types on stack
/// Using Box reduces stack size since AppEvent contains large data like Vec<LogEntry>
pub type BroadcastResult<T> = Result<T, Box<broadcast::error::SendError<AppEvent>>>;

use crate::models::{FileChangeEvent, TaskProgress};

/// Maximum number of events to buffer in each channel
const EVENT_BUFFER_SIZE: usize = 1000;

/// Type-safe event definitions with enum variants
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum AppEvent {
    // Search events
    SearchStart {
        message: String,
    },
    SearchProgress {
        progress: i32,
    },
    SearchResults {
        results: Vec<crate::models::LogEntry>,
    },
    SearchSummary {
        summary: crate::models::SearchResultSummary,
    },
    SearchComplete {
        count: usize,
    },
    SearchError {
        error: String,
    },

    // Async search events
    AsyncSearchStart {
        search_id: String,
    },
    AsyncSearchProgress {
        search_id: String,
        progress: u32,
    },
    AsyncSearchResults {
        results: Vec<crate::models::LogEntry>,
    },
    AsyncSearchComplete {
        search_id: String,
        count: usize,
    },
    AsyncSearchError {
        search_id: String,
        error: String,
    },

    // Task events
    TaskUpdate {
        progress: TaskProgress,
    },
    ImportComplete {
        task_id: String,
    },

    // File watcher events
    FileChanged {
        event: FileChangeEvent,
    },
    NewLogs {
        entries: Vec<crate::models::LogEntry>,
    },

    // System events
    SystemError {
        error: String,
        context: Option<String>,
    },
    SystemWarning {
        warning: String,
        context: Option<String>,
    },
    SystemInfo {
        info: String,
        context: Option<String>,
    },
}

/// Event bus with automatic subscriber management
pub struct EventBus {
    /// Main broadcast channel for all events
    sender: broadcast::Sender<AppEvent>,
    /// Receiver for the main channel (kept alive to prevent channel closure)
    _receiver: broadcast::Receiver<AppEvent>,
    /// Named subscribers for debugging and monitoring
    /// 只存储订阅者信息，不存储 receiver 避免内存泄漏
    /// receiver 由调用者持有和释放
    subscribers: Arc<parking_lot::Mutex<HashMap<String, SubscriberInfo>>>,
    /// Event statistics for monitoring
    stats: Arc<parking_lot::Mutex<EventStats>>,
}

/// 订阅者信息（不包含 receiver，避免内存泄漏）
#[derive(Debug, Clone)]
struct SubscriberInfo {
    /// 订阅时间（用于清理和统计）
    subscribed_at: std::time::Instant,
    /// 是否活跃（调用者是否还持有 receiver）
    /// 注意：我们无法直接检测 receiver 是否被 drop，这里主要用于监控
    is_active: bool,
}

/// Event statistics for monitoring and debugging
#[derive(Debug, Default)]
pub struct EventStats {
    pub total_events_sent: u64,
    pub total_subscribers: u64,
    pub events_by_type: HashMap<String, u64>,
    pub dropped_events: u64,
    pub subscriber_errors: u64,
}

impl EventBus {
    /// Create a new event bus
    pub fn new() -> Self {
        let (sender, receiver) = broadcast::channel(EVENT_BUFFER_SIZE);

        Self {
            sender,
            _receiver: receiver,
            subscribers: Arc::new(parking_lot::Mutex::new(HashMap::new())),
            stats: Arc::new(parking_lot::Mutex::new(EventStats::default())),
        }
    }

    /// Emit an event to all subscribers
    pub fn emit(&self, event: AppEvent) -> BroadcastResult<usize> {
        // Update statistics
        {
            let mut stats = self.stats.lock();
            stats.total_events_sent += 1;

            let event_type = match &event {
                AppEvent::SearchStart { .. } => "SearchStart",
                AppEvent::SearchProgress { .. } => "SearchProgress",
                AppEvent::SearchResults { .. } => "SearchResults",
                AppEvent::SearchSummary { .. } => "SearchSummary",
                AppEvent::SearchComplete { .. } => "SearchComplete",
                AppEvent::SearchError { .. } => "SearchError",
                AppEvent::AsyncSearchStart { .. } => "AsyncSearchStart",
                AppEvent::AsyncSearchProgress { .. } => "AsyncSearchProgress",
                AppEvent::AsyncSearchResults { .. } => "AsyncSearchResults",
                AppEvent::AsyncSearchComplete { .. } => "AsyncSearchComplete",
                AppEvent::AsyncSearchError { .. } => "AsyncSearchError",
                AppEvent::TaskUpdate { .. } => "TaskUpdate",
                AppEvent::ImportComplete { .. } => "ImportComplete",
                AppEvent::FileChanged { .. } => "FileChanged",
                AppEvent::NewLogs { .. } => "NewLogs",
                AppEvent::SystemError { .. } => "SystemError",
                AppEvent::SystemWarning { .. } => "SystemWarning",
                AppEvent::SystemInfo { .. } => "SystemInfo",
            };

            *stats
                .events_by_type
                .entry(event_type.to_string())
                .or_insert(0) += 1;
        }

        // Log event for debugging
        debug!(
            event_type = ?std::mem::discriminant(&event),
            "Emitting event"
        );

        // Send event
        match self.sender.send(event) {
            Ok(subscriber_count) => {
                debug!(subscriber_count, "Event sent successfully");
                Ok(subscriber_count)
            }
            Err(e) => {
                error!(error = %e, "Failed to send event");
                let mut stats = self.stats.lock();
                stats.dropped_events += 1;
                Err(Box::new(e))
            }
        }
    }

    /// Subscribe to events with a named subscriber
    ///
    /// 使用 tokio::sync::broadcast 的标准订阅模式
    /// 注意：只创建一个 receiver 并返回，避免重复订阅
    pub fn subscribe(&self, subscriber_name: String) -> broadcast::Receiver<AppEvent> {
        let receiver = self.sender.subscribe();

        {
            let mut subscribers = self.subscribers.lock();

            // 检查是否已存在同名订阅者
            if subscribers.contains_key(&subscriber_name) {
                // 如果已存在，更新订阅时间（视为重新订阅）
                let info = subscribers.get_mut(&subscriber_name).unwrap();
                info.subscribed_at = std::time::Instant::now();
                info.is_active = true;
                debug!(subscriber_name, "Existing subscriber renewed");
            } else {
                // 新增订阅者
                subscribers.insert(
                    subscriber_name.clone(),
                    SubscriberInfo {
                        subscribed_at: std::time::Instant::now(),
                        is_active: true,
                    },
                );
                debug!(subscriber_name, "New subscriber registered");
            }

            // 更新统计：活跃订阅者数量
            let mut stats = self.stats.lock();
            stats.total_subscribers = subscribers.len() as u64;
        }

        receiver
    }

    /// Remove a named subscriber
    pub fn unsubscribe(&self, subscriber_name: &str) {
        let mut subscribers = self.subscribers.lock();
        if subscribers.remove(subscriber_name).is_some() {
            info!(subscriber_name, "Subscriber removed");
        } else {
            warn!(
                subscriber_name,
                "Attempted to remove non-existent subscriber"
            );
        }
        // 更新统计
        let mut stats = self.stats.lock();
        stats.total_subscribers = subscribers.len() as u64;
    }

    /// Get current event statistics
    pub fn get_stats(&self) -> EventStats {
        let stats = self.stats.lock();
        EventStats {
            total_events_sent: stats.total_events_sent,
            total_subscribers: stats.total_subscribers,
            events_by_type: stats.events_by_type.clone(),
            dropped_events: stats.dropped_events,
            subscriber_errors: stats.subscriber_errors,
        }
    }

    /// Get list of active subscribers
    pub fn get_subscribers(&self) -> Vec<String> {
        let subscribers = self.subscribers.lock();
        subscribers.keys().cloned().collect()
    }

    /// Clear all statistics (useful for testing)
    pub fn clear_stats(&self) {
        let mut stats = self.stats.lock();
        *stats = EventStats::default();
    }

    /// Force cleanup of stale subscribers (public API)
    /// Removes entries that haven't been active for a while
    pub fn cleanup_stale_subscribers(&self, max_age_secs: u64) {
        let mut subscribers = self.subscribers.lock();
        let now = std::time::Instant::now();
        let mut stale = Vec::new();

        for (name, info) in subscribers.iter() {
            let age = now.duration_since(info.subscribed_at).as_secs();
            if age > max_age_secs && !info.is_active {
                stale.push(name.clone());
            }
        }

        for name in stale {
            subscribers.remove(&name);
            debug!(subscriber_name = %name, max_age_secs, "Removed stale subscriber");
        }

        // 更新统计
        let mut stats = self.stats.lock();
        stats.total_subscribers = subscribers.len() as u64;
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// Global event bus instance
static EVENT_BUS: std::sync::OnceLock<EventBus> = std::sync::OnceLock::new();

/// Get the global event bus instance
pub fn get_event_bus() -> &'static EventBus {
    EVENT_BUS.get_or_init(EventBus::new)
}

/// Initialize the global event bus (called during app startup)
pub fn init_event_bus() -> &'static EventBus {
    get_event_bus()
}

/// Convenience function to emit an event using the global event bus
pub fn emit_event(event: AppEvent) -> BroadcastResult<usize> {
    get_event_bus().emit(event)
}

/// Convenience function to subscribe using the global event bus
pub fn subscribe_to_events(subscriber_name: String) -> broadcast::Receiver<AppEvent> {
    get_event_bus().subscribe(subscriber_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn test_event_bus_basic_functionality() {
        let bus = EventBus::new();
        let mut receiver = bus.subscribe("test_subscriber".to_string());

        let event = AppEvent::SystemInfo {
            info: "Test message".to_string(),
            context: None,
        };

        // Send event
        let result = bus.emit(event.clone());
        assert!(result.is_ok());
        // 订阅者数量：1个返回的receiver + 1个内部跟踪的receiver = 2
        // 但实际接收事件的只有返回的那个receiver
        let subscriber_count = result.unwrap();
        assert!(
            subscriber_count >= 1,
            "Should have at least 1 subscriber, got {}",
            subscriber_count
        );

        // Receive event
        let received = timeout(Duration::from_millis(100), receiver.recv()).await;
        assert!(received.is_ok());

        let received_event = received.unwrap().unwrap();
        match received_event {
            AppEvent::SystemInfo { info, context } => {
                assert_eq!(info, "Test message");
                assert_eq!(context, None);
            }
            _ => panic!("Received wrong event type"),
        }
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let bus = EventBus::new();
        let mut receiver1 = bus.subscribe("subscriber1".to_string());
        let mut receiver2 = bus.subscribe("subscriber2".to_string());

        let event = AppEvent::SearchStart {
            message: "Starting search".to_string(),
        };

        let result = bus.emit(event);
        assert!(result.is_ok());
        // 至少有2个活跃的订阅者
        let subscriber_count = result.unwrap();
        assert!(
            subscriber_count >= 2,
            "Should have at least 2 subscribers, got {}",
            subscriber_count
        );

        // Both receivers should get the event
        let received1 = timeout(Duration::from_millis(100), receiver1.recv()).await;
        let received2 = timeout(Duration::from_millis(100), receiver2.recv()).await;

        assert!(received1.is_ok());
        assert!(received2.is_ok());
    }

    #[test]
    fn test_event_statistics() {
        let bus = EventBus::new();
        let _receiver = bus.subscribe("test".to_string());

        // Send different types of events
        let _ = bus.emit(AppEvent::SearchStart {
            message: "test".to_string(),
        });
        let _ = bus.emit(AppEvent::SearchProgress { progress: 50 });
        let _ = bus.emit(AppEvent::SearchStart {
            message: "test2".to_string(),
        });

        let stats = bus.get_stats();
        assert_eq!(stats.total_events_sent, 3);
        assert_eq!(stats.events_by_type.get("SearchStart"), Some(&2));
        assert_eq!(stats.events_by_type.get("SearchProgress"), Some(&1));
    }
}
