import { useEffect, ReactNode } from 'react';
import toast from 'react-hot-toast';
import { useAppStore } from './appStore';
import { useWorkspaceStore } from './workspaceStore';
import { useKeywordStore } from './keywordStore';
import { useTaskStore } from './taskStore';
import { useConfigManager } from '../hooks/useConfigManager';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { eventBus } from '../events/EventBus';
import type { TaskUpdateEvent, TaskRemovedEvent } from '../events/types';
import type { Workspace, KeywordGroup } from '../types/common';
import { logger } from '../utils/logger';

interface AppStoreProviderProps {
  children: ReactNode;
}

/**
 * AppStoreProvider - 初始化 zustand stores 并设置事件监听器
 * 
 * 这个组件负责：
 * 1. 在应用启动时加载配置
 * 2. 设置后端事件监听器
 * 3. 在组件卸载时清理事件监听器
 * 
 * ## 任务生命周期管理
 * 
 * 采用业内成熟的事件驱动架构：
 * - task-update: 任务状态更新（创建、进度、完成）
 * - task-removed: 任务自动清理（后端 Actor 触发）
 * - import-complete: 导入完成事件
 * - import-error: 导入错误事件
 */
export const AppStoreProvider = ({ children }: AppStoreProviderProps) => {
  const addToast = useAppStore((state) => state.addToast);
  const setWorkspaces = useWorkspaceStore((state) => state.setWorkspaces);
  const updateWorkspace = useWorkspaceStore((state) => state.updateWorkspace);
  const setKeywordGroups = useKeywordStore((state) => state.setKeywordGroups);
  const addTaskIfNotExists = useTaskStore((state) => state.addTaskIfNotExists);
  const updateTask = useTaskStore((state) => state.updateTask);
  const deleteTask = useTaskStore((state) => state.deleteTask);
  const setInitialized = useAppStore((state) => state.setInitialized);

  // 启用配置自动保存（防抖1000ms后保存到后端）
  useConfigManager();

  useEffect(() => {
    let isMounted = true;

    // 加载配置 - 不阻塞UI渲染
    const loadConfig = async () => {
      try {
        const config = await invoke<Record<string, unknown>>('load_config');
        if (!isMounted) return; // 组件已卸载，跳过状态更新

        if (config) {
          if (config.workspaces) {
            setWorkspaces(config.workspaces as Workspace[]);
          }
          if (config.keyword_groups) {
            setKeywordGroups(config.keyword_groups as KeywordGroup[]);
          }
        }

        // 标记应用已初始化
        setInitialized(true);
      } catch (error) {
        if (!isMounted) return; // 组件已卸载，跳过状态更新
        logger.error({ error }, 'Failed to load config');
        // 确保空默认值，避免应用因无工作区/关键词组而不可用
        setWorkspaces([]);
        setKeywordGroups([]);
        addToast('error', '加载配置失败，使用默认配置');
        setInitialized(true); // 关键：即使失败也标记为已初始化
      }
    };

    // 延迟加载配置，避免阻塞首屏渲染
    const timer = setTimeout(() => {
      loadConfig();
    }, 100);

    // ============================================================================
       // 企业级事件系统 - EventBus集成
       // ============================================================================

    // 注册任务更新事件处理器
    const unsubscribeTaskUpdate = eventBus.on<TaskUpdateEvent>(
      'task-update',
      (event) => {
        // EventBus已经验证过Schema，这里直接用
        const task = {
          id: event.task_id,
          type: event.task_type,
          target: event.target,
          progress: event.progress,
          message: event.message,
          status: event.status,
          workspaceId: event.workspace_id,
        };

        logger.debug({ task }, '[AppStoreProvider] Processing task update');

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
        logger.info({ taskId: event.task_id }, '[AppStoreProvider] Auto-removing task');
        deleteTask(event.task_id);
      }
    );

    // 设置Tauri原生事件监听（桥接到EventBus）
    const setupTauriListeners = async () => {
      // 监听任务更新事件（从Tauri后端）
      const taskUpdateUnlisten = await listen<TaskUpdateEvent>('task-update', (event) => {
        logger.debug({ payload: event.payload }, '[AppStoreProvider] Received task-update from Tauri');

        // 老王备注：Rust后端现在直接发送正确的字段名（task_id, task_type）
        // null值转undefined（Zod不允许null，使用 ?? 而非 || 以保留空字符串语义）
        const cleanedPayload = {
          ...event.payload,
          workspace_id: event.payload.workspace_id ?? undefined,
        };

        // 桥接到EventBus处理（Schema验证、幂等性检查）
        eventBus.processEvent('task-update', cleanedPayload).catch((error) => {
          logger.error({ error }, '[AppStoreProvider] Failed to process task-update event');
        });
      });

      // 监听任务移除事件（从Tauri后端）
      const taskRemovedUnlisten = await listen<TaskRemovedEvent>('task-removed', (event) => {
        logger.debug({ payload: event.payload }, '[AppStoreProvider] Received task-removed from Tauri');

        // 老王备注：Rust后端现在直接发送正确的字段名
        // 桥接到EventBus处理
        eventBus.processEvent('task-removed', event.payload).catch((error) => {
          logger.error({ error }, '[AppStoreProvider] Failed to process task-removed event');
        });
      });

      // 监听导入完成事件（从Tauri后端）
      type ImportCompletePayload = string | { task_id?: string; workspace_id?: string };
      const importCompleteUnlisten = await listen<ImportCompletePayload>('import-complete', (event) => {
        logger.debug({ payload: event.payload }, '[AppStoreProvider] Received import-complete from Tauri');
        
        // 支持两种 payload 格式：字符串（旧格式）或对象（新格式）
        const payload = event.payload;
        const taskId = typeof payload === 'string' ? payload : payload?.task_id;
        const workspaceId = typeof payload === 'object' ? payload?.workspace_id : null;
        
        if (taskId) {
          updateTask(taskId, { status: 'COMPLETED', progress: 100 });
        }
        
        // 如果有 workspace_id，更新 workspace 状态
        if (workspaceId) {
          logger.debug('[AppStoreProvider] import-complete with workspace_id, updating status to READY:', workspaceId);
          updateWorkspace(workspaceId, { status: 'READY' });
        } else if (taskId) {
          // 回退方案：从任务中查找 workspace_id
          const taskStore = useTaskStore.getState();
          const task = taskStore.tasks.find((t) => t.id === taskId);
          if (task?.workspaceId) {
            logger.debug('[AppStoreProvider] import-complete fallback, updating workspace status to READY:', task.workspaceId);
            updateWorkspace(task.workspaceId, { status: 'READY' });
          }
        }
      });

      // 监听导入错误事件（从Tauri后端）
      const importErrorUnlisten = await listen<string>('import-error', (event) => {
        logger.error({ payload: event.payload }, '[AppStoreProvider] Received import-error from Tauri');
        toast.error(`导入失败: ${event.payload}`);
      });

      return () => {
        taskUpdateUnlisten();
        taskRemovedUnlisten();
        importCompleteUnlisten();
        importErrorUnlisten();
      };
    };

    const cleanupPromise = setupTauriListeners();

    // 清理函数：同时清理EventBus订阅、Tauri监听和定时器
    return () => {
      isMounted = false;
      clearTimeout(timer);
      // 清理EventBus订阅
      unsubscribeTaskUpdate();
      unsubscribeTaskRemoved();

      // 清理Tauri监听
      cleanupPromise.then((cleanup) => cleanup());
    };
  }, [addToast, setWorkspaces, setKeywordGroups, addTaskIfNotExists, updateTask, deleteTask, updateWorkspace, setInitialized]);

  return <>{children}</>;
};
