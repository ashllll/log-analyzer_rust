import { useEffect } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { useAppStore } from '../stores/appStore';
import { useWorkspaceStore, type Workspace } from '../stores/workspaceStore';
import { useKeywordStore, type KeywordGroup } from '../stores/keywordStore';
import { logger } from '../utils/logger';
import { api, type SearchParams, type ExportParams } from '../services/api';
import { getFullErrorMessage } from '../services/errors';
import { useToast } from './useToast';
import { configQuery, queryKeys } from '../services/api';

// ============================================================================
// Configuration Queries
// ============================================================================

/**
 * Load application configuration from backend
 */
export const useConfigQuery = () => {
  const setWorkspaces = useWorkspaceStore((state) => state.setWorkspaces);
  const setKeywordGroups = useKeywordStore((state) => state.setKeywordGroups);

  const query = useQuery({
    ...configQuery,
    queryFn: async () => {
      logger.debug('[QUERY] Loading configuration');
      return configQuery.queryFn();
    },
    retry: 3,
    retryDelay: (attemptIndex) => Math.min(1000 * 2 ** attemptIndex, 30000),
  });

  // FIX(CR-10): 将 store 副作用从 queryFn 移到 useEffect，保持 queryFn 纯净
  useEffect(() => {
    if (query.data) {
      if (query.data.workspaces) {
        setWorkspaces(query.data.workspaces);
      }
      if (query.data.keyword_groups) {
        setKeywordGroups(query.data.keyword_groups);
      }
    }
  }, [query.data, setWorkspaces, setKeywordGroups]);

  return query;
};

/**
 * Save application configuration to backend
 */
export const useConfigMutation = () => {
  const queryClient = useQueryClient();
  const { showToast: addToast } = useToast();

  return useMutation({
    mutationFn: async (config: { keyword_groups: KeywordGroup[]; workspaces: Workspace[] }) => {
      logger.debug('[MUTATION] Saving configuration');
      await api.saveWorkspaceConfig(config);
      return config;
    },
    scope: {
      id: 'config-write',
    },
    onSuccess: () => {
      // Invalidate and refetch config
      queryClient.invalidateQueries({ queryKey: queryKeys.config });
      logger.debug('[MUTATION] Configuration saved successfully');
    },
    onError: (error) => {
      logger.error('[MUTATION] Failed to save configuration:', error);
      addToast('error', `Failed to save configuration: ${getFullErrorMessage(error)}`);
    },
    retry: 1,
    retryDelay: 1000,
  });
};

// ============================================================================
// Workspace Queries
// ============================================================================

/**
 * Load workspace index
 */
export const useLoadWorkspaceMutation = () => {
  const { showToast: addToast } = useToast();

  return useMutation({
    mutationFn: async (workspaceId: string) => {
      logger.debug('[MUTATION] Loading workspace:', workspaceId);
      await api.loadWorkspace(workspaceId);
      return workspaceId;
    },
    onSuccess: (workspaceId) => {
      logger.debug('[MUTATION] Workspace loaded successfully:', workspaceId);
    },
    onError: (error, workspaceId) => {
      logger.error('[MUTATION] Failed to load workspace:', workspaceId, error);
      addToast('error', `Failed to load workspace: ${getFullErrorMessage(error)}`);
    },
    retry: 1,
    retryDelay: 1000,
  });
};

/**
 * Import folder/file as workspace
 */
export const useImportFolderMutation = () => {
  const { showToast: addToast } = useToast();
  const addWorkspace = useWorkspaceStore((state) => state.addWorkspace);
  const deleteWorkspace = useWorkspaceStore((state) => state.deleteWorkspace);
  const setActiveWorkspace = useAppStore((state) => state.setActiveWorkspace);

  return useMutation({
    mutationFn: async ({ path, workspaceId }: { path: string; workspaceId: string }) => {
      logger.debug('[MUTATION] Importing folder:', path, workspaceId);
      const taskId = await api.importFolder(path, workspaceId);
      return { taskId, path, workspaceId };
    },
    onMutate: async ({ path, workspaceId }) => {
      // Optimistic update: add workspace immediately
      const previousActiveWorkspaceId = useAppStore.getState().activeWorkspaceId;
      const fileName = path.split(/[/\\]/).pop() || "New";
      const newWorkspace: Workspace = {
        id: workspaceId,
        name: fileName,
        path,
        status: 'PROCESSING',
        size: '-',
        files: 0
      };

      addWorkspace(newWorkspace);
      setActiveWorkspace(workspaceId);

      return { newWorkspace, previousActiveWorkspaceId };
    },
    onSuccess: ({ taskId }) => {
      logger.debug('[MUTATION] Import started successfully:', taskId);
      addToast('info', '导入已开始');
    },
    onError: (error, { workspaceId }, context) => {
      logger.error('[MUTATION] Failed to import folder:', error);
      addToast('error', `导入失败: ${getFullErrorMessage(error)}`);

      // Rollback optimistic update
      if (context?.newWorkspace) {
        deleteWorkspace(workspaceId);
      }
      setActiveWorkspace(context?.previousActiveWorkspaceId ?? null);
    },
    retry: 1,
    retryDelay: 1000,
  });
};

/**
 * Refresh workspace
 */
