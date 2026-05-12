import { useCallback } from 'react';
import { useShallow } from 'zustand/shallow';
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
  // 合并多个 store 选择器为一次调用，避免多次订阅
  const { workspaces, loading, error, setWorkspaces } = useWorkspaceStore(
    useShallow((state) => ({
      workspaces: state.workspaces,
      loading: state.loading,
      error: state.error,
      setWorkspaces: state.setWorkspaces,
    }))
  );

  const refreshWorkspaces = useCallback(async () => {
    try {
      if (import.meta.env.DEV) logger.debug('refreshWorkspaces called');
      const config = await api.loadConfig();
      // api.loadConfig() 已通过 AppConfigSchema 进行 Zod 验证，
      // config.workspaces 类型为 Workspace[]，无需类型断言
      if (config.workspaces) {
        setWorkspaces(config.workspaces);
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
