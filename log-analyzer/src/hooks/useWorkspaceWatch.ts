import { useCallback, useState } from 'react';
import { useAppStore } from '../stores/appStore';
import { useWorkspaceStore, type Workspace } from '../stores/workspaceStore';
import { logger } from '../utils/logger';
import { api } from '../services/api';
import { getFullErrorMessage } from '../services/errors';

export interface UseWorkspaceWatchReturn {
  toggleWatch: (workspace: Workspace) => Promise<void>;
  isLoading: boolean;
}

/**
 * 工作区监听Hook
 * 封装工作区文件监听切换操作
 */
export const useWorkspaceWatch = (): UseWorkspaceWatchReturn => {
  const addToast = useAppStore((state) => state.addToast);
  const updateWorkspace = useWorkspaceStore((state) => state.updateWorkspace);

  const [isLoading, setIsLoading] = useState(false);

  const toggleWatch = useCallback(async (workspace: Workspace) => {
    setIsLoading(true);

    try {
      if (workspace.watching) {
        await api.stopWatch(workspace.id);
        updateWorkspace(workspace.id, { watching: false });
        addToast('info', '停止监听');
      } else {
        await api.startWatch({
          workspaceId: workspace.id,
          autoSearch: false
        });
        updateWorkspace(workspace.id, { watching: true });
        addToast('info', '开始监听');
      }
    } catch (e) {
      logger.error('toggleWatch error:', e);
      addToast('error', `监听操作失败: ${getFullErrorMessage(e)}`);
    } finally {
      setIsLoading(false);
    }
  }, [addToast, updateWorkspace]);

  return {
    toggleWatch,
    isLoading,
  };
};
