import { useEffect, useRef } from 'react';
import toast from 'react-hot-toast';
import { useWorkspaceStore } from './workspaceStore';
import { useTaskStore } from './taskStore';
import { eventBus } from '../events/EventBus';
import type { TaskUpdateEvent, TaskRemovedEvent } from '../events/types';
import { logger } from '../utils/logger';

/**
 * EventBus 订阅 Hook
 *
 * 负责注册应用层 EventBus 事件处理器：
 * - task-update: 任务状态更新（去重添加、工作区状态联动、toast 通知）
 * - task-removed: 任务自动清理
 *
 * 使用 initRef 防止 React StrictMode 重复注册。
 */
export const useEventBusSubscriptions = () => {
  const updateWorkspace = useWorkspaceStore((state) => state.updateWorkspace);
  const addTaskIfNotExists = useTaskStore((state) => state.addTaskIfNotExists);
  const updateTask = useTaskStore((state) => state.updateTask);
  const deleteTask = useTaskStore((state) => state.deleteTask);

  const initRef = useRef(false);

  useEffect(() => {
    // 防止 React StrictMode 重复初始化
    if (initRef.current) {
      return () => {
        // 空清理函数
      };
    }
    initRef.current = true;

    // 注册任务更新事件处理器
    const unsubscribeTaskUpdate = eventBus.on<TaskUpdateEvent>(
      'task-update',
      (event) => {
        const task = {
          id: event.task_id,
          type: event.task_type,
          target: event.target,
          progress: event.progress,
          message: event.message,
          status: event.status,
          workspaceId: event.workspace_id,
        };

        logger.debug({ task }, '[EventBusSubscriptions] Processing task update');

        // 使用去重添加
        addTaskIfNotExists(task);
        updateTask(task.id, task);

        // 更新工作区状态并发送 toast 通知
        if (task.workspaceId) {
          if (task.status === 'COMPLETED') {
            updateWorkspace(task.workspaceId, { status: 'READY' });
            toast.success('导入完成');
          } else if (task.status === 'RUNNING') {
            updateWorkspace(task.workspaceId, { status: 'PROCESSING' });
          } else if (task.status === 'FAILED') {
            updateWorkspace(task.workspaceId, { status: 'OFFLINE' });
          }
        }
      }
    );

    // 注册任务移除事件处理器
    const unsubscribeTaskRemoved = eventBus.on<TaskRemovedEvent>(
      'task-removed',
      (event) => {
        logger.info({ taskId: event.task_id }, '[EventBusSubscriptions] Auto-removing task');
        deleteTask(event.task_id);
      }
    );

    return () => {
      unsubscribeTaskUpdate();
      unsubscribeTaskRemoved();
    };
  }, [addTaskIfNotExists, updateTask, deleteTask, updateWorkspace]);
};