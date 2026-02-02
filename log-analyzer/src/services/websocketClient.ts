/**
 * WebSocket Client Service
 * 
 * Native WebSocket API client with automatic reconnection, type-safe event handling,
 * and connection status management.
 * 
 * Requirements: 2.3 - Real-time UI updates without manual refresh
 */

import { logger } from '../utils/logger';
import {
  ConnectionStatus,
  ConnectionInfo,
  WebSocketMessage,
  WebSocketClientConfig,
  DEFAULT_WEBSOCKET_CONFIG,
  SyncMetrics,
} from '../types/websocket';

// ============================================================================
// Event Emitter Types
// ============================================================================

type MessageHandler = (message: WebSocketMessage) => void;
type StatusHandler = (status: ConnectionStatus) => void;
type ErrorHandler = (error: Error) => void;

interface EventHandlers {
  message: Set<MessageHandler>;
  status: Set<StatusHandler>;
  error: Set<ErrorHandler>;
}

// ============================================================================
// WebSocket Client Class
// ============================================================================

export class WebSocketClient {
  private ws: WebSocket | null = null;
  private config: WebSocketClientConfig;
  private connectionInfo: ConnectionInfo;
  private metrics: SyncMetrics;
  private handlers: EventHandlers;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private pingTimer: ReturnType<typeof setInterval> | null = null;
  private connectionTimer: ReturnType<typeof setTimeout> | null = null;
  private isManualClose = false;
  private pendingMessages: string[] = [];

  constructor(config: Partial<WebSocketClientConfig> = {}) {
    this.config = { ...DEFAULT_WEBSOCKET_CONFIG, ...config };
    this.connectionInfo = {
      status: 'disconnected',
      connectedAt: null,
      lastMessageAt: null,
      reconnectAttempts: 0,
      latency: null,
    };
    this.metrics = {
      messagesReceived: 0,
      messagesSent: 0,
      successfulDeliveries: 0,
      failedDeliveries: 0,
      averageLatency: 0,
      lastSyncTime: null,
    };
    this.handlers = {
      message: new Set(),
      status: new Set(),
      error: new Set(),
    };
  }

  // ============================================================================
  // Connection Management
  // ============================================================================

  /**
   * Connect to WebSocket server
   */
  connect(): void {
    if (this.ws?.readyState === WebSocket.OPEN) {
      logger.debug('[WS] Already connected');
      return;
    }

    this.isManualClose = false;
    this.updateStatus('connecting');

    try {
      logger.debug('[WS] Connecting to:', this.config.url);
      this.ws = new WebSocket(this.config.url);
      this.setupEventHandlers();
      this.startConnectionTimeout();
    } catch (error) {
      logger.error('[WS] Connection error:', error);
      this.handleConnectionError(error as Error);
    }
  }

  /**
   * Disconnect from WebSocket server
   */
  disconnect(): void {
    this.isManualClose = true;
    this.cleanup();
    this.updateStatus('disconnected');
    logger.debug('[WS] Disconnected manually');
  }

  /**
   * Reconnect to WebSocket server
   */
  reconnect(): void {
    this.cleanup();
    this.connectionInfo.reconnectAttempts = 0;
    this.connect();
  }

  // ============================================================================
  // Message Handling
  // ============================================================================

  /**
   * Send a message to the server
   */
  send(message: WebSocketMessage): boolean {
    const messageStr = JSON.stringify(message);

    if (this.ws?.readyState === WebSocket.OPEN) {
      try {
        this.ws.send(messageStr);
        this.metrics.messagesSent++;
        this.metrics.successfulDeliveries++;
        logger.debug('[WS] Message sent:', message.type);
        return true;
      } catch (error) {
        logger.error('[WS] Send error:', error);
        this.metrics.failedDeliveries++;
        return false;
      }
    } else {
      // Queue message for later delivery
      this.pendingMessages.push(messageStr);
      logger.debug('[WS] Message queued (not connected):', message.type);
      return false;
    }
  }

  /**
   * Send ping to measure latency
   */
  sendPing(): void {
    const pingStart = Date.now();
    
    const handlePong = (message: WebSocketMessage) => {
      if (message.type === 'Pong') {
        const latency = Date.now() - pingStart;
        this.connectionInfo.latency = latency;
        this.updateAverageLatency(latency);
        this.off('message', handlePong);
      }
    };

    this.on('message', handlePong);
    this.send({ type: 'Ping' });
  }

