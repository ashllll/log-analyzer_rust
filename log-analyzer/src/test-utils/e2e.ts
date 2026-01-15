/**
 * Test utilities for E2E tests
 * Provides common helper functions for rendering and waiting
 */

import React from 'react';
import { render, waitFor, screen } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import App from '../../App';

// Mock Tauri API
jest.mock('@tauri-apps/api/core', () => ({
  invoke: jest.fn(),
}));

jest.mock('@tauri-apps/api/event', () => ({
  listen: jest.fn(),
  emit: jest.fn(),
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

export const createTestClient = () => {
  return new QueryClient({
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
};

export const TestWrapper: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const queryClient = createTestClient();

  return (
    <QueryClientProvider client={queryClient}>
      {children}
    </QueryClientProvider>
  );
};

/**
 * Render the App and wait for initialization to complete
 * This handles the async initialization (load_config, init_state_sync)
 * to avoid timing issues in tests
 */
export const renderAppAndWait = async () => {
  render(
    <TestWrapper>
      <App />
    </TestWrapper>
  );

  // Wait for app to initialize (isInitialized becomes true)
  // The loading screen should disappear
  await waitFor(
    () => {
      expect(screen.queryByText(/loading application/i)).not.toBeInTheDocument();
    },
    { timeout: 5000 }
  );
};

/**
 * Setup default mocks for common Tauri commands
 */
export const setupDefaultMocks = (mockInvoke: jest.Mock, mockListen: jest.Mock) => {
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
};
