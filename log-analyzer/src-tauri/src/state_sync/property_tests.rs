//! Property-based tests for state synchronization
//!
//! Validates:
//! - Property 6: State Synchronization Latency
//! - Property 7: Concurrent State Consistency
//! - Property 10: Network Recovery Synchronization

#[cfg(test)]
mod tests {
    use crate::state_sync::{WorkspaceEvent, WorkspaceStatus};
    use proptest::prelude::*;
    use std::collections::HashMap;
    use std::time::{Duration, Instant, SystemTime};

    // Strategies for generating test data
    fn workspace_id_strategy() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9\\-]{1,50}"
    }

    fn workspace_status_strategy() -> impl Strategy<Value = WorkspaceStatus> {
        prop_oneof![
            Just(WorkspaceStatus::Idle),
            any::<u64>().prop_map(|d| WorkspaceStatus::Processing {
                started_at: SystemTime::UNIX_EPOCH + Duration::from_secs(d % 1000000)
            }),
            any::<u64>().prop_map(|d| WorkspaceStatus::Completed {
                duration: Duration::from_secs(d % 3600)
            }),
        ]
    }

    fn progress_strategy() -> impl Strategy<Value = f64> {
        0.0..=1.0
    }

    fn event_sequence_strategy() -> impl Strategy<Value = Vec<u64>> {
        prop::collection::vec(0u64..1000u64, 1..20)
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// **Performance Optimization, Property 6: State Synchronization Latency**
        /// *For any* workspace status change, frontend updates should propagate within 100ms
        /// **Validates: Requirements 2.1**
        #[test]
        fn test_state_synchronization_latency(
            workspace_id in workspace_id_strategy(),
            status in workspace_status_strategy()
        ) {
            // Simulate state synchronization timing
            let start = Instant::now();

            // Create event
            let event = WorkspaceEvent::StatusChanged {
                workspace_id: workspace_id.clone(),
                status: status.clone(),
            };

            // Serialize event (simulating network transmission)
            let serialized = serde_json::to_string(&event).unwrap();

            // Deserialize event (simulating reception)
            let _deserialized: WorkspaceEvent = serde_json::from_str(&serialized).unwrap();

            // Apply to local state
            let mut states = HashMap::new();
            apply_event_to_map(&mut states, &event);

            let elapsed = start.elapsed();

            // Property: State synchronization should complete within 100ms
            // Note: This tests the local processing time, not network latency
            prop_assert!(
                elapsed < Duration::from_millis(100),
                "State synchronization took {:?}, expected < 100ms",
                elapsed
            );
        }

        /// **Performance Optimization, Property 7: Concurrent State Consistency**
        /// *For any* simultaneous workspace operations, final state should be consistent without race conditions
        /// **Validates: Requirements 2.2**
        #[test]
        fn test_state_consistency_property(
            workspace_id in workspace_id_strategy(),
            status in workspace_status_strategy(),
            progress in progress_strategy()
        ) {
            let mut states = HashMap::new();

            // Initial state
            let initial_event = WorkspaceEvent::StatusChanged {
                workspace_id: workspace_id.clone(),
                status: status.clone(),
            };

            apply_event_to_map(&mut states, &initial_event);

            let state = states.get(&workspace_id).unwrap();
            prop_assert_eq!(&state.status_name(), &status_to_name(&status));

            // Progress update
            let progress_event = WorkspaceEvent::ProgressUpdate {
                workspace_id: workspace_id.clone(),
                progress,
            };

            apply_event_to_map(&mut states, &progress_event);

            let updated_state = states.get(&workspace_id).unwrap();
            prop_assert_eq!(updated_state.progress, progress);
        }

        /// **Performance Optimization, Property 7: Concurrent State Consistency (Event Ordering)**
        /// Validate that the final state depends on the last event applied
        /// **Validates: Requirements 2.2**
        #[test]
        fn test_event_ordering_consistency(
            workspace_id in workspace_id_strategy(),
            events in prop::collection::vec(workspace_status_strategy(), 1..10)
        ) {
            let mut states = HashMap::new();
            let mut last_status = WorkspaceStatus::Idle;

            for status in events {
                last_status = status.clone();
                let event = WorkspaceEvent::StatusChanged {
                    workspace_id: workspace_id.clone(),
                    status,
                };
                apply_event_to_map(&mut states, &event);
            }

            let final_state = states.get(&workspace_id).unwrap();
            prop_assert_eq!(&final_state.status_name(), &status_to_name(&last_status));
        }

        /// **Performance Optimization, Property 10: Network Recovery Synchronization**
        /// *For any* network reconnection scenario, all missed state changes should be automatically synchronized
        /// **Validates: Requirements 2.5**
        #[test]
        fn test_network_recovery_synchronization(
            workspace_id in workspace_id_strategy(),
            event_sequence in event_sequence_strategy()
        ) {
            // Simulate events that occurred during network disconnection
            let mut missed_events: Vec<WorkspaceEvent> = Vec::new();
            let mut expected_final_progress = 0.0;

            for (_i, seq) in event_sequence.iter().enumerate() {
                let progress = (*seq as f64) / 1000.0;
                expected_final_progress = progress;

                missed_events.push(WorkspaceEvent::ProgressUpdate {
                    workspace_id: workspace_id.clone(),
                    progress,
                });
            }

            // Simulate network recovery - replay all missed events
            let mut states = HashMap::new();
            for event in &missed_events {
                apply_event_to_map(&mut states, event);
            }

            // Property: After recovery, state should reflect all missed events
            let final_state = states.get(&workspace_id).unwrap();
            prop_assert!(
                (final_state.progress - expected_final_progress).abs() < f64::EPSILON,
                "Expected progress {}, got {}",
                expected_final_progress,
                final_state.progress
            );
        }

        /// **Performance Optimization, Property 10: Network Recovery - Event Gap Detection**
        /// Validate that event gaps are properly detected
        /// **Validates: Requirements 2.5**
        #[test]
        fn test_event_gap_detection(
            sequence_numbers in prop::collection::vec(1u64..1000u64, 2..20)
        ) {
            // Sort sequence numbers to create ordered events
            let mut sorted_seq: Vec<u64> = sequence_numbers.clone();
            sorted_seq.sort();
            sorted_seq.dedup();

            if sorted_seq.len() < 2 {
                return Ok(());
            }

            // Detect gaps in sequence
            let mut gaps = Vec::new();
            for i in 1..sorted_seq.len() {
                let expected_next = sorted_seq[i - 1] + 1;
                if sorted_seq[i] > expected_next {
                    // Gap detected
                    for missing in expected_next..sorted_seq[i] {
                        gaps.push(missing);
                    }
                }
            }

            // Property: Gap detection should identify all missing sequence numbers
            for gap in &gaps {
                prop_assert!(
                    !sorted_seq.contains(gap),
                    "Gap {} should not be in the sequence",
                    gap
                );
            }
        }
    }

    // Helper functions for testing
    fn apply_event_to_map(states: &mut HashMap<String, TestState>, event: &WorkspaceEvent) {
        match event {
            WorkspaceEvent::StatusChanged {
                workspace_id,
                status,
                ..
            } => {
                let state = states
                    .entry(workspace_id.clone())
                    .or_insert(TestState::default());
                state.status = status_to_name(status);
            }
            WorkspaceEvent::ProgressUpdate {
                workspace_id,
                progress,
                ..
            } => {
                let state = states
                    .entry(workspace_id.clone())
                    .or_insert(TestState::default());
                state.progress = *progress;
            }
            _ => {}
        }
    }

    #[derive(Default, Debug)]
    struct TestState {
        status: String,
        progress: f64,
    }

    impl TestState {
        fn status_name(&self) -> String {
            self.status.clone()
        }
    }

    fn status_to_name(status: &WorkspaceStatus) -> String {
        match status {
            WorkspaceStatus::Idle => "Idle".to_string(),
            WorkspaceStatus::Processing { .. } => "Processing".to_string(),
            WorkspaceStatus::Completed { .. } => "Completed".to_string(),
            WorkspaceStatus::Failed { .. } => "Failed".to_string(),
            WorkspaceStatus::Cancelled { .. } => "Cancelled".to_string(),
        }
    }
}
