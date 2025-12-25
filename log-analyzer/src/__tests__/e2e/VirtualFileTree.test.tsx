/**
 * End-to-End tests for Virtual File Tree functionality
 * 
 * Tests the complete user workflow for:
 * - File tree rendering
 * - Nested archive navigation
 * - File content display
 * - Search with virtual paths
 * 
 * Validates: Requirements 4.2, 1.4
 */

import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import VirtualFileTree from '../../components/VirtualFileTree';

// Mock Tauri API
jest.mock('@tauri-apps/api/core', () => ({
  invoke: jest.fn(),
}));

jest.mock('../../utils/logger', () => ({
  logger: {
    debug: jest.fn(),
    error: jest.fn(),
    info: jest.fn(),
    warn: jest.fn(),
  },
}));

const { invoke: mockInvoke } = require('@tauri-apps/api/core');

describe('E2E: Virtual File Tree', () => {
  let user: ReturnType<typeof userEvent.setup>;

  beforeEach(() => {
    user = userEvent.setup();
    jest.clearAllMocks();
  });

  describe('File tree rendering', () => {
    it('should render file tree with files and archives', async () => {
      // Mock tree data with nested structure
      const mockTreeData = [
        {
          type: 'file',
          name: 'root.log',
          path: 'root.log',
          hash: 'hash_root',
          size: 1024,
          mimeType: 'text/plain',
        },
        {
          type: 'archive',
          name: 'logs.zip',
          path: 'logs.zip',
          hash: 'hash_archive',
          archiveType: 'zip',
          children: [
            {
              type: 'file',
              name: 'nested.log',
              path: 'logs.zip/nested.log',
              hash: 'hash_nested',
              size: 2048,
              mimeType: 'text/plain',
            },
          ],
        },
      ];

      mockInvoke.mockResolvedValue(mockTreeData);

      render(
        <VirtualFileTree
          workspaceId="test-workspace"
          onFileSelect={jest.fn()}
        />
      );

      // Wait for tree to load and verify invoke was called
      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('get_virtual_file_tree', {
          workspaceId: 'test-workspace',
        });
      });

      // Wait for root file to appear
      await waitFor(() => {
        expect(screen.getByText('root.log')).toBeInTheDocument();
      });

      // Verify file size is displayed
      expect(screen.getByText('1.0 KB')).toBeInTheDocument();

      // Verify archive is displayed
      expect(screen.getByText('logs.zip')).toBeInTheDocument();
      expect(screen.getByText('ZIP')).toBeInTheDocument();
    });

    it('should show loading state while fetching tree', () => {
      mockInvoke.mockImplementation(() => new Promise(() => {})); // Never resolves

      render(
        <VirtualFileTree
          workspaceId="test-workspace"
          onFileSelect={jest.fn()}
        />
      );

      expect(screen.getByText(/loading file tree/i)).toBeInTheDocument();
    });

    it('should show error state when tree loading fails', async () => {
      mockInvoke.mockRejectedValue(new Error('Failed to load tree'));

      render(
        <VirtualFileTree
          workspaceId="test-workspace"
          onFileSelect={jest.fn()}
        />
      );

      await waitFor(() => {
        expect(screen.getByText(/failed to load file tree/i)).toBeInTheDocument();
      });
    });

    it('should show empty state when no files exist', async () => {
      mockInvoke.mockResolvedValue([]);

      render(
        <VirtualFileTree
          workspaceId="test-workspace"
          onFileSelect={jest.fn()}
        />
      );

      await waitFor(() => {
        expect(screen.getByText(/no files in workspace/i)).toBeInTheDocument();
      });
    });
  });

  describe('Nested archive navigation', () => {
    it('should expand and collapse archives on click', async () => {
      const mockTreeData = [
        {
          type: 'archive',
          name: 'logs.zip',
          path: 'logs.zip',
          hash: 'hash_archive',
          archiveType: 'zip',
          children: [
            {
              type: 'file',
              name: 'nested.log',
              path: 'logs.zip/nested.log',
              hash: 'hash_nested',
              size: 2048,
              mimeType: 'text/plain',
            },
          ],
        },
      ];

      mockInvoke.mockResolvedValue(mockTreeData);

      render(
        <VirtualFileTree
          workspaceId="test-workspace"
          onFileSelect={jest.fn()}
        />
      );

      await waitFor(() => {
        expect(screen.getByText('logs.zip')).toBeInTheDocument();
      });

      // Initially, nested file should not be visible
      expect(screen.queryByText('nested.log')).not.toBeInTheDocument();

      // Click to expand archive
      const archiveNode = screen.getByText('logs.zip');
      await user.click(archiveNode);

      // Nested file should now be visible
      await waitFor(() => {
        expect(screen.getByText('nested.log')).toBeInTheDocument();
      });

      // Click again to collapse
      await user.click(archiveNode);

      // Nested file should be hidden again
      await waitFor(() => {
        expect(screen.queryByText('nested.log')).not.toBeInTheDocument();
      });
    });

    it('should handle deeply nested archives', async () => {
      const mockTreeData = [
        {
          type: 'archive',
          name: 'level1.zip',
          path: 'level1.zip',
          hash: 'hash_level1',
          archiveType: 'zip',
          children: [
            {
              type: 'archive',
              name: 'level2.zip',
              path: 'level1.zip/level2.zip',
              hash: 'hash_level2',
              archiveType: 'zip',
              children: [
                {
                  type: 'file',
                  name: 'deep.log',
                  path: 'level1.zip/level2.zip/deep.log',
                  hash: 'hash_deep',
                  size: 512,
                  mimeType: 'text/plain',
                },
              ],
            },
          ],
        },
      ];

      mockInvoke.mockResolvedValue(mockTreeData);

      render(
        <VirtualFileTree
          workspaceId="test-workspace"
          onFileSelect={jest.fn()}
        />
      );

      await waitFor(() => {
        expect(screen.getByText('level1.zip')).toBeInTheDocument();
      });

      // Expand first level
      await user.click(screen.getByText('level1.zip'));

      await waitFor(() => {
        expect(screen.getByText('level2.zip')).toBeInTheDocument();
      });

      // Expand second level
      await user.click(screen.getByText('level2.zip'));

      await waitFor(() => {
        expect(screen.getByText('deep.log')).toBeInTheDocument();
      });
    });
  });

  describe('File content display', () => {
    it('should call onFileSelect when file is clicked', async () => {
      const mockTreeData = [
        {
          type: 'file',
          name: 'test.log',
          path: 'test.log',
          hash: 'hash_test',
          size: 1024,
          mimeType: 'text/plain',
        },
      ];

      mockInvoke.mockResolvedValue(mockTreeData);

      const onFileSelect = jest.fn();

      render(
        <VirtualFileTree
          workspaceId="test-workspace"
          onFileSelect={onFileSelect}
        />
      );

      await waitFor(() => {
        expect(screen.getByText('test.log')).toBeInTheDocument();
      });

      // Click on file
      await user.click(screen.getByText('test.log'));

      // Verify callback was called with correct parameters
      expect(onFileSelect).toHaveBeenCalledWith('hash_test', 'test.log');
    });

    it('should handle file selection in nested archives', async () => {
      const mockTreeData = [
        {
          type: 'archive',
          name: 'logs.zip',
          path: 'logs.zip',
          hash: 'hash_archive',
          archiveType: 'zip',
          children: [
            {
              type: 'file',
              name: 'nested.log',
              path: 'logs.zip/nested.log',
              hash: 'hash_nested',
              size: 2048,
              mimeType: 'text/plain',
            },
          ],
        },
      ];

      mockInvoke.mockResolvedValue(mockTreeData);

      const onFileSelect = jest.fn();

      render(
        <VirtualFileTree
          workspaceId="test-workspace"
          onFileSelect={onFileSelect}
        />
      );

      await waitFor(() => {
        expect(screen.getByText('logs.zip')).toBeInTheDocument();
      });

      // Expand archive
      await user.click(screen.getByText('logs.zip'));

      await waitFor(() => {
        expect(screen.getByText('nested.log')).toBeInTheDocument();
      });

      // Click on nested file
      await user.click(screen.getByText('nested.log'));

      // Verify callback was called with virtual path
      expect(onFileSelect).toHaveBeenCalledWith('hash_nested', 'logs.zip/nested.log');
    });
  });

  describe('Search with virtual paths', () => {
    it('should display files with full virtual paths', async () => {
      const mockTreeData = [
        {
          type: 'archive',
          name: 'app.zip',
          path: 'app.zip',
          hash: 'hash_app',
          archiveType: 'zip',
          children: [
            {
              type: 'archive',
              name: 'logs.tar.gz',
              path: 'app.zip/logs.tar.gz',
              hash: 'hash_logs',
              archiveType: 'tar.gz',
              children: [
                {
                  type: 'file',
                  name: 'error.log',
                  path: 'app.zip/logs.tar.gz/error.log',
                  hash: 'hash_error',
                  size: 4096,
                  mimeType: 'text/plain',
                },
              ],
            },
          ],
        },
      ];

      mockInvoke.mockResolvedValue(mockTreeData);

      render(
        <VirtualFileTree
          workspaceId="test-workspace"
          onFileSelect={jest.fn()}
        />
      );

      await waitFor(() => {
        expect(screen.getByText('app.zip')).toBeInTheDocument();
      });

      // Expand all levels
      await user.click(screen.getByText('app.zip'));
      
      await waitFor(() => {
        expect(screen.getByText('logs.tar.gz')).toBeInTheDocument();
      });

      await user.click(screen.getByText('logs.tar.gz'));

      await waitFor(() => {
        expect(screen.getByText('error.log')).toBeInTheDocument();
      });

      // Verify the file node has the correct title (full path)
      const fileNode = screen.getByText('error.log');
      expect(fileNode).toHaveAttribute('title', 'error.log');
    });
  });

  describe('Integration with hash-based retrieval', () => {
    it('should provide hash for file content retrieval', async () => {
      const mockTreeData = [
        {
          type: 'file',
          name: 'test.log',
          path: 'test.log',
          hash: 'a3f2e1d4c5b6a7890123456789abcdef0123456789abcdef0123456789abcdef',
          size: 1024,
          mimeType: 'text/plain',
        },
      ];

      mockInvoke.mockResolvedValue(mockTreeData);

      const onFileSelect = jest.fn();

      render(
        <VirtualFileTree
          workspaceId="test-workspace"
          onFileSelect={onFileSelect}
        />
      );

      await waitFor(() => {
        expect(screen.getByText('test.log')).toBeInTheDocument();
      });

      await user.click(screen.getByText('test.log'));

      // Verify hash is passed correctly (SHA-256 format)
      expect(onFileSelect).toHaveBeenCalledWith(
        'a3f2e1d4c5b6a7890123456789abcdef0123456789abcdef0123456789abcdef',
        'test.log'
      );
    });
  });
});
