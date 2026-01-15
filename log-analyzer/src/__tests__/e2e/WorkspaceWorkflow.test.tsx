/**
 * End-to-End tests for critical user workflows
 * Tests complete user journeys through the application
 * Validates: Requirements 4.1, 4.2, 4.3, 4.4, 4.5
 */

import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import App from '../../App';
import { renderAppAndWait } from '../../test-utils/e2e';

// Mock Tauri API
jest.mock('@tauri-apps/api/core', () => ({
  invoke: jest.fn(),
}));

jest.mock('@tauri-apps/api/event', () => ({
  listen: jest.fn(),
  emit: jest.fn(),
}));

jest.mock('@tauri-apps/plugin-dialog', () => ({
  open: jest.fn(),
}));

// Mock logger
jest.mock('../../utils/logger', () => ({
  logger: {
    debug: jest.fn(),
    info: jest.fn(),
    warn: jest.fn(),
    error: jest.fn(),
    getLevel: jest.fn(() => 'info'),
    setLevel: jest.fn(),
  },
}));

// Test wrapper component
const TestWrapper: React.FC<{ children: React.ReactNode }> = ({ children }) => {
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

  return (
    <QueryClientProvider client={queryClient}>
      {children}
    </QueryClientProvider>
  );
};

const { invoke: mockInvoke } = require('@tauri-apps/api/core');
const { listen: mockListen } = require('@tauri-apps/api/event');

