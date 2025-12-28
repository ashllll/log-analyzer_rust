import { useEffect, ReactNode } from 'react';
import { useAppStore } from './appStore';
import { useWorkspaceStore } from './workspaceStore';
import { useKeywordStore } from './keywordStore';
import { useTaskStore } from './taskStore';
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

  useEffect(() => {
    // 加载配置
    const loadConfig = async () => {
      try {
        const config = await invoke<Record<string, unknown>>('load_config');
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
        console.error('Failed to load config:', error);
        addToast('error', '加载配置失败');
        setInitialized(false, String(error));
      }
    };

    loadConfig();

    // ============================================================================
       // 企业级事件系统 - EventBus集成
       // ============================================================================

    // 注册任务更新事件处理器
    const unsubscribeTaskUpdate = eventBus.on<TaskUpdateEvent>(
      'task-update',
      (event) => {
        // 老王备注：EventBus已经验证过Schema，这里直接用
        const task = {
          id: event.task_id,
          type: event.task_type,  // 老王备注：后端已修复，直接使用task_type
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

        // 更新工作区状态
        if (task.workspaceId) {
          if (task.status === 'COMPLETED') {
            updateWorkspace(task.workspaceId, { status: 'READY' });
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
        // 老王备注：null值转undefined（Zod不允许null）
        const cleanedPayload = {
          ...event.payload,
          workspace_id: event.payload.workspace_id || undefined,
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

      return () => {
        taskUpdateUnlisten();
        taskRemovedUnlisten();
      };
    };

    const cleanupPromise = setupTauriListeners();

    // 清理函数：同时清理EventBus订阅和Tauri监听
    return () => {
      // 清理EventBus订阅
      unsubscribeTaskUpdate();
      unsubscribeTaskRemoved();

      // 清理Tauri监听
      cleanupPromise.then((cleanup) => cleanup());
    };
  }, [addToast, setWorkspaces, setKeywordGroups, addTaskIfNotExists, updateTask, deleteTask, updateWorkspace, setInitialized]);

  return <>{children}</>;
};
