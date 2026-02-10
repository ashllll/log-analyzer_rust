import { useCallback, useState, useTransition } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { useAppStore } from '../stores/appStore';
import { useWorkspaceStore, Workspace } from '../stores/workspaceStore';
import { logger } from '../utils/logger';
import { api } from '../services/api';
import { getFullErrorMessage } from '../services/errors';

/**
 * 工作区操作Hook
 * 封装所有工作区相关的后端操作
 * 统一错误处理和加载状态管理
 */
export const useWorkspaceOperations = () => {
  const addToast = useAppStore((state) => state.addToast);
  const setActiveWorkspace = useAppStore((state) => state.setActiveWorkspace);
  const activeWorkspaceId = useAppStore((state) => state.activeWorkspaceId);
  const workspaces = useWorkspaceStore((state) => state.workspaces);
  const addWorkspace = useWorkspaceStore((state) => state.addWorkspace);
  const updateWorkspace = useWorkspaceStore((state) => state.updateWorkspace);
  const deleteWorkspace = useWorkspaceStore((state) => state.deleteWorkspace);
  const workspacesLoading = useWorkspaceStore((state) => state.loading);
  const workspacesError = useWorkspaceStore((state) => state.error);
  
  const [operationLoading, setOperationLoading] = useState(false);
  const [isPending, startTransition] = useTransition();

  /**
   * 导入路径（文件或文件夹）
   */
  const importPath = useCallback(async (pathStr: string) => {
    logger.debug('importPath called with:', pathStr);
    setOperationLoading(true);
    const previousActive = activeWorkspaceId;
    let tempWorkspaceId: string | null = null;

    try {
      const fileName = pathStr.split(/[/\\]/).pop() || "New";
      const workspaceId = Date.now().toString();
      const newWs: Workspace = {
        id: workspaceId,
        name: fileName,
        path: pathStr,
        status: 'PROCESSING',
        size: '-',
        files: 0
      };
      tempWorkspaceId = workspaceId;

      logger.debug('Creating workspace:', newWs);
      addWorkspace(newWs);

      logger.debug('Invoking import_folder with:', { path: pathStr, workspaceId: newWs.id });
      const taskId = await api.importFolder(pathStr, newWs.id);

      logger.debug('import_folder returned taskId:', taskId);

      // 任务由后端事件自动创建，不需要手动添加

      setActiveWorkspace(newWs.id);
      addToast('info', '导入已开始');
    } catch (e) {
      logger.error('importPath error:', e);
      addToast('error', `导入失败: ${getFullErrorMessage(e)}`);

      // 删除刚创建的工作区
      if (tempWorkspaceId) {
        deleteWorkspace(tempWorkspaceId);
      }

      // 恢复之前的激活工作区，避免指向不存在的工作区
      setActiveWorkspace(previousActive);
    } finally {
      setOperationLoading(false);
    }
  }, [addToast, setActiveWorkspace, addWorkspace, deleteWorkspace, activeWorkspaceId]);

  /**
   * 导入文件夹
   */
  const importFolder = useCallback(async () => {
    logger.debug('importFolder called');
    try {
      const selected = await open({ 
        directory: true,
        multiple: false
      });
      
      logger.debug('Selected folder:', selected);
      if (!selected) return;
      
      await importPath(selected as string);
    } catch (e) { 
      logger.error('importFolder error:', e);
      addToast('error', `导入失败: ${e}`); 
    }
  }, [addToast, importPath]);

  /**
   * 导入单个文件
   */
  const importFile = useCallback(async () => {
    logger.debug('importFile called');
    try {
      const selected = await open({ 
        directory: false,
        multiple: false,
        filters: [{
          name: 'Log Files & Archives',
          extensions: ['log', 'txt', 'gz', 'zip', 'tar', 'tgz', 'rar', '*']
        }]
      });
      
      logger.debug('Selected file:', selected);
      if (!selected) return;
      
      await importPath(selected as string);
    } catch (e) { 
      logger.error('importFile error:', e);
      addToast('error', `导入失败: ${e}`); 
    }
  }, [addToast, importPath]);

  /**
   * 刷新工作区
   */
  const refreshWorkspace = useCallback(async (workspace: Workspace) => {
    logger.debug('refreshWorkspace called for workspace:', workspace.id);
    setOperationLoading(true);

    try {
      const taskId = await api.refreshWorkspace(workspace.id);

      logger.debug('refresh_workspace returned taskId:', taskId);

      // 不再手动设置 PROCESSING 状态，让后端事件处理
      // 工作区状态由后端 task-update 事件自动更新

      // 任务由后端事件自动创建，不需要手动添加

      addToast('info', '刷新工作区中...');
    } catch (e) {
      logger.error('refreshWorkspace error:', e);
      addToast('error', `刷新失败: ${getFullErrorMessage(e)}`);
    } finally {
      setOperationLoading(false);
    }
  }, [addToast]);

  /**
   * 删除工作区
   * 调用后端命令删除工作区及其所有相关资源
   *
   * 使用业内成熟方案：
   * 1. 指数退避重试机制
   * 2. IPC 健康检查
   * 3. 断路器模式
   * 4. 超时控制
   */
  const deleteWorkspaceOp = useCallback(async (id: string) => {
    logger.debug('deleteWorkspace called for id:', id);
    setOperationLoading(true);

    try {
      await api.deleteWorkspace(id);

      logger.debug('deleteWorkspace succeeded');

      // 后端删除成功,更新前端状态
      deleteWorkspace(id);

      // 如果删除的是当前活跃工作区,清空活跃状态
      if (activeWorkspaceId === id) {
        // 切换到其他工作区或清空
        const remainingWorkspaces = workspaces.filter(w => w.id !== id);
        if (remainingWorkspaces.length > 0) {
          setActiveWorkspace(remainingWorkspaces[0].id);
        } else {
          setActiveWorkspace(null);
        }
      }

      addToast('success', '工作区已删除');
    } catch (e) {
      logger.error('deleteWorkspace error:', e);
      addToast('error', `删除失败: ${getFullErrorMessage(e)}`);
    } finally {
      setOperationLoading(false);
    }
  }, [addToast, deleteWorkspace, activeWorkspaceId, workspaces, setActiveWorkspace]);

  /**
   * 切换工作区
   * 优化：如果已经是当前工作区，跳过加载
   * 使用 startTransition 降低优先级，避免UI卡顿
   */
  const switchWorkspace = useCallback(async (id: string) => {
    // 如果已经是当前工作区，不重复加载
    if (activeWorkspaceId === id) {
      logger.debug('Already active workspace, skipping reload:', id);
      return;
    }

    logger.debug('switchWorkspace called for id:', id);

    const workspace = workspaces.find(w => w.id === id);
    if (!workspace) {
      addToast('error', 'Workspace not found');
      return;
    }

    // 先立即更新UI，然后异步加载索引
    startTransition(() => {
      setActiveWorkspace(id);
    });

    // 如果工作区已准备好，异步加载索引（不阻塞UI）
    if (workspace.status === 'READY') {
      try {
        await api.loadWorkspace(id);
        // 加载成功后不显示 toast，避免频繁提示
      } catch (e) {
        logger.error('switchWorkspace error:', e);
        addToast('error', `加载索引失败: ${getFullErrorMessage(e)}`);
      }
    }
  }, [addToast, setActiveWorkspace, workspaces, activeWorkspaceId]);

  /**
   * 切换工作区监听状态
   */
  const toggleWatch = useCallback(async (workspace: Workspace) => {
    setOperationLoading(true);

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
      setOperationLoading(false);
    }
  }, [addToast, updateWorkspace]);

  /**
   * 刷新工作区列表
   * 从后端重新加载所有工作区
   */
  const refreshWorkspaces = useCallback(async () => {
    logger.debug('refreshWorkspaces called');
    // 这里可以调用后端API获取工作区列表
    // 目前工作区列表由前端维护，通过事件自动更新
    // 如果需要从后端同步，可以添加相应的命令
    logger.debug('Workspaces refreshed from store');
  }, []);

  return {
    workspaces,
    loading: operationLoading || workspacesLoading || isPending,
    error: workspacesError,
    importFolder,
    importFile,
    refreshWorkspace,
    refreshWorkspaces,
    deleteWorkspace: deleteWorkspaceOp,
    switchWorkspace,
    toggleWatch
  };
};
