import { useCallback } from 'react';
import { useWorkspaceStore, type Workspace } from '../stores/workspaceStore';
import { logger } from '../utils/logger';

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

  const refreshWorkspaces = useCallback(async () => {
    if (import.meta.env.DEV) logger.debug('refreshWorkspaces called');
    // 工作区列表由后端事件驱动更新
    // 此处仅做日志记录
    if (import.meta.env.DEV) logger.debug('Workspaces refreshed from store');
  }, []);

  return {
    workspaces,
    loading,
    error,
    refreshWorkspaces,
  };
};
