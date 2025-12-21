/**
 * useWebSocket Hook
 * 
 * React hook for WebSocket connection management with automatic reconnection,
 * type-safe event handling, and connection status indicators.
 * 
 * Requirements: 2.3 - Real-time UI updates without manual refresh
 */

import { useEffect, useCallback, useRef, useState } from 'react';
import { WebSocketClient, getWebSocketClient } from '../services/websocketClient';
import {
  ConnectionStatus,
  ConnectionInfo,
  WebSocketMessage,
  WebSocketClientConfig,
  SyncMetrics,
  EventNotificationMessage,
} from '../types/websocket';

// ============================================================================
// Hook Return Type
// ============================================================================

export interface UseWebSocketReturn {
  // Connection state
  status: ConnectionStatus;
  connectionInfo: ConnectionInfo;
  metrics: SyncMetrics;
  isConnected: boolean;
  
  // Connection actions
  connect: () => void;
  disconnect: () => void;
  reconnect: () => void;
  
  // Messaging
  send: (message: WebSocketMessage) => boolean;
  subscribe: (workspaceIds: string[]) => void;
  unsubscribe: (workspaceIds: string[]) => void;
  
  // Last received event (for reactive updates)
  lastEvent: EventNotificationMessage | null;
  lastError: Error | null;
}

// ============================================================================
// Hook Options
// ============================================================================

export interface UseWebSocketOptions {
  config?: Partial<WebSocketClientConfig>;
  autoConnect?: boolean;
  onMessage?: (message: WebSocketMessage) => void;
  onEvent?: (event: EventNotificationMessage) => void;
  onStatusChange?: (status: ConnectionStatus) => void;
  onError?: (error: Error) => void;
}

// ============================================================================
// Hook Implementation
// ============================================================================

export function useWebSocket(options: UseWebSocketOptions = {}): UseWebSocketReturn {
  const {
    config,
    autoConnect = false,
    onMessage,
    onEvent,
    onStatusChange,
    onError,
  } = options;

  // State
  const [status, setStatus] = useState<ConnectionStatus>('disconnected');
  const [connectionInfo, setConnectionInfo] = useState<ConnectionInfo>({
    status: 'disconnected',
    connectedAt: null,
    lastMessageAt: null,
    reconnectAttempts: 0,
    latency: null,
  });
  const [metrics, setMetrics] = useState<SyncMetrics>({
    messagesReceived: 0,
    messagesSent: 0,
    successfulDeliveries: 0,
    failedDeliveries: 0,
    averageLatency: 0,
    lastSyncTime: null,
  });
  const [lastEvent, setLastEvent] = useState<EventNotificationMessage | null>(null);
  const [lastError, setLastError] = useState<Error | null>(null);

  // Refs
  const clientRef = useRef<WebSocketClient | null>(null);
  const callbacksRef = useRef({ onMessage, onEvent, onStatusChange, onError });

  // Update callbacks ref when they change
  useEffect(() => {
    callbacksRef.current = { onMessage, onEvent, onStatusChange, onError };
  }, [onMessage, onEvent, onStatusChange, onError]);

  // Initialize client
  useEffect(() => {
    clientRef.current = getWebSocketClient(config);
    
    // Set up event handlers
    const handleMessage = (message: WebSocketMessage) => {
      // Update metrics
      setMetrics(clientRef.current?.getMetrics() ?? metrics);
      setConnectionInfo(clientRef.current?.getConnectionInfo() ?? connectionInfo);
      
      // Call user callback
      callbacksRef.current.onMessage?.(message);
      
      // Handle event notifications specifically
      if (message.type === 'EventNotification') {
        const eventMessage = message as EventNotificationMessage;
        setLastEvent(eventMessage);
        callbacksRef.current.onEvent?.(eventMessage);
      }
    };

    const handleStatus = (newStatus: ConnectionStatus) => {
      setStatus(newStatus);
      setConnectionInfo(clientRef.current?.getConnectionInfo() ?? connectionInfo);
      callbacksRef.current.onStatusChange?.(newStatus);
    };

    const handleError = (error: Error) => {
      setLastError(error);
      callbacksRef.current.onError?.(error);
    };

    clientRef.current.on('message', handleMessage);
    clientRef.current.on('status', handleStatus);
    clientRef.current.on('error', handleError);

    // Auto-connect if enabled
    if (autoConnect) {
      clientRef.current.connect();
    }

    // Cleanup
    return () => {
      if (clientRef.current) {
        clientRef.current.off('message', handleMessage);
        clientRef.current.off('status', handleStatus);
        clientRef.current.off('error', handleError);
      }
    };
  }, [config, autoConnect]);

  // Connection actions
  const connect = useCallback(() => {
    clientRef.current?.connect();
  }, []);

  const disconnect = useCallback(() => {
    clientRef.current?.disconnect();
  }, []);

  const reconnect = useCallback(() => {
    clientRef.current?.reconnect();
  }, []);

  // Messaging actions
  const send = useCallback((message: WebSocketMessage): boolean => {
    return clientRef.current?.send(message) ?? false;
  }, []);

  const subscribe = useCallback((workspaceIds: string[]) => {
    clientRef.current?.subscribe(workspaceIds);
  }, []);

  const unsubscribe = useCallback((workspaceIds: string[]) => {
    clientRef.current?.unsubscribe(workspaceIds);
  }, []);

  return {
    status,
    connectionInfo,
    metrics,
    isConnected: status === 'connected',
    connect,
    disconnect,
    reconnect,
    send,
    subscribe,
    unsubscribe,
    lastEvent,
    lastError,
  };
}

