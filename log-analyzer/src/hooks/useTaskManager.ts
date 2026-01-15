import { useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useAppStore } from '../stores/appStore';
import { useTaskStore } from '../stores/taskStore';

/**
 * 任务管理Hook
 * 提供任务操作方法
 * 注意：任务事件监听已在 appStore 中处理
 */
export const useTaskManager = () => {
  const addToast = useAppStore((state) => state.addToast);
  const tasks = useTaskStore((state) => state.tasks);
  const tasksLoading = useTaskStore((state) => state.loading);
  const tasksError = useTaskStore((state) => state.error);
  const deleteTaskAction = useTaskStore((state) => state.deleteTask);
  const updateTaskAction = useTaskStore((state) => state.updateTask);

  /**
   * 删除任务
   */
  const deleteTask = useCallback((id: string) => {
    deleteTaskAction(id);
    addToast('info', '任务已删除');
  }, [addToast, deleteTaskAction]);

  /**
   * 取消任务
   */
  const cancelTask = useCallback(async (id: string) => {
    try {
      await invoke('cancel_task', { taskId: id });
      // 更新本地状态
      updateTaskAction(id, { status: 'STOPPED' });
      addToast('info', '任务已取消');
    } catch (error) {
      addToast('error', `取消任务失败: ${error}`);
    }
  }, [addToast, updateTaskAction]);

  return {
    tasks,
    loading: tasksLoading,
    error: tasksError,
    deleteTask,
    cancelTask
  };
};
