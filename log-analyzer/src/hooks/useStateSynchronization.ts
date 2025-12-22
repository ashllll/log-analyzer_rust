/**
 * useStateSynchronization Hook
 * 
 * React state management that responds to WebSocket events with optimistic updates,
 * automatic rollback on conflicts, and UI update batching.
 * 
 * Requirements: 2.3 - Automatic UI updates without manual refresh
 */

import { useEffect, useCallback, useRef, useState } from 'react';
import { useAppStore, Workspace, Task } from '../stores/appStore';
import { useWebSocket } from './useWebSocket';
import { logger } from '../utils/logger';
import {
  EventNotificationMessage,
  ConnectionStatus,
} from '../types/websocket';

// ============================================================================
// Types
// ============================================================================

export interface OptimisticUpdate {
  id: string;
  type: 'workspace' | 'task';
  originalState: Workspace | Task | null;
  pendingState: Partial<Workspace> | Partial<Task>;
  timestamp: number;
  timeout: ReturnType<typeof setTimeout>;
}

export interface SyncState {
  isConnected: boolean;
  isSyncing: boolean;
  lastSyncTime: Date | null;
  pendingUpdates: number;
  syncErrors: string[];
}

export interface UseStateSynchronizationReturn {
  syncState: SyncState;
  connectionStatus: ConnectionStatus;
  
  // Manual sync actions
  refreshWorkspaces: () => void;
  refreshTasks: () => void;
  
  // Optimistic update helpers
  optimisticUpdateWorkspace: (id: string, updates: Partial<Workspace>) => void;
  optimisticUpdateTask: (id: string, updates: Partial<Task>) => void;
  
  // Connection actions
  connect: () => void;
  disconnect: () => void;
}

// ============================================================================
// Constants
// ============================================================================

const OPTIMISTIC_UPDATE_TIMEOUT = 5000; // 5 seconds
const UPDATE_BATCH_DELAY = 50; // 50ms batching

// ============================================================================
// Hook Implementation
// ============================================================================

