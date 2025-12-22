import { renderHook, act } from '@testing-library/react';
import { useTaskStore, type Task } from '../taskStore';

/**
 * Task Store 测试
 * 
 * 测试任务状态管理的核心功能，特别是任务去重逻辑
 */

describe('TaskStore', () => {
  beforeEach(() => {
    // 重置 store 状态
    const { result } = renderHook(() => useTaskStore());
    act(() => {
      result.current.setTasks([]);
      result.current.setError(null);
      result.current.setLoading(false);
    });
  });

  describe('Basic Operations', () => {
    it('should add task', () => {
      const { result } = renderHook(() => useTaskStore());

      const task: Task = {
        id: 'task-1',
        type: 'Import',
        target: '/test/file.zip',
        progress: 0,
        message: 'Starting',
        status: 'RUNNING',
      };

      act(() => {
        result.current.addTask(task);
      });

      expect(result.current.tasks).toHaveLength(1);
      expect(result.current.tasks[0]).toEqual(task);
    });

    it('should update task', () => {
      const { result } = renderHook(() => useTaskStore());

      const task: Task = {
        id: 'task-1',
        type: 'Import',
        target: '/test/file.zip',
        progress: 0,
        message: 'Starting',
        status: 'RUNNING',
      };

      act(() => {
        result.current.addTask(task);
      });

      act(() => {
        result.current.updateTask('task-1', {
          progress: 50,
          message: 'Processing',
        });
      });

      expect(result.current.tasks[0].progress).toBe(50);
      expect(result.current.tasks[0].message).toBe('Processing');
    });

    it('should delete task', () => {
      const { result } = renderHook(() => useTaskStore());

      const task: Task = {
        id: 'task-1',
        type: 'Import',
        target: '/test/file.zip',
        progress: 0,
        message: 'Starting',
        status: 'RUNNING',
      };

      act(() => {
        result.current.addTask(task);
      });

      expect(result.current.tasks).toHaveLength(1);

      act(() => {
        result.current.deleteTask('task-1');
      });

      expect(result.current.tasks).toHaveLength(0);
    });

    it('should set tasks', () => {
      const { result } = renderHook(() => useTaskStore());

      const tasks: Task[] = [
        {
          id: 'task-1',
          type: 'Import',
          target: '/test/file1.zip',
          progress: 0,
          message: 'Starting',
          status: 'RUNNING',
        },
        {
          id: 'task-2',
          type: 'Export',
          target: '/test/output.csv',
          progress: 100,
          message: 'Completed',
          status: 'COMPLETED',
        },
      ];

      act(() => {
        result.current.setTasks(tasks);
      });

      expect(result.current.tasks).toHaveLength(2);
      expect(result.current.tasks).toEqual(tasks);
    });
  });

  describe('Task Deduplication', () => {
    it('should add task if not exists', () => {
      const { result } = renderHook(() => useTaskStore());

      const task: Task = {
        id: 'task-1',
        type: 'Import',
        target: '/test/file.zip',
        progress: 0,
        message: 'Starting',
        status: 'RUNNING',
      };

      act(() => {
        result.current.addTaskIfNotExists(task);
      });

      expect(result.current.tasks).toHaveLength(1);
    });

    it('should not add duplicate task', () => {
      const { result } = renderHook(() => useTaskStore());

      const task: Task = {
        id: 'task-1',
        type: 'Import',
        target: '/test/file.zip',
        progress: 0,
        message: 'Starting',
        status: 'RUNNING',
      };

      act(() => {
        result.current.addTaskIfNotExists(task);
        result.current.addTaskIfNotExists(task);
        result.current.addTaskIfNotExists(task);
      });

      expect(result.current.tasks).toHaveLength(1);
    });

    it('should not update existing task when adding duplicate', () => {
      const { result } = renderHook(() => useTaskStore());

      const task: Task = {
        id: 'task-1',
        type: 'Import',
        target: '/test/file.zip',
        progress: 0,
        message: 'Starting',
        status: 'RUNNING',
      };

      act(() => {
        result.current.addTaskIfNotExists(task);
      });

      act(() => {
        result.current.addTaskIfNotExists({
          ...task,
          progress: 50,
          message: 'Processing',
        });
      });

      // 验证任务没有被更新
      expect(result.current.tasks[0].progress).toBe(0);
      expect(result.current.tasks[0].message).toBe('Starting');
    });
  });

  describe('Loading and Error States', () => {
    it('should set loading state', () => {
      const { result } = renderHook(() => useTaskStore());

      expect(result.current.loading).toBe(false);

      act(() => {
        result.current.setLoading(true);
      });

      expect(result.current.loading).toBe(true);

      act(() => {
        result.current.setLoading(false);
      });

      expect(result.current.loading).toBe(false);
    });

    it('should set error state', () => {
      const { result } = renderHook(() => useTaskStore());

      expect(result.current.error).toBeNull();

      act(() => {
        result.current.setError('Test error message');
      });

      expect(result.current.error).toBe('Test error message');

      act(() => {
        result.current.setError(null);
      });

      expect(result.current.error).toBeNull();
    });
  });

  describe('Edge Cases', () => {
    it('should handle updating non-existent task', () => {
      const { result } = renderHook(() => useTaskStore());

      act(() => {
        result.current.updateTask('non-existent', { progress: 50 });
      });

      expect(result.current.tasks).toHaveLength(0);
    });

    it('should handle deleting non-existent task', () => {
      const { result } = renderHook(() => useTaskStore());

      act(() => {
        result.current.deleteTask('non-existent');
      });

      expect(result.current.tasks).toHaveLength(0);
    });

    it('should handle multiple updates to same task', () => {
      const { result } = renderHook(() => useTaskStore());

      const task: Task = {
        id: 'task-1',
        type: 'Import',
        target: '/test/file.zip',
        progress: 0,
        message: 'Starting',
        status: 'RUNNING',
      };

      act(() => {
        result.current.addTask(task);
      });

      act(() => {
        result.current.updateTask('task-1', { progress: 25 });
        result.current.updateTask('task-1', { progress: 50 });
        result.current.updateTask('task-1', { progress: 75 });
        result.current.updateTask('task-1', { progress: 100, status: 'COMPLETED' });
      });

      expect(result.current.tasks[0].progress).toBe(100);
      expect(result.current.tasks[0].status).toBe('COMPLETED');
    });
  });
});
