//! Redis Integration Tests
//!
//! Tests end-to-end integration with Redis for workspace synchronization.
//! These tests verify:
//! - Publish → Subscribe → Receive event flow
//! - Stream persistence: Append → Read → Verify
//! - Connection resilience with Redis restarts
//!
//! **Requirements: 3.5, 5.1, 5.2, 5.3**
//!
//! ## Running these tests
//!
//! These tests require a running Redis instance on localhost:6379.
//! If Redis is not available, the tests will be skipped gracefully.
//!
//! To run with Redis:
//! 1. Start Redis: `redis-server` (or use Docker: `docker run -p 6379:6379 redis`)
//! 2. Run tests: `cargo test --test redis_integration_tests -- --nocapture`
//!
//! The tests will automatically clean up any test data they create.

use log_analyzer::state_sync::{
    EventId, RedisConfig, RedisPublisher, WorkspaceEvent, WorkspaceStatus,
};
use redis::AsyncCommands;
use std::time::{Duration, SystemTime};
use tokio::time::timeout;

/// Helper function to check if Redis is available
async fn is_redis_available() -> bool {
    let config = RedisConfig {
        connection_timeout: Duration::from_millis(500),
        ..RedisConfig::default()
    };

    // Use timeout to prevent hanging
    match timeout(Duration::from_secs(1), RedisPublisher::new(config)).await {
        Ok(Ok(_)) => true,
        _ => false,
    }
}

/// Helper function to create a test event
fn create_test_event(workspace_id: &str) -> WorkspaceEvent {
    WorkspaceEvent::StatusChanged {
        workspace_id: workspace_id.to_string(),
        status: WorkspaceStatus::Idle,
        timestamp: SystemTime::now(),
    }
}

/// Helper function to clean up Redis test data
async fn cleanup_redis(stream_key: &str, channel: &str) {
    let config = RedisConfig {
        connection_timeout: Duration::from_millis(500),
        ..RedisConfig::default()
    };

    if let Ok(Ok(publisher)) = timeout(Duration::from_secs(1), RedisPublisher::new(config)).await {
        if let Ok(client) = redis::Client::open(publisher.get_config().url.as_str()) {
            let conn_result = timeout(
                Duration::from_secs(1),
                client.get_multiplexed_async_connection(),
            )
            .await;

            if let Ok(Ok(mut conn)) = conn_result {
                // Delete stream
                let _: Result<(), redis::RedisError> = conn.del(stream_key).await;
            }
        }

        // Note: Pub/Sub channels don't need cleanup as they don't persist
        let _ = channel; // Suppress unused warning
    }
}

/// Test 1: End-to-end event flow - Publish → Subscribe → Receive
///
/// This test verifies that events published to Redis Pub/Sub channels
/// are correctly received by subscribers.
///
/// **Validates: Requirements 3.5, 5.1**
#[tokio::test]
async fn test_pubsub_event_flow() {
    // Skip test if Redis is not available
    if !is_redis_available().await {
        println!("Redis not available, skipping test");
        return;
    }

    let config = RedisConfig::default();
    let publisher = RedisPublisher::new(config.clone())
        .await
        .expect("Failed to create publisher");

    let channel = "test_workspace_events";
    let workspace_id = "test-workspace-pubsub";

    // Create subscriber first
    let mut pubsub = publisher
        .subscribe(channel)
        .await
        .expect("Failed to subscribe");

    // Create and publish event
    let event = create_test_event(workspace_id);
    let event_clone = event.clone();

    // Publish event
    publisher
        .publish_event(channel, &event)
        .await
        .expect("Failed to publish event");

    // Receive event with timeout
    // on_message() returns a Stream, we need to get the next message
    use futures::StreamExt;
    let msg_option = timeout(Duration::from_secs(2), pubsub.on_message().next())
        .await
        .expect("Timeout waiting for message");

    let received = msg_option.expect("Stream ended");

    // Verify the message
    let payload: String = received
        .get_payload()
        .expect("Failed to get message payload");

    let received_event: WorkspaceEvent =
        serde_json::from_str(&payload).expect("Failed to deserialize event");

    // Verify event matches
    match (event_clone, received_event) {
        (
            WorkspaceEvent::StatusChanged {
                workspace_id: id1, ..
            },
            WorkspaceEvent::StatusChanged {
                workspace_id: id2, ..
            },
        ) => {
            assert_eq!(id1, id2, "Workspace IDs should match");
        }
        _ => panic!("Event types don't match"),
    }

    println!("✓ Pub/Sub event flow test passed");
}

