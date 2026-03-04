/**
 * useWorkspaceMutations Hook 单元测试
 *
 * 测试工作区 mutations Hook 的乐观更新和回滚功能
 */

import React from 'react';
import { renderHook, act, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { useWorkspaceMutations } from '../useWorkspaceMutations';
import { useWorkspaceStore, Workspace } from '../../stores/workspaceStore';
import { useAppStore } from '../../stores/appStore';
import { api } from '../../services/api';

// Mock API
jest.mock('../../services/api', () => ({
  api: {
    importFolder: jest.fn(),
    refreshWorkspace: jest.fn(),
    deleteWorkspace: jest.fn(),
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

// Mock react-hot-toast
jest.mock('react-hot-toast', () => ({
  __esModule: true,
  default: {
    success: jest.fn(),
    error: jest.fn(),
    toast: jest.fn(),
  },
}));

const mockToast = require('react-hot-toast').default;

// Mock errors service
jest.mock('../../services/errors', () => ({
  getFullErrorMessage: (err: unknown) => `Error: ${err}`,
}));

// 创建 QueryClient wrapper
const createWrapper = () => {
  const queryClient = new QueryClient({
    defaultOptions: {
      mutations: {
        retry: false,
      },
    },
  });

  return function QueryClientWrapper({ children }: { children: React.ReactNode }) {
    return (
      <QueryClientProvider client={queryClient}>
        {children}
      </QueryClientProvider>
    );
  };
};

describe('useWorkspaceMutations Hook', () => {
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
    jest.clearAllMocks();
  });

  describe('初始状态', () => {
    it('应该返回所有 mutations 方法', () => {
      const wrapper = createWrapper();
      const { result } = renderHook(() => useWorkspaceMutations(), { wrapper });

      expect(typeof result.current.importPath).toBe('function');
      expect(typeof result.current.deleteWorkspace).toBe('function');
      expect(typeof result.current.toggleWatch).toBe('function');
    });

    it('初始状态应该不是 loading', () => {
      const wrapper = createWrapper();
      const { result } = renderHook(() => useWorkspaceMutations(), { wrapper });

      expect(result.current.isLoading).toBe(false);
    });
  });

  describe('importPath mutation', () => {
    it('应该成功导入路径（乐观更新）', async () => {
      mockApi.importFolder.mockResolvedValueOnce('task-1');

      const wrapper = createWrapper();
      const { result } = renderHook(() => useWorkspaceMutations(), { wrapper });

      act(() => {
        result.current.importPath({
          path: '/path/to/folder',
          workspaceId: 'ws-new',
        });
      });

      // 乐观更新：工作区应该立即创建
      expect(useWorkspaceStore.getState().workspaces).toHaveLength(1);
      expect(useWorkspaceStore.getState().workspaces[0].name).toBe('folder');
      expect(useWorkspaceStore.getState().workspaces[0].status).toBe('PROCESSING');
      expect(useAppStore.getState().activeWorkspaceId).toBe('ws-new');

      await waitFor(() => {
        expect(mockApi.importFolder).toHaveBeenCalledWith('/path/to/folder', 'ws-new');
      });
    });

    it('导入失败应该回滚（删除工作区并恢复状态）', async () => {
      mockApi.importFolder.mockRejectedValueOnce(new Error('Import failed'));

      const wrapper = createWrapper();
      const { result } = renderHook(() => useWorkspaceMutations(), { wrapper });

      const previousActive = 'ws-previous';
      act(() => {
        useAppStore.getState().setActiveWorkspace(previousActive);
      });

      await act(async () => {
        try {
          await result.current.importPathAsync({
            path: '/path/to/folder',
            workspaceId: 'ws-new',
          });
        } catch (e) {
          // 预期会抛出错误
        }
      });

      // 回滚：工作区应该被删除
      expect(useWorkspaceStore.getState().workspaces).toHaveLength(0);
      // 回滚：应该恢复之前的活跃工作区
      expect(useAppStore.getState().activeWorkspaceId).toBe(previousActive);
      expect(mockToast.error).toHaveBeenCalled();
    });
  });

  describe('deleteWorkspace mutation', () => {
    it('应该成功删除工作区（乐观更新）', async () => {
      mockApi.deleteWorkspace.mockResolvedValueOnce(undefined);

      const wrapper = createWrapper();
      const { result } = renderHook(() => useWorkspaceMutations(), { wrapper });

      act(() => {
        useWorkspaceStore.getState().addWorkspace(mockWorkspace);
      });
      expect(useWorkspaceStore.getState().workspaces).toHaveLength(1);

      await act(async () => {
        result.current.deleteWorkspace({
          workspaceId: mockWorkspace.id,
        });
      });

      // 乐观更新：工作区应该立即删除
      expect(useWorkspaceStore.getState().workspaces).toHaveLength(0);

      await waitFor(() => {
        expect(mockApi.deleteWorkspace).toHaveBeenCalledWith(mockWorkspace.id);
      });
    });

    it('删除当前活跃工作区应该切换到其他工作区', async () => {
      mockApi.deleteWorkspace.mockResolvedValueOnce(undefined);

      const ws2: Workspace = {
        id: 'ws-2',
        name: 'Workspace 2',
        path: '/path/2',
        status: 'READY',
        size: '50MB',
        files: 50,
      };

      const wrapper = createWrapper();
      const { result } = renderHook(() => useWorkspaceMutations(), { wrapper });

      act(() => {
        useWorkspaceStore.getState().addWorkspace(mockWorkspace);
        useWorkspaceStore.getState().addWorkspace(ws2);
        useAppStore.getState().setActiveWorkspace(mockWorkspace.id);
      });

      expect(useAppStore.getState().activeWorkspaceId).toBe(mockWorkspace.id);

      await act(async () => {
        result.current.deleteWorkspace({
          workspaceId: mockWorkspace.id,
        });
      });

      // 应该切换到 ws-2
      expect(useAppStore.getState().activeWorkspaceId).toBe('ws-2');
    });

    it('删除最后一个工作区应该清空活跃状态', async () => {
      mockApi.deleteWorkspace.mockResolvedValueOnce(undefined);

      const wrapper = createWrapper();
      const { result } = renderHook(() => useWorkspaceMutations(), { wrapper });

      act(() => {
        useWorkspaceStore.getState().addWorkspace(mockWorkspace);
        useAppStore.getState().setActiveWorkspace(mockWorkspace.id);
      });

      expect(useAppStore.getState().activeWorkspaceId).toBe(mockWorkspace.id);

      await act(async () => {
        result.current.deleteWorkspace({
          workspaceId: mockWorkspace.id,
        });
      });

      expect(useAppStore.getState().activeWorkspaceId).toBe(null);
    });
  });

  describe('toggleWatch mutation', () => {
    it('应该开始监听工作区（乐观更新）', async () => {
      mockApi.startWatch.mockResolvedValueOnce(undefined);

      const ws: Workspace = {
        ...mockWorkspace,
        watching: false,
      };

      const wrapper = createWrapper();
      const { result } = renderHook(() => useWorkspaceMutations(), { wrapper });

      act(() => {
        useWorkspaceStore.getState().addWorkspace(ws);
      });

      act(() => {
        result.current.toggleWatch({ workspace: ws });
      });

      // 乐观更新：状态应该立即切换
      expect(useWorkspaceStore.getState().workspaces[0].watching).toBe(true);

      await waitFor(() => {
        expect(mockApi.startWatch).toHaveBeenCalledWith({
          workspaceId: ws.id,
          autoSearch: false,
        });
      });
    });

    it('应该停止监听工作区（乐观更新）', async () => {
      mockApi.stopWatch.mockResolvedValueOnce(undefined);

      const ws: Workspace = {
        ...mockWorkspace,
        watching: true,
      };

      const wrapper = createWrapper();
      const { result } = renderHook(() => useWorkspaceMutations(), { wrapper });

      act(() => {
        useWorkspaceStore.getState().addWorkspace(ws);
      });

      act(() => {
        result.current.toggleWatch({ workspace: ws });
      });

      // 乐观更新：状态应该立即切换
      expect(useWorkspaceStore.getState().workspaces[0].watching).toBe(false);

      await waitFor(() => {
        expect(mockApi.stopWatch).toHaveBeenCalledWith(ws.id);
      });
    });

    it('监听失败应该回滚（恢复状态）', async () => {
      mockApi.startWatch.mockRejectedValueOnce(new Error('Watch failed'));

      const toast = require('react-hot-toast').default;
      const ws: Workspace = {
        ...mockWorkspace,
        watching: false,
      };

      const wrapper = createWrapper();
      const { result } = renderHook(() => useWorkspaceMutations(), { wrapper });

      act(() => {
        useWorkspaceStore.getState().addWorkspace(ws);
      });

      await act(async () => {
        try {
          await result.current.toggleWatchAsync({ workspace: ws });
        } catch (e) {
          // 预期会抛出错误
        }
      });

      // 回滚：状态应该恢复
      expect(useWorkspaceStore.getState().workspaces[0].watching).toBe(false);
      expect(toast.error).toHaveBeenCalled();
    });
  });

  describe('loading 状态', () => {
    it('import 完成后应该不处于 loading 状态', async () => {
      mockApi.importFolder.mockResolvedValueOnce('task-1');

      const wrapper = createWrapper();
      const { result } = renderHook(() => useWorkspaceMutations(), { wrapper });

      await act(async () => {
        result.current.importPath({
          path: '/path',
          workspaceId: 'ws-1',
        });
      });

      // 完成后 loading 应该是 false
      await waitFor(() => {
        expect(result.current.isLoading).toBe(false);
      });
    });

    it('delete 完成后应该不处于 loading 状态', async () => {
      mockApi.deleteWorkspace.mockResolvedValueOnce(undefined);

      const wrapper = createWrapper();
      const { result } = renderHook(() => useWorkspaceMutations(), { wrapper });

      act(() => {
        useWorkspaceStore.getState().addWorkspace(mockWorkspace);
      });

      await act(async () => {
        result.current.deleteWorkspace({
          workspaceId: mockWorkspace.id,
        });
      });

      // 完成后 loading 应该是 false
      await waitFor(() => {
        expect(result.current.isLoading).toBe(false);
      });
    });
  });
});
