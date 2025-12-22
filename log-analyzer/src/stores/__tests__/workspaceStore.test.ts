import { renderHook, act } from '@testing-library/react';
import { useWorkspaceStore, type Workspace } from '../workspaceStore';

/**
 * Workspace Store 测试
 * 
 * 测试工作区状态管理的核心功能
 */

describe('WorkspaceStore', () => {
  beforeEach(() => {
    // 重置 store 状态
    const { result } = renderHook(() => useWorkspaceStore());
    act(() => {
      result.current.setWorkspaces([]);
      result.current.setError(null);
      result.current.setLoading(false);
    });
  });

  describe('Basic Operations', () => {
    it('should add workspace', () => {
      const { result } = renderHook(() => useWorkspaceStore());

      const workspace: Workspace = {
        id: 'ws-1',
        name: 'Test Workspace',
        path: '/test/path',
        status: 'READY',
        size: '100MB',
        files: 50,
      };

      act(() => {
        result.current.addWorkspace(workspace);
      });

      expect(result.current.workspaces).toHaveLength(1);
      expect(result.current.workspaces[0]).toEqual(workspace);
    });

    it('should update workspace', () => {
      const { result } = renderHook(() => useWorkspaceStore());

      const workspace: Workspace = {
        id: 'ws-1',
        name: 'Test Workspace',
        path: '/test/path',
        status: 'PROCESSING',
        size: '0',
        files: 0,
      };

      act(() => {
        result.current.addWorkspace(workspace);
      });

      act(() => {
        result.current.updateWorkspace('ws-1', {
          status: 'READY',
          size: '150MB',
          files: 75,
        });
      });

      expect(result.current.workspaces[0].status).toBe('READY');
      expect(result.current.workspaces[0].size).toBe('150MB');
      expect(result.current.workspaces[0].files).toBe(75);
    });

    it('should delete workspace', () => {
      const { result } = renderHook(() => useWorkspaceStore());

      const workspace: Workspace = {
        id: 'ws-1',
        name: 'Test Workspace',
        path: '/test/path',
        status: 'READY',
        size: '100MB',
        files: 50,
      };

      act(() => {
        result.current.addWorkspace(workspace);
      });

      expect(result.current.workspaces).toHaveLength(1);

      act(() => {
        result.current.deleteWorkspace('ws-1');
      });

      expect(result.current.workspaces).toHaveLength(0);
    });

    it('should set workspaces', () => {
      const { result } = renderHook(() => useWorkspaceStore());

      const workspaces: Workspace[] = [
        {
          id: 'ws-1',
          name: 'Workspace 1',
          path: '/path/1',
          status: 'READY',
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
      ];

      act(() => {
        result.current.setWorkspaces(workspaces);
      });

      expect(result.current.workspaces).toHaveLength(2);
      expect(result.current.workspaces).toEqual(workspaces);
    });
  });

  describe('Loading and Error States', () => {
    it('should set loading state', () => {
      const { result } = renderHook(() => useWorkspaceStore());

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
      const { result } = renderHook(() => useWorkspaceStore());

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
    it('should handle updating non-existent workspace', () => {
      const { result } = renderHook(() => useWorkspaceStore());

      act(() => {
        result.current.updateWorkspace('non-existent', { status: 'READY' });
      });

      expect(result.current.workspaces).toHaveLength(0);
    });

    it('should handle deleting non-existent workspace', () => {
      const { result } = renderHook(() => useWorkspaceStore());

      act(() => {
        result.current.deleteWorkspace('non-existent');
      });

      expect(result.current.workspaces).toHaveLength(0);
    });

    it('should handle multiple updates to same workspace', () => {
      const { result } = renderHook(() => useWorkspaceStore());

      const workspace: Workspace = {
        id: 'ws-1',
        name: 'Test Workspace',
        path: '/test/path',
        status: 'OFFLINE',
        size: '0',
        files: 0,
      };

      act(() => {
        result.current.addWorkspace(workspace);
      });

      act(() => {
        result.current.updateWorkspace('ws-1', { status: 'PROCESSING' });
        result.current.updateWorkspace('ws-1', { size: '50MB' });
        result.current.updateWorkspace('ws-1', { files: 25 });
        result.current.updateWorkspace('ws-1', { status: 'READY' });
      });

      expect(result.current.workspaces[0].status).toBe('READY');
      expect(result.current.workspaces[0].size).toBe('50MB');
      expect(result.current.workspaces[0].files).toBe(25);
    });
  });
});
