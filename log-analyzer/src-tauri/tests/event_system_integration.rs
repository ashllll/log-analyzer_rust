//! Event system integration tests
//!
//! Tests the tokio::sync::broadcast event system and error communication

use log_analyzer::events::{emit_event, get_event_bus, AppEvent, EventBus};
use proptest::prelude::*;
use std::time::Duration;
use tokio::time::timeout;

/// Test that search error events are properly communicated
#[tokio::test]
async fn test_search_error_communication() {
    let bus = EventBus::new();
    let mut receiver = bus.subscribe("test_search_errors".to_string());

    let error_message = "Search failed due to invalid query";
    let event = AppEvent::SearchError {
        error: error_message.to_string(),
    };

    // Emit the error event
    let result = bus.emit(event.clone());
    assert!(result.is_ok());
    // 至少有1个订阅者（可能更多由于内部监控）
    assert!(result.unwrap() >= 1, "Should have at least 1 subscriber");

    // Receive the event
    let received = timeout(Duration::from_millis(100), receiver.recv()).await;
    assert!(received.is_ok());

    let received_event = received.unwrap().unwrap();
    match received_event {
        AppEvent::SearchError { error } => {
            assert_eq!(error, error_message);
        }
        _ => panic!("Received wrong event type"),
    }
}

/// Test that frontend error messages are properly formatted and communicated
#[tokio::test]
async fn test_frontend_error_messages() {
    let bus = EventBus::new();
    let mut receiver = bus.subscribe("test_frontend_errors".to_string());

    let error_message = "Component failed to render";
    let context = Some("SearchPage".to_string());
    let event = AppEvent::SystemError {
        error: error_message.to_string(),
        context,
    };

    // Emit the error event
    let result = bus.emit(event.clone());
    assert!(result.is_ok());

    // Receive the event
    let received = timeout(Duration::from_millis(100), receiver.recv()).await;
    assert!(received.is_ok());

    let received_event = received.unwrap().unwrap();
    match received_event {
        AppEvent::SystemError { error, context } => {
            assert_eq!(error, error_message);
            assert_eq!(context, Some("SearchPage".to_string()));
        }
        _ => panic!("Received wrong event type"),
    }
}

/// Test event bus statistics tracking
#[tokio::test]
async fn test_event_statistics() {
    let bus = EventBus::new();
    let _receiver1 = bus.subscribe("subscriber1".to_string());
    let _receiver2 = bus.subscribe("subscriber2".to_string());

    // Send different types of events
    let _ = bus.emit(AppEvent::SearchStart {
        message: "test".to_string(),
    });
    let _ = bus.emit(AppEvent::SearchProgress { progress: 50 });
    let _ = bus.emit(AppEvent::SearchError {
        error: "test error".to_string(),
    });

    let stats = bus.get_stats();
    assert_eq!(stats.total_events_sent, 3);
    assert_eq!(stats.events_by_type.get("SearchStart"), Some(&1));
    assert_eq!(stats.events_by_type.get("SearchProgress"), Some(&1));
    assert_eq!(stats.events_by_type.get("SearchError"), Some(&1));
}

/// Test multiple subscribers receive the same event
#[tokio::test]
async fn test_multiple_subscribers() {
    let bus = EventBus::new();
    let mut receiver1 = bus.subscribe("subscriber1".to_string());
    let mut receiver2 = bus.subscribe("subscriber2".to_string());
    let mut receiver3 = bus.subscribe("subscriber3".to_string());

    let event = AppEvent::SearchComplete { count: 42 };
    let result = bus.emit(event);
    assert!(result.is_ok());
    // 至少有3个订阅者（可能更多由于内部监控）
    assert!(result.unwrap() >= 3, "Should have at least 3 subscribers");

    // All receivers should get the event
    let received1 = timeout(Duration::from_millis(100), receiver1.recv()).await;
    let received2 = timeout(Duration::from_millis(100), receiver2.recv()).await;
    let received3 = timeout(Duration::from_millis(100), receiver3.recv()).await;

    assert!(received1.is_ok());
    assert!(received2.is_ok());
    assert!(received3.is_ok());

    // All should receive the same event
    for received in [received1, received2, received3] {
        match received.unwrap().unwrap() {
            AppEvent::SearchComplete { count } => {
                assert_eq!(count, 42);
            }
            _ => panic!("Received wrong event type"),
        }
    }
}

proptest! {
    /// **Feature: bug-fixes, Property 7: Search Error Communication**
    /// **Validates: Requirements 2.5**
    #[test]
    fn prop_search_error_communication(error_message in "\\PC{1,100}") {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let bus = EventBus::new();
            let mut receiver = bus.subscribe("prop_test_search_errors".to_string());

            let event = AppEvent::SearchError {
                error: error_message.clone()
            };

            // Emit the error event
            let result = bus.emit(event);
            prop_assert!(result.is_ok());

            // Receive the event within timeout
            let received = timeout(Duration::from_millis(100), receiver.recv()).await;
            prop_assert!(received.is_ok());

            let received_event = received.unwrap().unwrap();
            match received_event {
                AppEvent::SearchError { error } => {
                    prop_assert_eq!(error, error_message);
                    Ok(())
                }
                _ => Err(proptest::test_runner::TestCaseError::fail("Wrong event type")),
            }
        })?;
    }
}