export function useStateSynchronization(): UseStateSynchronizationReturn {
  // Store actions
  const updateWorkspace = useAppStore((state) => state.updateWorkspace);
  const deleteWorkspace = useAppStore((state) => state.deleteWorkspace);
  const updateTask = useAppStore((state) => state.updateTask);
  const addToast = useAppStore((state) => state.addToast);
  const workspaces = useAppStore((state) => state.workspaces);
  const tasks = useAppStore((state) => state.tasks);

  // Local state
  const [syncState, setSyncState] = useState<SyncState>({
    isConnected: false,
    isSyncing: false,
    lastSyncTime: null,
    pendingUpdates: 0,
    syncErrors: [],
  });

  // Refs for optimistic updates and batching
  const optimisticUpdatesRef = useRef<Map<string, OptimisticUpdate>>(new Map());
  const batchedUpdatesRef = useRef<Map<string, () => void>>(new Map());
  const batchTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // ============================================================================
  // Update Batching
  // ============================================================================

  const scheduleBatchedUpdate = useCallback((key: string, updateFn: () => void) => {
    batchedUpdatesRef.current.set(key, updateFn);

    if (!batchTimerRef.current) {
      batchTimerRef.current = setTimeout(() => {
        // Execute all batched updates
        batchedUpdatesRef.current.forEach((fn) => fn());
        batchedUpdatesRef.current.clear();
        batchTimerRef.current = null;
      }, UPDATE_BATCH_DELAY);
    }
  }, []);

  // ============================================================================
  // Optimistic Updates
  // ============================================================================

  const optimisticUpdateWorkspace = useCallback((id: string, updates: Partial<Workspace>) => {
    const currentWorkspace = workspaces.find((w) => w.id === id);
    
    // Store original state for potential rollback
    const optimisticUpdate: OptimisticUpdate = {
      id: `workspace:${id}`,
      type: 'workspace',
      originalState: currentWorkspace ? { ...currentWorkspace } : null,
      pendingState: updates,
      timestamp: Date.now(),
      timeout: setTimeout(() => {
        // Rollback if server doesn't confirm within timeout
        logger.warn('[SYNC] Optimistic update timeout, rolling back:', id);
        rollbackOptimisticUpdate(`workspace:${id}`);
      }, OPTIMISTIC_UPDATE_TIMEOUT),
    };

    optimisticUpdatesRef.current.set(`workspace:${id}`, optimisticUpdate);
    
    // Apply optimistic update immediately
    updateWorkspace(id, updates);
    
    setSyncState((prev) => ({
      ...prev,
      pendingUpdates: prev.pendingUpdates + 1,
    }));
  }, [workspaces, updateWorkspace]);

  const optimisticUpdateTask = useCallback((id: string, updates: Partial<Task>) => {
    const currentTask = tasks.find((t) => t.id === id);
    
    const optimisticUpdate: OptimisticUpdate = {
      id: `task:${id}`,
      type: 'task',
      originalState: currentTask ? { ...currentTask } : null,
      pendingState: updates,
      timestamp: Date.now(),
      timeout: setTimeout(() => {
        logger.warn('[SYNC] Optimistic update timeout, rolling back:', id);
        rollbackOptimisticUpdate(`task:${id}`);
      }, OPTIMISTIC_UPDATE_TIMEOUT),
    };

    optimisticUpdatesRef.current.set(`task:${id}`, optimisticUpdate);
    updateTask(id, updates);
    
    setSyncState((prev) => ({
      ...prev,
      pendingUpdates: prev.pendingUpdates + 1,
    }));
  }, [tasks, updateTask]);

  const rollbackOptimisticUpdate = useCallback((key: string) => {
    const update = optimisticUpdatesRef.current.get(key);
    if (!update) return;

    clearTimeout(update.timeout);
    optimisticUpdatesRef.current.delete(key);

    if (update.originalState) {
      if (update.type === 'workspace') {
        updateWorkspace((update.originalState as Workspace).id, update.originalState as Workspace);
      } else if (update.type === 'task') {
        updateTask((update.originalState as Task).id, update.originalState as Partial<Task>);
      }
    }

    setSyncState((prev) => ({
      ...prev,
      pendingUpdates: Math.max(0, prev.pendingUpdates - 1),
      syncErrors: [...prev.syncErrors, `Update failed for ${key}`],
    }));

    addToast('error', 'Update failed, changes reverted');
  }, [updateWorkspace, updateTask, addToast]);

  const confirmOptimisticUpdate = useCallback((key: string) => {
    const update = optimisticUpdatesRef.current.get(key);
    if (!update) return;

    clearTimeout(update.timeout);
    optimisticUpdatesRef.current.delete(key);

    setSyncState((prev) => ({
      ...prev,
      pendingUpdates: Math.max(0, prev.pendingUpdates - 1),
      lastSyncTime: new Date(),
    }));
  }, []);

  // ============================================================================
  // WebSocket Event Handlers
  // ============================================================================

  const handleWorkspaceEvent = useCallback((event: EventNotificationMessage) => {
    const payload = event.payload;

    if ('StatusChanged' in payload) {
      const { workspace_id, status } = payload.StatusChanged;
      
      // Map backend status to frontend status
      let frontendStatus: Workspace['status'] = 'READY';
      if (typeof status === 'object') {
        if ('Processing' in status) frontendStatus = 'PROCESSING';
        else if ('Failed' in status) frontendStatus = 'OFFLINE';
        else if ('Completed' in status) frontendStatus = 'READY';
        else if ('Cancelled' in status) frontendStatus = 'OFFLINE';
      } else if (status === 'Idle') {
        frontendStatus = 'READY';
      }

      // Confirm any pending optimistic update
      confirmOptimisticUpdate(`workspace:${workspace_id}`);

      // Batch the update
      scheduleBatchedUpdate(`workspace:${workspace_id}:status`, () => {
        updateWorkspace(workspace_id, { status: frontendStatus });
        logger.debug('[SYNC] Workspace status updated:', workspace_id, frontendStatus);
      });
    } else if ('ProgressUpdate' in payload) {
      const { workspace_id, progress } = payload.ProgressUpdate;
      
      scheduleBatchedUpdate(`workspace:${workspace_id}:progress`, () => {
        // Progress updates don't change workspace status directly
        // but we can use this for task progress
        logger.debug('[SYNC] Progress update:', workspace_id, progress);
      });
    } else if ('TaskCompleted' in payload) {
      const { workspace_id, task_id } = payload.TaskCompleted;
      
      confirmOptimisticUpdate(`task:${task_id}`);
      
      scheduleBatchedUpdate(`task:${task_id}:complete`, () => {
        updateTask(task_id, { status: 'COMPLETED', progress: 100 });
        updateWorkspace(workspace_id, { status: 'READY' });
        logger.debug('[SYNC] Task completed:', task_id);
      });
    } else if ('Error' in payload) {
      const { workspace_id, error } = payload.Error;
      
      scheduleBatchedUpdate(`workspace:${workspace_id}:error`, () => {
        updateWorkspace(workspace_id, { status: 'OFFLINE' });
        addToast('error', `Workspace error: ${error}`);
        logger.error('[SYNC] Workspace error:', workspace_id, error);
      });
    } else if ('WorkspaceDeleted' in payload) {
      const { workspace_id } = payload.WorkspaceDeleted;
      
      confirmOptimisticUpdate(`workspace:${workspace_id}`);
      
      scheduleBatchedUpdate(`workspace:${workspace_id}:delete`, () => {
        deleteWorkspace(workspace_id);
        logger.debug('[SYNC] Workspace deleted:', workspace_id);
      });
    } else if ('WorkspaceCreated' in payload) {
      const { workspace_id } = payload.WorkspaceCreated;
      
      // For new workspaces, we might need to fetch full details
      logger.debug('[SYNC] Workspace created:', workspace_id);
    }

    setSyncState((prev) => ({
      ...prev,
      lastSyncTime: new Date(),
    }));
  }, [updateWorkspace, deleteWorkspace, updateTask, addToast, confirmOptimisticUpdate, scheduleBatchedUpdate]);

  // ============================================================================
  // WebSocket Connection
  // ============================================================================

  const handleStatusChange = useCallback((status: ConnectionStatus) => {
    setSyncState((prev) => ({
      ...prev,
      isConnected: status === 'connected',
      isSyncing: status === 'connecting' || status === 'reconnecting',
    }));

    if (status === 'connected') {
      logger.debug('[SYNC] WebSocket connected');
    } else if (status === 'disconnected' || status === 'error') {
      logger.warn('[SYNC] WebSocket disconnected:', status);
    }
  }, []);

  const handleError = useCallback((error: Error) => {
    setSyncState((prev) => ({
      ...prev,
      syncErrors: [...prev.syncErrors.slice(-9), error.message], // Keep last 10 errors
    }));
    logger.error('[SYNC] WebSocket error:', error);
  }, []);

  const ws = useWebSocket({
    autoConnect: true,
    onEvent: handleWorkspaceEvent,
    onStatusChange: handleStatusChange,
    onError: handleError,
  });

  // Subscribe to all workspaces when connected
  useEffect(() => {
    if (ws.isConnected && workspaces.length > 0) {
      const workspaceIds = workspaces.map((w) => w.id);
      ws.subscribe(workspaceIds);
      logger.debug('[SYNC] Subscribed to workspaces:', workspaceIds);
    }
  }, [ws.isConnected, workspaces, ws.subscribe]);

  // ============================================================================
  // Manual Refresh Actions
  // ============================================================================

  const refreshWorkspaces = useCallback(() => {
    // Trigger a manual refresh by re-subscribing
    if (ws.isConnected && workspaces.length > 0) {
      const workspaceIds = workspaces.map((w) => w.id);
      ws.unsubscribe(workspaceIds);
      ws.subscribe(workspaceIds);
      logger.debug('[SYNC] Manual workspace refresh triggered');
    }
  }, [ws, workspaces]);

  const refreshTasks = useCallback(() => {
    // Tasks are typically refreshed through workspace events
    logger.debug('[SYNC] Manual task refresh triggered');
  }, []);

  // ============================================================================
  // Cleanup
  // ============================================================================

  useEffect(() => {
    return () => {
      // Clear all pending timeouts
      optimisticUpdatesRef.current.forEach((update) => {
        clearTimeout(update.timeout);
      });
      optimisticUpdatesRef.current.clear();

      if (batchTimerRef.current) {
        clearTimeout(batchTimerRef.current);
      }
    };
  }, []);

  return {
    syncState,
    connectionStatus: ws.status,
    refreshWorkspaces,
    refreshTasks,
    optimisticUpdateWorkspace,
    optimisticUpdateTask,
    connect: ws.connect,
    disconnect: ws.disconnect,
  };
}

export default useStateSynchronization;
