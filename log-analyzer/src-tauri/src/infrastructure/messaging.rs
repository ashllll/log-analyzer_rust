//! 消息基础设施模块
//!
//! 提供事件发布/订阅的消息基础设施实现，包括：
//! - 内存事件总线
//! - 事件处理器注册
//! - 异步事件分发

use std::collections::HashMap;
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use parking_lot::RwLock;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::domain::shared::DomainEvent;

/// 事件 ID 类型
pub type EventId = Uuid;

/// 事件时间戳类型
pub type EventTimestamp = chrono::DateTime<chrono::Utc>;

/// 事件处理器函数类型
pub type EventHandlerFn<E> =
    Arc<dyn Fn(E) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

/// 事件消息封装
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMessage<E: Serialize + Clone> {
    /// 事件 ID
    pub id: EventId,
    /// 事件类型
    pub event_type: String,
    /// 事件负载
    pub payload: E,
    /// 创建时间
    pub created_at: EventTimestamp,
    /// 来源
    pub source: Option<String>,
    /// 关联 ID（用于追踪）
    pub correlation_id: Option<String>,
}

impl<E: Serialize + Clone> EventMessage<E> {
    /// 创建新的事件消息
    pub fn new(event_type: impl Into<String>, payload: E) -> Self {
        Self {
            id: Uuid::new_v4(),
            event_type: event_type.into(),
            payload,
            created_at: chrono::Utc::now(),
            source: None,
            correlation_id: None,
        }
    }

    /// 设置来源
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// 设置关联 ID
    pub fn with_correlation_id(mut self, id: impl Into<String>) -> Self {
        self.correlation_id = Some(id.into());
        self
    }
}

/// 订阅句柄
pub struct Subscription {
    /// 订阅 ID
    pub id: Uuid,
    /// 事件类型
    pub event_type: String,
}

/// 内存事件总线实现
///
/// 使用 broadcast channel 实现发布/订阅模式
pub struct InMemoryEventBus {
    /// 事件发送器映射 (event_type -> sender)
    senders: Arc<RwLock<HashMap<String, broadcast::Sender<Vec<u8>>>>>,
    /// 订阅管理
    subscriptions: Arc<RwLock<HashMap<Uuid, String>>>,
    /// 配置
    config: EventBusConfig,
}

/// 事件总线配置
#[derive(Debug, Clone)]
pub struct EventBusConfig {
    /// 每个通道的缓冲区大小
    pub channel_capacity: usize,
    /// 是否启用事件日志
    pub enable_logging: bool,
}

impl Default for EventBusConfig {
    fn default() -> Self {
        Self {
            channel_capacity: 1000,
            enable_logging: true,
        }
    }
}

impl InMemoryEventBus {
    /// 创建新的事件总线
    pub fn new(config: EventBusConfig) -> Self {
        Self {
            senders: Arc::new(RwLock::new(HashMap::new())),
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// 获取或创建指定事件类型的发送器
    fn get_or_create_sender(&self, event_type: &str) -> broadcast::Sender<Vec<u8>> {
        let mut senders = self.senders.write();
        if !senders.contains_key(event_type) {
            let (tx, _) = broadcast::channel(self.config.channel_capacity);
            senders.insert(event_type.to_string(), tx);
        }
        senders.get(event_type).unwrap().clone()
    }

    /// 发布事件
    pub fn publish<E: Serialize + Clone + Debug>(&self, message: EventMessage<E>) {
        let event_type = message.event_type.clone();

        if self.config.enable_logging {
            debug!(
                event_id = %message.id,
                event_type = %event_type,
                correlation_id = ?message.correlation_id,
                "发布事件"
            );
        }

        // 序列化事件
        let payload = match serde_json::to_vec(&message) {
            Ok(p) => p,
            Err(e) => {
                error!("序列化事件失败: {}", e);
                return;
            }
        };

        // 发送事件
        let sender = self.get_or_create_sender(&event_type);
        if let Err(e) = sender.send(payload) {
            warn!("发送事件失败 (无订阅者): {}", e);
        }
    }

    /// 订阅事件
    pub fn subscribe<E: Serialize + DeserializeOwned + Clone + Debug + Send + 'static>(
        &self,
        event_type: &str,
        handler: EventHandlerFn<E>,
    ) -> Subscription {
        let sender = self.get_or_create_sender(event_type);
        let mut receiver = sender.subscribe();

        let subscription_id = Uuid::new_v4();

        // 记录订阅
        self.subscriptions
            .write()
            .insert(subscription_id, event_type.to_string());

        // 启动异步处理任务
        let sub_id = subscription_id;
        let event_type_owned = event_type.to_string();
        let enable_logging = self.config.enable_logging;

        tokio::spawn(async move {
            loop {
                match receiver.recv().await {
                    Ok(payload) => {
                        // 反序列化事件
                        let message: EventMessage<E> = match serde_json::from_slice(&payload) {
                            Ok(m) => m,
                            Err(e) => {
                                error!("反序列化事件失败: {}", e);
                                continue;
                            }
                        };

                        if enable_logging {
                            debug!(
                                subscription_id = %sub_id,
                                event_type = %event_type_owned,
                                "处理事件"
                            );
                        }

                        // 调用处理器
                        (handler)(message.payload).await;
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        info!("事件通道已关闭: {}", event_type_owned);
                        break;
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!("事件处理落后 {} 条消息: {}", n, event_type_owned);
                    }
                }
            }
        });

        Subscription {
            id: subscription_id,
            event_type: event_type.to_string(),
        }
    }

    /// 取消订阅
    pub fn unsubscribe(&self, subscription: &Subscription) {
        self.subscriptions.write().remove(&subscription.id);
        debug!(
            "取消订阅: {} ({})",
            subscription.event_type, subscription.id
        );
    }

    /// 获取活跃订阅数量
    pub fn active_subscription_count(&self) -> usize {
        self.subscriptions.read().len()
    }

    /// 获取指定事件类型的订阅数量
    pub fn subscription_count_for_type(&self, event_type: &str) -> usize {
        self.subscriptions
            .read()
            .values()
            .filter(|t| *t == event_type)
            .count()
    }
}

impl Default for InMemoryEventBus {
    fn default() -> Self {
        Self::new(EventBusConfig::default())
    }
}

/// 领域事件发布器
///
/// 用于领域模型中发布事件
pub struct DomainEventPublisher {
    event_bus: Arc<InMemoryEventBus>,
    source: String,
}

impl DomainEventPublisher {
    /// 创建新的领域事件发布器
    pub fn new(event_bus: Arc<InMemoryEventBus>, source: impl Into<String>) -> Self {
        Self {
            event_bus,
            source: source.into(),
        }
    }

