import { useCallback } from 'react';
import { useWorkspaceStore, type Workspace } from '../stores/workspaceStore';
import { logger } from '../utils/logger';
import { api } from '../services/api';
import { useAsyncAction } from './useAsyncAction';

/**
 * 工作区文件监听Hook
 * 封装文件监听启动/停止操作
 */
export const useWorkspaceWatch = () => {
  const { execute, isLoading } = useAsyncAction();
  const updateWorkspace = useWorkspaceStore((state) => state.updateWorkspace);

  const toggleWatch = useCallback(async (workspace: Workspace) => {
    if (workspace.watching) {
      await execute(
        () => api.stopWatch(workspace.id),
        {
          successMessage: '停止监听',
          errorPrefix: '停止监听失败',
          onSuccess: () => updateWorkspace(workspace.id, { watching: false }),
          onError: (e) => { logger.error('toggleWatch error:', e); },
        },
      );
    } else {
      await execute(
        () => api.startWatch({ workspaceId: workspace.id, autoSearch: false }),
        {
          successMessage: '开始监听',
          errorPrefix: '开始监听失败',
          onSuccess: () => updateWorkspace(workspace.id, { watching: true }),
          onError: (e) => { logger.error('toggleWatch error:', e); },
        },
      );
    }
  }, [execute, updateWorkspace]);

  return {
    toggleWatch,
    isLoading,
  };
};