// ============================================================================
// Specialized Hook for Workspace Events
// ============================================================================

export interface UseWorkspaceEventsOptions {
  workspaceIds?: string[];
  onStatusChanged?: (workspaceId: string, status: unknown) => void;
  onProgressUpdate?: (workspaceId: string, progress: number) => void;
  onTaskCompleted?: (workspaceId: string, taskId: string) => void;
  onError?: (workspaceId: string, error: string) => void;
  onWorkspaceDeleted?: (workspaceId: string) => void;
  onWorkspaceCreated?: (workspaceId: string) => void;
}

export function useWorkspaceEvents(options: UseWorkspaceEventsOptions = {}) {
  const {
    workspaceIds = [],
    onStatusChanged,
    onProgressUpdate,
    onTaskCompleted,
    onError,
    onWorkspaceDeleted,
    onWorkspaceCreated,
  } = options;

  // Store callbacks in ref to avoid re-subscriptions
  const callbacksRef = useRef({
    onStatusChanged,
    onProgressUpdate,
    onTaskCompleted,
    onError,
    onWorkspaceDeleted,
    onWorkspaceCreated,
  });

  useEffect(() => {
    callbacksRef.current = {
      onStatusChanged,
      onProgressUpdate,
      onTaskCompleted,
      onError,
      onWorkspaceDeleted,
      onWorkspaceCreated,
    };
  }, [onStatusChanged, onProgressUpdate, onTaskCompleted, onError, onWorkspaceDeleted, onWorkspaceCreated]);

  const handleEvent = useCallback((event: EventNotificationMessage) => {
    const payload = event.payload;
    
    if ('StatusChanged' in payload) {
      const { workspace_id, status } = payload.StatusChanged;
      callbacksRef.current.onStatusChanged?.(workspace_id, status);
    } else if ('ProgressUpdate' in payload) {
      const { workspace_id, progress } = payload.ProgressUpdate;
      callbacksRef.current.onProgressUpdate?.(workspace_id, progress);
    } else if ('TaskCompleted' in payload) {
      const { workspace_id, task_id } = payload.TaskCompleted;
      callbacksRef.current.onTaskCompleted?.(workspace_id, task_id);
    } else if ('Error' in payload) {
      const { workspace_id, error } = payload.Error;
      callbacksRef.current.onError?.(workspace_id, error);
    } else if ('WorkspaceDeleted' in payload) {
      const { workspace_id } = payload.WorkspaceDeleted;
      callbacksRef.current.onWorkspaceDeleted?.(workspace_id);
    } else if ('WorkspaceCreated' in payload) {
      const { workspace_id } = payload.WorkspaceCreated;
      callbacksRef.current.onWorkspaceCreated?.(workspace_id);
    }
  }, []);

  const ws = useWebSocket({
    autoConnect: workspaceIds.length > 0,
    onEvent: handleEvent,
  });

  // Subscribe to workspaces when connected
  useEffect(() => {
    if (ws.isConnected && workspaceIds.length > 0) {
      ws.subscribe(workspaceIds);
      
      return () => {
        ws.unsubscribe(workspaceIds);
      };
    }
  }, [ws.isConnected, workspaceIds, ws.subscribe, ws.unsubscribe]);

  return ws;
}
