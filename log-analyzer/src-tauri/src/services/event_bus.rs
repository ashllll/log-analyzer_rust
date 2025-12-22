use eyre::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

use crate::models::{FileChangeEvent, TaskProgress};

/// 应用事件类型定义
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum AppEvent {
    /// 任务进度更新事件
    TaskUpdate(TaskProgress),
    /// 文件变化事件
    FileChange(FileChangeEvent),
    /// 搜索错误事件
    SearchError {
        workspace_id: String,
        query: String,
        error: String,
    },
    /// 工作区状态变化事件
    WorkspaceStatusChange {
        workspace_id: String,
        status: String,
        message: Option<String>,
    },
    /// 缓存统计更新事件
    CacheStatsUpdate {
        hit_rate: f64,
        total_requests: u64,
        cache_size: usize,
    },
    /// 系统错误事件
    SystemError {
        component: String,
        error: String,
        context: Option<String>,
    },
}

/// 事件总线 - 使用 tokio::sync::broadcast 实现
pub struct EventBus {
    sender: broadcast::Sender<AppEvent>,
    /// 保留一个接收器以防止通道关闭
    _receiver: broadcast::Receiver<AppEvent>,
}

impl EventBus {
    /// 创建新的事件总线
    /// 
    /// # Arguments
    /// * `capacity` - 通道容量，默认推荐 1000
    pub fn new(capacity: usize) -> Self {
        let (sender, receiver) = broadcast::channel(capacity);
        info!("EventBus initialized with capacity: {}", capacity);
        
        Self {
            sender,
            _receiver: receiver,
        }
    }

    /// 发布事件到所有订阅者
    /// 
    /// # Arguments
    /// * `event` - 要发布的事件
    /// 
    /// # Returns
    /// 成功时返回接收到事件的订阅者数量
    pub fn publish(&self, event: AppEvent) -> Result<usize> {
        match self.sender.send(event.clone()) {
            Ok(count) => {
                debug!("Event published to {} subscribers: {:?}", count, event);
                Ok(count)
            }
            Err(e) => {
                warn!("Failed to publish event (no active subscribers): {:?}", e);
                Ok(0) // 没有订阅者不算错误
            }
        }
    }

    /// 订阅事件
    /// 
    /// # Returns
    /// 返回一个新的接收器，可以用来接收事件
    pub fn subscribe(&self) -> broadcast::Receiver<AppEvent> {
        let receiver = self.sender.subscribe();
        debug!("New subscriber added to EventBus");
        receiver
    }

    /// 获取当前订阅者数量
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }

    /// 发布任务更新事件
    pub fn publish_task_update(&self, progress: TaskProgress) -> Result<()> {
        self.publish(AppEvent::TaskUpdate(progress))?;
        Ok(())
    }

    /// 发布文件变化事件
    pub fn publish_file_change(&self, event: FileChangeEvent) -> Result<()> {
        self.publish(AppEvent::FileChange(event))?;
        Ok(())
    }

    /// 发布搜索错误事件
    pub fn publish_search_error(
        &self,
        workspace_id: String,
        query: String,
        error: String,
    ) -> Result<()> {
        self.publish(AppEvent::SearchError {
            workspace_id,
            query,
            error,
        })?;
        Ok(())
    }

    /// 发布工作区状态变化事件
    pub fn publish_workspace_status(
        &self,
        workspace_id: String,
        status: String,
        message: Option<String>,
    ) -> Result<()> {
        self.publish(AppEvent::WorkspaceStatusChange {
            workspace_id,
            status,
            message,
        })?;
        Ok(())
    }

    /// 发布缓存统计更新事件
    pub fn publish_cache_stats(
        &self,
        hit_rate: f64,
        total_requests: u64,
        cache_size: usize,
    ) -> Result<()> {
        self.publish(AppEvent::CacheStatsUpdate {
            hit_rate,
            total_requests,
            cache_size,
        })?;
        Ok(())
    }

    /// 发布系统错误事件
    pub fn publish_system_error(
        &self,
        component: String,
        error: String,
        context: Option<String>,
    ) -> Result<()> {
        self.publish(AppEvent::SystemError {
            component,
            error,
            context,
        })?;
        Ok(())
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(1000)
    }
}

/// 事件订阅者 - 用于自动管理订阅生命周期
pub struct EventSubscriber {
    receiver: broadcast::Receiver<AppEvent>,
    name: String,
}

impl EventSubscriber {
    /// 创建新的事件订阅者
    pub fn new(receiver: broadcast::Receiver<AppEvent>, name: impl Into<String>) -> Self {
        let name = name.into();
        info!("EventSubscriber '{}' created", name);
        Self { receiver, name }
    }

