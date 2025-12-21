/**
 * WebSocket Types for Real-time State Synchronization
 * 
 * Type-safe definitions for WebSocket communication with the backend
 * state synchronization system.
 * 
 * Requirements: 2.3 - Real-time UI updates
 */

// ============================================================================
// Connection Types
// ============================================================================

export type ConnectionStatus = 'connecting' | 'connected' | 'disconnected' | 'reconnecting' | 'error';

export interface ConnectionInfo {
  status: ConnectionStatus;
  connectedAt: Date | null;
  lastMessageAt: Date | null;
  reconnectAttempts: number;
  latency: number | null;
}

// ============================================================================
// Workspace Status Types (matching backend)
// ============================================================================

export type WorkspaceStatusType = 
  | { type: 'Idle' }
  | { type: 'Processing'; started_at: string }
  | { type: 'Completed'; duration: number }
  | { type: 'Failed'; error: string; failed_at: string }
  | { type: 'Cancelled'; cancelled_at: string };

// ============================================================================
// WebSocket Message Types (matching backend WebSocketMessage enum)
// ============================================================================

export interface WorkspaceUpdateMessage {
  type: 'WorkspaceUpdate';
  workspace_id: string;
  state: Record<string, unknown>;
}

export interface EventNotificationMessage {
  type: 'EventNotification';
  event_id: string;
  event_type: string;
  payload: WorkspaceEventPayload;
}

export interface ConnectionAckMessage {
  type: 'ConnectionAck';
  user_id: string;
  connected_at: string;
}

export interface ErrorMessage {
  type: 'Error';
  code: string;
  message: string;
}

export interface PingMessage {
  type: 'Ping';
}

export interface PongMessage {
  type: 'Pong';
}

export interface AuthRequestMessage {
  type: 'AuthRequest';
  token: string;
  client_info?: string;
}

export interface AuthResponseMessage {
  type: 'AuthResponse';
  success: boolean;
  user_id?: string;
  error?: string;
}

export interface SubscribeMessage {
  type: 'Subscribe';
  workspace_ids: string[];
}

export interface UnsubscribeMessage {
  type: 'Unsubscribe';
  workspace_ids: string[];
}

export type WebSocketMessage =
  | WorkspaceUpdateMessage
  | EventNotificationMessage
  | ConnectionAckMessage
  | ErrorMessage
  | PingMessage
  | PongMessage
  | AuthRequestMessage
  | AuthResponseMessage
  | SubscribeMessage
  | UnsubscribeMessage;

// ============================================================================
// Workspace Event Types (matching backend WorkspaceEvent enum)
// ============================================================================

export interface StatusChangedEvent {
  StatusChanged: {
    workspace_id: string;
    status: WorkspaceStatusType;
    timestamp: string;
  };
}

export interface ProgressUpdateEvent {
  ProgressUpdate: {
    workspace_id: string;
    progress: number;
    timestamp: string;
  };
}

export interface TaskCompletedEvent {
  TaskCompleted: {
    workspace_id: string;
    task_id: string;
    timestamp: string;
  };
}

export interface WorkspaceErrorEvent {
  Error: {
    workspace_id: string;
    error: string;
    timestamp: string;
  };
}

export interface WorkspaceDeletedEvent {
  WorkspaceDeleted: {
    workspace_id: string;
    timestamp: string;
  };
}

export interface WorkspaceCreatedEvent {
  WorkspaceCreated: {
    workspace_id: string;
    timestamp: string;
  };
}

export type WorkspaceEventPayload =
  | StatusChangedEvent
  | ProgressUpdateEvent
  | TaskCompletedEvent
  | WorkspaceErrorEvent
  | WorkspaceDeletedEvent
  | WorkspaceCreatedEvent;

// ============================================================================
// Synchronization Monitoring Types
// ============================================================================

export interface SyncMetrics {
  messagesReceived: number;
  messagesSent: number;
  successfulDeliveries: number;
  failedDeliveries: number;
  averageLatency: number;
  lastSyncTime: Date | null;
}

export interface SyncStats {
  connectionStatus: ConnectionStatus;
  metrics: SyncMetrics;
  connectionInfo: ConnectionInfo;
}

// ============================================================================
// WebSocket Client Configuration
// ============================================================================

export interface WebSocketClientConfig {
  url: string;
  reconnectInterval: number;
  maxReconnectAttempts: number;
  pingInterval: number;
  connectionTimeout: number;
  enableAutoReconnect: boolean;
}

export const DEFAULT_WEBSOCKET_CONFIG: WebSocketClientConfig = {
  url: 'ws://localhost:8080/ws',
  reconnectInterval: 1000,
  maxReconnectAttempts: 10,
  pingInterval: 30000,
  connectionTimeout: 10000,
  enableAutoReconnect: true,
};
