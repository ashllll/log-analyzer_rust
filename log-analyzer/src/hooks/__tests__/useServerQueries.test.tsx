/**
 * Integration tests for React Query server state management
 * Tests integration between React Query and zustand store
 * Validates: Requirements 4.2, 4.3
 */

import React, { ReactNode } from 'react';
import { renderHook, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { useConfigQuery, useImportFolderMutation } from '../useServerQueries';
import { useAppStore } from '../../stores/appStore';
import { useWorkspaceStore } from '../../stores/workspaceStore';
import { useKeywordStore } from '../../stores/keywordStore';

// Mock Tauri invoke function
jest.mock('@tauri-apps/api/core', () => ({
  invoke: jest.fn(),
}));

const mockInvoke = require('@tauri-apps/api/core').invoke;

// Mock react-hot-toast
jest.mock('react-hot-toast', () => ({
  toast: {
    success: jest.fn(),
    error: jest.fn(),
    default: jest.fn(),
  },
}));

const mockToast = require('react-hot-toast').toast;

// Mock logger
jest.mock('../../utils/logger', () => ({
  logger: {
    debug: jest.fn(),
    error: jest.fn(),
    info: jest.fn(),
    warn: jest.fn(),
  },
}));

// Test wrapper with QueryClient
const createWrapper = () => {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
        gcTime: 0,
      },
      mutations: {
        retry: false,
      },
    },
  });

  return ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={queryClient}>
      {children}
    </QueryClientProvider>
  );
};