    /// 发布领域事件
    pub fn publish<E: DomainEvent + Serialize + Clone + Debug>(&self, event: E) {
        let message = EventMessage::new(event.event_type(), event)
            .with_source(&self.source)
            .with_correlation_id(Uuid::new_v4().to_string());

        self.event_bus.publish(message);
    }

    /// 发布带有关联 ID 的领域事件
    pub fn publish_with_correlation<E: DomainEvent + Serialize + Clone + Debug>(
        &self,
        event: E,
        correlation_id: impl Into<String>,
    ) {
        let message = EventMessage::new(event.event_type(), event)
            .with_source(&self.source)
            .with_correlation_id(correlation_id);

        self.event_bus.publish(message);
    }
}

/// 事件重放器
///
/// 用于重放历史事件（需要配合持久化使用）
pub struct EventReplayer<E: Serialize + DeserializeOwned + Clone> {
    events: Vec<EventMessage<E>>,
}

impl<E: Serialize + DeserializeOwned + Clone> EventReplayer<E> {
    /// 创建新的事件重放器
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    /// 添加事件
    pub fn add_event(&mut self, event: EventMessage<E>) {
        self.events.push(event);
    }

    /// 从 JSON 加载事件
    pub fn load_from_json(&mut self, json: &str) -> Result<(), String> {
        let events: Vec<EventMessage<E>> =
            serde_json::from_str(json).map_err(|e| format!("解析事件失败: {}", e))?;
        self.events.extend(events);
        Ok(())
    }

    /// 导出为 JSON
    pub fn export_to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(&self.events).map_err(|e| format!("序列化事件失败: {}", e))
    }

    /// 重放所有事件
    pub async fn replay<F, Fut>(&self, handler: F)
    where
        F: Fn(EventMessage<E>) -> Fut,
        Fut: Future<Output = ()>,
    {
        for event in &self.events {
            handler(event.clone()).await;
        }
    }

    /// 获取事件数量
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// 清空事件
    pub fn clear(&mut self) {
        self.events.clear();
    }
}

impl<E: Serialize + DeserializeOwned + Clone> Default for EventReplayer<E> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct TestEvent {
        message: String,
        count: i32,
    }

    impl DomainEvent for TestEvent {
        fn event_type(&self) -> &'static str {
            "test_event"
        }

        fn timestamp(&self) -> chrono::DateTime<chrono::Utc> {
            chrono::Utc::now()
        }
    }

    #[test]
    fn test_event_message_creation() {
        let event = TestEvent {
            message: "hello".to_string(),
            count: 42,
        };

        let message = EventMessage::new("test_event", event)
            .with_source("test_module")
            .with_correlation_id("corr-123");

        assert_eq!(message.event_type, "test_event");
        assert_eq!(message.source, Some("test_module".to_string()));
        assert_eq!(message.correlation_id, Some("corr-123".to_string()));
    }

    #[tokio::test]
    async fn test_event_bus_publish_subscribe() {
        let event_bus = Arc::new(InMemoryEventBus::new(EventBusConfig {
            channel_capacity: 100,
            enable_logging: false,
        }));

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        // 订阅事件
        let handler: EventHandlerFn<TestEvent> = Arc::new(move |event| {
            let counter = counter_clone.clone();
            Box::pin(async move {
                if event.count > 0 {
                    counter.fetch_add(1, Ordering::SeqCst);
                }
            })
        });

        let _subscription = event_bus.subscribe("test_event", handler);

        // 等待订阅生效
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // 发布事件
        for i in 1..=5 {
            let event = TestEvent {
                message: format!("event {}", i),
                count: i,
            };
            event_bus.publish(EventMessage::new("test_event", event));
        }

        // 等待事件处理
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        assert_eq!(counter.load(Ordering::SeqCst), 5);
    }

    #[test]
    fn test_event_replayer() {
        let mut replayer = EventReplayer::<TestEvent>::new();

        // 添加事件
        for i in 1..=3 {
            let event = TestEvent {
                message: format!("event {}", i),
                count: i,
            };
            replayer.add_event(EventMessage::new("test_event", event));
        }

        assert_eq!(replayer.event_count(), 3);

        // 导出和加载
        let json = replayer.export_to_json().unwrap();
        let mut replayer2: EventReplayer<TestEvent> = EventReplayer::new();
        replayer2.load_from_json(&json).unwrap();
        assert_eq!(replayer2.event_count(), 3);
    }
}
