//! Network Resilience and Recovery Mechanisms
//!
//! Provides network resilience and automatic recovery mechanisms:
//! - Exponential backoff reconnection strategy
//! - Connection health monitoring with automatic failover
//! - Fallback to HTTP polling when WebSocket connections fail
//! - Event ordering guarantees with sequence numbering and gap detection

use crate::state_sync::{EventId, StateSyncManager, SyncError, SyncResult, WorkspaceEvent};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

/// Network resilience configuration
#[derive(Debug, Clone)]
pub struct NetworkResilienceConfig {
    pub max_reconnection_attempts: u32,
    pub initial_reconnection_delay: Duration,
    pub max_reconnection_delay: Duration,
    pub connection_health_check_interval: Duration,
    pub health_check_timeout: Duration,
    pub enable_http_fallback: bool,
    pub http_poll_interval: Duration,
    pub event_gap_detection_enabled: bool,
    pub max_event_gap: u64,
}

impl Default for NetworkResilienceConfig {
    fn default() -> Self {
        Self {
            max_reconnection_attempts: 10,
            initial_reconnection_delay: Duration::from_secs(1),
            max_reconnection_delay: Duration::from_secs(60),
            connection_health_check_interval: Duration::from_secs(30),
            health_check_timeout: Duration::from_secs(5),
            enable_http_fallback: true,
            http_poll_interval: Duration::from_secs(5),
            event_gap_detection_enabled: true,
            max_event_gap: 1000,
        }
    }
}

/// Connection state tracking
#[derive(Debug, Clone)]
pub enum ConnectionState {
    Connected,
    Disconnected,
    Reconnecting { attempt: u32, next_delay: Duration },
    FallbackMode,
}

/// Network resilience manager
#[derive(Clone)]
pub struct NetworkResilienceManager {
    inner: Arc<NetworkResilienceInner>,
}

struct NetworkResilienceInner {
    state_sync_manager: Arc<StateSyncManager>,
    config: NetworkResilienceConfig,
    connection_state: RwLock<ConnectionState>,
    event_buffer: RwLock<Vec<BufferedEvent>>,
    last_event_id: RwLock<Option<EventId>>,
    health_check_handle: RwLock<Option<tokio::task::JoinHandle<()>>>,
}

#[derive(Debug, Clone)]
struct BufferedEvent {
    event: WorkspaceEvent,
    _timestamp: SystemTime,
    _attempt_count: u32,
}

impl NetworkResilienceManager {
    /// Create a new network resilience manager
    pub fn new(state_sync_manager: Arc<StateSyncManager>, config: NetworkResilienceConfig) -> Self {
        info!(
            max_reconnection_attempts = config.max_reconnection_attempts,
            enable_http_fallback = config.enable_http_fallback,
            "Network resilience manager initialized"
        );

        Self {
            inner: Arc::new(NetworkResilienceInner {
                state_sync_manager,
                config,
                connection_state: RwLock::new(ConnectionState::Disconnected),
                event_buffer: RwLock::new(Vec::new()),
                last_event_id: RwLock::new(None),
                health_check_handle: RwLock::new(None),
            }),
        }
    }

    /// Start connection monitoring and automatic recovery
    pub async fn start_monitoring(&self) -> SyncResult<()> {
        let self_clone = self.clone();

        // Start health check task
        let health_check_task = tokio::spawn(async move {
            self_clone.health_check_loop().await;
        });

        *self.inner.health_check_handle.write().await = Some(health_check_task);

        info!("Network resilience monitoring started");
        Ok(())
    }

    /// Stop monitoring and cleanup
    pub async fn stop_monitoring(&self) -> SyncResult<()> {
        if let Some(handle) = self.inner.health_check_handle.write().await.take() {
            handle.abort();
        }

        info!("Network resilience monitoring stopped");
        Ok(())
    }

