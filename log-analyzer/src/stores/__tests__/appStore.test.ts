import { renderHook, act } from '@testing-library/react';
import { useTaskStore, type Task } from '../taskStore';
import { useWorkspaceStore, type Workspace } from '../workspaceStore';

// Mock Tauri APIs
jest.mock('@tauri-apps/api/core');
jest.mock('@tauri-apps/api/event');
jest.mock('../../utils/logger');

/**
 * 状态管理集成测试
 * 
 * 测试 zustand store 的核心功能：
 * - Property 12: Task Deduplication（任务去重）
 * - Property 13: Workspace Status Consistency（工作区状态一致性）
 */

describe('AppStore - State Management Integration Tests', () => {
  beforeEach(() => {
    // 重置 store 状态
    const taskStore = renderHook(() => useTaskStore());
    const workspaceStore = renderHook(() => useWorkspaceStore());
    act(() => {
      taskStore.result.current.setTasks([]);
      workspaceStore.result.current.setWorkspaces([]);
    });
  });

  describe('Property 12: Task Deduplication', () => {
    /**
     * **Feature: bug-fixes, Property 12: Task Deduplication**
     * 
     * *For any* duplicate task event from backend, the frontend should prevent creating duplicate tasks
     * **Validates: Requirements 4.1**
     */
    it('should prevent duplicate task creation when same task ID is added multiple times', () => {
      const { result } = renderHook(() => useTaskStore());

      const task: Task = {
        id: 'task-123',
        type: 'Import',
        target: '/path/to/folder',
        progress: 0,
        message: 'Starting import',
        status: 'RUNNING',
        workspaceId: 'ws-1',
      };

      // 第一次添加任务
      act(() => {
        result.current.addTaskIfNotExists(task);
      });

      expect(result.current.tasks).toHaveLength(1);
      expect(result.current.tasks[0].id).toBe('task-123');

      // 尝试添加相同 ID 的任务（模拟重复事件）
      act(() => {
        result.current.addTaskIfNotExists({
          ...task,
          progress: 50,
          message: 'Processing',
        });
      });

      // 验证任务没有重复
      expect(result.current.tasks).toHaveLength(1);
      expect(result.current.tasks[0].id).toBe('task-123');
      // 验证任务状态没有被更新（因为是去重，不是更新）
      expect(result.current.tasks[0].progress).toBe(0);
      expect(result.current.tasks[0].message).toBe('Starting import');
    });

    it('should allow adding tasks with different IDs', () => {
      const { result } = renderHook(() => useTaskStore());

      const task1: Task = {
        id: 'task-1',
        type: 'Import',
        target: '/path/1',
        progress: 0,
        message: 'Task 1',
        status: 'RUNNING',
      };

      const task2: Task = {
        id: 'task-2',
        type: 'Import',
        target: '/path/2',
        progress: 0,
        message: 'Task 2',
        status: 'RUNNING',
      };

      act(() => {
        result.current.addTaskIfNotExists(task1);
        result.current.addTaskIfNotExists(task2);
      });

      expect(result.current.tasks).toHaveLength(2);
      expect(result.current.tasks.map((t) => t.id)).toEqual(['task-1', 'task-2']);
    });

    it('should handle rapid duplicate task events correctly', () => {
      const { result } = renderHook(() => useTaskStore());

      const task: Task = {
        id: 'task-rapid',
        type: 'Import',
        target: '/path/to/folder',
        progress: 0,
        message: 'Starting',
        status: 'RUNNING',
      };

      // 模拟快速连续的重复事件
      act(() => {
        for (let i = 0; i < 10; i++) {
          result.current.addTaskIfNotExists({
            ...task,
            progress: i * 10,
            message: `Progress ${i * 10}%`,
          });
        }
      });

      // 验证只创建了一个任务
      expect(result.current.tasks).toHaveLength(1);
      expect(result.current.tasks[0].id).toBe('task-rapid');
    });
  });

  describe('Property 13: Workspace Status Consistency', () => {
    /**
     * **Feature: bug-fixes, Property 13: Workspace Status Consistency**
     * 
     * *For any* workspace operation completion, the workspace status should be updated consistently
     * **Validates: Requirements 4.2**
     */
    it('should update workspace status consistently when operation completes', () => {
      const { result } = renderHook(() => useWorkspaceStore());

      const workspace: Workspace = {
        id: 'ws-1',
        name: 'Test Workspace',
        path: '/path/to/workspace',
        status: 'PROCESSING',
        size: '100MB',
        files: 50,
      };

      // 添加工作区
      act(() => {
        result.current.addWorkspace(workspace);
      });

      expect(result.current.workspaces).toHaveLength(1);
      expect(result.current.workspaces[0].status).toBe('PROCESSING');

      // 模拟操作完成，更新状态为 READY
      act(() => {
        result.current.updateWorkspace('ws-1', { status: 'READY' });
      });

      // 验证状态已更新
      expect(result.current.workspaces[0].status).toBe('READY');
    });

    it('should maintain workspace status consistency across multiple updates', () => {
      const { result } = renderHook(() => useWorkspaceStore());

      const workspace: Workspace = {
        id: 'ws-multi',
        name: 'Multi Update Workspace',
        path: '/path/to/workspace',
        status: 'OFFLINE',
        size: '0',
        files: 0,
      };

      act(() => {
        result.current.addWorkspace(workspace);
      });

      // 模拟一系列状态变化
      const statusSequence: Array<'OFFLINE' | 'PROCESSING' | 'READY' | 'SCANNING'> = [
        'PROCESSING',
        'SCANNING',
        'READY',
      ];

      statusSequence.forEach((status) => {
        act(() => {
          result.current.updateWorkspace('ws-multi', { status });
        });

        // 验证每次更新后状态都是一致的
        expect(result.current.workspaces[0].status).toBe(status);
      });
    });

    it('should handle concurrent workspace status updates correctly', () => {
      const { result } = renderHook(() => useWorkspaceStore());

      // 添加多个工作区
      const workspaces: Workspace[] = [
        {
          id: 'ws-1',
          name: 'Workspace 1',
          path: '/path/1',
          status: 'PROCESSING',
          size: '100MB',
          files: 50,
        },
        {
          id: 'ws-2',
          name: 'Workspace 2',
          path: '/path/2',
          status: 'PROCESSING',
          size: '200MB',
          files: 100,
        },
        {
          id: 'ws-3',
          name: 'Workspace 3',
          path: '/path/3',
          status: 'PROCESSING',
          size: '300MB',
          files: 150,
        },
      ];

      act(() => {
        workspaces.forEach((ws) => result.current.addWorkspace(ws));
      });

      // 模拟并发更新多个工作区的状态
      act(() => {
        result.current.updateWorkspace('ws-1', { status: 'READY' });
        result.current.updateWorkspace('ws-2', { status: 'OFFLINE' });
        result.current.updateWorkspace('ws-3', { status: 'READY' });
      });

      // 验证所有工作区的状态都正确更新
      expect(result.current.workspaces[0].status).toBe('READY');
      expect(result.current.workspaces[1].status).toBe('OFFLINE');
      expect(result.current.workspaces[2].status).toBe('READY');
    });

    it('should not affect other workspace properties when updating status', () => {
      const { result } = renderHook(() => useWorkspaceStore());

      const workspace: Workspace = {
        id: 'ws-props',
        name: 'Props Test Workspace',
        path: '/path/to/workspace',
        status: 'PROCESSING',
        size: '500MB',
        files: 250,
        watching: true,
      };

      act(() => {
        result.current.addWorkspace(workspace);
      });

      // 更新状态
      act(() => {
        result.current.updateWorkspace('ws-props', { status: 'READY' });
      });

      // 验证其他属性没有被改变
      const updatedWorkspace = result.current.workspaces[0];
      expect(updatedWorkspace.status).toBe('READY');
      expect(updatedWorkspace.name).toBe('Props Test Workspace');
      expect(updatedWorkspace.path).toBe('/path/to/workspace');
      expect(updatedWorkspace.size).toBe('500MB');
      expect(updatedWorkspace.files).toBe(250);
      expect(updatedWorkspace.watching).toBe(true);
    });

    it('should handle workspace status update for non-existent workspace gracefully', () => {
      const { result } = renderHook(() => useWorkspaceStore());

      // 尝试更新不存在的工作区
      act(() => {
        result.current.updateWorkspace('non-existent', { status: 'READY' });
      });

      // 验证没有抛出错误，且工作区列表为空
      expect(result.current.workspaces).toHaveLength(0);
    });
  });

  describe('Integration: Task and Workspace Status Coordination', () => {
    it('should coordinate task completion with workspace status update', () => {
      const taskStore = renderHook(() => useTaskStore());
      const workspaceStore = renderHook(() => useWorkspaceStore());

      const workspace: Workspace = {
        id: 'ws-coord',
        name: 'Coordination Test',
        path: '/path/to/workspace',
        status: 'PROCESSING',
        size: '0',
        files: 0,
      };

      const task: Task = {
        id: 'task-coord',
        type: 'Import',
        target: '/path/to/workspace',
        progress: 0,
        message: 'Starting',
        status: 'RUNNING',
        workspaceId: 'ws-coord',
      };

      // 添加工作区和任务
      act(() => {
        workspaceStore.result.current.addWorkspace(workspace);
        taskStore.result.current.addTaskIfNotExists(task);
      });

      expect(workspaceStore.result.current.workspaces[0].status).toBe('PROCESSING');
      expect(taskStore.result.current.tasks[0].status).toBe('RUNNING');

      // 模拟任务完成，同时更新工作区状态
      act(() => {
        taskStore.result.current.updateTask('task-coord', {
          status: 'COMPLETED',
          progress: 100,
          message: 'Done',
        });
        workspaceStore.result.current.updateWorkspace('ws-coord', { status: 'READY' });
      });

      // 验证两者状态都已更新
      expect(taskStore.result.current.tasks[0].status).toBe('COMPLETED');
      expect(taskStore.result.current.tasks[0].progress).toBe(100);
      expect(workspaceStore.result.current.workspaces[0].status).toBe('READY');
    });
  });
});
