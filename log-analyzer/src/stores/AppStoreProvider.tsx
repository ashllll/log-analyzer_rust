import { useEffect, useRef, ReactNode } from 'react';
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

  // 使用 ref 存储 Tauri 清理函数，确保在组件卸载时同步调用
  // 注意：必须在组件顶层声明，不能在 useEffect 内部
  const tauriCleanupRef = useRef<(() => void) | null>(null);

  // 使用 ref 跟踪初始化状态，防止 React StrictMode 重复初始化
  const initRef = useRef<{ tauri: boolean; eventBus: boolean }>({
    tauri: false,
    eventBus: false,
  });

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
        // 配置加载失败时提供默认工作区作为降级方案
        const defaultWorkspace: Workspace = {
          id: 'default-workspace',
          name: '默认工作区',
          path: '', // 空路径，用户需要添加实际路径
          status: 'OFFLINE',
          size: '0 B',
          files: 0,
          watching: false,
        };
        setWorkspaces([defaultWorkspace]);
        setKeywordGroups([]);
        addToast('error', '加载配置失败，使用默认工作区');
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

    // 防止 React StrictMode 重复初始化
    if (initRef.current.eventBus) {
      // EventBus 订阅已初始化，跳过
      return () => {
        // 空清理函数，因为没有新的订阅
      };
    }
    initRef.current.eventBus = true;

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
      // 防止 React StrictMode 重复初始化
      if (initRef.current.tauri) {
        logger.debug('[AppStoreProvider] Tauri listeners already initialized, skipping');
        return () => {
          // 空清理函数
        };
      }
      initRef.current.tauri = true;

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

        // 统一解析 payload：支持字符串（旧格式）或对象（新格式）
        const payload = event.payload;
        let taskId: string | null = null;
        let workspaceId: string | null = null;

        if (typeof payload === 'string') {
          // 旧格式：直接是 task_id 字符串
          taskId = payload;
        } else if (payload !== null && typeof payload === 'object') {
          // 新格式：对象包含 task_id 和 workspace_id
          taskId = payload.task_id ?? null;
          workspaceId = payload.workspace_id ?? null;
        }
        // 注意：null 和 undefined 不满足任何条件，无需处理
        
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
        // 逐个 try-catch，防止第一个 unlisten 抛出时后续监听器泄漏
        [taskUpdateUnlisten, taskRemovedUnlisten, importCompleteUnlisten, importErrorUnlisten]
          .forEach((unlisten) => {
            try { unlisten(); } catch { /* 静默处理，Tauri unlisten 不应抛出 */ }
          });
      };
    };

    // 异步设置 Tauri 监听器
    setupTauriListeners()
      .then((cleanup) => {
        if (isMounted) {
          // 组件仍挂载，正常保存清理函数
          tauriCleanupRef.current = cleanup;
        } else {
          // 组件已卸载（如 React StrictMode 双重挂载、快速路由切换），立即清理
          cleanup();
        }
      })
      .catch((error: unknown) => {
        // 捕获 listen() 失败等初始化错误，避免静默丢弃
        // 不向用户显示 toast（初始化阶段 store 可能未就绪），仅记录诊断日志
        logger.error({ error }, '[AppStoreProvider] Tauri 事件监听器初始化失败，部分实时更新不可用');
      });

    // 清理函数：同时清理EventBus订阅、Tauri监听和定时器
    return () => {
      isMounted = false;
      clearTimeout(timer);
      // 清理EventBus订阅
      unsubscribeTaskUpdate();
      unsubscribeTaskRemoved();

      // 清理Tauri监听（同步调用，避免 Promise 时序问题）
      if (tauriCleanupRef.current) {
        tauriCleanupRef.current();
        tauriCleanupRef.current = null;
      }
    };
  }, [addToast, setWorkspaces, setKeywordGroups, addTaskIfNotExists, updateTask, deleteTask, updateWorkspace, setInitialized]);

  return <>{children}</>;
};
