import { useCallback } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { logger } from '../utils/logger';
import { useAsyncAction } from './useAsyncAction';
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
  const { execute, isLoading: actionLoading } = useAsyncAction();
  const { mutateAsync: importWorkspace, isPending } = useImportFolderMutation();

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

    await execute(
      async () => {
        const selected = await open({ directory: true, multiple: false });
        if (import.meta.env.DEV) logger.debug('Selected folder:', selected);
        if (!selected) return;
        await importPath(selected as string);
      },
      {
        errorPrefix: '导入失败',
        onError: (e) => { logger.error('importFolder error:', e); },
      },
    );
  }, [execute, importPath]);

  const importFile = useCallback(async () => {
    if (import.meta.env.DEV) logger.debug('importFile called');

    await execute(
      async () => {
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
      },
      {
        errorPrefix: '导入失败',
        onError: (e) => { logger.error('importFile error:', e); },
      },
    );
  }, [execute, importPath]);

  return {
    importPath,
    importFolder,
    importFile,
    isLoading: actionLoading || isPending,
  };
};