  /**
   * Subscribe to workspace events
   */
  subscribe(workspaceIds: string[]): void {
    this.send({
      type: 'Subscribe',
      workspace_ids: workspaceIds,
    });
  }

  /**
   * Unsubscribe from workspace events
   */
  unsubscribe(workspaceIds: string[]): void {
    this.send({
      type: 'Unsubscribe',
      workspace_ids: workspaceIds,
    });
  }

  // ============================================================================
  // Event Subscription
  // ============================================================================

  /**
   * Subscribe to events
   */
  on(event: 'message', handler: MessageHandler): void;
  on(event: 'status', handler: StatusHandler): void;
  on(event: 'error', handler: ErrorHandler): void;
  on(event: keyof EventHandlers, handler: MessageHandler | StatusHandler | ErrorHandler): void {
    (this.handlers[event] as Set<typeof handler>).add(handler);
  }

  /**
   * Unsubscribe from events
   */
  off(event: 'message', handler: MessageHandler): void;
  off(event: 'status', handler: StatusHandler): void;
  off(event: 'error', handler: ErrorHandler): void;
  off(event: keyof EventHandlers, handler: MessageHandler | StatusHandler | ErrorHandler): void {
    (this.handlers[event] as Set<typeof handler>).delete(handler);
  }

  // ============================================================================
  // State Getters
  // ============================================================================

  /**
   * Get current connection status
   */
  getStatus(): ConnectionStatus {
    return this.connectionInfo.status;
  }

  /**
   * Get connection info
   */
  getConnectionInfo(): ConnectionInfo {
    return { ...this.connectionInfo };
  }

  /**
   * Get sync metrics
   */
  getMetrics(): SyncMetrics {
    return { ...this.metrics };
  }

  /**
   * Check if connected
   */
  isConnected(): boolean {
    return this.ws?.readyState === WebSocket.OPEN;
  }

  // ============================================================================
  // Private Methods
  // ============================================================================

  private setupEventHandlers(): void {
    if (!this.ws) return;

    this.ws.onopen = () => {
      this.clearConnectionTimeout();
      this.connectionInfo.connectedAt = new Date();
      this.connectionInfo.reconnectAttempts = 0;
      this.updateStatus('connected');
      this.startPingInterval();
      this.flushPendingMessages();
      logger.debug('[WS] Connection established');
    };

    this.ws.onclose = (event) => {
      logger.debug('[WS] Connection closed:', event.code, event.reason);
      this.cleanup();

      if (!this.isManualClose && this.config.enableAutoReconnect) {
        this.scheduleReconnect();
      } else {
        this.updateStatus('disconnected');
      }
    };

    this.ws.onerror = (event) => {
      logger.error('[WS] WebSocket error:', event);
      this.handleConnectionError(new Error('WebSocket error'));
    };

    this.ws.onmessage = (event) => {
      this.handleMessage(event.data);
    };
  }

  private handleMessage(data: string): void {
    this.connectionInfo.lastMessageAt = new Date();
    this.metrics.messagesReceived++;
    this.metrics.lastSyncTime = new Date();

    try {
      // Handle simple ping/pong strings
      if (data === 'pong') {
        this.emitMessage({ type: 'Pong' });
        return;
      }

      const message = JSON.parse(data) as WebSocketMessage;
      logger.debug('[WS] Message received:', message.type);
      this.emitMessage(message);
    } catch (error) {
      logger.error('[WS] Failed to parse message:', error);
    }
  }

  private emitMessage(message: WebSocketMessage): void {
    this.handlers.message.forEach((handler) => {
      try {
        handler(message);
      } catch (error) {
        logger.error('[WS] Message handler error:', error);
      }
    });
  }

  private updateStatus(status: ConnectionStatus): void {
    this.connectionInfo.status = status;
    this.handlers.status.forEach((handler) => {
      try {
        handler(status);
      } catch (error) {
        logger.error('[WS] Status handler error:', error);
      }
    });
  }