export const useRefreshWorkspaceMutation = () => {
  const queryClient = useQueryClient();
  const { showToast: addToast } = useToast();

  return useMutation({
    mutationFn: async ({ workspaceId, path }: { workspaceId: string; path: string }) => {
      logger.debug('[MUTATION] Refreshing workspace:', workspaceId);
      const taskId = await api.refreshWorkspace(workspaceId, path);
      return { taskId, workspaceId };
    },
    onSuccess: ({ taskId, workspaceId }) => {
      logger.debug('[MUTATION] Refresh started successfully:', taskId);
      addToast('info', 'Refreshing workspace...');

      // Invalidate workspace-specific queries
      queryClient.invalidateQueries({ queryKey: queryKeys.workspace(workspaceId) });
    },
    onError: (error) => {
      logger.error('[MUTATION] Failed to refresh workspace:', error);
      addToast('error', `Refresh failed: ${getFullErrorMessage(error)}`);
    },
    retry: 1,
    retryDelay: 1000,
  });
};

/**
 * Delete workspace
 */
export const useDeleteWorkspaceMutation = () => {
  const queryClient = useQueryClient();
  const { showToast: addToast } = useToast();
  const deleteWorkspaceAndResolveActive = useWorkspaceStore(
    (state) => state.deleteWorkspaceAndResolveActive
  );
  const setActiveWorkspace = useAppStore((state) => state.setActiveWorkspace);

  return useMutation({
    mutationFn: async (workspaceId: string) => {
      logger.debug('[MUTATION] Deleting workspace:', workspaceId);
      await api.deleteWorkspace(workspaceId);
      return workspaceId;
    },
    onSuccess: (workspaceId) => {
      logger.debug('[MUTATION] Workspace deleted successfully:', workspaceId);

      // Read latest active workspace via getState() to avoid stale mutation closure
      const currentActiveId = useAppStore.getState().activeWorkspaceId;
      const nextActiveWorkspaceId = deleteWorkspaceAndResolveActive(workspaceId, currentActiveId);
      setActiveWorkspace(nextActiveWorkspaceId);

      addToast('success', 'Workspace deleted');

      // Invalidate queries
      queryClient.invalidateQueries({ queryKey: queryKeys.config });
      queryClient.removeQueries({ queryKey: queryKeys.workspace(workspaceId) });
    },
    onError: (error) => {
      logger.error('[MUTATION] Failed to delete workspace:', error);
      addToast('error', `Delete failed: ${getFullErrorMessage(error)}`);
    },
    retry: 1,
    retryDelay: 1000,
  });
};

/**
 * Toggle workspace watching
 */
export const useToggleWatchMutation = () => {
  const { showToast: addToast } = useToast();
  const updateWorkspace = useWorkspaceStore((state) => state.updateWorkspace);

  return useMutation({
    mutationFn: async ({ workspace, enable }: { workspace: Workspace; enable: boolean }) => {
      logger.debug('[MUTATION] Toggling watch:', workspace.id, enable);

      if (enable) {
        await api.startWatch({ workspaceId: workspace.id, autoSearch: false });
      } else {
        await api.stopWatch(workspace.id);
      }

      return { workspaceId: workspace.id, enable };
    },
    onMutate: async ({ workspace, enable }) => {
      // Optimistic update
      updateWorkspace(workspace.id, { watching: enable });
      return { workspaceId: workspace.id, previousWatching: workspace.watching };
    },
    onSuccess: ({ enable }) => {
      addToast('info', enable ? 'Started watching' : 'Stopped watching');
    },
    onError: (error, _variables, context) => {
      logger.error('[MUTATION] Failed to toggle watch:', error);
      addToast('error', `Watch operation failed: ${getFullErrorMessage(error)}`);

      // Rollback optimistic update
      if (context) {
        updateWorkspace(context.workspaceId, { watching: context.previousWatching });
      }
    },
    retry: 1,
    retryDelay: 1000,
  });
};

// ============================================================================
// Search Queries
// ============================================================================

/**
 * Perform search operation
 */
export const useSearchMutation = () => {
  const { showToast: addToast } = useToast();

  return useMutation({
    mutationFn: async (searchParams: SearchParams) => {
      logger.debug('[MUTATION] Performing search:', searchParams);
      const results = await api.searchLogs(searchParams);
      return results;
    },
    onError: (error) => {
      logger.error('[MUTATION] Search failed:', error);
      addToast('error', `Search failed: ${getFullErrorMessage(error)}`);
    },
    retry: 2,
    retryDelay: (attemptIndex) => Math.min(1000 * 2 ** attemptIndex, 5000),
  });
};

// ============================================================================
// Export Queries
// ============================================================================

/**
 * Export search results
 */
export const useExportMutation = () => {
  const { showToast: addToast } = useToast();

  return useMutation({
    mutationFn: async (exportParams: ExportParams) => {
      logger.debug('[MUTATION] Exporting results:', exportParams);
      await api.exportResults(exportParams);
      return exportParams;
    },
    onSuccess: () => {
      addToast('success', 'Export completed');
    },
    onError: (error) => {
      logger.error('[MUTATION] Export failed:', error);
      addToast('error', `Export failed: ${getFullErrorMessage(error)}`);
    },
    retry: 1,
    retryDelay: 1000,
  });
};
