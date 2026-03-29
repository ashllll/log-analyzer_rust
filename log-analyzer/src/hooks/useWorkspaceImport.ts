import { useCallback, useState } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { useAppStore } from '../stores/appStore';
import { useWorkspaceStore, type Workspace } from '../stores/workspaceStore';
import { logger } from '../utils/logger';
import { api } from '../services/api';
import { getFullErrorMessage } from '../services/errors';
import { useToast } from './useToast';

export interface UseWorkspaceImportReturn {
  importPath: (pathStr: string) => Promise<void>;
  importFolder: () => Promise<void>;
  importFile: () => Promise<void>;
  isLoading: boolean;
}

/**
 * 工作区导入Hook
 * 封装工作区导入相关操作
 */
export const useWorkspaceImport = (): UseWorkspaceImportReturn => {
  const { showToast: addToast } = useToast();
  const setActiveWorkspace = useAppStore((state) => state.setActiveWorkspace);
  const activeWorkspaceId = useAppStore((state) => state.activeWorkspaceId);
  const addWorkspace = useWorkspaceStore((state) => state.addWorkspace);
  const deleteWorkspace = useWorkspaceStore((state) => state.deleteWorkspace);

  const [isLoading, setIsLoading] = useState(false);

  const importPath = useCallback(async (pathStr: string) => {
    if (import.meta.env.DEV) logger.debug('importPath called with:', pathStr);
    setIsLoading(true);
    const previousActive = activeWorkspaceId;
    let tempWorkspaceId: string | null = null;

    try {
      const fileName = pathStr.split(/[/\\]/).pop() || "New";
      const workspaceId = crypto.randomUUID();
      const newWs: Workspace = {
        id: workspaceId,
        name: fileName,
        path: pathStr,
        status: 'PROCESSING',
        size: '-',
        files: 0
      };
      tempWorkspaceId = workspaceId;

      if (import.meta.env.DEV) logger.debug('Creating workspace:', newWs);
      addWorkspace(newWs);

      if (import.meta.env.DEV) logger.debug('Invoking import_folder with:', { path: pathStr, workspaceId: newWs.id });
      const taskId = await api.importFolder(pathStr, newWs.id);

      if (import.meta.env.DEV) logger.debug('import_folder returned taskId:', taskId);

      setActiveWorkspace(newWs.id);
      addToast('info', '导入已开始');
    } catch (e) {
      logger.error('importPath error:', e);
      addToast('error', `导入失败: ${getFullErrorMessage(e)}`);

      if (tempWorkspaceId) {
        deleteWorkspace(tempWorkspaceId);
      }

      setActiveWorkspace(previousActive);
    } finally {
      setIsLoading(false);
    }
  }, [addToast, setActiveWorkspace, addWorkspace, deleteWorkspace, activeWorkspaceId]);

  const importFolder = useCallback(async () => {
    if (import.meta.env.DEV) logger.debug('importFolder called');
    try {
      const selected = await open({
        directory: true,
        multiple: false
      });

      if (import.meta.env.DEV) logger.debug('Selected folder:', selected);
      if (!selected) return;

      await importPath(selected as string);
    } catch (e) {
      logger.error('importFolder error:', e);
      addToast('error', `导入失败: ${getFullErrorMessage(e)}`);
    }
  }, [addToast, importPath]);

  const importFile = useCallback(async () => {
    if (import.meta.env.DEV) logger.debug('importFile called');
    try {
      const selected = await open({
        directory: false,
        multiple: false,
        filters: [{
          name: 'Log Files & Archives',
          extensions: ['log', 'txt', 'gz', 'zip', 'tar', 'tgz', 'rar', '*']
        }]
      });

      if (import.meta.env.DEV) logger.debug('Selected file:', selected);
      if (!selected) return;

      await importPath(selected as string);
    } catch (e) {
      logger.error('importFile error:', e);
      addToast('error', `导入失败: ${getFullErrorMessage(e)}`);
    }
  }, [addToast, importPath]);

  return {
    importPath,
    importFolder,
    importFile,
    isLoading,
  };
};
