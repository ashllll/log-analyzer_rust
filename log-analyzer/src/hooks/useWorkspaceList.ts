import { useCallback } from 'react';
import { useWorkspaceStore, type Workspace } from '../stores/workspaceStore';
import { logger } from '../utils/logger';
import { api } from '../services/api';

export interface UseWorkspaceListReturn {
  workspaces: Workspace[];
  loading: boolean;
  error: string | null;
  refreshWorkspaces: () => Promise<void>;
}

/**
 * 工作区列表Hook
 * 提供工作区列表访问和刷新功能
 */
export const useWorkspaceList = (): UseWorkspaceListReturn => {
  const workspaces = useWorkspaceStore((state) => state.workspaces);
  const loading = useWorkspaceStore((state) => state.loading);
  const error = useWorkspaceStore((state) => state.error);
  const setWorkspaces = useWorkspaceStore((state) => state.setWorkspaces);

  const refreshWorkspaces = useCallback(async () => {
    try {
      if (import.meta.env.DEV) logger.debug('refreshWorkspaces called');
      const config = await api.loadConfig();
      if (config.workspaces) {
        setWorkspaces(config.workspaces as Workspace[]);
      }
      if (import.meta.env.DEV) logger.debug('Workspaces refreshed from backend');
    } catch (err) {
      logger.error('Failed to refresh workspaces:', err);
    }
  }, [setWorkspaces]);

  return {
    workspaces,
    loading,
    error,
    refreshWorkspaces,
  };
};
