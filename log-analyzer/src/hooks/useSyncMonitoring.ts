/**
 * useSyncMonitoring Hook
 * 
 * Implements latency measurement, success rate tracking, and error handling
 * for state synchronization monitoring.
 * 
 * Requirements: 4.2 - Synchronization latency and success rate tracking
 */

import { useEffect, useCallback, useRef, useState } from 'react';
import { useWebSocket } from './useWebSocket';
import { logger } from '../utils/logger';
import { ConnectionStatus } from '../types/websocket';

// ============================================================================
// Types
// ============================================================================

export interface LatencyMeasurement {
  timestamp: Date;
  latency: number;
  type: 'ping' | 'event' | 'message';
}

export interface SyncMonitoringStats {
  // Latency metrics
  currentLatency: number | null;
  averageLatency: number;
  minLatency: number;
  maxLatency: number;
  latencyHistory: LatencyMeasurement[];
  
  // Success rate metrics
  totalMessages: number;
  successfulMessages: number;
  failedMessages: number;
  successRate: number;
  
  // Connection metrics
  connectionUptime: number;
  reconnectCount: number;
  lastConnectedAt: Date | null;
  lastDisconnectedAt: Date | null;
  
  // Error tracking
  errors: SyncError[];
  lastError: SyncError | null;
}

export interface SyncError {
  timestamp: Date;
  message: string;
  code?: string;
  recoverable: boolean;
}

export interface UseSyncMonitoringReturn {
  stats: SyncMonitoringStats;
  isHealthy: boolean;
  healthStatus: 'healthy' | 'degraded' | 'unhealthy';
  
  // Actions
  measureLatency: () => Promise<number>;
  clearErrors: () => void;
  resetStats: () => void;
  manualRefresh: () => void;
}

// ============================================================================
// Constants
// ============================================================================

const MAX_LATENCY_HISTORY = 100;
const MAX_ERROR_HISTORY = 50;
const LATENCY_THRESHOLD_HEALTHY = 100; // ms
const LATENCY_THRESHOLD_DEGRADED = 500; // ms
const SUCCESS_RATE_THRESHOLD_HEALTHY = 0.95;
const SUCCESS_RATE_THRESHOLD_DEGRADED = 0.8;

// ============================================================================
// Initial State
// ============================================================================

const initialStats: SyncMonitoringStats = {
  currentLatency: null,
  averageLatency: 0,
  minLatency: Infinity,
  maxLatency: 0,
  latencyHistory: [],
  totalMessages: 0,
  successfulMessages: 0,
  failedMessages: 0,
  successRate: 1,
  connectionUptime: 0,
  reconnectCount: 0,
  lastConnectedAt: null,
  lastDisconnectedAt: null,
  errors: [],
  lastError: null,
};

// ============================================================================
// Hook Implementation
// ============================================================================

