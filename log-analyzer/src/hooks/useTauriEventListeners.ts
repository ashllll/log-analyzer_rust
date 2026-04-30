import { useEffect, useRef } from 'react';
import toast from 'react-hot-toast';
import { listen } from '@tauri-apps/api/event';
import { useWorkspaceStore } from '../stores/workspaceStore';
import { useTaskStore } from '../stores/taskStore';
import { eventBus } from '../events/EventBus';
import type { TaskUpdateEvent, TaskRemovedEvent } from '../events/types';
import { logger } from '../utils/logger';

/**
 * Tauri 原生事件监听 Hook
 *
 * 负责将 Tauri 后端 IPC 事件桥接到应用层 EventBus：
 * - task-update → eventBus.processEvent('task-update')
 * - task-removed → eventBus.processEvent('task-removed')
 * - import-complete → 直接更新 task/workspace 状态
 * - import-error → toast 错误提示
 * - import-warning → toast 警告提示
 *
 * 使用 tauriCleanupRef 确保异步注册完成后同步清理。
 */
export const useTauriEventListeners = () => {
  const updateWorkspace = useWorkspaceStore((state) => state.updateWorkspace);
  const updateTask = useTaskStore((state) => state.updateTask);

  // 使用 ref 存储 Tauri 清理函数，确保在组件卸载时同步调用
  const tauriCleanupRef = useRef<(() => void) | null>(null);
  useEffect(() => {
    let isMounted = true;

    const setupTauriListeners = async () => {
      // 监听任务更新事件（从Tauri后端）
      const taskUpdateUnlisten = await listen<TaskUpdateEvent>('task-update', (event) => {
        logger.debug({ payload: event.payload }, '[TauriEventListeners] Received task-update from Tauri');

        // Rust后端直接发送正确的字段名（task_id, task_type）
        // null值转undefined（Zod不允许null，使用 ?? 而非 || 以保留空字符串语义）
        const cleanedPayload = {
          ...event.payload,
          workspace_id: event.payload.workspace_id ?? undefined,
        };

        // 桥接到EventBus处理（Schema验证、幂等性检查）
        eventBus.processEvent('task-update', cleanedPayload).catch((error) => {
          logger.error({ error }, '[TauriEventListeners] Failed to process task-update event');
        });
      });

      // 监听任务移除事件（从Tauri后端）
      const taskRemovedUnlisten = await listen<TaskRemovedEvent>('task-removed', (event) => {
        logger.debug({ payload: event.payload }, '[TauriEventListeners] Received task-removed from Tauri');

        // 桥接到EventBus处理
        eventBus.processEvent('task-removed', event.payload).catch((error) => {
          logger.error({ error }, '[TauriEventListeners] Failed to process task-removed event');
        });
      });

      // 监听导入完成事件（从Tauri后端）
      type ImportCompletePayload = string | { task_id?: string; workspace_id?: string };
      const importCompleteUnlisten = await listen<ImportCompletePayload>('import-complete', (event) => {
        logger.debug({ payload: event.payload }, '[TauriEventListeners] Received import-complete from Tauri');

        // 统一解析 payload：支持字符串（旧格式）或对象（新格式）
        const payload = event.payload;
        let taskId: string | null = null;
        let workspaceId: string | null = null;

        if (typeof payload === 'string') {
          taskId = payload;
        } else if (payload !== null && typeof payload === 'object') {
          taskId = payload.task_id ?? null;
          workspaceId = payload.workspace_id ?? null;
        }

        if (taskId) {
          updateTask(taskId, { status: 'COMPLETED', progress: 100 });
        }

        if (workspaceId) {
          logger.debug('[TauriEventListeners] import-complete with workspace_id, updating status to READY:', workspaceId);
          updateWorkspace(workspaceId, { status: 'READY' });
        } else if (taskId) {
          // 回退方案：从任务中查找 workspace_id
          const taskStore = useTaskStore.getState();
          const task = taskStore.tasks.find((t) => t.id === taskId);
          if (task?.workspaceId) {
            logger.debug('[TauriEventListeners] import-complete fallback, updating workspace status to READY:', task.workspaceId);
            updateWorkspace(task.workspaceId, { status: 'READY' });
          }
        }
      });

      // 监听导入错误事件（从Tauri后端）
      const importErrorUnlisten = await listen<string>('import-error', (event) => {
        logger.error({ payload: event.payload }, '[TauriEventListeners] Received import-error from Tauri');
        toast.error(`导入失败: ${event.payload}`);
      });

      // 监听导入警告事件（从Tauri后端）
      const importWarningUnlisten = await listen<string>('import-warning', (event) => {
        logger.warn({ payload: event.payload }, '[TauriEventListeners] Received import-warning from Tauri');
        toast(`导入提示: ${event.payload}`);
      });

      return () => {
        // 逐个 try-catch，防止第一个 unlisten 抛出时后续监听器泄漏
        [taskUpdateUnlisten, taskRemovedUnlisten, importCompleteUnlisten, importErrorUnlisten, importWarningUnlisten]
          .forEach((unlisten) => {
            try { unlisten(); } catch { /* 静默处理，Tauri unlisten 不应抛出 */ }
          });
      };
    };

    // 异步设置 Tauri 监听器
    setupTauriListeners()
      .then((cleanup) => {
        if (isMounted) {
          tauriCleanupRef.current = cleanup;
        } else {
          // 组件已卸载（如 React StrictMode 双重挂载、快速路由切换），立即清理
          cleanup();
        }
      })
      .catch((error: unknown) => {
        logger.error({ error }, '[TauriEventListeners] Tauri 事件监听器初始化失败，部分实时更新不可用');
      });

    return () => {
      isMounted = false;

      // 清理Tauri监听（同步调用，避免 Promise 时序问题）
      if (tauriCleanupRef.current) {
        tauriCleanupRef.current();
        tauriCleanupRef.current = null;
      }
    };
  }, [updateWorkspace, updateTask]);
};
