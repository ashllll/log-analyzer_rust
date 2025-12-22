import { useCallback } from 'react';
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

  /**
   * 删除任务
   */
  const deleteTask = useCallback((id: string) => {
    deleteTaskAction(id);
    addToast('info', '任务已删除');
  }, [addToast, deleteTaskAction]);

  return {
    tasks,
    loading: tasksLoading,
    error: tasksError,
    deleteTask
  };
};
