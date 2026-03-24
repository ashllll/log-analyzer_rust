import { useCallback, useState } from 'react';
import { useAppStore } from '../stores/appStore';
import { useWorkspaceStore, type Workspace } from '../stores/workspaceStore';
import { logger } from '../utils/logger';
import { api } from '../services/api';
import { getFullErrorMessage } from '../services/errors';

export interface UseWorkspaceManagementReturn {
  deleteWorkspace: (id: string) => Promise<void>;
  refreshWorkspace: (workspace: Workspace) => Promise<void>;
  isLoading: boolean;
}

/**
 * 工作区管理Hook
 * 封装工作区删除、刷新等管理操作
 */
export const useWorkspaceManagement = (): UseWorkspaceManagementReturn => {
  const addToast = useAppStore((state) => state.addToast);
  const setActiveWorkspace = useAppStore((state) => state.setActiveWorkspace);
  const deleteWorkspace = useWorkspaceStore((state) => state.deleteWorkspace);

  const [isLoading, setIsLoading] = useState(false);

  const deleteWorkspaceOp = useCallback(async (id: string) => {
    if (import.meta.env.DEV) logger.debug('deleteWorkspace called for id:', id);
    setIsLoading(true);

    try {
      await api.deleteWorkspace(id);

      if (import.meta.env.DEV) logger.debug('deleteWorkspace succeeded');

      deleteWorkspace(id);

      // 从 store 读取最新快照（deleteWorkspace 已更新 store，旧闭包快照不可靠）
      const currentActiveId = useAppStore.getState().activeWorkspaceId;
      if (currentActiveId === id) {
        const remaining = useWorkspaceStore.getState().workspaces;
        setActiveWorkspace(remaining.length > 0 ? remaining[0].id : null);
      }

      addToast('success', '工作区已删除');
    } catch (e) {
      logger.error('deleteWorkspace error:', e);
      addToast('error', `删除失败: ${getFullErrorMessage(e)}`);
    } finally {
      setIsLoading(false);
    }
  }, [addToast, deleteWorkspace, setActiveWorkspace]);

  const refreshWorkspace = useCallback(async (workspace: Workspace) => {
    if (import.meta.env.DEV) logger.debug('refreshWorkspace called for workspace:', workspace.id);
    setIsLoading(true);

    try {
      const taskId = await api.refreshWorkspace(workspace.id);

      if (import.meta.env.DEV) logger.debug('refresh_workspace returned taskId:', taskId);

      addToast('info', '刷新工作区中...');
    } catch (e) {
      logger.error('refreshWorkspace error:', e);
      addToast('error', `刷新失败: ${getFullErrorMessage(e)}`);
    } finally {
      setIsLoading(false);
    }
  }, [addToast]);

  return {
    deleteWorkspace: deleteWorkspaceOp,
    refreshWorkspace,
    isLoading,
  };
};
