import { useCallback, useState } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { logger } from '../utils/logger';
import { getFullErrorMessage } from '../services/errors';
import { useToast } from './useToast';
import { useImportFolderMutation } from './useServerQueries';

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
  const { mutateAsync: importWorkspace, isPending } = useImportFolderMutation();

  const [isLoading, setIsLoading] = useState(false);

  const importPath = useCallback(async (pathStr: string) => {
    if (import.meta.env.DEV) logger.debug('importPath called with:', pathStr);

    try {
      const workspaceId = crypto.randomUUID();
      await importWorkspace({ path: pathStr, workspaceId });
    } catch (e) {
      logger.error('importPath error:', e);
    }
  }, [importWorkspace]);

  const importFolder = useCallback(async () => {
    if (import.meta.env.DEV) logger.debug('importFolder called');
    setIsLoading(true);
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
    } finally {
      setIsLoading(false);
    }
  }, [addToast, importPath]);

  const importFile = useCallback(async () => {
    if (import.meta.env.DEV) logger.debug('importFile called');
    setIsLoading(true);
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
    } finally {
      setIsLoading(false);
    }
  }, [addToast, importPath]);

  return {
    importPath,
    importFolder,
    importFile,
    isLoading: isLoading || isPending,
  };
};
