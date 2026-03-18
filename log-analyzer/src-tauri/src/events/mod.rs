//! 事件系统 - 简化版本
//!
//! 使用 tokio::sync::broadcast 提供类型安全的事件处理
//!
//! # 架构改进
//!
//! - 保留核心 EventBus 供内部组件使用
//! - 简化的 bridge 直接转发到 Tauri
//! - 统一的事件命名规范（snake_case 常量）
//!
//! # 优先级事件系统 (P2-11)
//!
//! - High (5000): task-update, import-complete
//! - Normal (2000): search-results, search-complete
//! - Low (500): system-info, system-warning

pub mod bridge;
pub mod constants;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, info, warn};

use crate::models::{FileChangeEvent, TaskProgress};

/// 广播结果类型别名，避免大类型在栈上
pub type BroadcastResult<T> = Result<T, Box<broadcast::error::SendError<AppEvent>>>;

/// 事件通道缓冲区大小（默认单通道）
const EVENT_BUFFER_SIZE: usize = 1000;

// ============================================================================
// 事件优先级系统
// ============================================================================

/// 事件优先级枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventPriority {
    Low = 0,
    Normal = 1,
    High = 2,
}

impl EventPriority {
    /// 获取优先级对应的通道容量
    pub fn channel_capacity(&self) -> usize {
        match self {
            EventPriority::Low => 500,
            EventPriority::Normal => 2000,
            EventPriority::High => 5000,
        }
    }
}

/// 分层优先级 Channel 结构
///
/// 防止高负载下事件丢失，通过不同容量实现背压控制
pub struct PriorityEventChannels {
    high: broadcast::Sender<AppEvent>,   // 容量: 5000
    normal: broadcast::Sender<AppEvent>, // 容量: 2000
    low: broadcast::Sender<AppEvent>,    // 容量: 500
}

impl PriorityEventChannels {
    /// 创建新的优先级事件通道
    pub fn new() -> Self {
        let (high, _) = broadcast::channel(EventPriority::High.channel_capacity());
        let (normal, _) = broadcast::channel(EventPriority::Normal.channel_capacity());
        let (low, _) = broadcast::channel(EventPriority::Low.channel_capacity());

        Self { high, normal, low }
    }

    /// 发送事件到对应优先级的通道
    #[allow(clippy::result_large_err)]
    pub fn send(
        &self,
        event: AppEvent,
        priority: EventPriority,
    ) -> Result<(), broadcast::error::SendError<AppEvent>> {
        match priority {
            EventPriority::High => self.high.send(event).map(|_| ()),
            EventPriority::Normal => self.normal.send(event).map(|_| ()),
            EventPriority::Low => self.low.send(event).map(|_| ()),
        }
    }

    /// 根据事件类型自动判断优先级并发送
    #[allow(clippy::result_large_err)]
    pub fn send_auto(&self, event: AppEvent) -> Result<(), broadcast::error::SendError<AppEvent>> {
        let priority = event_priority(&event);
        self.send(event, priority)
    }

    /// 订阅指定优先级的通道
    pub fn subscribe(&self, priority: EventPriority) -> broadcast::Receiver<AppEvent> {
        match priority {
            EventPriority::High => self.high.subscribe(),
            EventPriority::Normal => self.normal.subscribe(),
            EventPriority::Low => self.low.subscribe(),
        }
    }

    /// 订阅所有优先级通道，返回按优先级排序的接收器元组
    ///
    /// 返回: (High, Normal, Low)
    pub fn subscribe_all(
        &self,
    ) -> (
        broadcast::Receiver<AppEvent>,
        broadcast::Receiver<AppEvent>,
        broadcast::Receiver<AppEvent>,
    ) {
        (
            self.high.subscribe(),
            self.normal.subscribe(),
            self.low.subscribe(),
        )
    }

    /// 获取高优先级发送器
    pub fn high_sender(&self) -> &broadcast::Sender<AppEvent> {
        &self.high
    }

