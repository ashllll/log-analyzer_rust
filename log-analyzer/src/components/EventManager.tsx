import { useEffect, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';
import toast from 'react-hot-toast';
import { useAppStore } from '../stores/appStore';
import { logger } from '../utils/logger';

/**
 * EventManager component that handles all backend event listeners
 * using React's native event management patterns with proper cleanup.
 * 
 * 采用 Zustand 推荐的模式：在事件回调中使用 getState() 获取最新状态，
 * 避免 React 闭包陷阱导致的状态过期问题。
 */
export const EventManager = () => {
  const createdTaskIdsRef = useRef(new Set<string>());
  const isInitializedRef = useRef(false);

  useEffect(() => {
    // 防止重复初始化（StrictMode 下会调用两次）
    if (isInitializedRef.current) {
      return;
    }
    isInitializedRef.current = true;
    
    logger.debug('[EVENT_MANAGER] Initializing event listeners');
    
    // Array to store cleanup functions
    const cleanupFunctions: (() => void)[] = [];
    
    // Listen for task updates
    // 使用 getState() 在回调中获取最新状态，避免闭包捕获旧值
    const setupTaskUpdateListener = async () => {
      const unlisten = await listen<any>('task-update', (event) => {
        const { task_id, task_type, target, status, message, progress, workspace_id } = event.payload;
        logger.debug('[EVENT] task-update:', event.payload);
        
        // 使用 getState() 获取最新的 store 状态和 actions
        const store = useAppStore.getState();
        const existingTask = store.tasks.find(t => t.id === task_id);
        
        if (!existingTask && !createdTaskIdsRef.current.has(task_id)) {
          // Create new task
          logger.debug('[EVENT] Task not found, creating new task:', task_id);
          createdTaskIdsRef.current.add(task_id);
          store.addTaskIfNotExists({
            id: task_id,
            type: task_type,
            target,
            progress,
            status: status as 'RUNNING' | 'COMPLETED' | 'FAILED' | 'STOPPED',
            message,
            workspaceId: workspace_id
          });
        } else if (existingTask || createdTaskIdsRef.current.has(task_id)) {
          // Update existing task
          store.updateTask(task_id, {
            type: task_type,
            target,
            status: status as 'RUNNING' | 'COMPLETED' | 'FAILED' | 'STOPPED',
            message,
            progress
          });
        }
        
        // Update workspace status based on task completion
        if (status === 'COMPLETED' && workspace_id) {
          logger.info('[EVENT] ✅ Task completed, updating workspace status to READY:', workspace_id);
          store.updateWorkspace(workspace_id, { status: 'READY' });
          toast.success('导入完成');
        } else if (status === 'FAILED' && workspace_id) {
          logger.error('[EVENT] ❌ Task failed, updating workspace status to OFFLINE:', workspace_id);
          store.updateWorkspace(workspace_id, { status: 'OFFLINE' });
        }
      });
      
      cleanupFunctions.push(unlisten);
    };
    
    // Listen for import complete events
    // 同样使用 getState() 模式
    const setupImportCompleteListener = async () => {
      const unlisten = await listen<any>('import-complete', (event) => {
        logger.debug('[EVENT] import-complete:', event.payload);
        const store = useAppStore.getState();
        
        // 支持两种 payload 格式：字符串（旧格式）或对象（新格式）
        const payload = event.payload;
        const taskId = typeof payload === 'string' ? payload : payload?.task_id;
        const workspaceId = typeof payload === 'object' ? payload?.workspace_id : null;
        
        if (taskId) {
          store.updateTask(taskId, { status: 'COMPLETED', progress: 100 });
        }
        
        // 如果有 workspace_id，更新 workspace 状态
        if (workspaceId) {
          logger.debug('[EVENT] import-complete with workspace_id, updating status to READY:', workspaceId);
          store.updateWorkspace(workspaceId, { status: 'READY' });
        } else if (taskId) {
          // 回退方案：从任务中查找 workspace_id
          const task = store.tasks.find(t => t.id === taskId);
          if (task?.workspaceId) {
            logger.debug('[EVENT] import-complete fallback, updating workspace status to READY:', task.workspaceId);
            store.updateWorkspace(task.workspaceId, { status: 'READY' });
          }
        }
      });
      
      cleanupFunctions.push(unlisten);
    };
    
    // Listen for import error events
    const setupImportErrorListener = async () => {
      const unlisten = await listen<string>('import-error', (event) => {
        logger.error('[EVENT] import-error:', event.payload);
        toast.error(`导入失败: ${event.payload}`);
      });
      
      cleanupFunctions.push(unlisten);
    };
    
    // Initialize all listeners
    const initializeListeners = async () => {
      try {
        await Promise.all([
          setupTaskUpdateListener(),
          setupImportCompleteListener(),
          setupImportErrorListener()
        ]);
        logger.debug('[EVENT_MANAGER] All event listeners initialized');
      } catch (error) {
        logger.error('[EVENT_MANAGER] Failed to initialize event listeners:', error);
      }
    };
    
    initializeListeners();
    
    // Cleanup function - this is React's native cleanup pattern
    return () => {
      logger.debug('[EVENT_MANAGER] Cleaning up event listeners');
      isInitializedRef.current = false;
      
      // Clean up all event listeners
      cleanupFunctions.forEach(cleanup => {
        try {
          cleanup();
        } catch (error) {
          logger.error('[EVENT_MANAGER] Error during cleanup:', error);
        }
      });
      
      // Clear the created tasks set
      createdTaskIdsRef.current.clear();
    };
  }, []); // 空依赖数组，只在组件挂载时初始化一次

  // This component doesn't render anything - it's just for event management
  return null;
};