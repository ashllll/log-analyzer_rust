//! WebSocket Manager
//!
//! WebSocket server infrastructure with connection lifecycle management,
//! user session management, connection authentication, and message routing.
//!
//! Requirements: 2.1, 6.2

use crate::state_sync::{SyncError, SyncResult, UserId};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tracing::{debug, error, info, warn};

/// WebSocket connection sender for broadcasting messages
pub type WebSocketSender = mpsc::UnboundedSender<Message>;

/// WebSocket connection configuration
#[derive(Debug, Clone)]
pub struct WebSocketConfig {
    pub max_connections: usize,
    pub ping_interval: Duration,
    pub connection_timeout: Duration,
    pub max_message_size: usize,
    pub require_authentication: bool,
    pub auth_timeout: Duration,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            max_connections: 1000,
            ping_interval: Duration::from_secs(30),
            connection_timeout: Duration::from_secs(60),
            max_message_size: 10 * 1024 * 1024, // 10MB
            require_authentication: false,
            auth_timeout: Duration::from_secs(10),
        }
    }
}

/// Authentication token for WebSocket connections
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthToken {
    pub token: String,
    pub user_id: String,
    pub expires_at: Option<SystemTime>,
}

/// Authentication request message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthRequest {
    pub token: String,
    pub client_info: Option<String>,
}

/// Authentication response message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub success: bool,
    pub user_id: Option<String>,
    pub error: Option<String>,
}

/// WebSocket message types for state synchronization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WebSocketMessage {
    /// Workspace state update
    WorkspaceUpdate {
        workspace_id: String,
        state: serde_json::Value,
    },
    /// Event notification
    EventNotification {
        event_id: String,
        event_type: String,
        payload: serde_json::Value,
    },
    /// Connection acknowledgment
    ConnectionAck {
        user_id: String,
        connected_at: SystemTime,
    },
    /// Error notification
    Error {
        code: String,
        message: String,
    },
    /// Ping/Pong for connection health
    Ping,
    Pong,
    /// Authentication request
    AuthRequest(AuthRequest),
    /// Authentication response
    AuthResponse(AuthResponse),
    /// Subscription request for workspace events
    Subscribe {
        workspace_ids: Vec<String>,
    },
    /// Unsubscription request
    Unsubscribe {
        workspace_ids: Vec<String>,
    },
}

/// WebSocket connection information
#[derive(Debug, Clone)]
struct WebSocketConnection {
    _user_id: UserId,
    sender: WebSocketSender,
    _connected_at: SystemTime,
    last_activity: SystemTime,
    authenticated: bool,
    subscribed_workspaces: Vec<String>,
}

/// Authentication validator trait for custom authentication logic
pub trait AuthValidator: Send + Sync {
    fn validate(&self, token: &str) -> Option<String>;
}

/// Default authentication validator (accepts all tokens)
pub struct DefaultAuthValidator;

impl AuthValidator for DefaultAuthValidator {
    fn validate(&self, token: &str) -> Option<String> {
        // Default: accept any non-empty token and use it as user_id
        if token.is_empty() {
            None
        } else {
            Some(token.to_string())
        }
    }
}

/// WebSocket manager with connection lifecycle management
pub struct WebSocketManager {
    config: WebSocketConfig,
    connections: Arc<RwLock<HashMap<UserId, WebSocketConnection>>>,
    connection_count: Arc<RwLock<usize>>,
    auth_validator: Arc<dyn AuthValidator>,
}

impl WebSocketManager {
    /// Create a new WebSocket manager
    pub fn new(config: WebSocketConfig) -> Self {
        info!(
            max_connections = config.max_connections,
            ping_interval_ms = config.ping_interval.as_millis(),
            require_auth = config.require_authentication,
            "WebSocket manager initialized"
        );

        Self {
            config,
            connections: Arc::new(RwLock::new(HashMap::new())),
            connection_count: Arc::new(RwLock::new(0)),
            auth_validator: Arc::new(DefaultAuthValidator),
        }
    }