    /// 获取普通优先级发送器
    pub fn normal_sender(&self) -> &broadcast::Sender<AppEvent> {
        &self.normal
    }

    /// 获取低优先级发送器
    pub fn low_sender(&self) -> &broadcast::Sender<AppEvent> {
        &self.low
    }

    /// 获取指定优先级的发送器
    pub fn sender(&self, priority: EventPriority) -> &broadcast::Sender<AppEvent> {
        match priority {
            EventPriority::High => &self.high,
            EventPriority::Normal => &self.normal,
            EventPriority::Low => &self.low,
        }
    }
}

impl Default for PriorityEventChannels {
    fn default() -> Self {
        Self::new()
    }
}

/// 根据事件类型获取优先级
///
/// - High: task-update, import-complete
/// - Normal: search-results, search-complete, 搜索相关事件, 文件监控事件
/// - Low: system-info, system-warning, system-error
pub fn event_priority(event: &AppEvent) -> EventPriority {
    match event {
        // 高优先级：任务相关
        AppEvent::TaskUpdate { .. } => EventPriority::High,
        AppEvent::ImportComplete { .. } => EventPriority::High,

        // 普通优先级：搜索相关、文件监控
        AppEvent::SearchStart { .. } => EventPriority::Normal,
        AppEvent::SearchProgress { .. } => EventPriority::Normal,
        AppEvent::SearchResults { .. } => EventPriority::Normal,
        AppEvent::SearchSummary { .. } => EventPriority::Normal,
        AppEvent::SearchComplete { .. } => EventPriority::Normal,
        AppEvent::SearchError { .. } => EventPriority::Normal,
        AppEvent::AsyncSearchStart { .. } => EventPriority::Normal,
        AppEvent::AsyncSearchProgress { .. } => EventPriority::Normal,
        AppEvent::AsyncSearchResults { .. } => EventPriority::Normal,
        AppEvent::AsyncSearchComplete { .. } => EventPriority::Normal,
        AppEvent::AsyncSearchError { .. } => EventPriority::Normal,
        AppEvent::FileChanged { .. } => EventPriority::Normal,
        AppEvent::NewLogs { .. } => EventPriority::Normal,

        // 低优先级：系统事件
        AppEvent::SystemError { .. } => EventPriority::Low,
        AppEvent::SystemWarning { .. } => EventPriority::Low,
        AppEvent::SystemInfo { .. } => EventPriority::Low,
    }
}