    /// 接收下一个事件（阻塞）
    pub async fn recv(&mut self) -> Result<AppEvent> {
        loop {
            match self.receiver.recv().await {
                Ok(event) => {
                    debug!("Subscriber '{}' received event: {:?}", self.name, event);
                    return Ok(event);
                }
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    warn!(
                        "Subscriber '{}' lagged behind, {} events skipped",
                        self.name, skipped
                    );
                    // 继续循环接收下一个事件
                    continue;
                }
                Err(e) => {
                    error!("Subscriber '{}' recv error: {}", self.name, e);
                    return Err(eyre::eyre!("Failed to receive event: {}", e));
                }
            }
        }
    }

    /// 尝试接收事件（非阻塞）
    pub fn try_recv(&mut self) -> Option<AppEvent> {
        match self.receiver.try_recv() {
            Ok(event) => {
                debug!(
                    "Subscriber '{}' try_recv received event: {:?}",
                    self.name, event
                );
                Some(event)
            }
            Err(broadcast::error::TryRecvError::Empty) => None,
            Err(broadcast::error::TryRecvError::Lagged(skipped)) => {
                warn!(
                    "Subscriber '{}' lagged behind, {} events skipped",
                    self.name, skipped
                );
                self.try_recv()
            }
            Err(e) => {
                error!("Subscriber '{}' try_recv error: {}", self.name, e);
                None
            }
        }
    }
}

impl Drop for EventSubscriber {
    fn drop(&mut self) {
        info!("EventSubscriber '{}' dropped", self.name);
    }
}

/// 全局事件总线实例
static EVENT_BUS: once_cell::sync::Lazy<Arc<EventBus>> =
    once_cell::sync::Lazy::new(|| Arc::new(EventBus::default()));