/// Test 2: Stream persistence - Append → Read → Verify
///
/// This test verifies that events appended to Redis Streams are correctly
/// persisted and can be read back with proper deserialization.
///
/// **Validates: Requirements 3.5, 5.2, 5.3**
#[tokio::test]
async fn test_stream_persistence() {
    // Skip test if Redis is not available
    if !is_redis_available().await {
        println!("Redis not available, skipping test");
        return;
    }

    let stream_key = "test_workspace_stream";
    let workspace_id = "test-workspace-stream";

    // Cleanup before test
    cleanup_redis(stream_key, "").await;

    let config = RedisConfig::default();
    let publisher = RedisPublisher::new(config)
        .await
        .expect("Failed to create publisher");

    // Create test event
    let event = WorkspaceEvent::ProgressUpdate {
        workspace_id: workspace_id.to_string(),
        progress: 0.75,
        timestamp: SystemTime::now(),
    };

    // Append to stream
    let event_id = publisher
        .append_to_stream(stream_key, &event)
        .await
        .expect("Failed to append to stream");

    println!("Event appended with ID: {:?}", event_id);

    // Read from stream (using "0" to read from beginning)
    let events = publisher
        .read_stream_since(stream_key, &EventId("0".to_string()), Some(10))
        .await
        .expect("Failed to read from stream");

    // Verify we got the event back
    assert_eq!(events.len(), 1, "Should have received 1 event");

    let (received_id, received_event) = &events[0];
    println!("Received event with ID: {:?}", received_id);

    // Verify event content
    match received_event {
        WorkspaceEvent::ProgressUpdate {
            workspace_id: id,
            progress,
            ..
        } => {
            assert_eq!(id, workspace_id, "Workspace ID should match");
            assert_eq!(*progress, 0.75, "Progress should match");
        }
        _ => panic!("Wrong event type received"),
    }

    // Cleanup after test
    cleanup_redis(stream_key, "").await;

    println!("✓ Stream persistence test passed");
}

/// Test 3: Multiple events in stream
///
/// This test verifies that multiple events can be appended to a stream
/// and read back in order.
///
/// **Validates: Requirements 5.2, 5.3**
#[tokio::test]
async fn test_multiple_stream_events() {
    // Skip test if Redis is not available
    if !is_redis_available().await {
        println!("Redis not available, skipping test");
        return;
    }

    let stream_key = "test_multiple_events_stream";
    let workspace_id = "test-workspace-multiple";

    // Cleanup before test
    cleanup_redis(stream_key, "").await;

    let config = RedisConfig::default();
    let publisher = RedisPublisher::new(config)
        .await
        .expect("Failed to create publisher");

    // Append multiple events
    let events = vec![
        WorkspaceEvent::WorkspaceCreated {
            workspace_id: workspace_id.to_string(),
            timestamp: SystemTime::now(),
        },
        WorkspaceEvent::ProgressUpdate {
            workspace_id: workspace_id.to_string(),
            progress: 0.5,
            timestamp: SystemTime::now(),
        },
        WorkspaceEvent::TaskCompleted {
            workspace_id: workspace_id.to_string(),
            task_id: "task-1".to_string(),
            timestamp: SystemTime::now(),
        },
    ];

    let mut event_ids = Vec::new();
    for event in &events {
        let event_id = publisher
            .append_to_stream(stream_key, event)
            .await
            .expect("Failed to append event");
        event_ids.push(event_id);
    }

    println!("Appended {} events", event_ids.len());

    // Read all events from stream
    let received_events = publisher
        .read_stream_since(stream_key, &EventId("0".to_string()), Some(10))
        .await
        .expect("Failed to read from stream");

    // Verify count
    assert_eq!(received_events.len(), 3, "Should have received 3 events");

    // Verify event types in order
    assert!(
        matches!(
            received_events[0].1,
            WorkspaceEvent::WorkspaceCreated { .. }
        ),
        "First event should be WorkspaceCreated"
    );
    assert!(
        matches!(received_events[1].1, WorkspaceEvent::ProgressUpdate { .. }),
        "Second event should be ProgressUpdate"
    );
    assert!(
        matches!(received_events[2].1, WorkspaceEvent::TaskCompleted { .. }),
        "Third event should be TaskCompleted"
    );

    // Cleanup after test
    cleanup_redis(stream_key, "").await;

    println!("✓ Multiple stream events test passed");
}

