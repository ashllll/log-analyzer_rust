import { useEffect, useCallback } from 'react';
import toast from 'react-hot-toast';
import { useWorkspaceStore } from '../stores/workspaceStore';
import { useTaskStore } from '../stores/taskStore';
import { eventBus } from '../events/EventBus';
import { api } from '../services/api';
import type { TaskUpdateEvent, TaskRemovedEvent, WorkspaceEvent } from '../events/types';
import { logger } from '../utils/logger';

/**
 * EventBus 订阅 Hook
 *
 * 负责注册应用层 EventBus 事件处理器：
 * - task-update: 任务状态更新（upsert 合并操作、工作区状态联动、toast 通知）
 * - task-removed: 任务自动清理
 * - workspace-event: 工作区状态变更（统一处理，替代 App.tsx 中的直接监听）
 *
 */
export const useEventBusSubscriptions = () => {
  const updateWorkspace = useWorkspaceStore((state) => state.updateWorkspace);
  const setWorkspaces = useWorkspaceStore((state) => state.setWorkspaces);
  const upsertTask = useTaskStore((state) => state.upsertTask);
  const deleteTask = useTaskStore((state) => state.deleteTask);

  // 统一刷新工作区列表的方法
  const refreshWorkspaces = useCallback(async () => {
    try {
      const config = await api.loadConfig();
      if (config.workspaces) {
        setWorkspaces(config.workspaces);
      }
    } catch (err) {
      logger.error('Failed to refresh workspaces from event:', err);
    }
  }, [setWorkspaces]);

  useEffect(() => {
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

        // 使用 upsert 合并添加/更新为单次操作
        upsertTask(task);

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

    // 注册工作区事件处理器（统一处理工作区状态变更）
    const unsubscribeWorkspaceEvent = eventBus.on<WorkspaceEvent>(
      'workspace-event',
      (event) => {
        logger.debug({ event }, '[EventBusSubscriptions] Processing workspace event');

        switch (event.type) {
          case 'StatusChanged': {
            const toastType = event.status === 'Cancelled' ? 'error' : 'success';
            const toastMessage = event.status === 'Cancelled'
              ? 'Workspace deleted'
              : event.status === 'Completed'
                ? 'Workspace updated'
                : event.message ?? 'Workspace status changed';

            toast[toastType](toastMessage);

            // 刷新工作区列表以同步最新状态
            refreshWorkspaces();
            break;
          }
          case 'Created': {
            toast.success(event.name ? `Workspace "${event.name}" created` : 'Workspace created');
            refreshWorkspaces();
            break;
          }
          case 'Deleted': {
            toast.success('Workspace deleted');
            refreshWorkspaces();
            break;
          }
        }
      }
    );

    return () => {
      unsubscribeTaskUpdate();
      unsubscribeTaskRemoved();
      unsubscribeWorkspaceEvent();
    };
  }, [upsertTask, deleteTask, updateWorkspace, refreshWorkspaces]);
};
