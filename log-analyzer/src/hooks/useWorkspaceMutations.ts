import { useMutation, useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { useAppStore } from '../stores/appStore';
import { useWorkspaceStore, Workspace } from '../stores/workspaceStore';
import { logger } from '../utils/logger';

/**
 * 工作区 Mutations Hook
 * 使用 @tanstack/react-query 管理服务器状态
 * 提供乐观更新和自动回滚功能
 */

interface ImportPathParams {
  path: string;
  workspaceId: string;
}

interface RefreshWorkspaceParams {
  workspaceId: string;
  path: string;
}

interface DeleteWorkspaceParams {
  workspaceId: string;
}

interface ToggleWatchParams {
  workspace: Workspace;
}

export const useWorkspaceMutations = () => {
  const queryClient = useQueryClient();
  const addToast = useAppStore((state) => state.addToast);
  const addWorkspace = useWorkspaceStore((state) => state.addWorkspace);
  const updateWorkspace = useWorkspaceStore((state) => state.updateWorkspace);
  const deleteWorkspace = useWorkspaceStore((state) => state.deleteWorkspace);
  const setActiveWorkspace = useAppStore((state) => state.setActiveWorkspace);
  const workspaces = useWorkspaceStore((state) => state.workspaces);
  const activeWorkspaceId = useAppStore((state) => state.activeWorkspaceId);

  /**
   * 导入路径 Mutation
   * 支持乐观更新：立即创建工作区，失败时回滚
   */
  const importPathMutation = useMutation({
    mutationFn: async ({ path, workspaceId }: ImportPathParams) => {
      logger.debug('Invoking import_folder with:', { path, workspaceId });
      const taskId = await invoke<string>('import_folder', {
        path,
        workspaceId,
      });
      return { taskId, workspaceId };
    },
    onMutate: async ({ path, workspaceId }) => {
      // 乐观更新：立即创建工作区
      const fileName = path.split(/[/\\]/).pop() || 'New';
      const newWs: Workspace = {
        id: workspaceId,
        name: fileName,
        path,
        status: 'PROCESSING',
        size: '-',
        files: 0,
      };

      logger.debug('Creating workspace (optimistic):', newWs);
      addWorkspace(newWs);
      setActiveWorkspace(workspaceId);

      // 返回上下文用于回滚
      return { previousActive: activeWorkspaceId, workspaceId };
    },
    onSuccess: () => {
      addToast('info', '导入已开始');
      // 使工作区查询失效，触发重新获取
      queryClient.invalidateQueries({ queryKey: ['workspaces'] });
    },
    onError: (error, _variables, context) => {
      logger.error('importPath error:', error);
      addToast('error', `导入失败: ${error}`);

      // 回滚：删除刚创建的工作区
      if (context?.workspaceId) {
        deleteWorkspace(context.workspaceId);
      }

      // 恢复之前的激活工作区
      if (context?.previousActive) {
        setActiveWorkspace(context.previousActive);
      }
    },
  });

  /**
   * 刷新工作区 Mutation
   */
  const refreshWorkspaceMutation = useMutation({
    mutationFn: async ({ workspaceId, path }: RefreshWorkspaceParams) => {
      logger.debug('Invoking refresh_workspace with:', { workspaceId, path });
      const taskId = await invoke<string>('refresh_workspace', {
        workspaceId,
        path,
      });
      return { taskId, workspaceId };
    },
    onSuccess: () => {
      addToast('info', '刷新工作区中...');
      queryClient.invalidateQueries({ queryKey: ['workspaces'] });
    },
    onError: (error) => {
      logger.error('refreshWorkspace error:', error);
      addToast('error', `刷新失败: ${error}`);
    },
  });

  /**
   * 删除工作区 Mutation
   * 支持乐观更新：立即删除工作区，失败时回滚
   */
  const deleteWorkspaceMutation = useMutation({
    mutationFn: async ({ workspaceId }: DeleteWorkspaceParams) => {
      logger.debug('Invoking delete_workspace with:', workspaceId);
      await invoke('delete_workspace', { workspaceId });
      return workspaceId;
    },
    onMutate: async ({ workspaceId }) => {
      // 取消正在进行的查询
      await queryClient.cancelQueries({ queryKey: ['workspaces'] });

      // 保存当前状态用于回滚
      const previousWorkspaces = workspaces;
      const previousActive = activeWorkspaceId;

      // 乐观更新：立即删除工作区
      deleteWorkspace(workspaceId);

      // 如果删除的是当前活跃工作区，切换到其他工作区
      if (activeWorkspaceId === workspaceId) {
        const remainingWorkspaces = workspaces.filter((w) => w.id !== workspaceId);
        if (remainingWorkspaces.length > 0) {
          setActiveWorkspace(remainingWorkspaces[0].id);
        } else {
          setActiveWorkspace(null);
        }
      }

      return { previousWorkspaces, previousActive };
    },
    onSuccess: () => {
      addToast('success', '工作区已删除');
      queryClient.invalidateQueries({ queryKey: ['workspaces'] });
    },
    onError: (error, _variables, context) => {
      logger.error('deleteWorkspace error:', error);
      addToast('error', `删除失败: ${error}`);

      // 回滚：恢复工作区列表
      if (context?.previousWorkspaces) {
        context.previousWorkspaces.forEach((ws) => {
          if (!workspaces.find((w) => w.id === ws.id)) {
            addWorkspace(ws);
          }
        });
      }

      // 恢复之前的激活工作区
      if (context?.previousActive) {
        setActiveWorkspace(context.previousActive);
      }
    },
  });

  /**
   * 切换监听状态 Mutation
   * 支持乐观更新：立即更新监听状态，失败时回滚
   */
  const toggleWatchMutation = useMutation({
    mutationFn: async ({ workspace }: ToggleWatchParams) => {
      if (workspace.watching) {
        await invoke('stop_watch', { workspaceId: workspace.id });
        return { workspaceId: workspace.id, watching: false };
      } else {
        await invoke('start_watch', {
          workspaceId: workspace.id,
          path: workspace.path,
          autoSearch: false,
        });
        return { workspaceId: workspace.id, watching: true };
      }
    },
    onMutate: async ({ workspace }) => {
      // 保存当前状态用于回滚
      const previousWatching = workspace.watching;

      // 乐观更新：立即切换监听状态
      updateWorkspace(workspace.id, { watching: !workspace.watching });

      return { workspaceId: workspace.id, previousWatching };
    },
    onSuccess: (data) => {
      addToast('info', data.watching ? '开始监听' : '停止监听');
      queryClient.invalidateQueries({ queryKey: ['workspaces'] });
    },
    onError: (error, _variables, context) => {
      logger.error('toggleWatch error:', error);
      addToast('error', `监听操作失败: ${error}`);

      // 回滚：恢复之前的监听状态
      if (context) {
        updateWorkspace(context.workspaceId, { watching: context.previousWatching });
      }
    },
  });

  return {
    importPath: importPathMutation.mutate,
    importPathAsync: importPathMutation.mutateAsync,
    refreshWorkspace: refreshWorkspaceMutation.mutate,
    refreshWorkspaceAsync: refreshWorkspaceMutation.mutateAsync,
    deleteWorkspace: deleteWorkspaceMutation.mutate,
    deleteWorkspaceAsync: deleteWorkspaceMutation.mutateAsync,
    toggleWatch: toggleWatchMutation.mutate,
    toggleWatchAsync: toggleWatchMutation.mutateAsync,
    isLoading:
      importPathMutation.isPending ||
      refreshWorkspaceMutation.isPending ||
      deleteWorkspaceMutation.isPending ||
      toggleWatchMutation.isPending,
  };
};