proptest! {
    /// **Feature: bug-fixes, Property 28: Frontend Error Messages**
    /// **Validates: Requirements 7.2**
    #[test]
    fn prop_frontend_error_messages(
        error_message in "\\PC{1,100}",
        context in prop::option::of("\\PC{1,50}")
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let bus = EventBus::new();
            let mut receiver = bus.subscribe("prop_test_frontend_errors".to_string());

            let event = AppEvent::SystemError {
                error: error_message.clone(),
                context: context.clone()
            };

            // Emit the error event
            let result = bus.emit(event);
            prop_assert!(result.is_ok());

            // Receive the event within timeout
            let received = timeout(Duration::from_millis(100), receiver.recv()).await;
            prop_assert!(received.is_ok());

            let received_event = received.unwrap().unwrap();
            match received_event {
                AppEvent::SystemError { error, context: recv_context } => {
                    prop_assert_eq!(error, error_message);
                    prop_assert_eq!(recv_context, context);
                    Ok(())
                }
                _ => Err(proptest::test_runner::TestCaseError::fail("Wrong event type")),
            }
        })?;
    }
}

proptest! {
    #[test]
    fn prop_event_ordering(events in prop::collection::vec(
        prop_oneof![
            "\\PC{1,50}".prop_map(|msg| AppEvent::SearchStart { message: msg }),
            (0i32..100).prop_map(|p| AppEvent::SearchProgress { progress: p }),
            "\\PC{1,50}".prop_map(|err| AppEvent::SearchError { error: err }),
            (0usize..1000).prop_map(|c| AppEvent::SearchComplete { count: c }),
        ],
        1..10
    )) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let bus = EventBus::new();
            let mut receiver = bus.subscribe("prop_test_ordering".to_string());

            // Send all events
            for event in &events {
                let result = bus.emit(event.clone());
                prop_assert!(result.is_ok());
            }

            // Receive all events in order
            for expected_event in &events {
                let received = timeout(Duration::from_millis(100), receiver.recv()).await;
                prop_assert!(received.is_ok());

                let received_event = received.unwrap().unwrap();

                // Check that events match (simplified comparison)
                match (expected_event, &received_event) {
                    (AppEvent::SearchStart { message: m1 }, AppEvent::SearchStart { message: m2 }) => {
                        prop_assert_eq!(m1, m2);
                    }
                    (AppEvent::SearchProgress { progress: p1 }, AppEvent::SearchProgress { progress: p2 }) => {
                        prop_assert_eq!(p1, p2);
                    }
                    (AppEvent::SearchError { error: e1 }, AppEvent::SearchError { error: e2 }) => {
                        prop_assert_eq!(e1, e2);
                    }
                    (AppEvent::SearchComplete { count: c1 }, AppEvent::SearchComplete { count: c2 }) => {
                        prop_assert_eq!(c1, c2);
                    }
                    _ => return Err(proptest::test_runner::TestCaseError::fail("Event type mismatch")),
                }
            }

            Ok(())
        })?;
    }
}

/// Test event bus subscriber management
#[tokio::test]
async fn test_subscriber_management() {
    let bus = EventBus::new();

    // Initially no subscribers
    assert_eq!(bus.get_subscribers().len(), 0);

    // Add subscribers
    let _receiver1 = bus.subscribe("sub1".to_string());
    let _receiver2 = bus.subscribe("sub2".to_string());

    let subscribers = bus.get_subscribers();
    assert_eq!(subscribers.len(), 2);
    assert!(subscribers.contains(&"sub1".to_string()));
    assert!(subscribers.contains(&"sub2".to_string()));

    // Remove subscriber
    bus.unsubscribe("sub1");
    let subscribers = bus.get_subscribers();
    assert_eq!(subscribers.len(), 1);
    assert!(subscribers.contains(&"sub2".to_string()));
    assert!(!subscribers.contains(&"sub1".to_string()));
}

/// Test event bus with high load
#[tokio::test]
async fn test_high_load_events() {
    let bus = EventBus::new();
    let mut receiver = bus.subscribe("high_load_test".to_string());

    let num_events = 1000;

    // Send many events rapidly
    for i in 0..num_events {
        let event = AppEvent::SearchProgress { progress: i as i32 };
        let result = bus.emit(event);
        assert!(result.is_ok());
    }

    // Receive all events
    for i in 0..num_events {
        let received = timeout(Duration::from_millis(10), receiver.recv()).await;
        assert!(received.is_ok(), "Failed to receive event {}", i);

        match received.unwrap().unwrap() {
            AppEvent::SearchProgress { progress } => {
                assert_eq!(progress, i as i32);
            }
            _ => panic!("Received wrong event type at index {}", i),
        }
    }

    let stats = bus.get_stats();
    assert_eq!(stats.total_events_sent, num_events as u64);
}

/// Test global event bus functionality
#[tokio::test]
async fn test_global_event_bus() {
    // Clear any existing stats
    get_event_bus().clear_stats();

    let mut receiver = get_event_bus().subscribe("global_test".to_string());

    let event = AppEvent::SystemInfo {
        info: "Global event bus test".to_string(),
        context: None,
    };

    // Use the global emit function
    let result = emit_event(event.clone());
    assert!(result.is_ok());

    // Receive the event
    let received = timeout(Duration::from_millis(100), receiver.recv()).await;
    assert!(received.is_ok());

    let received_event = received.unwrap().unwrap();
    match received_event {
        AppEvent::SystemInfo { info, context } => {
            assert_eq!(info, "Global event bus test");
            assert_eq!(context, None);
        }
        _ => panic!("Received wrong event type"),
    }

    // Check global stats
    let stats = get_event_bus().get_stats();
    assert!(stats.total_events_sent >= 1);
}
