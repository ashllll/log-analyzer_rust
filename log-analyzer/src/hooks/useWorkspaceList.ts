import { useCallback, useRef } from 'react';
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

/** 刷新工作区列表的最小间隔（毫秒） */
const REFRESH_DEBOUNCE_MS = 500;

/**
 * 工作区列表Hook
 * 提供工作区列表访问和刷新功能
 *
 * 优化：添加防抖机制，避免频繁事件触发导致全量配置重复加载
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

  const lastRefreshTime = useRef<number>(0);
  const pendingRefresh = useRef<Promise<void> | null>(null);

  const refreshWorkspaces = useCallback(async () => {
    // 防抖：如果距离上次刷新不足 REFRESH_DEBOUNCE_MS，跳过本次请求
    const now = Date.now();
    if (now - lastRefreshTime.current < REFRESH_DEBOUNCE_MS) {
      if (import.meta.env.DEV) logger.debug('refreshWorkspaces skipped (debounced)');
      return;
    }

    // 如果已有进行中的刷新请求，等待其完成
    if (pendingRefresh.current) {
      return pendingRefresh.current;
    }

    lastRefreshTime.current = now;

    const refreshPromise = (async () => {
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
        throw err;
      } finally {
        pendingRefresh.current = null;
      }
    })();

    pendingRefresh.current = refreshPromise;
    return refreshPromise;
  }, [setWorkspaces]);

  return {
    workspaces,
    loading,
    error,
    refreshWorkspaces,
  };
};
