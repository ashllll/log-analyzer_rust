import { useCallback } from 'react';
import { useShallow } from 'zustand/shallow';
import { useTaskStore } from '../stores/taskStore';
import { api } from '../services/api';
import { useAsyncAction } from './useAsyncAction';
import { useToast } from './useToast';

/**
 * 任务管理Hook
 * 提供任务操作方法
 * 注意：任务事件监听已在 appStore 中处理
 */
export const useTaskManager = () => {
  const tasks = useTaskStore(useShallow((state) => state.tasks));
  const tasksLoading = useTaskStore((state) => state.loading);
  const tasksError = useTaskStore((state) => state.error);
  const deleteTaskAction = useTaskStore((state) => state.deleteTask);
  const updateTaskAction = useTaskStore((state) => state.updateTask);
  const { showToast: addToast } = useToast();
  const { execute } = useAsyncAction();

  /**
   * 删除任务（同步）
   */
  const deleteTask = useCallback((id: string) => {
    deleteTaskAction(id);
    addToast('info', '任务已删除');
  }, [addToast, deleteTaskAction]);

  /**
   * 取消任务
   */
  const cancelTask = useCallback(async (id: string) => {
    await execute(
      () => api.cancelTask(id),
      {
        successMessage: '任务已取消',
        errorPrefix: '取消任务失败',
        onSuccess: () => updateTaskAction(id, { status: 'STOPPED' }),
      },
    );
  }, [execute, updateTaskAction]);

  return {
    tasks,
    loading: tasksLoading,
    error: tasksError,
    deleteTask,
    cancelTask
  };
};
