import { useCallback } from 'react';
import { useAppStore } from '../stores/appStore';
import { useWorkspaceStore, type Workspace } from '../stores/workspaceStore';
import { logger } from '../utils/logger';
import { api } from '../services/api';
import { useAsyncAction } from './useAsyncAction';

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
  const { execute, isLoading } = useAsyncAction();
  const activeWorkspaceId = useAppStore((state) => state.activeWorkspaceId);
  const setActiveWorkspace = useAppStore((state) => state.setActiveWorkspace);
  const deleteWorkspaceAndResolveActive = useWorkspaceStore(
    (state) => state.deleteWorkspaceAndResolveActive
  );

  const deleteWorkspace = useCallback(async (id: string) => {
    if (import.meta.env.DEV) logger.debug('deleteWorkspace called for id:', id);

    await execute(
      () => api.deleteWorkspace(id),
      {
        successMessage: '工作区已删除',
        errorPrefix: '删除失败',
        onSuccess: () => {
          if (import.meta.env.DEV) logger.debug('deleteWorkspace succeeded');
          const nextActiveWorkspaceId = deleteWorkspaceAndResolveActive(id, activeWorkspaceId);
          setActiveWorkspace(nextActiveWorkspaceId);
        },
        onError: (e) => {
          logger.error('deleteWorkspace error:', e);
        },
      },
    );
  }, [activeWorkspaceId, deleteWorkspaceAndResolveActive, execute, setActiveWorkspace]);

  const refreshWorkspace = useCallback(async (workspace: Workspace) => {
    if (import.meta.env.DEV) logger.debug('refreshWorkspace called for workspace:', workspace.id);

    await execute(
      () => api.refreshWorkspace(workspace.id, workspace.path),
      {
        successMessage: '刷新工作区中...',
        errorPrefix: '刷新失败',
        onError: (e) => {
          logger.error('refreshWorkspace error:', e);
        },
      },
    );
  }, [execute]);

  return {
    deleteWorkspace,
    refreshWorkspace,
    isLoading,
  };
};