    /// Create a new WebSocket manager with custom authentication validator
    pub fn with_auth_validator(config: WebSocketConfig, validator: Arc<dyn AuthValidator>) -> Self {
        info!(
            max_connections = config.max_connections,
            ping_interval_ms = config.ping_interval.as_millis(),
            require_auth = config.require_authentication,
            "WebSocket manager initialized with custom auth validator"
        );

        Self {
            config,
            connections: Arc::new(RwLock::new(HashMap::new())),
            connection_count: Arc::new(RwLock::new(0)),
            auth_validator: validator,
        }
    }

    /// Handle new WebSocket connection
    pub async fn handle_connection(
        &self,
        user_id: UserId,
        ws_stream: WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>,
    ) -> SyncResult<()> {
        // Check connection limit
        {
            let count = *self.connection_count.read().await;
            if count >= self.config.max_connections {
                return Err(SyncError::ConnectionError(format!(
                    "Maximum connections reached: {}",
                    self.config.max_connections
                )));
            }
        }

        // Split WebSocket stream
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        // Create channel for sending messages to this connection
        let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

        // Store connection information
        let connection = WebSocketConnection {
            _user_id: user_id.clone(),
            sender: tx.clone(),
            _connected_at: SystemTime::now(),
            last_activity: SystemTime::now(),
            authenticated: !self.config.require_authentication, // Auto-authenticated if auth not required
            subscribed_workspaces: Vec::new(),
        };

        {
            let mut connections = self.connections.write().await;
            connections.insert(user_id.clone(), connection);
            *self.connection_count.write().await += 1;
        }

        info!(
            user_id = %user_id.0,
            total_connections = *self.connection_count.read().await,
            "WebSocket connection established"
        );

        // Send connection acknowledgment
        let ack_message = WebSocketMessage::ConnectionAck {
            user_id: user_id.0.clone(),
            connected_at: SystemTime::now(),
        };

        if let Ok(json_message) = serde_json::to_string(&ack_message) {
            let _ = tx.send(Message::Text(json_message));
        }

        // Spawn task for handling outgoing messages
        let user_id_clone = user_id.clone();
        let connections_clone = self.connections.clone();

        tokio::spawn(async move {
            while let Some(message) = rx.recv().await {
                if let Err(e) = ws_sender.send(message).await {
                    error!(user_id = %user_id_clone.0, error = %e, "Failed to send WebSocket message");
                    break;
                }
            }

            // Remove connection when sender is dropped
            let mut connections = connections_clone.write().await;
            connections.remove(&user_id_clone);
        });

        // Handle incoming messages
        while let Some(message) = ws_receiver.next().await {
            match message {
                Ok(Message::Text(text)) => {
                    debug!(user_id = %user_id.0, message = %text, "Received WebSocket message");

                    // Update last activity
                    {
                        let mut connections = self.connections.write().await;
                        if let Some(conn) = connections.get_mut(&user_id) {
                            conn.last_activity = SystemTime::now();
                        }
                    }

                    // Handle ping/pong
                    if text == "ping" {
                        let _ = tx.send(Message::Text("pong".to_string()));
                        continue;
                    }

                    // Try to parse as WebSocketMessage
                    if let Ok(ws_msg) = serde_json::from_str::<WebSocketMessage>(&text) {
                        match ws_msg {
                            WebSocketMessage::AuthRequest(auth_req) => {
                                let auth_response =
                                    self.handle_auth_request(&user_id, &auth_req).await;
                                if let Ok(json) = serde_json::to_string(
                                    &WebSocketMessage::AuthResponse(auth_response),
                                ) {
                                    let _ = tx.send(Message::Text(json));
                                }
                            }
                            WebSocketMessage::Subscribe { workspace_ids } => {
                                self.handle_subscribe(&user_id, workspace_ids).await;
                            }
                            WebSocketMessage::Unsubscribe { workspace_ids } => {
                                self.handle_unsubscribe(&user_id, workspace_ids).await;
                            }
                            WebSocketMessage::Ping => {
                                if let Ok(json) = serde_json::to_string(&WebSocketMessage::Pong) {
                                    let _ = tx.send(Message::Text(json));
                                }
                            }
                            _ => {
                                debug!(user_id = %user_id.0, "Received unhandled message type");
                            }
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    info!(user_id = %user_id.0, "WebSocket connection closed by client");
                    break;
                }
                Ok(Message::Ping(_)) => {
                    // Respond with pong automatically handled by tungstenite
                }
                Err(e) => {
                    error!(user_id = %user_id.0, error = %e, "WebSocket error");
                    break;
                }
                _ => {}
            }
        }

        // Clean up connection
        {
            let mut connections = self.connections.write().await;
            connections.remove(&user_id);
            *self.connection_count.write().await -= 1;
        }

        info!(
            user_id = %user_id.0,
            remaining_connections = *self.connection_count.read().await,
            "WebSocket connection cleaned up"
        );

        Ok(())
    }

    /// Broadcast message to all connected clients
    pub async fn broadcast(&self, message: WebSocketMessage) -> SyncResult<usize> {
        let json_message = serde_json::to_string(&message)
            .map_err(|e| SyncError::SerializationError(e.to_string()))?;

        let connections = self.connections.read().await;
        let mut success_count = 0;

        for (user_id, connection) in connections.iter() {
            let ws_message = Message::Text(json_message.clone());

            match connection.sender.send(ws_message) {
                Ok(_) => success_count += 1,
                Err(e) => {
                    warn!(user_id = %user_id.0, error = %e, "Failed to broadcast message to user");
                }
            }
        }

        debug!(
            message_type = std::any::type_name::<WebSocketMessage>(),
            recipients = connections.len(),
            successful = success_count,
            "Broadcast completed"
        );

        Ok(success_count)
    }

    /// Send message to specific user
    pub async fn send_to_user(
        &self,
        user_id: &UserId,
        message: WebSocketMessage,
    ) -> SyncResult<()> {
        let connections = self.connections.read().await;

        if let Some(connection) = connections.get(user_id) {
            let json_message = serde_json::to_string(&message)
                .map_err(|e| SyncError::SerializationError(e.to_string()))?;

            connection
                .sender
                .send(Message::Text(json_message))
                .map_err(|e| SyncError::WebSocketError(e.to_string()))?;

            Ok(())
        } else {
            Err(SyncError::ConnectionError(format!(
                "User not connected: {}",
                user_id.0
            )))
        }
    }

    /// Get current connection statistics
    pub async fn get_connection_stats(&self) -> ConnectionStats {
        let connections = self.connections.read().await;
        let active_connections = connections.len();

        let mut user_ids = Vec::new();
        for (user_id, _) in connections.iter() {
            user_ids.push(user_id.clone());
        }

        ConnectionStats {
            active_connections,
            max_connections: self.config.max_connections,
            user_ids,
        }
    }

    /// Disconnect a specific user
    pub async fn disconnect_user(&self, user_id: &UserId) -> SyncResult<()> {
        let mut connections = self.connections.write().await;

        if connections.remove(user_id).is_some() {
            *self.connection_count.write().await -= 1;
            info!(user_id = %user_id.0, "User disconnected");
            Ok(())
        } else {
            Err(SyncError::ConnectionError(format!(
                "User not found: {}",
                user_id.0
            )))
        }
    }

    /// Get WebSocket configuration
    pub fn get_config(&self) -> &WebSocketConfig {
        &self.config
    }

    /// Handle authentication request
    async fn handle_auth_request(&self, user_id: &UserId, auth_req: &AuthRequest) -> AuthResponse {
        if let Some(validated_user_id) = self.auth_validator.validate(&auth_req.token) {
            // Update connection as authenticated
            let mut connections = self.connections.write().await;
            if let Some(conn) = connections.get_mut(user_id) {
                conn.authenticated = true;
            }

            info!(user_id = %user_id.0, validated_user = %validated_user_id, "User authenticated successfully");

            AuthResponse {
                success: true,
                user_id: Some(validated_user_id),
                error: None,
            }
        } else {
            warn!(user_id = %user_id.0, "Authentication failed");

            AuthResponse {
                success: false,
                user_id: None,
                error: Some("Invalid authentication token".to_string()),
            }
        }
    }

    /// Handle workspace subscription request
    async fn handle_subscribe(&self, user_id: &UserId, workspace_ids: Vec<String>) {
        let mut connections = self.connections.write().await;
        if let Some(conn) = connections.get_mut(user_id) {
            for workspace_id in workspace_ids {
                if !conn.subscribed_workspaces.contains(&workspace_id) {
                    conn.subscribed_workspaces.push(workspace_id.clone());
                    debug!(user_id = %user_id.0, workspace_id = %workspace_id, "User subscribed to workspace");
                }
            }
        }
    }

    /// Handle workspace unsubscription request
    async fn handle_unsubscribe(&self, user_id: &UserId, workspace_ids: Vec<String>) {
        let mut connections = self.connections.write().await;
        if let Some(conn) = connections.get_mut(user_id) {
            conn.subscribed_workspaces
                .retain(|id| !workspace_ids.contains(id));
            debug!(user_id = %user_id.0, "User unsubscribed from workspaces");
        }
    }

    /// Broadcast message to users subscribed to a specific workspace
    pub async fn broadcast_to_workspace(
        &self,
        workspace_id: &str,
        message: WebSocketMessage,
    ) -> SyncResult<usize> {
        let json_message = serde_json::to_string(&message)
            .map_err(|e| SyncError::SerializationError(e.to_string()))?;

        let connections = self.connections.read().await;
        let mut success_count = 0;

        for (user_id, connection) in connections.iter() {
            // Only send to authenticated users subscribed to this workspace
            if connection.authenticated
                && connection
                    .subscribed_workspaces
                    .contains(&workspace_id.to_string())
            {
                let ws_message = Message::Text(json_message.clone());

                match connection.sender.send(ws_message) {
                    Ok(_) => success_count += 1,
                    Err(e) => {
                        warn!(user_id = %user_id.0, error = %e, "Failed to send message to user");
                    }
                }
            }
        }

        debug!(
            workspace_id = %workspace_id,
            recipients = success_count,
            "Workspace broadcast completed"
        );

        Ok(success_count)
    }

    /// Check if a user is authenticated
    pub async fn is_authenticated(&self, user_id: &UserId) -> bool {
        let connections = self.connections.read().await;
        connections
            .get(user_id)
            .map(|c| c.authenticated)
            .unwrap_or(false)
    }

    /// Get subscribed workspaces for a user
    pub async fn get_subscribed_workspaces(&self, user_id: &UserId) -> Vec<String> {
        let connections = self.connections.read().await;
        connections
            .get(user_id)
            .map(|c| c.subscribed_workspaces.clone())
            .unwrap_or_default()
    }
}

/// Connection statistics
#[derive(Debug, Clone)]
pub struct ConnectionStats {
    pub active_connections: usize,
    pub max_connections: usize,
    pub user_ids: Vec<UserId>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;
    use tokio_tungstenite::tungstenite::protocol::Role;

    #[tokio::test]
    async fn test_websocket_manager_creation() {
        let config = WebSocketConfig::default();
        let manager = WebSocketManager::new(config);

        assert_eq!(manager.get_config().max_connections, 1000);
    }

    #[tokio::test]
    async fn test_connection_stats() {
        let config = WebSocketConfig::default();
        let manager = WebSocketManager::new(config);

        let stats = manager.get_connection_stats().await;
        assert_eq!(stats.active_connections, 0);
        assert_eq!(stats.max_connections, 1000);
    }

    #[tokio::test]
    async fn test_broadcast_without_connections() {
        let config = WebSocketConfig::default();
        let manager = WebSocketManager::new(config);

        let message = WebSocketMessage::Ping;
        let result = manager.broadcast(message).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0); // No connections, so 0 successful broadcasts
    }
}