/// 获取全局事件总线实例
pub fn get_event_bus() -> Arc<EventBus> {
    Arc::clone(&EVENT_BUS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_bus_publish_subscribe() {
        let bus = EventBus::new(10);
        let mut subscriber = EventSubscriber::new(bus.subscribe(), "test-subscriber");

        // 发布任务更新事件
        let progress = TaskProgress {
            task_id: "test-task".to_string(),
            task_type: "Import".to_string(),
            target: "/test/path".to_string(),
            status: "running".to_string(),
            message: "Processing...".to_string(),
            progress: 50,
            workspace_id: Some("ws-1".to_string()),
        };

        bus.publish_task_update(progress.clone()).unwrap();

        // 接收事件
        let received = subscriber.recv().await.unwrap();
        match received {
            AppEvent::TaskUpdate(p) => {
                assert_eq!(p.task_id, "test-task");
                assert_eq!(p.progress, 50);
            }
            _ => panic!("Expected TaskUpdate event"),
        }
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let bus = EventBus::new(10);
        let mut sub1 = EventSubscriber::new(bus.subscribe(), "sub1");
        let mut sub2 = EventSubscriber::new(bus.subscribe(), "sub2");

        // subscriber_count 包括内部保留的接收器，所以是 3
        assert_eq!(bus.subscriber_count(), 3);

        // 发布搜索错误事件
        bus.publish_search_error(
            "ws-1".to_string(),
            "test query".to_string(),
            "test error".to_string(),
        )
        .unwrap();

        // 两个订阅者都应该收到事件
        let event1 = sub1.recv().await.unwrap();
        let event2 = sub2.recv().await.unwrap();

        match (event1, event2) {
            (AppEvent::SearchError { .. }, AppEvent::SearchError { .. }) => {}
            _ => panic!("Expected SearchError events"),
        }
    }

    #[test]
    fn test_try_recv_empty() {
        let bus = EventBus::new(10);
        let mut subscriber = EventSubscriber::new(bus.subscribe(), "test");

        // 没有事件时应该返回 None
        assert!(subscriber.try_recv().is_none());
    }

    #[test]
    fn test_global_event_bus() {
        let bus1 = get_event_bus();
        let bus2 = get_event_bus();

        // 应该是同一个实例
        assert!(Arc::ptr_eq(&bus1, &bus2));
    }
}

// ============================================================================
// Property Tests
// ============================================================================

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    // ============================================================================
    // Property 7: Search Error Communication
    // ============================================================================
    // **Feature: bug-fixes, Property 7: Search Error Communication**
    // **Validates: Requirements 2.5**
    //
    // For any search operation error, the system should emit appropriate error
    // events to the frontend

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn property_search_error_communication(
            workspace_id in "[a-zA-Z0-9\\-_]{1,50}",
            query in "[a-zA-Z0-9\\s]{1,100}",
            error_msg in "[a-zA-Z0-9\\s:]{1,200}",
        ) {
            // 创建事件总线
            let bus = EventBus::new(100);
            let mut subscriber = EventSubscriber::new(bus.subscribe(), "test-subscriber");

            // 发布搜索错误事件
            let result = bus.publish_search_error(
                workspace_id.clone(),
                query.clone(),
                error_msg.clone(),
            );

            // 验证事件发布成功
            prop_assert!(result.is_ok(), "Failed to publish search error event");

            // 验证订阅者能接收到事件
            let runtime = tokio::runtime::Runtime::new().unwrap();
            let received = runtime.block_on(async {
                tokio::time::timeout(
                    std::time::Duration::from_millis(100),
                    subscriber.recv()
                ).await
            });

            prop_assert!(received.is_ok(), "Timeout waiting for event");
            let event = received.unwrap();
            prop_assert!(event.is_ok(), "Failed to receive event");

            // 验证事件内容
            match event.unwrap() {
                AppEvent::SearchError {
                    workspace_id: ws_id,
                    query: q,
                    error: err,
                } => {
                    prop_assert_eq!(ws_id, workspace_id, "Workspace ID mismatch");
                    prop_assert_eq!(q, query, "Query mismatch");
                    prop_assert_eq!(err, error_msg, "Error message mismatch");
                }
                _ => {
                    return Err(proptest::test_runner::TestCaseError::fail(
                        "Expected SearchError event"
                    ));
                }
            }
        }

        #[test]
        fn property_search_error_multiple_subscribers(
            workspace_id in "[a-zA-Z0-9\\-_]{1,50}",
            query in "[a-zA-Z0-9\\s]{1,100}",
            error_msg in "[a-zA-Z0-9\\s:]{1,200}",
            subscriber_count in 1usize..10,
        ) {
            // 创建事件总线
            let bus = EventBus::new(100);
            
            // 创建多个订阅者
            let mut subscribers: Vec<EventSubscriber> = (0..subscriber_count)
                .map(|i| EventSubscriber::new(bus.subscribe(), format!("sub-{}", i)))
                .collect();

            // 发布搜索错误事件
            let result = bus.publish_search_error(
                workspace_id.clone(),
                query.clone(),
                error_msg.clone(),
            );

            prop_assert!(result.is_ok(), "Failed to publish search error event");

            // 验证所有订阅者都能接收到事件
            let runtime = tokio::runtime::Runtime::new().unwrap();
            for (i, subscriber) in subscribers.iter_mut().enumerate() {
                let received = runtime.block_on(async {
                    tokio::time::timeout(
                        std::time::Duration::from_millis(100),
                        subscriber.recv()
                    ).await
                });

                prop_assert!(
                    received.is_ok(),
                    "Subscriber {} timeout waiting for event",
                    i
                );
                let event = received.unwrap();
                prop_assert!(
                    event.is_ok(),
                    "Subscriber {} failed to receive event",
                    i
                );

                // 验证事件内容
                match event.unwrap() {
                    AppEvent::SearchError { .. } => {}
                    _ => {
                        return Err(proptest::test_runner::TestCaseError::fail(
                            format!("Subscriber {} received wrong event type", i)
                        ));
                    }
                }
            }
        }

        #[test]
        fn property_system_error_communication(
            component in "[a-zA-Z0-9\\-_]{1,50}",
            error_msg in "[a-zA-Z0-9\\s:]{1,200}",
            has_context in proptest::bool::ANY,
        ) {
            // 创建事件总线
            let bus = EventBus::new(100);
            let mut subscriber = EventSubscriber::new(bus.subscribe(), "test-subscriber");

            let context = if has_context {
                Some("Additional context information".to_string())
            } else {
                None
            };

            // 发布系统错误事件
            let result = bus.publish_system_error(
                component.clone(),
                error_msg.clone(),
                context.clone(),
            );

            // 验证事件发布成功
            prop_assert!(result.is_ok(), "Failed to publish system error event");

            // 验证订阅者能接收到事件
            let runtime = tokio::runtime::Runtime::new().unwrap();
            let received = runtime.block_on(async {
                tokio::time::timeout(
                    std::time::Duration::from_millis(100),
                    subscriber.recv()
                ).await
            });

            prop_assert!(received.is_ok(), "Timeout waiting for event");
            let event = received.unwrap();
            prop_assert!(event.is_ok(), "Failed to receive event");

            // 验证事件内容
            match event.unwrap() {
                AppEvent::SystemError {
                    component: comp,
                    error: err,
                    context: ctx,
                } => {
                    prop_assert_eq!(comp, component, "Component mismatch");
                    prop_assert_eq!(err, error_msg, "Error message mismatch");
                    prop_assert_eq!(ctx, context, "Context mismatch");
                }
                _ => {
                    return Err(proptest::test_runner::TestCaseError::fail(
                        "Expected SystemError event"
                    ));
                }
            }
        }
    }
}

