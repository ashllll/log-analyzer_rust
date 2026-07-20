//! Cross-language wire-format contract tests for `workspace-event`.
//!
//! The JSON fixture at `log-analyzer/src/events/__fixtures__/workspace-event-contract.json`
//! is the single source of truth for the wire shape. These tests pin the
//! backend (serde) side: any drift in `WorkspaceEvent` / `WorkspaceStatus`
//! serialization fails here; the frontend pins the same fixture against its
//! zod schema in `src/events/__tests__/workspaceEventContract.test.ts`.

#[cfg(test)]
mod tests {
    use crate::state_sync::{WorkspaceEvent, WorkspaceStatus};
    use std::time::{Duration, SystemTime};

    const FIXTURE_JSON: &str =
        include_str!("../../../src/events/__fixtures__/workspace-event-contract.json");

    fn fixture_entries() -> Vec<serde_json::Value> {
        serde_json::from_str(FIXTURE_JSON).expect("contract fixture must be valid JSON")
    }

    /// Production-constructible events, in the same order as the fixture.
    fn production_events() -> Vec<WorkspaceEvent> {
        let ws = || "ws-contract".to_string();
        vec![
            WorkspaceEvent::StatusChanged {
                workspace_id: ws(),
                status: WorkspaceStatus::Idle,
            },
            WorkspaceEvent::StatusChanged {
                workspace_id: ws(),
                status: WorkspaceStatus::Processing {
                    started_at: SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000),
                },
            },
            WorkspaceEvent::StatusChanged {
                workspace_id: ws(),
                status: WorkspaceStatus::Completed {
                    duration: Duration::from_secs(0),
                },
            },
            WorkspaceEvent::StatusChanged {
                workspace_id: ws(),
                status: WorkspaceStatus::Failed {
                    error: "disk full".to_string(),
                    failed_at: SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_001),
                },
            },
            WorkspaceEvent::StatusChanged {
                workspace_id: ws(),
                status: WorkspaceStatus::Cancelled {
                    cancelled_at: SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_002),
                },
            },
        ]
    }

    #[test]
    fn backend_serialization_matches_contract_fixture() {
        let entries = fixture_entries();
        let events = production_events();
        assert_eq!(
            entries.len(),
            events.len(),
            "fixture and production event list out of sync"
        );

        for (idx, (expected, event)) in entries.iter().zip(events.iter()).enumerate() {
            let actual = serde_json::to_value(event).expect("event must serialize");
            assert_eq!(
                &actual, expected,
                "wire shape drifted from contract fixture at index {idx}"
            );
        }
    }

    #[test]
    fn contract_fixture_deserializes_and_round_trips() {
        for (idx, entry) in fixture_entries().iter().enumerate() {
            let event: WorkspaceEvent = serde_json::from_value(entry.clone())
                .unwrap_or_else(|e| panic!("fixture entry {idx} must deserialize: {e}"));
            let round_tripped = serde_json::to_value(&event).expect("event must re-serialize");
            assert_eq!(
                &round_tripped, entry,
                "fixture entry {idx} must round-trip losslessly"
            );
        }
    }
}
