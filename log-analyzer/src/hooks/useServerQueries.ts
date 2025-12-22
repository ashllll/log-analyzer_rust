import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { useAppStore } from '../stores/appStore';
import { useWorkspaceStore, type Workspace } from '../stores/workspaceStore';
import { useKeywordStore, type KeywordGroup } from '../stores/keywordStore';
import { logger } from '../utils/logger';

// ============================================================================
// Query Keys
// ============================================================================

export const queryKeys = {
  config: ['config'] as const,
  workspaces: ['workspaces'] as const,
  workspace: (id: string) => ['workspace', id] as const,
  keywordGroups: ['keywordGroups'] as const,
  tasks: ['tasks'] as const,
} as const;

// ============================================================================
// Configuration Queries
// ============================================================================

/**
 * Load application configuration from backend
 */
export const useConfigQuery = () => {
  const setWorkspaces = useWorkspaceStore((state) => state.setWorkspaces);
  const setKeywordGroups = useKeywordStore((state) => state.setKeywordGroups);
  
  return useQuery({
    queryKey: queryKeys.config,
    queryFn: async () => {
      logger.debug('[QUERY] Loading configuration');
      const config = await invoke<any>('load_config');
      
      // Update zustand store with loaded data
      if (config.workspaces) {
        setWorkspaces(config.workspaces);
      }
      if (config.keyword_groups) {
        setKeywordGroups(config.keyword_groups);
      }
      
      return config;
    },
    staleTime: 5 * 60 * 1000, // 5 minutes
    retry: 3,
    retryDelay: (attemptIndex) => Math.min(1000 * 2 ** attemptIndex, 30000),
  });
};

/**
 * Save application configuration to backend
 */
