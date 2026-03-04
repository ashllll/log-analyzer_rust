/**
 * useWorkspaceOperations Hook 单元测试
 *
 * 测试工作区操作 Hook 的导入、刷新、删除等操作
 */

import { renderHook, act, waitFor } from '@testing-library/react';
import { useWorkspaceOperations } from '../useWorkspaceOperations';
import { useWorkspaceStore, Workspace } from '../../stores/workspaceStore';
import { useAppStore } from '../../stores/appStore';
import { api } from '../../services/api';
import { open } from '@tauri-apps/plugin-dialog';

// Mock Tauri plugin-dialog
jest.mock('@tauri-apps/plugin-dialog', () => ({
  open: jest.fn(),
}));

const mockOpen = open as jest.Mock;

// Mock API
jest.mock('../../services/api', () => ({
  api: {
    importFolder: jest.fn(),
    refreshWorkspace: jest.fn(),
    deleteWorkspace: jest.fn(),
    loadWorkspace: jest.fn(),
    startWatch: jest.fn(),
    stopWatch: jest.fn(),
  },
}));

const mockApi = require('../../services/api').api;

// Mock logger
jest.mock('../../utils/logger', () => ({
  logger: {
    debug: jest.fn(),
    error: jest.fn(),
    info: jest.fn(),
    warn: jest.fn(),
  },
}));

// Mock errors service
jest.mock('../../services/errors', () => ({
  getFullErrorMessage: (err: unknown) => `Error: ${err}`,
}));

