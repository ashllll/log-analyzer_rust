import { useCallback, useState } from 'react';
import { useAppStore } from '../stores/appStore';
import { useWorkspaceStore, type Workspace } from '../stores/workspaceStore';
import { logger } from '../utils/logger';
import { api } from '../services/api';
import { getFullErrorMessage } from '../services/errors';
import { useToast } from './useToast';

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
  const { showToast: addToast } = useToast();
  const activeWorkspaceId = useAppStore((state) => state.activeWorkspaceId);
  const setActiveWorkspace = useAppStore((state) => state.setActiveWorkspace);
  const workspaces = useWorkspaceStore((state) => state.workspaces);
  const deleteWorkspace = useWorkspaceStore((state) => state.deleteWorkspace);

  const [isLoading, setIsLoading] = useState(false);

  const deleteWorkspaceOp = useCallback(async (id: string) => {
    if (import.meta.env.DEV) logger.debug('deleteWorkspace called for id:', id);
    setIsLoading(true);

    try {
      await api.deleteWorkspace(id);

      if (import.meta.env.DEV) logger.debug('deleteWorkspace succeeded');

      deleteWorkspace(id);

      if (activeWorkspaceId === id) {
        const remaining = workspaces.filter((workspace) => workspace.id !== id);
        setActiveWorkspace(remaining.length > 0 ? remaining[0].id : null);
      }

      addToast('success', '工作区已删除');
    } catch (e) {
      logger.error('deleteWorkspace error:', e);
      addToast('error', `删除失败: ${getFullErrorMessage(e)}`);
    } finally {
      setIsLoading(false);
    }
  }, [activeWorkspaceId, addToast, deleteWorkspace, setActiveWorkspace, workspaces]);

  const refreshWorkspace = useCallback(async (workspace: Workspace) => {
    if (import.meta.env.DEV) logger.debug('refreshWorkspace called for workspace:', workspace.id);
    setIsLoading(true);

    try {
      const taskId = await api.refreshWorkspace(workspace.id, workspace.path);

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