export const useConfigMutation = () => {
  const queryClient = useQueryClient();
  const addToast = useAppStore((state) => state.addToast);
  
  return useMutation({
    mutationFn: async (config: { keyword_groups: KeywordGroup[]; workspaces: Workspace[] }) => {
      logger.debug('[MUTATION] Saving configuration');
      await invoke('save_config', { config });
      return config;
    },
    onSuccess: () => {
      // Invalidate and refetch config
      queryClient.invalidateQueries({ queryKey: queryKeys.config });
      logger.debug('[MUTATION] Configuration saved successfully');
    },
    onError: (error) => {
      logger.error('[MUTATION] Failed to save configuration:', error);
      addToast('error', 'Failed to save configuration');
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
  const addToast = useAppStore((state) => state.addToast);
  
  return useMutation({
    mutationFn: async (workspaceId: string) => {
      logger.debug('[MUTATION] Loading workspace:', workspaceId);
      await invoke('load_workspace', { workspaceId });
      return workspaceId;
    },
    onSuccess: (workspaceId) => {
      logger.debug('[MUTATION] Workspace loaded successfully:', workspaceId);
    },
    onError: (error, workspaceId) => {
      logger.error('[MUTATION] Failed to load workspace:', workspaceId, error);
      addToast('error', `Failed to load workspace: ${error}`);
    },
    retry: 1,
    retryDelay: 1000,
  });
};

/**
 * Import folder/file as workspace
 */
export const useImportFolderMutation = () => {
  const queryClient = useQueryClient();
  const addToast = useAppStore((state) => state.addToast);
  const addWorkspace = useWorkspaceStore((state) => state.addWorkspace);
  const setActiveWorkspace = useAppStore((state) => state.setActiveWorkspace);
  
  return useMutation({
    mutationFn: async ({ path, workspaceId }: { path: string; workspaceId: string }) => {
      logger.debug('[MUTATION] Importing folder:', path, workspaceId);
      const taskId = await invoke<string>('import_folder', { path, workspaceId });
      return { taskId, path, workspaceId };
    },
    onMutate: async ({ path, workspaceId }) => {
      // Optimistic update: add workspace immediately
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
      
      return { newWorkspace };
    },
    onSuccess: ({ taskId }) => {
      logger.debug('[MUTATION] Import started successfully:', taskId);
      addToast('info', 'Import started');
      
      // Invalidate workspaces query to refetch
      queryClient.invalidateQueries({ queryKey: queryKeys.workspaces });
    },
    onError: (error, { workspaceId }, context) => {
      logger.error('[MUTATION] Failed to import folder:', error);
      addToast('error', `Import failed: ${error}`);
      
      // Rollback optimistic update
      if (context?.newWorkspace) {
        const deleteWorkspace = useWorkspaceStore.getState().deleteWorkspace;
        deleteWorkspace(workspaceId);
      }
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
  const addToast = useAppStore((state) => state.addToast);
  
  return useMutation({
    mutationFn: async ({ workspaceId, path }: { workspaceId: string; path: string }) => {
      logger.debug('[MUTATION] Refreshing workspace:', workspaceId);
      const taskId = await invoke<string>('refresh_workspace', { workspaceId, path });
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
      addToast('error', `Refresh failed: ${error}`);
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
  const addToast = useAppStore((state) => state.addToast);
  const deleteWorkspace = useWorkspaceStore((state) => state.deleteWorkspace);
  const setActiveWorkspace = useAppStore((state) => state.setActiveWorkspace);
  const workspaces = useWorkspaceStore((state) => state.workspaces);
  const activeWorkspaceId = useAppStore((state) => state.activeWorkspaceId);
  
  return useMutation({
    mutationFn: async (workspaceId: string) => {
      logger.debug('[MUTATION] Deleting workspace:', workspaceId);
      await invoke('delete_workspace', { workspaceId });
      return workspaceId;
    },
    onSuccess: (workspaceId) => {
      logger.debug('[MUTATION] Workspace deleted successfully:', workspaceId);
      
      // Update zustand store
      deleteWorkspace(workspaceId);
      
      // Handle active workspace switching
      if (activeWorkspaceId === workspaceId) {
        const remainingWorkspaces = workspaces.filter((w: Workspace) => w.id !== workspaceId);
        if (remainingWorkspaces.length > 0) {
          setActiveWorkspace(remainingWorkspaces[0].id);
        } else {
          setActiveWorkspace(null);
        }
      }
      
      addToast('success', 'Workspace deleted');
      
      // Invalidate queries
      queryClient.invalidateQueries({ queryKey: queryKeys.workspaces });
      queryClient.removeQueries({ queryKey: queryKeys.workspace(workspaceId) });
    },
    onError: (error) => {
      logger.error('[MUTATION] Failed to delete workspace:', error);
      addToast('error', `Delete failed: ${error}`);
    },
    retry: 1,
    retryDelay: 1000,
  });
};

/**
 * Toggle workspace watching
 */
export const useToggleWatchMutation = () => {
  const addToast = useAppStore((state) => state.addToast);
  const updateWorkspace = useWorkspaceStore((state) => state.updateWorkspace);
  
  return useMutation({
    mutationFn: async ({ workspace, enable }: { workspace: Workspace; enable: boolean }) => {
      logger.debug('[MUTATION] Toggling watch:', workspace.id, enable);
      
      if (enable) {
        await invoke('start_watch', {
          workspaceId: workspace.id,
          path: workspace.path,
          autoSearch: false
        });
      } else {
        await invoke('stop_watch', { workspaceId: workspace.id });
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
      addToast('error', `Watch operation failed: ${error}`);
      
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
  const addToast = useAppStore((state) => state.addToast);
  
  return useMutation({
    mutationFn: async (searchParams: any) => {
      logger.debug('[MUTATION] Performing search:', searchParams);
      const results = await invoke('search_logs', searchParams);
      return results;
    },
    onError: (error) => {
      logger.error('[MUTATION] Search failed:', error);
      addToast('error', `Search failed: ${error}`);
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
  const addToast = useAppStore((state) => state.addToast);
  
  return useMutation({
    mutationFn: async (exportParams: any) => {
      logger.debug('[MUTATION] Exporting results:', exportParams);
      await invoke('export_results', exportParams);
      return exportParams;
    },
    onSuccess: () => {
      addToast('success', 'Export completed');
    },
    onError: (error) => {
      logger.error('[MUTATION] Export failed:', error);
      addToast('error', `Export failed: ${error}`);
    },
    retry: 1,
    retryDelay: 1000,
  });
};