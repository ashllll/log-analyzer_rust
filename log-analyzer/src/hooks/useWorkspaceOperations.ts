import { useCallback, useState, useTransition } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { useApp, useWorkspaceState, useTaskState, Workspace } from '../contexts/AppContext';

// 日志工具
const logger = {
  debug: (message: string, ...args: any[]) => {
    if (import.meta.env.DEV) {
      console.log(`[DEBUG] ${message}`, ...args);
    }
  },
  info: (message: string, ...args: any[]) => {
    console.log(`[INFO] ${message}`, ...args);
  },
  error: (message: string, ...args: any[]) => {
    console.error(`[ERROR] ${message}`, ...args);
  }
};

/**
 * 工作区操作Hook
 * 封装所有工作区相关的后端操作
 * 统一错误处理和加载状态管理
 */
export const useWorkspaceOperations = () => {
  const { addToast, setActiveWorkspace, state: appState } = useApp();
  const { state: workspaceState, dispatch: workspaceDispatch } = useWorkspaceState();
  const { dispatch: taskDispatch } = useTaskState();
  
  const [operationLoading, setOperationLoading] = useState(false);
  const [isPending, startTransition] = useTransition();

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
  }, [addToast]);

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
  }, [addToast]);

  /**
   * 导入路径（文件或文件夹）
   */
  const importPath = useCallback(async (pathStr: string) => {
    logger.debug('importPath called with:', pathStr);
    setOperationLoading(true);
    
    try {
      const fileName = pathStr.split(/[/\\]/).pop() || "New";
      const newWs: Workspace = { 
        id: Date.now().toString(), 
        name: fileName, 
        path: pathStr, 
        status: 'PROCESSING', 
        size: '-', 
        files: 0 
      };
      
      logger.debug('Creating workspace:', newWs);
      workspaceDispatch({ type: 'ADD_WORKSPACE', payload: newWs });
      setActiveWorkspace(newWs.id);
      
      logger.debug('Invoking import_folder with:', { path: pathStr, workspaceId: newWs.id });
      const taskId = await invoke<string>("import_folder", { 
        path: pathStr, 
        workspaceId: newWs.id
      });
      
      logger.debug('import_folder returned taskId:', taskId);
      
      // 任务由后端事件自动创建，不需要手动添加
      
      addToast('info', '导入已开始');
    } catch (e) {
      logger.error('importPath error:', e);
      addToast('error', `导入失败: ${e}`);
      
      // 删除刚创建的工作区
      workspaceDispatch({ 
        type: 'DELETE_WORKSPACE', 
        payload: Date.now().toString() 
      });
    } finally {
      setOperationLoading(false);
    }
  }, [addToast, setActiveWorkspace, workspaceDispatch, taskDispatch]);

  /**
   * 刷新工作区
   */
  const refreshWorkspace = useCallback(async (workspace: Workspace) => {
    logger.debug('refreshWorkspace called for workspace:', workspace.id);
    setOperationLoading(true);
    
    try {
      const taskId = await invoke<string>("refresh_workspace", { 
        workspaceId: workspace.id,
        path: workspace.path
      });
      
      logger.debug('refresh_workspace returned taskId:', taskId);
      
      // 不再手动设置 PROCESSING 状态，让后端事件处理
      // 工作区状态由后端 task-update 事件自动更新
      
      // 任务由后端事件自动创建，不需要手动添加
      
      addToast('info', '刷新工作区中...');
    } catch (e) {
      logger.error('refreshWorkspace error:', e);
      addToast('error', `刷新失败: ${e}`);
    } finally {
      setOperationLoading(false);
    }
  }, [addToast, workspaceDispatch, taskDispatch]);

  /**
   * 删除工作区
   */
  const deleteWorkspace = useCallback(async (id: string) => {
    try {
      workspaceDispatch({ type: 'DELETE_WORKSPACE', payload: id });
      addToast('info', '工作区已删除');
    } catch (e) {
      logger.error('deleteWorkspace error:', e);
      addToast('error', `删除失败: ${e}`);
    }
  }, [addToast, workspaceDispatch]);

  /**
   * 切换工作区
   * 优化：如果已经是当前工作区，跳过加载
   * 使用 startTransition 降低优先级，避免UI卡顿
   */
  const switchWorkspace = useCallback(async (id: string) => {
    // 如果已经是当前工作区，不重复加载
    if (appState.activeWorkspaceId === id) {
      logger.debug('Already active workspace, skipping reload:', id);
      return;
    }
    
    logger.debug('switchWorkspace called for id:', id);
    
    const workspace = workspaceState.workspaces.find(w => w.id === id);
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
        await invoke('load_workspace', { workspaceId: id });
        // 加载成功后不显示 toast，避免频繁提示
      } catch (e) {
        logger.error('switchWorkspace error:', e);
        addToast('error', `加载索引失败: ${e}`);
      }
    }
  }, [addToast, setActiveWorkspace, workspaceState.workspaces, appState.activeWorkspaceId]);

  /**
   * 切换工作区监听状态
   */
  const toggleWatch = useCallback(async (workspace: Workspace) => {
    setOperationLoading(true);
    
    try {
      if (workspace.watching) {
        await invoke('stop_watch', { workspaceId: workspace.id });
        workspaceDispatch({
          type: 'UPDATE_WORKSPACE',
          payload: { id: workspace.id, updates: { watching: false } }
        });
        addToast('info', '停止监听');
      } else {
        await invoke('start_watch', { 
          workspaceId: workspace.id, 
          path: workspace.path,
          autoSearch: false 
        });
        workspaceDispatch({
          type: 'UPDATE_WORKSPACE',
          payload: { id: workspace.id, updates: { watching: true } }
        });
        addToast('info', '开始监听');
      }
    } catch (e) {
      logger.error('toggleWatch error:', e);
      addToast('error', `监听操作失败: ${e}`);
    } finally {
      setOperationLoading(false);
    }
  }, [addToast, workspaceDispatch]);

  return {
    workspaces: workspaceState.workspaces,
    loading: operationLoading || workspaceState.loading || isPending,
    error: workspaceState.error,
    importFolder,
    importFile,
    refreshWorkspace,
    deleteWorkspace,
    switchWorkspace,
    toggleWatch
  };
};