export function useSyncMonitoring(): UseSyncMonitoringReturn {
  const [stats, setStats] = useState<SyncMonitoringStats>(initialStats);
  
  // Refs for tracking
  const connectionStartRef = useRef<Date | null>(null);
  const uptimeIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const latencyResolverRef = useRef<((latency: number) => void) | null>(null);
  const pingStartRef = useRef<number>(0);

  // WebSocket connection
  const ws = useWebSocket({
    onMessage: (message) => {
      // Track successful message
      setStats((prev) => ({
        ...prev,
        totalMessages: prev.totalMessages + 1,
        successfulMessages: prev.successfulMessages + 1,
        successRate: (prev.successfulMessages + 1) / (prev.totalMessages + 1),
      }));

      // Handle pong for latency measurement
      if (message.type === 'Pong' && latencyResolverRef.current) {
        const latency = Date.now() - pingStartRef.current;
        latencyResolverRef.current(latency);
        latencyResolverRef.current = null;
        
        recordLatency(latency, 'ping');
      }
    },
    onStatusChange: (status) => {
      handleStatusChange(status);
    },
    onError: (error) => {
      recordError(error.message, undefined, true);
      
      setStats((prev) => ({
        ...prev,
        totalMessages: prev.totalMessages + 1,
        failedMessages: prev.failedMessages + 1,
        successRate: prev.successfulMessages / (prev.totalMessages + 1),
      }));
    },
  });

  // ============================================================================
  // Status Change Handler
  // ============================================================================

  const handleStatusChange = useCallback((status: ConnectionStatus) => {
    if (status === 'connected') {
      connectionStartRef.current = new Date();
      
      setStats((prev) => ({
        ...prev,
        lastConnectedAt: new Date(),
      }));

      // Start uptime tracking
      if (uptimeIntervalRef.current) {
        clearInterval(uptimeIntervalRef.current);
      }
      uptimeIntervalRef.current = setInterval(() => {
        if (connectionStartRef.current) {
          const uptime = Date.now() - connectionStartRef.current.getTime();
          setStats((prev) => ({
            ...prev,
            connectionUptime: uptime,
          }));
        }
      }, 1000);
    } else if (status === 'disconnected' || status === 'error') {
      connectionStartRef.current = null;
      
      setStats((prev) => ({
        ...prev,
        lastDisconnectedAt: new Date(),
      }));

      if (uptimeIntervalRef.current) {
        clearInterval(uptimeIntervalRef.current);
        uptimeIntervalRef.current = null;
      }
    } else if (status === 'reconnecting') {
      setStats((prev) => ({
        ...prev,
        reconnectCount: prev.reconnectCount + 1,
      }));
    }
  }, []);

  // ============================================================================
  // Latency Recording
  // ============================================================================

  const recordLatency = useCallback((latency: number, type: LatencyMeasurement['type']) => {
    setStats((prev) => {
      const newMeasurement: LatencyMeasurement = {
        timestamp: new Date(),
        latency,
        type,
      };

      const newHistory = [...prev.latencyHistory, newMeasurement].slice(-MAX_LATENCY_HISTORY);
      
      // Calculate new averages
      const latencies = newHistory.map((m) => m.latency);
      const avgLatency = latencies.reduce((a, b) => a + b, 0) / latencies.length;
      const minLatency = Math.min(...latencies);
      const maxLatency = Math.max(...latencies);

      return {
        ...prev,
        currentLatency: latency,
        averageLatency: avgLatency,
        minLatency,
        maxLatency,
        latencyHistory: newHistory,
      };
    });
  }, []);

  // ============================================================================
  // Error Recording
  // ============================================================================

  const recordError = useCallback((message: string, code?: string, recoverable = true) => {
    const error: SyncError = {
      timestamp: new Date(),
      message,
      code,
      recoverable,
    };

    setStats((prev) => ({
      ...prev,
      errors: [...prev.errors, error].slice(-MAX_ERROR_HISTORY),
      lastError: error,
    }));

    logger.error('[SYNC_MONITOR] Error recorded:', message);
  }, []);

  // ============================================================================
  // Actions
  // ============================================================================

  const measureLatency = useCallback((): Promise<number> => {
    return new Promise((resolve, reject) => {
      if (!ws.isConnected) {
        reject(new Error('Not connected'));
        return;
      }

      pingStartRef.current = Date.now();
      latencyResolverRef.current = resolve;

      // Send ping
      ws.send({ type: 'Ping' });

      // Timeout after 5 seconds
      setTimeout(() => {
        if (latencyResolverRef.current) {
          latencyResolverRef.current = null;
          reject(new Error('Latency measurement timeout'));
        }
      }, 5000);
    });
  }, [ws]);

  const clearErrors = useCallback(() => {
    setStats((prev) => ({
      ...prev,
      errors: [],
      lastError: null,
    }));
  }, []);

  const resetStats = useCallback(() => {
    setStats(initialStats);
  }, []);

  const manualRefresh = useCallback(() => {
    if (ws.isConnected) {
      ws.reconnect();
    } else {
      ws.connect();
    }
  }, [ws]);

  // ============================================================================
  // Health Status Calculation
  // ============================================================================

  const calculateHealthStatus = useCallback((): 'healthy' | 'degraded' | 'unhealthy' => {
    if (!ws.isConnected) {
      return 'unhealthy';
    }

    const { averageLatency, successRate } = stats;

    // Check latency
    if (averageLatency > LATENCY_THRESHOLD_DEGRADED) {
      return 'unhealthy';
    }
    if (averageLatency > LATENCY_THRESHOLD_HEALTHY) {
      return 'degraded';
    }

    // Check success rate
    if (successRate < SUCCESS_RATE_THRESHOLD_DEGRADED) {
      return 'unhealthy';
    }
    if (successRate < SUCCESS_RATE_THRESHOLD_HEALTHY) {
      return 'degraded';
    }

    return 'healthy';
  }, [ws.isConnected, stats]);

  const healthStatus = calculateHealthStatus();
  const isHealthy = healthStatus === 'healthy';

  // ============================================================================
  // Cleanup
  // ============================================================================

  useEffect(() => {
    return () => {
      if (uptimeIntervalRef.current) {
        clearInterval(uptimeIntervalRef.current);
      }
    };
  }, []);

  return {
    stats,
    isHealthy,
    healthStatus,
    measureLatency,
    clearErrors,
    resetStats,
    manualRefresh,
  };
}

export default useSyncMonitoring;