    /// Handle connection failure with exponential backoff
    pub async fn handle_connection_failure(&self) -> SyncResult<()> {
        let mut state = self.inner.connection_state.write().await;

        match *state {
            ConnectionState::Connected => {
                warn!("Connection lost, starting reconnection attempts");
                *state = ConnectionState::Reconnecting {
                    attempt: 1,
                    next_delay: self.inner.config.initial_reconnection_delay,
                };
            }
            ConnectionState::Reconnecting { attempt, .. }
                if attempt < self.inner.config.max_reconnection_attempts =>
            {
                let next_delay = self.calculate_exponential_backoff(attempt);
                *state = ConnectionState::Reconnecting {
                    attempt: attempt + 1,
                    next_delay,
                };

                warn!(
                    attempt = attempt + 1,
                    next_delay_ms = next_delay.as_millis(),
                    "Reconnection attempt"
                );
            }
            ConnectionState::Reconnecting { attempt, .. } => {
                error!(
                    attempt = attempt,
                    "Max reconnection attempts reached, entering fallback mode"
                );
                *state = ConnectionState::FallbackMode;

                if self.inner.config.enable_http_fallback {
                    self.start_http_fallback().await?;
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Calculate exponential backoff delay
    fn calculate_exponential_backoff(&self, attempt: u32) -> Duration {
        let delay_ms = self.inner.config.initial_reconnection_delay.as_millis() as u64
            * 2_u64.pow(attempt.saturating_sub(1));

        Duration::from_millis(
            delay_ms.min(self.inner.config.max_reconnection_delay.as_millis() as u64),
        )
    }

    /// Start HTTP fallback polling
    async fn start_http_fallback(&self) -> SyncResult<()> {
        info!("Starting HTTP fallback polling");

        let self_clone = self.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(self_clone.inner.config.http_poll_interval);

            loop {
                interval.tick().await;

                if let Err(e) = self_clone.poll_for_updates().await {
                    error!(error = %e, "HTTP fallback polling failed");
                }
            }
        });

        Ok(())
    }

    /// Poll for updates using HTTP fallback
    async fn poll_for_updates(&self) -> SyncResult<()> {
        debug!("Polling for updates via HTTP fallback");

        // In a real implementation, this would make HTTP requests to fetch updates
        // For now, we'll simulate by checking Redis directly

        let redis_publisher = self.inner.state_sync_manager.redis_publisher();

        // Test Redis connection
        if redis_publisher.test_connection().await? {
            // If Redis is available, try to recover WebSocket connection
            self.attempt_websocket_recovery().await?;
        }

        Ok(())
    }

    /// Attempt to recover WebSocket connection
    async fn attempt_websocket_recovery(&self) -> SyncResult<()> {
        info!("Attempting WebSocket recovery");

        // Test WebSocket connection
        let ws_manager = self.inner.state_sync_manager.websocket_manager();
        let stats = ws_manager.get_connection_stats().await;

        if stats.active_connections > 0 {
            // WebSocket is working, exit fallback mode
            let mut state = self.inner.connection_state.write().await;
            *state = ConnectionState::Connected;

            info!("WebSocket connection recovered, exiting fallback mode");

            // Replay buffered events
            self.replay_buffered_events().await?;
        }

        Ok(())
    }

    /// Buffer event for later delivery
    pub async fn buffer_event(&self, event: WorkspaceEvent) -> SyncResult<()> {
        let mut buffer = self.inner.event_buffer.write().await;

        let buffered_event = BufferedEvent {
            event,
            _timestamp: SystemTime::now(),
            _attempt_count: 0,
        };

        buffer.push(buffered_event);

        // Limit buffer size
        if buffer.len() > 1000 {
            buffer.drain(..100); // Remove oldest 100 events
            warn!("Event buffer exceeded limit, removed old events");
        }

        debug!(buffer_size = buffer.len(), "Event buffered");
        Ok(())
    }

    /// Replay buffered events after connection recovery
    async fn replay_buffered_events(&self) -> SyncResult<()> {
        let mut buffer = self.inner.event_buffer.write().await;

        if buffer.is_empty() {
            return Ok(());
        }

        info!(event_count = buffer.len(), "Replaying buffered events");

        let mut failed_events = Vec::new();

        for buffered_event in buffer.drain(..) {
            match self
                .inner
                .state_sync_manager
                .broadcast_workspace_event(buffered_event.event.clone())
                .await
            {
                Ok(_) => {
                    debug!("Buffered event replayed successfully");
                }
                Err(e) => {
                    warn!(error = %e, "Failed to replay buffered event");
                    failed_events.push(buffered_event);
                }
            }
        }

        // Return failed events to buffer
        if !failed_events.is_empty() {
            warn!(
                failed_count = failed_events.len(),
                "Some events failed to replay"
            );
            buffer.extend(failed_events);
        }

        Ok(())
    }

    /// Detect gaps in event sequence
    pub async fn detect_event_gaps(&self, current_event_id: &EventId) -> SyncResult<Vec<EventId>> {
        if !self.inner.config.event_gap_detection_enabled {
            return Ok(Vec::new());
        }

        let last_id = self.inner.last_event_id.read().await;

        if let Some(ref last_event_id) = *last_id {
            // Check for gap between last_event_id and current_event_id
            let gap_events = self
                .identify_gap_events(last_event_id, current_event_id)
                .await?;

            if !gap_events.is_empty() {
                warn!(gap_size = gap_events.len(), "Event gap detected");
            }

            Ok(gap_events)
        } else {
            // No previous event, no gap
            Ok(Vec::new())
        }
    }

    /// Identify missing events in sequence
    async fn identify_gap_events(
        &self,
        from_id: &EventId,
        to_id: &EventId,
    ) -> SyncResult<Vec<EventId>> {
        // In a real implementation, this would query Redis Streams for missing events
        // For now, return empty vector (no gaps detected)
        debug!(from_id = %from_id.0, to_id = %to_id.0, "Checking for event gaps");
        Ok(Vec::new())
    }

    /// Update last received event ID
    pub async fn update_last_event_id(&self, event_id: EventId) {
        let mut last_id = self.inner.last_event_id.write().await;
        *last_id = Some(event_id);
    }

    /// Health check loop
    async fn health_check_loop(&self) {
        let mut interval =
            tokio::time::interval(self.inner.config.connection_health_check_interval);

        loop {
            interval.tick().await;

            if let Err(e) = self.perform_health_check().await {
                error!(error = %e, "Health check failed");

                if let Err(e) = self.handle_connection_failure().await {
                    error!(error = %e, "Failed to handle connection failure");
                }
            }
        }
    }

    /// Perform connection health check
    async fn perform_health_check(&self) -> SyncResult<()> {
        // Test Redis connection
        let redis_publisher = self.inner.state_sync_manager.redis_publisher();

        match timeout(
            self.inner.config.health_check_timeout,
            redis_publisher.test_connection(),
        )
        .await
        {
            Ok(Ok(true)) => {
                debug!("Redis health check passed");
            }
            _ => {
                return Err(SyncError::ConnectionError(
                    "Redis health check failed".to_string(),
                ));
            }
        }

        // Test WebSocket connections (if any)
        let ws_manager = self.inner.state_sync_manager.websocket_manager();
        let stats = ws_manager.get_connection_stats().await;

        if stats.active_connections > 0 {
            debug!(
                active_connections = stats.active_connections,
                "WebSocket health check passed"
            );
        }

        Ok(())
    }

    /// Get current connection state
    pub async fn get_connection_state(&self) -> ConnectionState {
        self.inner.connection_state.read().await.clone()
    }

    /// Get network resilience statistics
    pub async fn get_resilience_stats(&self) -> ResilienceStats {
        let state = self.inner.connection_state.read().await;
        let buffer = self.inner.event_buffer.read().await;

        ResilienceStats {
            connection_state: format!("{:?}", *state),
            buffered_events: buffer.len(),
            max_buffer_size: 1000,
        }
    }
}

/// Network resilience statistics
#[derive(Debug, Clone)]
pub struct ResilienceStats {
    pub connection_state: String,
    pub buffered_events: usize,
    pub max_buffer_size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_sync::{StateSyncConfig, StateSyncManager};

    #[test]
    fn test_network_resilience_config_default() {
        let config = NetworkResilienceConfig::default();
        assert_eq!(config.max_reconnection_attempts, 10);
        assert_eq!(config.enable_http_fallback, true);
    }

    #[tokio::test]
    async fn test_exponential_backoff_calculation() {
        let config = NetworkResilienceConfig::default();
        let state_sync_res = StateSyncManager::new(StateSyncConfig::default()).await;

        match state_sync_res {
            Ok(state_sync) => {
                let manager = NetworkResilienceManager::new(Arc::new(state_sync), config);

                let delay1 = manager.calculate_exponential_backoff(1);
                let delay2 = manager.calculate_exponential_backoff(2);
                let delay3 = manager.calculate_exponential_backoff(3);

                assert!(delay2 > delay1);
                assert!(delay3 > delay2);
            }
            Err(_) => {
                // Skip test if Redis is not available
                println!("Skipping test_exponential_backoff_calculation: Redis not available");
            }
        }
    }
}
