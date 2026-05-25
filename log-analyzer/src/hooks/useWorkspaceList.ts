import { useCallback, useRef } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { useShallow } from 'zustand/shallow';
import { useWorkspaceStore, type Workspace } from '../stores/workspaceStore';
import { logger } from '../utils/logger';
import { queryKeys } from '../services/api';

export interface UseWorkspaceListReturn {
  workspaces: Workspace[];
  loading: boolean;
  error: string | null;
  refreshWorkspaces: () => Promise<void>;
}

/** 刷新工作区列表的最小间隔（毫秒） */
const REFRESH_DEBOUNCE_MS = 500;

/**
 * 工作区列表Hook
 * 提供工作区列表访问和刷新功能
 *
 * 优化：添加防抖机制，避免频繁事件触发导致全量配置重复加载
 */
export const useWorkspaceList = (): UseWorkspaceListReturn => {
  const queryClient = useQueryClient();
  // 合并多个 store 选择器为一次调用，避免多次订阅
  const { workspaces, loading, error, setLoading, setError } = useWorkspaceStore(
    useShallow((state) => ({
      workspaces: state.workspaces,
      loading: state.loading,
      error: state.error,
      setLoading: state.setLoading,
      setError: state.setError,
    }))
  );

  const lastRefreshTime = useRef<number>(0);
  const pendingRefresh = useRef<Promise<void> | null>(null);

  const refreshWorkspaces = useCallback(async () => {
    // 如果已有进行中的刷新请求，复用同一个 Promise，避免并发全量加载
    if (pendingRefresh.current) {
      return pendingRefresh.current;
    }

    // 防抖：如果距离上次刷新不足 REFRESH_DEBOUNCE_MS，跳过本次请求
    const now = Date.now();
    if (now - lastRefreshTime.current < REFRESH_DEBOUNCE_MS) {
      if (import.meta.env.DEV) logger.debug('refreshWorkspaces skipped (debounced)');
      return;
    }

    const refreshPromise = (async () => {
      try {
        setLoading(true);
        setError(null);
        if (import.meta.env.DEV) logger.debug('refreshWorkspaces called');
        await queryClient.invalidateQueries({ queryKey: queryKeys.config });
        if (import.meta.env.DEV) logger.debug('Workspaces refresh requested via config query');
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        setError(message);
        logger.error('Failed to refresh workspaces:', err);
        throw err;
      } finally {
        setLoading(false);
        lastRefreshTime.current = Date.now();
        pendingRefresh.current = null;
      }
    })();

    pendingRefresh.current = refreshPromise;
    return refreshPromise;
  }, [queryClient, setError, setLoading]);

  return {
    workspaces,
    loading,
    error,
    refreshWorkspaces,
  };
};