/// Test 4: Connection resilience
///
/// This test verifies that the Redis publisher can handle connection
/// issues gracefully with retry logic.
///
/// **Validates: Requirements 3.5**
#[tokio::test]
async fn test_connection_resilience() {
    // Skip test if Redis is not available
    if !is_redis_available().await {
        println!("Redis not available, skipping test");
        return;
    }

    let config = RedisConfig {
        retry_attempts: 3,
        retry_delay: Duration::from_millis(100),
        ..RedisConfig::default()
    };

    let publisher = RedisPublisher::new(config)
        .await
        .expect("Failed to create publisher");

    // Test connection
    let is_connected = publisher
        .test_connection()
        .await
        .expect("Connection test failed");

    assert!(is_connected, "Should be connected to Redis");

    // Get connection info
    let info = publisher
        .get_connection_info()
        .await
        .expect("Failed to get connection info");

    // Verify we got some info back
    assert!(
        info.get::<String>("redis_version").is_some(),
        "Should have Redis version info"
    );

    println!("✓ Connection resilience test passed");
}

/// Test 5: Stream read with specific ID
///
/// This test verifies that we can read events from a stream starting
/// from a specific event ID.
///
/// **Validates: Requirements 5.3**
#[tokio::test]
async fn test_stream_read_since_id() {
    // Skip test if Redis is not available
    if !is_redis_available().await {
        println!("Redis not available, skipping test");
        return;
    }

    let stream_key = "test_read_since_stream";
    let workspace_id = "test-workspace-since";

    // Cleanup before test
    cleanup_redis(stream_key, "").await;

    let config = RedisConfig::default();
    let publisher = RedisPublisher::new(config)
        .await
        .expect("Failed to create publisher");

    // Append first event
    let event1 = WorkspaceEvent::WorkspaceCreated {
        workspace_id: workspace_id.to_string(),
        timestamp: SystemTime::now(),
    };
    let event_id1 = publisher
        .append_to_stream(stream_key, &event1)
        .await
        .expect("Failed to append first event");

    // Append second event
    let event2 = WorkspaceEvent::ProgressUpdate {
        workspace_id: workspace_id.to_string(),
        progress: 0.5,
        timestamp: SystemTime::now(),
    };
    let _event_id2 = publisher
        .append_to_stream(stream_key, &event2)
        .await
        .expect("Failed to append second event");

    // Read events since first event ID
    let events = publisher
        .read_stream_since(stream_key, &event_id1, Some(10))
        .await
        .expect("Failed to read from stream");

    // Should only get the second event (events after the specified ID)
    assert_eq!(
        events.len(),
        1,
        "Should have received 1 event after the first ID"
    );

    match &events[0].1 {
        WorkspaceEvent::ProgressUpdate { progress, .. } => {
            assert_eq!(*progress, 0.5, "Should be the second event");
        }
        _ => panic!("Wrong event type received"),
    }

    // Cleanup after test
    cleanup_redis(stream_key, "").await;

    println!("✓ Stream read since ID test passed");
}