describe('Server Queries Integration Tests', () => {
  beforeEach(() => {
    // Reset all store states (Zustand stores)
    useAppStore.setState({
      page: 'workspaces',
      toasts: [],
      activeWorkspaceId: null,
    });

    useWorkspaceStore.setState({
      workspaces: [],
      loading: false,
      error: null,
    });

    useKeywordStore.setState({
      keywordGroups: [],
      loading: false,
      error: null,
    });

    // Reset mocks
    jest.clearAllMocks();
  });

  describe('useConfigQuery', () => {
    it('should load configuration and update zustand store', async () => {
      const mockConfig = {
        workspaces: [
          {
            id: 'workspace-1',
            name: 'Test Workspace',
            path: '/test/path',
            status: 'READY',
            size: '100MB',
            files: 50
          }
        ],
        keyword_groups: [
          {
            id: 'group-1',
            name: 'Test Group',
            color: 'blue',
            patterns: [{ regex: 'test', comment: 'Test pattern' }],
            enabled: true
          }
        ]
      };

      mockInvoke.mockResolvedValueOnce(mockConfig);

      const wrapper = createWrapper();
      const { result } = renderHook(() => useConfigQuery(), { wrapper });

      await waitFor(() => {
        expect(result.current.isSuccess).toBe(true);
      });

      // Check that the stores were updated
      const workspaceStore = useWorkspaceStore.getState();
      const keywordStore = useKeywordStore.getState();
      expect(workspaceStore.workspaces).toHaveLength(1);
      expect(workspaceStore.workspaces[0].id).toBe('workspace-1');
      expect(keywordStore.keywordGroups).toHaveLength(1);
      expect(keywordStore.keywordGroups[0].id).toBe('group-1');
    });

    it('should handle configuration loading errors', async () => {
      const mockError = new Error('Failed to load config');
      mockInvoke.mockRejectedValueOnce(mockError);

      const wrapper = createWrapper();
      const { result } = renderHook(() => useConfigQuery(), { wrapper });

      await waitFor(() => {
        expect(result.current.isError).toBe(true);
      });

      expect(result.current.error).toEqual(mockError);
    });
  });

  describe('useImportFolderMutation', () => {
    it('should perform optimistic updates and handle success', async () => {
      const mockTaskId = 'task-123';
      mockInvoke.mockResolvedValueOnce(mockTaskId);

      const wrapper = createWrapper();
      const { result } = renderHook(() => useImportFolderMutation(), { wrapper });

      const importParams = {
        path: '/test/folder',
        workspaceId: 'workspace-123'
      };

      // Trigger the mutation
      result.current.mutate(importParams);

      // Check optimistic update - workspace should be added immediately
      await waitFor(() => {
        const workspaceStore = useWorkspaceStore.getState();
        const appStore = useAppStore.getState();
        expect(workspaceStore.workspaces).toHaveLength(1);
        expect(workspaceStore.workspaces[0].id).toBe('workspace-123');
        expect(workspaceStore.workspaces[0].status).toBe('PROCESSING');
        expect(appStore.activeWorkspaceId).toBe('workspace-123');
      });

      // Wait for mutation to complete
      await waitFor(() => {
        expect(result.current.isSuccess).toBe(true);
      });

      // Check that toast was called
      expect(mockToast.default).toHaveBeenCalledWith('Import started', { duration: 3000, icon: 'ℹ️' });
    });

    it('should rollback optimistic updates on error', async () => {
      const mockError = new Error('Import failed');
      mockInvoke.mockRejectedValueOnce(mockError);

      const wrapper = createWrapper();
      const { result } = renderHook(() => useImportFolderMutation(), { wrapper });

      const importParams = {
        path: '/test/folder',
        workspaceId: 'workspace-123'
      };

      // Trigger the mutation
      result.current.mutate(importParams);

      // Check optimistic update
      await waitFor(() => {
        const workspaceStore = useWorkspaceStore.getState();
        expect(workspaceStore.workspaces).toHaveLength(1);
      });

      // Wait for mutation to fail
      await waitFor(() => {
        expect(result.current.isError).toBe(true);
      });

      // Check that optimistic update was rolled back
      const workspaceStore = useWorkspaceStore.getState();
      expect(workspaceStore.workspaces).toHaveLength(0);

      // Check that error toast was called
      expect(mockToast.error).toHaveBeenCalledWith('Import failed: Error: Import failed', { duration: 4000 });
    });

    it('should handle concurrent mutations correctly', async () => {
      mockInvoke
        .mockResolvedValueOnce('task-1')
        .mockResolvedValueOnce('task-2');

      const wrapper = createWrapper();
      const { result } = renderHook(() => useImportFolderMutation(), { wrapper });

      // Trigger two mutations concurrently
      result.current.mutate({
        path: '/test/folder1',
        workspaceId: 'workspace-1'
      });

      result.current.mutate({
        path: '/test/folder2',
        workspaceId: 'workspace-2'
      });

      // Wait for both mutations to complete
      await waitFor(() => {
        expect(result.current.isSuccess).toBe(true);
      });

      // Check that both workspaces were added
      const workspaceStore = useWorkspaceStore.getState();
      expect(workspaceStore.workspaces).toHaveLength(2);

      const workspaceIds = workspaceStore.workspaces.map(w => w.id);
      expect(workspaceIds).toContain('workspace-1');
      expect(workspaceIds).toContain('workspace-2');
    });
  });

  describe('React Query and Zustand Integration', () => {
    it('should maintain state consistency between React Query and Zustand', async () => {
      const mockConfig = {
        workspaces: [
          {
            id: 'workspace-1',
            name: 'Test Workspace',
            path: '/test/path',
            status: 'READY',
            size: '100MB',
            files: 50
          }
        ],
        keyword_groups: []
      };

      mockInvoke.mockResolvedValueOnce(mockConfig);

      const wrapper = createWrapper();
      
      // Load config using React Query
      const { result: configResult } = renderHook(() => useConfigQuery(), { wrapper });
      
      await waitFor(() => {
        expect(configResult.current.isSuccess).toBe(true);
      });

      // Check that Zustand store was updated
      const workspaceStore = useWorkspaceStore.getState();
      expect(workspaceStore.workspaces).toHaveLength(1);
      expect(workspaceStore.workspaces[0].id).toBe('workspace-1');

      // Now modify the store directly
      workspaceStore.updateWorkspace('workspace-1', { status: 'PROCESSING' });

      // Verify the change
      expect(useWorkspaceStore.getState().workspaces[0].status).toBe('PROCESSING');

      // React Query cache should still have the original data
      expect(configResult.current.data).toEqual(mockConfig);
    });

    it('should handle automatic background refetching', async () => {
      const mockConfig1 = {
        workspaces: [{ id: 'workspace-1', name: 'Test 1', path: '/test1', status: 'READY', size: '100MB', files: 50 }],
        keyword_groups: []
      };

      const mockConfig2 = {
        workspaces: [{ id: 'workspace-2', name: 'Test 2', path: '/test2', status: 'READY', size: '200MB', files: 100 }],
        keyword_groups: []
      };

      mockInvoke
        .mockResolvedValueOnce(mockConfig1)
        .mockResolvedValueOnce(mockConfig2);

      const wrapper = createWrapper();
      const { result } = renderHook(() => useConfigQuery(), { wrapper });

      // Wait for first load
      await waitFor(() => {
        expect(result.current.isSuccess).toBe(true);
      });

      expect(useWorkspaceStore.getState().workspaces[0].id).toBe('workspace-1');

      // Trigger refetch
      result.current.refetch();

      // Wait for refetch to complete
      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledTimes(2);
      });

      // Store should be updated with new data
      expect(useWorkspaceStore.getState().workspaces[0].id).toBe('workspace-2');
    });
  });
});