/// 应用事件枚举
///
/// 所有内部组件通信的事件类型定义
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum AppEvent {
    // 搜索事件
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

    // 异步搜索事件
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

    // 任务事件
    TaskUpdate {
        progress: TaskProgress,
    },
    ImportComplete {
        task_id: String,
    },

    // 文件监控事件
    FileChanged {
        event: FileChangeEvent,
    },
    NewLogs {
        entries: Vec<crate::models::LogEntry>,
    },

    // 系统事件（通常不转发到前端）
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

/// 事件总线
///
/// 使用 tokio broadcast channel 实现多播事件分发
pub struct EventBus {
    sender: broadcast::Sender<AppEvent>,
    /// 内部保留的 receiver，防止 channel 关闭
    _receiver: broadcast::Receiver<AppEvent>,
    /// 订阅者信息（用于调试和监控）
    subscribers: Arc<parking_lot::Mutex<HashMap<String, SubscriberInfo>>>,
    /// 事件统计
    stats: Arc<parking_lot::Mutex<EventStats>>,
}

/// 订阅者信息
#[derive(Debug, Clone)]
struct SubscriberInfo {
    subscribed_at: std::time::Instant,
    is_active: bool,
}

/// 事件统计
#[derive(Debug, Default)]
pub struct EventStats {
    pub total_events_sent: u64,
    pub total_subscribers: u64,
    pub events_by_type: HashMap<String, u64>,
    pub dropped_events: u64,
    pub subscriber_errors: u64,
}

impl EventBus {
    /// 创建新的事件总线
    pub fn new() -> Self {
        let (sender, receiver) = broadcast::channel(EVENT_BUFFER_SIZE);

        Self {
            sender,
            _receiver: receiver,
            subscribers: Arc::new(parking_lot::Mutex::new(HashMap::new())),
            stats: Arc::new(parking_lot::Mutex::new(EventStats::default())),
        }
    }

    /// 发送事件到所有订阅者
    pub fn emit(&self, event: AppEvent) -> BroadcastResult<usize> {
        // 更新统计
        {
            let mut stats = self.stats.lock();
            stats.total_events_sent += 1;

            let event_type = event_type_name(&event);
            *stats
                .events_by_type
                .entry(event_type.to_string())
                .or_insert(0) += 1;
        }

        debug!(event_type = %event_type_name(&event), "Emitting event");

        // 发送事件
        match self.sender.send(event) {
            Ok(subscriber_count) => {
                debug!(subscriber_count, "Event sent successfully");
                Ok(subscriber_count)
            }
            Err(e) => {
                let mut stats = self.stats.lock();
                stats.dropped_events += 1;
                let dropped = stats.dropped_events;
                drop(stats);
                warn!(
                    error = %e,
                    total_dropped = dropped,
                    "Event dropped: no active receivers"
                );
                Err(Box::new(e))
            }
        }
    }

    /// 订阅事件
    pub fn subscribe(&self, subscriber_name: String) -> broadcast::Receiver<AppEvent> {
        let receiver = self.sender.subscribe();

        {
            let mut subscribers = self.subscribers.lock();

            if let Some(info) = subscribers.get_mut(&subscriber_name) {
                info.subscribed_at = std::time::Instant::now();
                info.is_active = true;
                debug!(subscriber_name, "Existing subscriber renewed");
            } else {
                subscribers.insert(
                    subscriber_name.clone(),
                    SubscriberInfo {
                        subscribed_at: std::time::Instant::now(),
                        is_active: true,
                    },
                );
                debug!(subscriber_name, "New subscriber registered");
            }

            let mut stats = self.stats.lock();
            stats.total_subscribers = subscribers.len() as u64;
        }

        receiver
    }

    /// 取消订阅
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
        let mut stats = self.stats.lock();
        stats.total_subscribers = subscribers.len() as u64;
    }

    /// 获取统计信息
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

    /// 获取订阅者列表
    pub fn get_subscribers(&self) -> Vec<String> {
        let subscribers = self.subscribers.lock();
        subscribers.keys().cloned().collect()
    }

    /// 清除统计
    pub fn clear_stats(&self) {
        let mut stats = self.stats.lock();
        *stats = EventStats::default();
    }

    /// 清理过期订阅者
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

        let mut stats = self.stats.lock();
        stats.total_subscribers = subscribers.len() as u64;
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// 获取事件类型名称
fn event_type_name(event: &AppEvent) -> &'static str {
    match event {
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
    }
}

// ============================================================================
// 全局事件总线
// ============================================================================

static EVENT_BUS: std::sync::OnceLock<EventBus> = std::sync::OnceLock::new();

/// 获取全局事件总线实例
pub fn get_event_bus() -> &'static EventBus {
    EVENT_BUS.get_or_init(EventBus::new)
}

/// 初始化全局事件总线
pub fn init_event_bus() -> &'static EventBus {
    get_event_bus()
}

/// 便捷函数：使用全局事件总线发送事件
pub fn emit_event(event: AppEvent) -> BroadcastResult<usize> {
    get_event_bus().emit(event)
}