  private handleConnectionError(error: Error): void {
    this.updateStatus('error');
    this.handlers.error.forEach((handler) => {
      try {
        handler(error);
      } catch (err) {
        logger.error('[WS] Error handler error:', err);
      }
    });

    if (this.config.enableAutoReconnect && !this.isManualClose) {
      this.scheduleReconnect();
    }
  }

  private scheduleReconnect(): void {
    if (this.connectionInfo.reconnectAttempts >= this.config.maxReconnectAttempts) {
      logger.error('[WS] Max reconnect attempts reached');
      this.updateStatus('error');
      return;
    }

    this.updateStatus('reconnecting');
    this.connectionInfo.reconnectAttempts++;

    // Exponential backoff
    const delay = Math.min(
      this.config.reconnectInterval * Math.pow(2, this.connectionInfo.reconnectAttempts - 1),
      30000 // Max 30 seconds
    );

    logger.debug(`[WS] Reconnecting in ${delay}ms (attempt ${this.connectionInfo.reconnectAttempts})`);

    this.reconnectTimer = setTimeout(() => {
      this.connect();
    }, delay);
  }

  private startPingInterval(): void {
    this.stopPingInterval();
    this.pingTimer = setInterval(() => {
      if (this.isConnected()) {
        this.sendPing();
      }
    }, this.config.pingInterval);
  }

  private stopPingInterval(): void {
    if (this.pingTimer) {
      clearInterval(this.pingTimer);
      this.pingTimer = null;
    }
  }

  private startConnectionTimeout(): void {
    this.clearConnectionTimeout();
    this.connectionTimer = setTimeout(() => {
      if (this.connectionInfo.status === 'connecting') {
        logger.error('[WS] Connection timeout');
        this.ws?.close();
        this.handleConnectionError(new Error('Connection timeout'));
      }
    }, this.config.connectionTimeout);
  }

  private clearConnectionTimeout(): void {
    if (this.connectionTimer) {
      clearTimeout(this.connectionTimer);
      this.connectionTimer = null;
    }
  }

  private flushPendingMessages(): void {
    const MAX_PENDING_MESSAGES = 1000;
    let processedCount = 0;

    while (this.pendingMessages.length > 0 && this.isConnected() && processedCount < MAX_PENDING_MESSAGES) {
      const message = this.pendingMessages.shift();
      processedCount++;

      if (!message) {
        continue;
      }

      if (this.ws) {
        try {
          this.ws.send(message);
          this.metrics.messagesSent++;
          this.metrics.successfulDeliveries++;
        } catch (error) {
          logger.error('[WS] Failed to send pending message:', error);
          this.metrics.failedDeliveries++;
        }
      }
    }

    if (this.pendingMessages.length > MAX_PENDING_MESSAGES) {
      logger.warn('[WS] Pending messages overflow, clearing queue');
      this.pendingMessages = this.pendingMessages.slice(-MAX_PENDING_MESSAGES);
    }
  }

  private updateAverageLatency(latency: number): void {
    const totalMessages = this.metrics.messagesReceived;
    if (totalMessages === 0) {
      this.metrics.averageLatency = latency;
    } else {
      // Running average
      this.metrics.averageLatency = 
        (this.metrics.averageLatency * (totalMessages - 1) + latency) / totalMessages;
    }
  }

  private cleanup(): void {
    this.clearConnectionTimeout();
    this.stopPingInterval();

    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }

    if (this.ws) {
      this.ws.onopen = null;
      this.ws.onclose = null;
      this.ws.onerror = null;
      this.ws.onmessage = null;

      if (this.ws.readyState === WebSocket.OPEN || this.ws.readyState === WebSocket.CONNECTING) {
        this.ws.close();
      }
      this.ws = null;
    }

    this.connectionInfo.connectedAt = null;
    this.connectionInfo.latency = null;
  }
}

// ============================================================================
// Singleton Instance
// ============================================================================

let wsClientInstance: WebSocketClient | null = null;

/**
 * Get or create WebSocket client singleton
 */
export function getWebSocketClient(config?: Partial<WebSocketClientConfig>): WebSocketClient {
  if (!wsClientInstance) {
    wsClientInstance = new WebSocketClient(config);
  }
  return wsClientInstance;
}

/**
 * Reset WebSocket client (for testing)
 */
export function resetWebSocketClient(): void {
  if (wsClientInstance) {
    wsClientInstance.disconnect();
    wsClientInstance = null;
  }
}
