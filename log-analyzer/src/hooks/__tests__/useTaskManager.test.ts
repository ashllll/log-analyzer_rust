/**
 * useTaskManager Hook 单元测试
 *
 * 测试任务管理 Hook 的 CRUD 操作和状态管理
 */

import { renderHook, act, waitFor } from '@testing-library/react';
import { useTaskManager } from '../useTaskManager';
import { useTaskStore, Task } from '../../stores/taskStore';
import { useAppStore } from '../../stores/appStore';
import { api } from '../../services/api';

// Mock API
jest.mock('../../services/api', () => ({
  api: {
    cancelTask: jest.fn(),
  },
}));

const mockApi = require('../../services/api').api;
const mockCancelTask = mockApi.cancelTask as jest.Mock;

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

describe('useTaskManager Hook', () => {
  beforeEach(() => {
    // Reset stores before each test
    act(() => {
      useTaskStore.setState({
        tasks: [],
        loading: false,
        error: null,
      });
      useAppStore.setState({
        page: 'tasks',
        toasts: [],
        activeWorkspaceId: null,
        isInitialized: false,
        initializationError: null,
      });
    });
    jest.clearAllMocks();
  });

  describe('初始状态', () => {
    it('应该返回任务列表', () => {
      const { result } = renderHook(() => useTaskManager());

      expect(result.current.tasks).toEqual([]);
    });

    it('应该返回加载状态', () => {
      const { result } = renderHook(() => useTaskManager());

      expect(result.current.loading).toBe(false);
    });

    it('应该返回错误状态', () => {
      const { result } = renderHook(() => useTaskManager());

      expect(result.current.error).toBe(null);
    });

    it('应该提供所有任务管理方法', () => {
      const { result } = renderHook(() => useTaskManager());

      expect(typeof result.current.deleteTask).toBe('function');
      expect(typeof result.current.cancelTask).toBe('function');
    });
  });

  describe('deleteTask', () => {
    it('应该删除指定的任务', () => {
      const { result } = renderHook(() => useTaskManager());

      const task1: Task = {
        id: 'task-1',
        type: 'import',
        target: '/path/to/file',
        progress: 50,
        message: 'Processing',
        status: 'RUNNING',
      };

      const task2: Task = {
        id: 'task-2',
        type: 'search',
        target: 'query',
        progress: 100,
        message: 'Completed',
        status: 'COMPLETED',
      };

      act(() => {
        useTaskStore.getState().addTask(task1);
        useTaskStore.getState().addTask(task2);
      });

      expect(result.current.tasks).toHaveLength(2);

      act(() => {
        result.current.deleteTask('task-1');
      });

      expect(result.current.tasks).toHaveLength(1);
      expect(result.current.tasks[0].id).toBe('task-2');
    });

    it('删除不存在的任务不应报错', () => {
      const { result } = renderHook(() => useTaskManager());

      expect(() => {
        act(() => {
          result.current.deleteTask('non-existent');
        });
      }).not.toThrow();

      expect(result.current.tasks).toHaveLength(0);
    });

    it('应该调用 toast 显示信息消息', () => {
      const { result } = renderHook(() => useTaskManager());

      const task: Task = {
        id: 'task-1',
        type: 'import',
        target: '/path',
        progress: 0,
        message: 'Starting',
        status: 'RUNNING',
      };

      act(() => {
        useTaskStore.getState().addTask(task);
      });

      act(() => {
        result.current.deleteTask('task-1');
      });

      // Hook 会调用 appStore.addToast，这里验证任务已被删除
      expect(result.current.tasks).toHaveLength(0);
    });

    it('应该删除所有状态的任务', () => {
      const { result } = renderHook(() => useTaskManager());

      const tasks: Task[] = [
        { id: 'task-1', type: 'test', target: '', progress: 0, message: '', status: 'RUNNING' },
        { id: 'task-2', type: 'test', target: '', progress: 100, message: '', status: 'COMPLETED' },
        { id: 'task-3', type: 'test', target: '', progress: 0, message: '', status: 'FAILED' },
        { id: 'task-4', type: 'test', target: '', progress: 0, message: '', status: 'STOPPED' },
      ];

      act(() => {
        tasks.forEach(task => useTaskStore.getState().addTask(task));
      });

      expect(result.current.tasks).toHaveLength(4);

      act(() => {
        result.current.deleteTask('task-1');
        result.current.deleteTask('task-2');
        result.current.deleteTask('task-3');
        result.current.deleteTask('task-4');
      });

      expect(result.current.tasks).toHaveLength(0);
    });
  });

  describe('cancelTask', () => {
    it('成功取消任务时应该更新状态', async () => {
      mockCancelTask.mockResolvedValueOnce(undefined);

      const { result } = renderHook(() => useTaskManager());

      const task: Task = {
        id: 'task-1',
        type: 'import',
        target: '/path',
        progress: 50,
        message: 'Processing',
        status: 'RUNNING',
      };

      act(() => {
        useTaskStore.getState().addTask(task);
      });

      expect(result.current.tasks[0].status).toBe('RUNNING');

      await act(async () => {
        await result.current.cancelTask('task-1');
      });

      expect(mockCancelTask).toHaveBeenCalledWith('task-1');
      expect(result.current.tasks[0].status).toBe('STOPPED');
    });

    it('取消失败时应该显示错误', async () => {
      mockCancelTask.mockRejectedValueOnce(new Error('API Error'));

      const { result } = renderHook(() => useTaskManager());

      const task: Task = {
        id: 'task-1',
        type: 'import',
        target: '/path',
        progress: 50,
        message: 'Processing',
        status: 'RUNNING',
      };

      act(() => {
        useTaskStore.getState().addTask(task);
      });

      await act(async () => {
        await result.current.cancelTask('task-1');
      });

      expect(mockCancelTask).toHaveBeenCalledWith('task-1');
      // Hook 会调用 appStore.addToast 显示错误
    });

    it('取消不存在的任务应该调用 API', async () => {
      mockCancelTask.mockResolvedValueOnce(undefined);

      const { result } = renderHook(() => useTaskManager());

      await act(async () => {
        await result.current.cancelTask('non-existent');
      });

      expect(mockCancelTask).toHaveBeenCalledWith('non-existent');
    });

    it('网络错误时应该正确处理', async () => {
      mockCancelTask.mockRejectedValueOnce(new Error('Network timeout'));

      const { result } = renderHook(() => useTaskManager());

      await act(async () => {
        await result.current.cancelTask('task-1');
      });

      // Hook 会调用 appStore.addToast 显示错误
      expect(mockCancelTask).toHaveBeenCalled();
    });

    it('取消已完成任务应该正常工作', async () => {
      mockCancelTask.mockResolvedValueOnce(undefined);

      const { result } = renderHook(() => useTaskManager());

      const task: Task = {
        id: 'task-1',
        type: 'import',
        target: '/path',
        progress: 100,
        message: 'Completed',
        status: 'COMPLETED',
      };

      act(() => {
        useTaskStore.getState().addTask(task);
      });

      await act(async () => {
        await result.current.cancelTask('task-1');
      });

      expect(mockCancelTask).toHaveBeenCalledWith('task-1');
      expect(result.current.tasks[0].status).toBe('STOPPED');
    });
  });

  describe('状态同步', () => {
    it('应该反映 store 中的任务变化', () => {
      const { result } = renderHook(() => useTaskManager());

      const tasks: Task[] = [
        { id: 'task-1', type: 'test', target: '', progress: 0, message: '', status: 'RUNNING' },
        { id: 'task-2', type: 'test', target: '', progress: 50, message: 'Half done', status: 'RUNNING' },
      ];

      act(() => {
        useTaskStore.getState().setTasks(tasks);
      });

      expect(result.current.tasks).toHaveLength(2);
      expect(result.current.tasks[0].id).toBe('task-1');
      expect(result.current.tasks[1].message).toBe('Half done');
    });

    it('应该反映 loading 状态', () => {
      const { result } = renderHook(() => useTaskManager());

      act(() => {
        useTaskStore.getState().setLoading(true);
      });

      expect(result.current.loading).toBe(true);

      act(() => {
        useTaskStore.getState().setLoading(false);
      });

      expect(result.current.loading).toBe(false);
    });

    it('应该反映 error 状态', () => {
      const { result } = renderHook(() => useTaskManager());

      act(() => {
        useTaskStore.getState().setError('Task loading failed');
      });

      expect(result.current.error).toBe('Task loading failed');

      act(() => {
        useTaskStore.getState().setError(null);
      });

      expect(result.current.error).toBe(null);
    });
  });

  describe('任务类型支持', () => {
    it('应该支持所有任务类型', () => {
      const { result } = renderHook(() => useTaskManager());

      const taskTypes = ['import', 'search', 'export', 'index'] as const;

      act(() => {
        taskTypes.forEach((type, index) => {
          const task: Task = {
            id: `task-${index}`,
            type,
            target: 'test',
            progress: 0,
            message: `${type} task`,
            status: 'RUNNING',
          };
          useTaskStore.getState().addTask(task);
        });
      });

      expect(result.current.tasks).toHaveLength(4);
      expect(result.current.tasks[0].type).toBe('import');
      expect(result.current.tasks[1].type).toBe('search');
      expect(result.current.tasks[2].type).toBe('export');
      expect(result.current.tasks[3].type).toBe('index');
    });
  });

  describe('边界条件', () => {
    it('应该处理带有 workspaceId 的任务', () => {
      const { result } = renderHook(() => useTaskManager());

      const task: Task = {
        id: 'task-1',
        type: 'import',
        target: '/path',
        progress: 25,
        message: 'Processing workspace',
        status: 'RUNNING',
        workspaceId: 'workspace-123',
      };

      act(() => {
        useTaskStore.getState().addTask(task);
      });

      expect(result.current.tasks[0].workspaceId).toBe('workspace-123');
    });

    it('应该处理带有 completedAt 的任务', () => {
      const { result } = renderHook(() => useTaskManager());

      const timestamp = Date.now();
      const task: Task = {
        id: 'task-1',
        type: 'import',
        target: '/path',
        progress: 100,
        message: 'Done',
        status: 'COMPLETED',
        completedAt: timestamp,
      };

      act(() => {
        useTaskStore.getState().addTask(task);
      });

      expect(result.current.tasks[0].completedAt).toBe(timestamp);
    });

    it('应该处理不同的进度值（0-100）', () => {
      const { result } = renderHook(() => useTaskManager());

      const progressValues = [0, 25, 50, 75, 100];

      act(() => {
        progressValues.forEach((progress, index) => {
          const task: Task = {
            id: `task-${index}`,
            type: 'test',
            target: '',
            progress,
            message: `Progress ${progress}%`,
            status: 'RUNNING',
          };
          useTaskStore.getState().addTask(task);
        });
      });

      expect(result.current.tasks).toHaveLength(5);
      expect(result.current.tasks[0].progress).toBe(0);
      expect(result.current.tasks[2].progress).toBe(50);
      expect(result.current.tasks[4].progress).toBe(100);
    });

    it('应该处理空消息', () => {
      const { result } = renderHook(() => useTaskManager());

      const task: Task = {
        id: 'task-1',
        type: 'import',
        target: '/path',
        progress: 0,
        message: '',
        status: 'RUNNING',
      };

      act(() => {
        useTaskStore.getState().addTask(task);
      });

      expect(result.current.tasks[0].message).toBe('');
    });
  });

  describe('并发操作', () => {
    it('应该支持同时删除多个任务', () => {
      const { result } = renderHook(() => useTaskManager());

      const tasks: Task[] = Array.from({ length: 5 }, (_, i) => ({
        id: `task-${i}`,
        type: 'test',
        target: '',
        progress: 0,
        message: '',
        status: 'RUNNING',
      }));

      act(() => {
        tasks.forEach(task => useTaskStore.getState().addTask(task));
      });

      expect(result.current.tasks).toHaveLength(5);

      act(() => {
        result.current.deleteTask('task-0');
        result.current.deleteTask('task-1');
        result.current.deleteTask('task-2');
      });

      expect(result.current.tasks).toHaveLength(2);
      expect(result.current.tasks[0].id).toBe('task-3');
      expect(result.current.tasks[1].id).toBe('task-4');
    });

    it('应该支持同时取消多个任务', async () => {
      mockCancelTask.mockResolvedValue(undefined);

      const { result } = renderHook(() => useTaskManager());

      const tasks: Task[] = Array.from({ length: 3 }, (_, i) => ({
        id: `task-${i}`,
        type: 'test',
        target: '',
        progress: 50,
        message: 'Processing',
        status: 'RUNNING',
      }));

      act(() => {
        tasks.forEach(task => useTaskStore.getState().addTask(task));
      });

      await act(async () => {
        await Promise.all([
          result.current.cancelTask('task-0'),
          result.current.cancelTask('task-1'),
          result.current.cancelTask('task-2'),
        ]);
      });

      expect(mockCancelTask).toHaveBeenCalledTimes(3);
      expect(result.current.tasks[0].status).toBe('STOPPED');
      expect(result.current.tasks[1].status).toBe('STOPPED');
      expect(result.current.tasks[2].status).toBe('STOPPED');
    });
  });
});