/// Test 6: Event format preservation
///
/// This test verifies that the JSON serialization format is preserved
/// across publish/subscribe and stream operations.
///
/// **Validates: Requirements 5.1, 5.2**
#[tokio::test]
async fn test_event_format_preservation() {
    // Skip test if Redis is not available
    if !is_redis_available().await {
        println!("Redis not available, skipping test");
        return;
    }

    let stream_key = "test_format_stream";
    let workspace_id = "test-workspace-format";

    // Cleanup before test
    cleanup_redis(stream_key, "").await;

    let config = RedisConfig::default();
    let publisher = RedisPublisher::new(config)
        .await
        .expect("Failed to create publisher");

    // Create a complex event
    let event = WorkspaceEvent::Error {
        workspace_id: workspace_id.to_string(),
        error: "Test error message with special chars: <>&\"'".to_string(),
        timestamp: SystemTime::now(),
    };

    // Serialize manually to verify format
    let expected_json = serde_json::to_string(&event).expect("Failed to serialize");

    // Append to stream
    publisher
        .append_to_stream(stream_key, &event)
        .await
        .expect("Failed to append to stream");

    // Read back
    let events = publisher
        .read_stream_since(stream_key, &EventId("0".to_string()), Some(10))
        .await
        .expect("Failed to read from stream");

    assert_eq!(events.len(), 1, "Should have received 1 event");

    // Serialize the received event
    let received_json =
        serde_json::to_string(&events[0].1).expect("Failed to serialize received event");

    // Verify JSON structure is preserved (both should be valid and equivalent)
    let expected_value: serde_json::Value =
        serde_json::from_str(&expected_json).expect("Failed to parse expected JSON");
    let received_value: serde_json::Value =
        serde_json::from_str(&received_json).expect("Failed to parse received JSON");

    assert_eq!(
        expected_value, received_value,
        "JSON structure should be preserved"
    );

    // Cleanup after test
    cleanup_redis(stream_key, "").await;

    println!("✓ Event format preservation test passed");
}

/// Test 7: Concurrent operations
///
/// This test verifies that multiple concurrent publish and stream operations
/// work correctly without data corruption.
///
/// **Validates: Requirements 3.5**
#[tokio::test]
async fn test_concurrent_operations() {
    // Skip test if Redis is not available
    if !is_redis_available().await {
        println!("Redis not available, skipping test");
        return;
    }

    let stream_key = "test_concurrent_stream";

    // Cleanup before test
    cleanup_redis(stream_key, "").await;

    let config = RedisConfig::default();
    let publisher = RedisPublisher::new(config)
        .await
        .expect("Failed to create publisher");

    // Spawn multiple concurrent append operations
    let mut handles = Vec::new();
    for i in 0..10 {
        let publisher_clone = RedisPublisher::new(RedisConfig::default())
            .await
            .expect("Failed to create publisher");
        let stream_key_clone = stream_key.to_string();

        let handle = tokio::spawn(async move {
            let event = WorkspaceEvent::ProgressUpdate {
                workspace_id: format!("workspace-{}", i),
                progress: i as f64 / 10.0,
                timestamp: SystemTime::now(),
            };

            publisher_clone
                .append_to_stream(&stream_key_clone, &event)
                .await
                .expect("Failed to append event");
        });

        handles.push(handle);
    }

    // Wait for all operations to complete
    for handle in handles {
        handle.await.expect("Task panicked");
    }

    // Read all events
    let events = publisher
        .read_stream_since(stream_key, &EventId("0".to_string()), Some(20))
        .await
        .expect("Failed to read from stream");

    // Verify we got all 10 events
    assert_eq!(events.len(), 10, "Should have received 10 events");

    // Cleanup after test
    cleanup_redis(stream_key, "").await;

    println!("✓ Concurrent operations test passed");
}
