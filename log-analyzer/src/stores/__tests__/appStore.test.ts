/**
 * Integration tests for the zustand app store
 * Tests Property 12: Task Deduplication
 * Tests Property 13: Workspace Status Consistency
 * Validates: Requirements 4.1, 4.2
 */

import { act, renderHook } from '@testing-library/react';
import { useAppStore } from '../appStore';
import type { Task, Workspace } from '../appStore';

// Mock logger to avoid console output in tests
jest.mock('../../utils/logger', () => ({
  logger: {
    debug: jest.fn(),
    error: jest.fn(),
    info: jest.fn(),
    warn: jest.fn(),
  },
}));

describe('AppStore Integration Tests', () => {
  beforeEach(() => {
    jest.useFakeTimers();
    // Reset store state before each test
    useAppStore.setState({
      page: 'workspaces',
      toasts: [],
      activeWorkspaceId: null,
      workspaces: [],
      workspacesLoading: false,
      workspacesError: null,
      keywordGroups: [],
      keywordsLoading: false,
      keywordsError: null,
      tasks: [],
      tasksLoading: false,
      tasksError: null,
    });
  });

  afterEach(() => {
    jest.runOnlyPendingTimers();
    jest.useRealTimers();
  });

  describe('Property 12: Task Deduplication', () => {
    it('should prevent duplicate task creation when adding the same task multiple times', () => {
      const { result } = renderHook(() => useAppStore());
      
      const task: Task = {
        id: 'test-task-1',
        type: 'Import',
        target: 'test-folder',
        progress: 50,
        message: 'Processing...',
        status: 'RUNNING',
        workspaceId: 'workspace-1'
      };

      act(() => {
        // Add the same task multiple times
        result.current.addTaskIfNotExists(task);
        result.current.addTaskIfNotExists(task);
        result.current.addTaskIfNotExists(task);
      });

      // Should only have one task despite multiple additions
      expect(result.current.tasks).toHaveLength(1);
      expect(result.current.tasks[0]).toEqual(task);
    });

    it('should allow different tasks with different IDs', () => {
      const { result } = renderHook(() => useAppStore());
      
      const task1: Task = {
        id: 'test-task-1',
        type: 'Import',
        target: 'test-folder-1',
        progress: 50,
        message: 'Processing...',
        status: 'RUNNING',
        workspaceId: 'workspace-1'
      };

      const task2: Task = {
        id: 'test-task-2',
        type: 'Import',
        target: 'test-folder-2',
        progress: 30,
        message: 'Starting...',
        status: 'RUNNING',
        workspaceId: 'workspace-2'
      };

      act(() => {
        result.current.addTaskIfNotExists(task1);
        result.current.addTaskIfNotExists(task2);
      });

      // Should have both tasks
      expect(result.current.tasks).toHaveLength(2);
      expect(result.current.tasks).toContainEqual(task1);
      expect(result.current.tasks).toContainEqual(task2);
    });

    it('should update existing task when task with same ID is updated', () => {
      const { result } = renderHook(() => useAppStore());
      
      const initialTask: Task = {
        id: 'test-task-1',
        type: 'Import',
        target: 'test-folder',
        progress: 50,
        message: 'Processing...',
        status: 'RUNNING',
        workspaceId: 'workspace-1'
      };

      act(() => {
        result.current.addTaskIfNotExists(initialTask);
      });

      expect(result.current.tasks).toHaveLength(1);
      expect(result.current.tasks[0].progress).toBe(50);

      act(() => {
        result.current.updateTask('test-task-1', {
          progress: 100,
          status: 'COMPLETED',
          message: 'Done!'
        });
      });

      // Should still have one task but with updated values
      expect(result.current.tasks).toHaveLength(1);
      expect(result.current.tasks[0].progress).toBe(100);
      expect(result.current.tasks[0].status).toBe('COMPLETED');
      expect(result.current.tasks[0].message).toBe('Done!');
    });
  });

  describe('Property 13: Workspace Status Consistency', () => {
    it('should maintain consistent workspace status when operations complete', () => {
      const { result } = renderHook(() => useAppStore());
      
      const workspace: Workspace = {
        id: 'workspace-1',
        name: 'Test Workspace',
        path: '/test/path',
        status: 'PROCESSING',
        size: '100MB',
        files: 50
      };

      act(() => {
        result.current.addWorkspace(workspace);
      });

      expect(result.current.workspaces).toHaveLength(1);
      expect(result.current.workspaces[0].status).toBe('PROCESSING');

      act(() => {
        result.current.updateWorkspace('workspace-1', { status: 'READY' });
      });

      // Status should be consistently updated
      expect(result.current.workspaces[0].status).toBe('READY');
    });

    it('should prevent duplicate workspace creation', () => {
      const { result } = renderHook(() => useAppStore());
      
      const workspace: Workspace = {
        id: 'workspace-1',
        name: 'Test Workspace',
        path: '/test/path',
        status: 'READY',
        size: '100MB',
        files: 50
      };

      act(() => {
        // Add the same workspace multiple times
        result.current.addWorkspace(workspace);
        result.current.addWorkspace(workspace);
        result.current.addWorkspace(workspace);
      });

      // Should only have one workspace despite multiple additions
      expect(result.current.workspaces).toHaveLength(1);
      expect(result.current.workspaces[0]).toEqual(workspace);
    });

    it('should maintain workspace consistency when updating non-existent workspace', () => {
      const { result } = renderHook(() => useAppStore());
      
      // Try to update a workspace that doesn't exist
      act(() => {
        result.current.updateWorkspace('non-existent-id', { status: 'READY' });
      });

      // Should not crash and workspaces should remain empty
      expect(result.current.workspaces).toHaveLength(0);
    });

    it('should properly delete workspace and maintain consistency', () => {
      const { result } = renderHook(() => useAppStore());
      
      const workspace1: Workspace = {
        id: 'workspace-1',
        name: 'Test Workspace 1',
        path: '/test/path1',
        status: 'READY',
        size: '100MB',
        files: 50
      };

      const workspace2: Workspace = {
        id: 'workspace-2',
        name: 'Test Workspace 2',
        path: '/test/path2',
        status: 'READY',
        size: '200MB',
        files: 100
      };

      act(() => {
        result.current.addWorkspace(workspace1);
        result.current.addWorkspace(workspace2);
        result.current.setActiveWorkspace('workspace-1');
      });

      expect(result.current.workspaces).toHaveLength(2);
      expect(result.current.activeWorkspaceId).toBe('workspace-1');

      act(() => {
        result.current.deleteWorkspace('workspace-1');
      });

      // Should have one workspace left and active workspace should be cleared
      expect(result.current.workspaces).toHaveLength(1);
      expect(result.current.workspaces[0].id).toBe('workspace-2');
      // Note: Active workspace clearing is handled by the hook, not the store directly
    });
  });

  describe('Toast Management', () => {
    it('should add toasts with unique IDs', () => {
      const { result } = renderHook(() => useAppStore());
      
      act(() => {
        result.current.addToast('success', 'Test message 1');
      });

      // Small delay to ensure different timestamps
      act(() => {
        jest.advanceTimersByTime(1);
        result.current.addToast('error', 'Test message 2');
      });

      expect(result.current.toasts).toHaveLength(2);
      expect(result.current.toasts[0].type).toBe('success');
      expect(result.current.toasts[0].message).toBe('Test message 1');
      expect(result.current.toasts[1].type).toBe('error');
      expect(result.current.toasts[1].message).toBe('Test message 2');
      
      // IDs should be different
      expect(result.current.toasts[0].id).not.toBe(result.current.toasts[1].id);
    });

    it('should remove toasts by ID', () => {
      const { result } = renderHook(() => useAppStore());
      
      let toastId: number;
      
      act(() => {
        result.current.addToast('info', 'Test message');
      });

      expect(result.current.toasts).toHaveLength(1);
      toastId = result.current.toasts[0].id;

      act(() => {
        result.current.removeToast(toastId);
      });

      expect(result.current.toasts).toHaveLength(0);
    });
  });

  describe('Page Navigation', () => {
    it('should update page state correctly', () => {
      const { result } = renderHook(() => useAppStore());
      
      expect(result.current.page).toBe('workspaces');

      act(() => {
        result.current.setPage('search');
      });

      expect(result.current.page).toBe('search');

      act(() => {
        result.current.setPage('keywords');
      });

      expect(result.current.page).toBe('keywords');
    });
  });

  describe('Keyword Groups Management', () => {
    it('should prevent duplicate keyword group creation', () => {
      const { result } = renderHook(() => useAppStore());
      
      const keywordGroup = {
        id: 'group-1',
        name: 'Test Group',
        color: 'blue' as const,
        patterns: [{ regex: 'test', comment: 'Test pattern' }],
        enabled: true
      };

      act(() => {
        result.current.addKeywordGroup(keywordGroup);
        result.current.addKeywordGroup(keywordGroup);
      });

      expect(result.current.keywordGroups).toHaveLength(1);
      expect(result.current.keywordGroups[0]).toEqual(keywordGroup);
    });

    it('should toggle keyword group enabled state', () => {
      const { result } = renderHook(() => useAppStore());
      
      const keywordGroup = {
        id: 'group-1',
        name: 'Test Group',
        color: 'blue' as const,
        patterns: [{ regex: 'test', comment: 'Test pattern' }],
        enabled: true
      };

      act(() => {
        result.current.addKeywordGroup(keywordGroup);
      });

      expect(result.current.keywordGroups[0].enabled).toBe(true);

      act(() => {
        result.current.toggleKeywordGroup('group-1');
      });

      expect(result.current.keywordGroups[0].enabled).toBe(false);

      act(() => {
        result.current.toggleKeywordGroup('group-1');
      });

      expect(result.current.keywordGroups[0].enabled).toBe(true);
    });
  });
});