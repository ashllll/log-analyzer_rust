import { useEffect, ReactNode } from 'react';
import { useAppStore } from './appStore';
import { useWorkspaceStore } from './workspaceStore';
import { useKeywordStore } from './keywordStore';
import { useTaskStore } from './taskStore';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

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
 */
export const AppStoreProvider = ({ children }: AppStoreProviderProps) => {
  const addToast = useAppStore((state) => state.addToast);
  const setWorkspaces = useWorkspaceStore((state) => state.setWorkspaces);
  const updateWorkspace = useWorkspaceStore((state) => state.updateWorkspace);
  const setKeywordGroups = useKeywordStore((state) => state.setKeywordGroups);
  const addTaskIfNotExists = useTaskStore((state) => state.addTaskIfNotExists);
  const updateTask = useTaskStore((state) => state.updateTask);

  useEffect(() => {
    // 加载配置
    const loadConfig = async () => {
      try {
        const config = await invoke<any>('load_config');
        if (config) {
          if (config.workspaces) {
            setWorkspaces(config.workspaces);
          }
          if (config.keyword_groups) {
            setKeywordGroups(config.keyword_groups);
          }
        }
      } catch (error) {
        console.error('Failed to load config:', error);
        addToast('error', '加载配置失败');
      }
    };

    loadConfig();

    // 设置事件监听器
    const setupListeners = async () => {
      // 监听任务更新事件
      const taskUnlisten = await listen<any>('task-update', (event) => {
        const progress = event.payload;
        const task = {
          id: progress.task_id,
          type: progress.task_type,
          target: progress.target,
          progress: progress.progress,
          message: progress.message,
          status: progress.status,
          workspaceId: progress.workspace_id,
        };

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
      });

      return () => {
        taskUnlisten();
      };
    };

    const cleanupPromise = setupListeners();

    // 清理函数
    return () => {
      cleanupPromise.then((cleanup) => cleanup());
    };
  }, [addToast, setWorkspaces, setKeywordGroups, addTaskIfNotExists, updateTask, updateWorkspace]);

  return <>{children}</>;
};