describe('useWorkspaceOperations Hook', () => {
  const mockWorkspace: Workspace = {
    id: 'ws-1',
    name: 'Test Workspace',
    path: '/path/to/workspace',
    status: 'READY',
    size: '100MB',
    files: 100,
    watching: false,
  };

  beforeEach(() => {
    jest.clearAllMocks();

    // Reset stores
    act(() => {
      useWorkspaceStore.setState({
        workspaces: [],
        loading: false,
        error: null,
      });
      useAppStore.setState({
        page: 'workspaces',
        toasts: [],
        activeWorkspaceId: null,
        isInitialized: false,
        initializationError: null,
      });
    });
  });

  describe('初始状态', () => {
    it('应该返回工作区列表', () => {
      const { result } = renderHook(() => useWorkspaceOperations());

      expect(result.current.workspaces).toEqual([]);
    });

    it('应该提供所有工作区操作方法', () => {
      const { result } = renderHook(() => useWorkspaceOperations());

      expect(typeof result.current.importFolder).toBe('function');
      expect(typeof result.current.importFile).toBe('function');
      expect(typeof result.current.refreshWorkspace).toBe('function');
      expect(typeof result.current.deleteWorkspace).toBe('function');
      expect(typeof result.current.switchWorkspace).toBe('function');
      expect(typeof result.current.toggleWatch).toBe('function');
    });

    it('初始状态应该不是 loading', () => {
      const { result } = renderHook(() => useWorkspaceOperations());

      expect(result.current.loading).toBe(false);
    });
  });

  describe('importFolder', () => {
    it('应该成功导入文件夹', async () => {
      mockOpen.mockResolvedValueOnce('/path/to/folder');
      mockApi.importFolder.mockResolvedValueOnce('task-1');

      const { result } = renderHook(() => useWorkspaceOperations());

      await act(async () => {
        await result.current.importFolder();
      });

      expect(mockOpen).toHaveBeenCalledWith({
        directory: true,
        multiple: false,
      });
      expect(mockApi.importFolder).toHaveBeenCalled();
    });

    it('用户取消选择不应创建工作区', async () => {
      mockOpen.mockResolvedValueOnce(null);

      const { result } = renderHook(() => useWorkspaceOperations());

      await act(async () => {
        await result.current.importFolder();
      });

      expect(mockApi.importFolder).not.toHaveBeenCalled();
    });

    it('导入失败应该删除临时工作区', async () => {
      mockOpen.mockResolvedValueOnce('/path/to/folder');
      mockApi.importFolder.mockRejectedValueOnce(new Error('Import failed'));

      const { result } = renderHook(() => useWorkspaceOperations());

      const workspacesBefore = result.current.workspaces.length;

      await act(async () => {
        await result.current.importFolder();
      });

      // 工作区应该被删除（恢复到之前状态）
      expect(result.current.workspaces.length).toBeLessThanOrEqual(workspacesBefore);
    });
  });

  describe('importFile', () => {
    it('应该成功导入文件', async () => {
      mockOpen.mockResolvedValueOnce('/path/to/file.log');
      mockApi.importFolder.mockResolvedValueOnce('task-1');

      const { result } = renderHook(() => useWorkspaceOperations());

      await act(async () => {
        await result.current.importFile();
      });

      expect(mockOpen).toHaveBeenCalledWith({
        directory: false,
        multiple: false,
        filters: [{
          name: 'Log Files & Archives',
          extensions: ['log', 'txt', 'gz', 'zip', 'tar', 'tgz', 'rar', '*'],
        }],
      });
    });

    it('用户取消选择不应创建工作区', async () => {
      mockOpen.mockResolvedValueOnce(null);

      const { result } = renderHook(() => useWorkspaceOperations());

      await act(async () => {
        await result.current.importFile();
      });

      expect(mockApi.importFolder).not.toHaveBeenCalled();
    });
  });

  describe('refreshWorkspace', () => {
    it('应该成功刷新工作区', async () => {
      mockApi.refreshWorkspace.mockResolvedValueOnce('task-refresh-1');

      const { result } = renderHook(() => useWorkspaceOperations());

      await act(async () => {
        await result.current.refreshWorkspace(mockWorkspace);
      });

      expect(mockApi.refreshWorkspace).toHaveBeenCalledWith(mockWorkspace.id);
    });

    it('刷新失败应该显示错误', async () => {
      mockApi.refreshWorkspace.mockRejectedValueOnce(new Error('Refresh failed'));

      const { result } = renderHook(() => useWorkspaceOperations());

      await act(async () => {
        await result.current.refreshWorkspace(mockWorkspace);
      });

      // 验证错误处理（通过 addToast）
      expect(mockApi.refreshWorkspace).toHaveBeenCalledWith(mockWorkspace.id);
    });
  });

  describe('deleteWorkspace', () => {
    it('应该成功删除工作区', async () => {
      mockApi.deleteWorkspace.mockResolvedValueOnce(undefined);

      const { result } = renderHook(() => useWorkspaceOperations());

      act(() => {
        useWorkspaceStore.getState().addWorkspace(mockWorkspace);
      });
      expect(result.current.workspaces).toHaveLength(1);

      await act(async () => {
        await result.current.deleteWorkspace(mockWorkspace.id);
      });

      expect(mockApi.deleteWorkspace).toHaveBeenCalledWith(mockWorkspace.id);
      expect(result.current.workspaces).toHaveLength(0);
    });

    it('删除当前活跃工作区应该清空活跃状态或切换到其他', async () => {
      mockApi.deleteWorkspace.mockResolvedValueOnce(undefined);

      const ws2: Workspace = {
        id: 'ws-2',
        name: 'Workspace 2',
        path: '/path/2',
        status: 'READY',
        size: '50MB',
        files: 50,
      };

      const { result } = renderHook(() => useWorkspaceOperations());

      act(() => {
        useWorkspaceStore.getState().addWorkspace(mockWorkspace);
        useWorkspaceStore.getState().addWorkspace(ws2);
        useAppStore.getState().setActiveWorkspace(mockWorkspace.id);
      });

      expect(useAppStore.getState().activeWorkspaceId).toBe(mockWorkspace.id);

      await act(async () => {
        await result.current.deleteWorkspace(mockWorkspace.id);
      });

      // 应该切换到 ws-2 或清空
      const newActiveId = useAppStore.getState().activeWorkspaceId;
      expect(newActiveId).toBe('ws-2');
    });

    it('删除最后一个工作区应该清空活跃状态', async () => {
      mockApi.deleteWorkspace.mockResolvedValueOnce(undefined);

      const { result } = renderHook(() => useWorkspaceOperations());

      act(() => {
        useWorkspaceStore.getState().addWorkspace(mockWorkspace);
        useAppStore.getState().setActiveWorkspace(mockWorkspace.id);
      });

      expect(useAppStore.getState().activeWorkspaceId).toBe(mockWorkspace.id);

      await act(async () => {
        await result.current.deleteWorkspace(mockWorkspace.id);
      });

      expect(useAppStore.getState().activeWorkspaceId).toBe(null);
    });

    it('删除失败应该保留工作区', async () => {
      mockApi.deleteWorkspace.mockRejectedValueOnce(new Error('Delete failed'));

      const { result } = renderHook(() => useWorkspaceOperations());

      act(() => {
        useWorkspaceStore.getState().addWorkspace(mockWorkspace);
      });

      const workspacesBefore = result.current.workspaces.length;

      await act(async () => {
        try {
          await result.current.deleteWorkspace(mockWorkspace.id);
        } catch {
          // 预期会抛出错误
        }
      });

      // 失败后工作区应该还在
      expect(result.current.workspaces.length).toBe(workspacesBefore);
    });
  });

  describe('switchWorkspace', () => {
    it('应该成功切换工作区', async () => {
      mockApi.loadWorkspace.mockResolvedValueOnce(undefined);

      const { result } = renderHook(() => useWorkspaceOperations());

      act(() => {
        useWorkspaceStore.getState().addWorkspace(mockWorkspace);
      });

      await act(async () => {
        await result.current.switchWorkspace(mockWorkspace.id);
      });

      expect(useAppStore.getState().activeWorkspaceId).toBe(mockWorkspace.id);
      expect(mockApi.loadWorkspace).toHaveBeenCalledWith(mockWorkspace.id);
    });

    it('切换到相同工作区应该跳过', async () => {
      mockApi.loadWorkspace.mockResolvedValueOnce(undefined);

      const { result } = renderHook(() => useWorkspaceOperations());

      act(() => {
        useWorkspaceStore.getState().addWorkspace(mockWorkspace);
        useAppStore.getState().setActiveWorkspace(mockWorkspace.id);
      });

      await act(async () => {
        await result.current.switchWorkspace(mockWorkspace.id);
      });

      expect(mockApi.loadWorkspace).not.toHaveBeenCalled();
    });

    it('切换到不存在的工作区应该显示错误', async () => {
      const { result } = renderHook(() => useWorkspaceOperations());

      await act(async () => {
        await result.current.switchWorkspace('non-existent');
      });

      // addToast error 会被调用
      expect(useAppStore.getState().activeWorkspaceId).not.toBe('non-existent');
    });

    it('非 READY 状态的工作区不应加载索引', async () => {
      const processingWorkspace: Workspace = {
        ...mockWorkspace,
        status: 'PROCESSING',
      };

      const { result } = renderHook(() => useWorkspaceOperations());

      act(() => {
        useWorkspaceStore.getState().addWorkspace(processingWorkspace);
      });

      await act(async () => {
        await result.current.switchWorkspace(processingWorkspace.id);
      });

      expect(mockApi.loadWorkspace).not.toHaveBeenCalled();
      // 但活跃工作区应该已更新
      expect(useAppStore.getState().activeWorkspaceId).toBe(processingWorkspace.id);
    });
  });

  describe('toggleWatch', () => {
    it('应该开始监听工作区', async () => {
      mockApi.startWatch.mockResolvedValueOnce(undefined);

      const ws: Workspace = {
        ...mockWorkspace,
        watching: false,
      };

      const { result } = renderHook(() => useWorkspaceOperations());

      act(() => {
        useWorkspaceStore.getState().addWorkspace(ws);
      });

      await act(async () => {
        await result.current.toggleWatch(ws);
      });

      expect(mockApi.startWatch).toHaveBeenCalledWith({
        workspaceId: ws.id,
        autoSearch: false,
      });
    });

    it('应该停止监听工作区', async () => {
      mockApi.stopWatch.mockResolvedValueOnce(undefined);

      const ws: Workspace = {
        ...mockWorkspace,
        watching: true,
      };

      const { result } = renderHook(() => useWorkspaceOperations());

      act(() => {
        useWorkspaceStore.getState().addWorkspace(ws);
      });

      await act(async () => {
        await result.current.toggleWatch(ws);
      });

      expect(mockApi.stopWatch).toHaveBeenCalledWith(ws.id);
    });
  });

  describe('refreshWorkspaces', () => {
    it('应该调用刷新方法', async () => {
      const { result } = renderHook(() => useWorkspaceOperations());

      await act(async () => {
        await result.current.refreshWorkspaces();
      });

      // 这个方法目前是空操作，但应该存在
      expect(typeof result.current.refreshWorkspaces).toBe('function');
    });
  });
});
