import { useCallback, useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { useApp, useTaskState, useWorkspaceState } from '../contexts/AppContext';

interface TaskUpdateEvent {
  task_id: string;
  task_type: string;
  target: string;
  status: string;
  message: string;
  progress: number;
}

// 日志工具 - 供调试使用
// @ts-ignore - used for debugging
const logger = {
  debug: (message: string, ...args: any[]) => {
    if (import.meta.env.DEV) {
      console.log(`[DEBUG] ${message}`, ...args);
    }
  },
  error: (message: string, ...args: any[]) => {
    console.error(`[ERROR] ${message}`, ...args);
  }
};

/**
 * 任务管理Hook
 * 监听后端任务事件并更新状态
 * 提供任务操作方法
 */
export const useTaskManager = () => {
  const { addToast } = useApp();
  const { state: taskState, dispatch: taskDispatch } = useTaskState();
  const { dispatch: workspaceDispatch } = useWorkspaceState();

  /**
   * 删除任务
   */
  const deleteTask = useCallback((id: string) => {
    taskDispatch({ type: 'DELETE_TASK', payload: id });
    addToast('info', '任务已删除');
  }, [addToast, taskDispatch]);

  /**
   * 监听后端任务事件
   */
  useEffect(() => {
    // 任务更新节流器：合并200ms内的更新事件
    const taskUpdateBuffer = new Map<string, TaskUpdateEvent>();
    let updateTimer: ReturnType<typeof setTimeout> | null = null;
    
    const flushTaskUpdates = () => {
      if (taskUpdateBuffer.size === 0) return;
      
      // 批量更新所有任务
      taskUpdateBuffer.forEach((update, taskId) => {
        taskDispatch({
          type: 'UPDATE_TASK',
          payload: {
            id: taskId,
            updates: {
              type: update.task_type || 'Import',
              target: update.target || 'Unknown',
              status: update.status as any,
              message: update.message,
              progress: update.progress
            }
          }
        });
        
        // 如果任务不存在，添加它
        if (!taskState.tasks.find(t => t.id === taskId)) {
          taskDispatch({
            type: 'ADD_TASK',
            payload: {
              id: taskId,
              type: update.task_type || 'Import',
              target: update.target || 'Unknown',
              progress: update.progress,
              status: update.status as any,
              message: update.message
            }
          });
        }
      });
      
      taskUpdateBuffer.clear();
    };

    // 监听任务更新事件
    const u1 = listen<TaskUpdateEvent>('task-update', e => {
      const update = e.payload;
      
      // 将更新缓存到buffer
      taskUpdateBuffer.set(update.task_id, update);
      
      // 设置节流定时器
      if (updateTimer) clearTimeout(updateTimer);
      updateTimer = setTimeout(flushTaskUpdates, 200);
    });
    
    // 监听导入完成事件
    const u2 = listen('import-complete', (e: any) => {
      // 更新工作区状态为READY
      workspaceDispatch({
        type: 'UPDATE_WORKSPACE',
        payload: {
          id: e.payload.workspace_id || '',
          updates: { status: 'READY' }
        }
      });
      
      // 更新对应的任务状态
      const taskId = e.payload.task_id || e.payload;
      if (taskId) {
        taskDispatch({
          type: 'UPDATE_TASK',
          payload: {
            id: taskId,
            updates: {
              status: 'COMPLETED',
              progress: 100,
              message: 'Done'
            }
          }
        });
      }
      
      addToast('success', '处理完成');
    });
    
    // 监听错误事件
    const u3 = listen('import-error', (e) => {
      addToast('error', `错误: ${e.payload}`);
    });
    
    return () => {
      if (updateTimer) clearTimeout(updateTimer);
      flushTaskUpdates(); // 清理剩余更新
      u1.then(f => f());
      u2.then(f => f());
      u3.then(f => f());
    };
  }, [addToast, taskDispatch, workspaceDispatch, taskState.tasks]);

  return {
    tasks: taskState.tasks,
    loading: taskState.loading,
    error: taskState.error,
    deleteTask
  };
};