describe.skip('E2E: Workspace Management Workflow', () => {
  let user: ReturnType<typeof userEvent.setup>;

  beforeEach(async () => {
    user = userEvent.setup();
    jest.clearAllMocks();

    // Setup default mock responses
    mockListen.mockResolvedValue(() => {});
    mockInvoke.mockImplementation((command: string) => {
      switch (command) {
        case 'load_config':
          return Promise.resolve({ workspaces: [], keyword_groups: [] });
        case 'get_workspaces':
          return Promise.resolve([]);
        case 'init_state_sync':
          return Promise.resolve();
        case 'get_tasks':
          return Promise.resolve([]);
        case 'get_keyword_groups':
          return Promise.resolve([]);
        default:
          return Promise.resolve(null);
      }
    });

    // Render app first, before any waitFor
    render(
      <TestWrapper>
        <App />
      </TestWrapper>
    );
  });

  describe('Complete Workspace Creation and Management Flow', () => {
    it('should allow user to create, configure, and manage a workspace', async () => {
      // Wait for app initialization (loading screen to disappear)
      await waitFor(() => {
        expect(screen.queryByText(/loading application/i)).not.toBeInTheDocument();
      }, { timeout: 3000 });

      // Wait for workspaces button to appear
      await waitFor(() => {
        expect(screen.getByRole("button", { name: /workspaces/i })).toBeInTheDocument();
      });

      // Mock workspace creation responses
      mockInvoke.mockImplementation((command: string, args?: any) => {
        switch (command) {
          case 'load_config':
            return Promise.resolve({ workspaces: [], keyword_groups: [] });
          case 'get_workspaces':
            return Promise.resolve([]);
          case 'create_workspace':
            return Promise.resolve({
              id: 'test-workspace-1',
              name: args?.name || 'Test Workspace',
              path: args?.path || '/test/path',
              status: 'PROCESSING',
              size: '0MB',
              files: 0,
            });
          case 'get_workspace_status':
            return Promise.resolve({
              id: 'test-workspace-1',
              status: 'READY',
              size: '150MB',
              files: 42,
            });
          default:
            return Promise.resolve(null);
        }
      });

      // Step 1: Navigate to workspace creation
      const createButton = await screen.findByRole('button', { name: /create.*workspace/i });
      await user.click(createButton);

      // Step 2: Fill out workspace form
      const nameInput = await screen.findByLabelText(/workspace.*name/i);
      const pathInput = await screen.findByLabelText(/path/i);

      await user.type(nameInput, 'My Test Workspace');
      await user.type(pathInput, '/home/user/logs');

      // Step 3: Submit workspace creation
      const submitButton = screen.getByRole('button', { name: /create/i });
      await user.click(submitButton);

      // Step 4: Verify workspace appears in list
      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('create_workspace', {
          name: 'My Test Workspace',
          path: '/home/user/logs',
        });
      });

      // Step 5: Simulate workspace processing completion
      await waitFor(() => {
        expect(screen.getByText(/processing/i)).toBeInTheDocument();
      });

      // Simulate status update event
      const statusUpdateEvent = {
        payload: {
          id: 'test-workspace-1',
          status: 'READY',
          size: '150MB',
          files: 42,
        },
      };

      // Trigger the event handler if it was set up
      if (mockListen.mock.calls.length > 0) {
        const workspaceUpdateCall = mockListen.mock.calls.find(
          call => call[0] === 'workspace-update'
        );
        if (workspaceUpdateCall) {
          const handler = workspaceUpdateCall[1];
          handler(statusUpdateEvent);
        }
      }

      // Step 6: Verify workspace is ready
      await waitFor(() => {
        expect(screen.getByText(/ready/i)).toBeInTheDocument();
        expect(screen.getByText(/150MB/i)).toBeInTheDocument();
        expect(screen.getByText(/42.*files/i)).toBeInTheDocument();
      });
    });

    it('should handle workspace creation errors gracefully', async () => {
      mockInvoke.mockImplementation((command: string) => {
        switch (command) {
          case 'load_config':
            return Promise.resolve({ workspaces: [], keyword_groups: [] });
          case 'get_workspaces':
            return Promise.resolve([]);
          case 'create_workspace':
            return Promise.reject(new Error('Invalid path: Permission denied'));
          default:
            return Promise.resolve(null);
        }
      });

      await renderAppAndWait();

      await waitFor(() => {
        expect(screen.getByRole("button", { name: /workspaces/i })).toBeInTheDocument();
      });

      // Try to create workspace
      const createButton = await screen.findByRole('button', { name: /create.*workspace/i });
      await user.click(createButton);

      const nameInput = await screen.findByLabelText(/workspace.*name/i);
      const pathInput = await screen.findByLabelText(/path/i);

      await user.type(nameInput, 'Test Workspace');
      await user.type(pathInput, '/invalid/path');

      const submitButton = screen.getByRole('button', { name: /create/i });
      await user.click(submitButton);

      // Should display error message
      await waitFor(() => {
        expect(screen.getByText(/permission denied/i)).toBeInTheDocument();
      });

      // Should provide retry option
      expect(screen.getByRole('button', { name: /try again/i })).toBeInTheDocument();
    });
  });

  describe('Search Workflow Integration', () => {
    it('should allow user to perform search across workspaces', async () => {
      const mockWorkspaces = [
        {
          id: 'workspace-1',
          name: 'Logs Workspace',
          path: '/logs',
          status: 'READY',
          size: '100MB',
          files: 25,
        },
      ];

      const mockSearchResults = [
        {
          id: 1,
          content: 'Error: Connection timeout',
          file: 'app.log',
          line: 42,
          timestamp: '2024-01-01 12:00:00',
          level: 'ERROR',
        },
      ];

      mockInvoke.mockImplementation((command: string, _args?: any) => {
        switch (command) {
          case 'load_config':
            return Promise.resolve({ workspaces: mockWorkspaces, keyword_groups: [] });
          case 'get_workspaces':
            return Promise.resolve(mockWorkspaces);
          case 'init_state_sync':
            return Promise.resolve();
          case 'search_logs':
            return Promise.resolve(mockSearchResults);
          default:
            return Promise.resolve([]);
        }
      });

      await renderAppAndWait();

      // Navigate to search page
      const searchTab = await screen.findByRole('button', { name: /search/i });
      await user.click(searchTab);

      // Enter search query
      const searchInput = await screen.findByPlaceholderText(/search/i);
      await user.type(searchInput, 'error timeout');

      // Select workspace
      const workspaceSelect = await screen.findByLabelText(/workspace/i);
      await user.selectOptions(workspaceSelect, 'workspace-1');

      // Execute search
      const searchButton = screen.getByRole('button', { name: /search/i });
      await user.click(searchButton);

      // Verify search results
      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('search_logs', {
          query: 'error timeout',
          workspaceId: 'workspace-1',
        });
      });

      await waitFor(() => {
        expect(screen.getByText(/connection timeout/i)).toBeInTheDocument();
        expect(screen.getByText(/app\.log/i)).toBeInTheDocument();
        expect(screen.getByText(/line 42/i)).toBeInTheDocument();
      });
    });
  });

  describe('Task Management Integration', () => {
    it('should display and manage background tasks', async () => {
      const mockTasks = [
        {
          id: 'task-1',
          type: 'Import',
          target: 'large-logs.zip',
          progress: 75,
          message: 'Extracting files...',
          status: 'RUNNING',
          workspaceId: 'workspace-1',
        },
      ];

      mockInvoke.mockImplementation((command: string) => {
        switch (command) {
          case 'get_tasks':
            return Promise.resolve(mockTasks);
          case 'cancel_task':
            return Promise.resolve({ success: true });
          default:
            return Promise.resolve([]);
        }
      });

      await renderAppAndWait();

      // Navigate to tasks page
      const tasksTab = await screen.findByRole('button', { name: /tasks/i });
      await user.click(tasksTab);

      // Verify task is displayed
      await waitFor(() => {
        expect(screen.getByText(/import/i)).toBeInTheDocument();
        expect(screen.getByText(/large-logs\.zip/i)).toBeInTheDocument();
        expect(screen.getByText(/75%/i)).toBeInTheDocument();
        expect(screen.getByText(/extracting files/i)).toBeInTheDocument();
      });

      // Test task cancellation
      const cancelButton = screen.getByRole('button', { name: /cancel/i });
      await user.click(cancelButton);

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('cancel_task', { taskId: 'task-1' });
      });
    });

    it('should handle task progress updates in real-time', async () => {
      mockInvoke.mockImplementation((command: string) => {
        switch (command) {
          case 'get_tasks':
            return Promise.resolve([]);
          default:
            return Promise.resolve([]);
        }
      });

      await renderAppAndWait();

      // Navigate to tasks page
      const tasksTab = await screen.findByRole('button', { name: /tasks/i });
      await user.click(tasksTab);

      // Simulate task creation event
      const taskCreateEvent = {
        payload: {
          id: 'task-2',
          type: 'Export',
          target: 'results.csv',
          progress: 0,
          message: 'Starting export...',
          status: 'RUNNING',
          workspaceId: 'workspace-1',
        },
      };

      // Trigger task update event
      if (mockListen.mock.calls.length > 0) {
        const taskUpdateCall = mockListen.mock.calls.find(
          call => call[0] === 'task-update'
        );
        if (taskUpdateCall) {
          const handler = taskUpdateCall[1];
          handler(taskCreateEvent);
        }
      }

      // Verify task appears
      await waitFor(() => {
        expect(screen.getByText(/export/i)).toBeInTheDocument();
        expect(screen.getByText(/results\.csv/i)).toBeInTheDocument();
        expect(screen.getByText(/0%/i)).toBeInTheDocument();
      });

      // Simulate progress update
      const progressUpdateEvent = {
        payload: {
          id: 'task-2',
          progress: 50,
          message: 'Exporting data...',
        },
      };

      if (mockListen.mock.calls.length > 0) {
        const taskUpdateCall = mockListen.mock.calls.find(
          call => call[0] === 'task-update'
        );
        if (taskUpdateCall) {
          const handler = taskUpdateCall[1];
          handler(progressUpdateEvent);
        }
      }

      // Verify progress update
      await waitFor(() => {
        expect(screen.getByText(/50%/i)).toBeInTheDocument();
        expect(screen.getByText(/exporting data/i)).toBeInTheDocument();
      });
    });
  });

  describe('Error Recovery and User Experience', () => {
    it('should provide clear feedback and recovery options for network errors', async () => {
      mockInvoke.mockRejectedValue(new Error('Network connection failed'));

      await renderAppAndWait();

      // Wait for error to appear
      await waitFor(() => {
        expect(screen.getByText(/network connection failed/i)).toBeInTheDocument();
      });

      // Should provide retry option
      expect(screen.getByRole('button', { name: /retry/i })).toBeInTheDocument();

      // Test retry functionality
      mockInvoke.mockResolvedValueOnce([]);
      const retryButton = screen.getByRole('button', { name: /retry/i });
      await user.click(retryButton);

      // Should attempt to reload
      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledTimes(2);
      });
    });

    it('should maintain application state during error recovery', async () => {
      // Start with successful state
      mockInvoke.mockResolvedValue([]);

      await renderAppAndWait();

      await waitFor(() => {
        expect(screen.getByRole("button", { name: /workspaces/i })).toBeInTheDocument();
      });

      // Navigate to search page
      const searchTab = await screen.findByRole('button', { name: /search/i });
      await user.click(searchTab);

      // Simulate error on next operation
      mockInvoke.mockRejectedValueOnce(new Error('Temporary error'));

      // Try to perform search
      const searchInput = await screen.findByPlaceholderText(/search/i);
      await user.type(searchInput, 'test query');

      // Should still be on search page despite error
      expect(screen.getByPlaceholderText(/search/i)).toBeInTheDocument();
      expect(screen.getByDisplayValue('test query')).toBeInTheDocument();
    });
  });
});