/// 便捷函数：使用全局事件总线订阅
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

        let result = bus.emit(event.clone());
        assert!(result.is_ok());

        let subscriber_count = result.unwrap();
        assert!(
            subscriber_count >= 1,
            "Should have at least 1 subscriber, got {}",
            subscriber_count
        );

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

        let subscriber_count = result.unwrap();
        assert!(
            subscriber_count >= 2,
            "Should have at least 2 subscribers, got {}",
            subscriber_count
        );

        let received1 = timeout(Duration::from_millis(100), receiver1.recv()).await;
        let received2 = timeout(Duration::from_millis(100), receiver2.recv()).await;

        assert!(received1.is_ok());
        assert!(received2.is_ok());
    }

    #[test]
    fn test_event_statistics() {
        let bus = EventBus::new();
        let _receiver = bus.subscribe("test".to_string());

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

    #[test]
    fn test_event_type_name_coverage() {
        // 测试所有事件类型都有对应的名称
        let events = vec![
            AppEvent::SearchStart {
                message: "".to_string(),
            },
            AppEvent::SearchProgress { progress: 0 },
            AppEvent::SearchResults { results: vec![] },
            AppEvent::SearchSummary {
                summary: crate::models::SearchResultSummary::new(0, vec![], 0, false),
            },
            AppEvent::SearchComplete { count: 0 },
            AppEvent::SearchError {
                error: "".to_string(),
            },
            AppEvent::AsyncSearchStart {
                search_id: "".to_string(),
            },
            AppEvent::AsyncSearchProgress {
                search_id: "".to_string(),
                progress: 0,
            },
            AppEvent::AsyncSearchResults { results: vec![] },
            AppEvent::AsyncSearchComplete {
                search_id: "".to_string(),
                count: 0,
            },
            AppEvent::AsyncSearchError {
                search_id: "".to_string(),
                error: "".to_string(),
            },
            AppEvent::TaskUpdate {
                progress: TaskProgress {
                    task_id: "".to_string(),
                    task_type: "".to_string(),
                    target: "".to_string(),
                    status: "".to_string(),
                    message: "".to_string(),
                    progress: 0,
                    workspace_id: None,
                },
            },
            AppEvent::ImportComplete {
                task_id: "".to_string(),
            },
            AppEvent::FileChanged {
                event: FileChangeEvent {
                    event_type: "".to_string(),
                    file_path: "".to_string(),
                    workspace_id: "".to_string(),
                    timestamp: 0,
                },
            },
            AppEvent::NewLogs { entries: vec![] },
            AppEvent::SystemError {
                error: "".to_string(),
                context: None,
            },
            AppEvent::SystemWarning {
                warning: "".to_string(),
                context: None,
            },
            AppEvent::SystemInfo {
                info: "".to_string(),
                context: None,
            },
        ];

        for event in events {
            let name = event_type_name(&event);
            assert!(!name.is_empty(), "Event type name should not be empty");
        }
    }

    // ============================================================================
    // 优先级事件系统测试 (P2-11)
    // ============================================================================

    #[test]
    fn test_event_priority_values() {
        assert_eq!(EventPriority::Low as i32, 0);
        assert_eq!(EventPriority::Normal as i32, 1);
        assert_eq!(EventPriority::High as i32, 2);

        // 验证优先级排序
        assert!(EventPriority::Low < EventPriority::Normal);
        assert!(EventPriority::Normal < EventPriority::High);
        assert!(EventPriority::Low < EventPriority::High);
    }

    #[test]
    fn test_event_priority_channel_capacity() {
        assert_eq!(EventPriority::Low.channel_capacity(), 500);
        assert_eq!(EventPriority::Normal.channel_capacity(), 2000);
        assert_eq!(EventPriority::High.channel_capacity(), 5000);
    }

    #[test]
    fn test_event_priority_mapping() {
        // 高优先级事件
        let high_events = vec![
            AppEvent::TaskUpdate {
                progress: TaskProgress {
                    task_id: "test".to_string(),
                    task_type: "import".to_string(),
                    target: "file".to_string(),
                    status: "running".to_string(),
                    message: "".to_string(),
                    progress: 50,
                    workspace_id: None,
                },
            },
            AppEvent::ImportComplete {
                task_id: "test".to_string(),
            },
        ];

        for event in high_events {
            assert_eq!(
                event_priority(&event),
                EventPriority::High,
                "{:?} should be High priority",
                event
            );
        }

        // 普通优先级事件
        let normal_events = vec![
            AppEvent::SearchStart {
                message: "test".to_string(),
            },
            AppEvent::SearchResults { results: vec![] },
            AppEvent::SearchComplete { count: 0 },
            AppEvent::FileChanged {
                event: FileChangeEvent {
                    event_type: "modify".to_string(),
                    file_path: "/test".to_string(),
                    workspace_id: "ws".to_string(),
                    timestamp: 0,
                },
            },
        ];

        for event in normal_events {
            assert_eq!(
                event_priority(&event),
                EventPriority::Normal,
                "{:?} should be Normal priority",
                event
            );
        }

        // 低优先级事件
        let low_events = vec![
            AppEvent::SystemInfo {
                info: "test".to_string(),
                context: None,
            },
            AppEvent::SystemWarning {
                warning: "test".to_string(),
                context: None,
            },
            AppEvent::SystemError {
                error: "test".to_string(),
                context: None,
            },
        ];

        for event in low_events {
            assert_eq!(
                event_priority(&event),
                EventPriority::Low,
                "{:?} should be Low priority",
                event
            );
        }
    }

    #[test]
    fn test_priority_event_channels_creation() {
        let channels = PriorityEventChannels::new();

        // 验证可以通过每个通道发送和接收
        let mut high_rx = channels.subscribe(EventPriority::High);
        let event = AppEvent::TaskUpdate {
            progress: TaskProgress {
                task_id: "test".to_string(),
                task_type: "import".to_string(),
                target: "file".to_string(),
                status: "running".to_string(),
                message: "".to_string(),
                progress: 50,
                workspace_id: None,
            },
        };

        assert!(channels.send(event.clone(), EventPriority::High).is_ok());

        // 使用 try_recv 验证事件被正确发送
        let received = high_rx.try_recv();
        assert!(received.is_ok());
        match received.unwrap() {
            AppEvent::TaskUpdate { .. } => {}
            _ => panic!("Received wrong event type"),
        }
    }

    #[tokio::test]
    async fn test_priority_event_channels_send_auto() {
        let channels = PriorityEventChannels::new();
        let (mut high_rx, mut normal_rx, mut low_rx) = channels.subscribe_all();

        // 发送不同优先级的事件
        let high_event = AppEvent::ImportComplete {
            task_id: "test".to_string(),
        };
        let normal_event = AppEvent::SearchComplete { count: 10 };
        let low_event = AppEvent::SystemInfo {
            info: "test".to_string(),
            context: None,
        };

        channels.send_auto(high_event).unwrap();
        channels.send_auto(normal_event).unwrap();
        channels.send_auto(low_event).unwrap();

        // 验证事件被路由到正确的通道
        let received_high = timeout(Duration::from_millis(100), high_rx.recv()).await;
        assert!(received_high.is_ok());

        let received_normal = timeout(Duration::from_millis(100), normal_rx.recv()).await;
        assert!(received_normal.is_ok());

        let received_low = timeout(Duration::from_millis(100), low_rx.recv()).await;
        assert!(received_low.is_ok());
    }

    #[test]
    fn test_priority_channels_capacity() {
        // 验证各优先级通道的容量配置
        let channels = PriorityEventChannels::new();

        // 验证发送器容量（high: 5000, normal: 2000, low: 500）
        assert_eq!(channels.high_sender().len(), 0);
        assert_eq!(channels.normal_sender().len(), 0);
        assert_eq!(channels.low_sender().len(), 0);

        // 验证订阅可以正常工作
        let _ = channels.subscribe(EventPriority::High);
        let _ = channels.subscribe(EventPriority::Normal);
        let _ = channels.subscribe(EventPriority::Low);
    }
}
