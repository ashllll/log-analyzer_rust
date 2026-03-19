import { useCallback, useTransition } from 'react';
import { useAppStore } from '../stores/appStore';
import { useWorkspaceStore, type Workspace } from '../stores/workspaceStore';
import { logger } from '../utils/logger';
import { api } from '../services/api';
import { getFullErrorMessage } from '../services/errors';

export interface UseWorkspaceSelectionReturn {
  switchWorkspace: (id: string) => Promise<void>;
  activeWorkspaceId: string | null;
  activeWorkspace: Workspace | null;
  workspaces: Workspace[];
  isPending: boolean;
}

/**
 * 工作区选择Hook
 * 封装工作区切换相关操作
 */
export const useWorkspaceSelection = (): UseWorkspaceSelectionReturn => {
  const addToast = useAppStore((state) => state.addToast);
  const setActiveWorkspace = useAppStore((state) => state.setActiveWorkspace);
  const activeWorkspaceId = useAppStore((state) => state.activeWorkspaceId);
  const workspaces = useWorkspaceStore((state) => state.workspaces);

  const [isPending, startTransition] = useTransition();

  const switchWorkspace = useCallback(async (id: string) => {
    if (activeWorkspaceId === id) {
      if (import.meta.env.DEV) logger.debug('Already active workspace, skipping reload:', id);
      return;
    }

    if (import.meta.env.DEV) logger.debug('switchWorkspace called for id:', id);

    const workspace = workspaces.find(w => w.id === id);
    if (!workspace) {
      addToast('error', 'Workspace not found');
      return;
    }

    startTransition(() => {
      setActiveWorkspace(id);
    });

    if (workspace.status === 'READY') {
      try {
        await api.loadWorkspace(id);
      } catch (e) {
        logger.error('switchWorkspace error:', e);
        addToast('error', `加载索引失败: ${getFullErrorMessage(e)}`);
      }
    }
  }, [addToast, setActiveWorkspace, workspaces, activeWorkspaceId]);

  const activeWorkspace = workspaces.find(w => w.id === activeWorkspaceId) || null;

  return {
    switchWorkspace,
    activeWorkspaceId,
    activeWorkspace,
    workspaces,
    isPending,
  };
};
