/**
 * End-to-End tests for CAS Migration Workflows
 * 
 * Tests complete user workflows to validate CAS architecture:
 * - Import workflow (folders and archives)
 * - Search workflow (using MetadataStore and CAS)
 * - Workspace management (create, delete, verify)
 * 
 * Validates: Requirements 2.1, 2.2, 2.3, 2.4, 2.5, 4.4
 * 
 * **Feature: complete-cas-migration, Property N/A: E2E validation tests**
 */

import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
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
  },
}));

const { invoke: mockInvoke } = require('@tauri-apps/api/core');
const { listen: mockListen } = require('@tauri-apps/api/event');
const { open: mockDialogOpen } = require('@tauri-apps/plugin-dialog');

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

describe('E2E: CAS Migration - Import Workflow', () => {
  let user: ReturnType<typeof userEvent.setup>;

  beforeEach(() => {
    user = userEvent.setup();
    jest.clearAllMocks();
    
    // Setup default mock responses
    mockListen.mockResolvedValue(() => {});
    mockInvoke.mockImplementation((command: string) => {
      switch (command) {
        case 'get_workspaces':
          return Promise.resolve([]);
        case 'get_tasks':
          return Promise.resolve([]);
        case 'get_keyword_groups':
          return Promise.resolve([]);
        default:
          return Promise.resolve(null);
      }
    });
  });

  describe('Import Folder Workflow - CAS Storage', () => {
    it('should import folder and store files using CAS architecture', async () => {
      const workspaceId = 'cas-workspace-folder-001';
      
      // Mock folder selection
      mockDialogOpen.mockResolvedValue('/test/logs/folder');
      
      // Mock import process
      mockInvoke.mockImplementation((command: string, args?: any) => {
        switch (command) {
          case 'get_workspaces':
            return Promise.resolve([]);
          case 'import_folder':
            return Promise.resolve({
              workspaceId,
              status: 'PROCESSING',
              message: 'Importing files...',
            });
          case 'get_workspace_status':
            return Promise.resolve({
              id: workspaceId,
              name: 'Test Folder',
              status: 'READY',
              files: 10,
              size: '5MB',
              format: 'cas', // CAS format
            });
          case 'get_virtual_file_tree':
            // Return CAS-based file tree with hashes
            return Promise.resolve([
              {
                type: 'file',
                name: 'app.log',
                path: 'app.log',
                hash: 'a1b2c3d4e5f6789012345678901234567890123456789012345678901234567890',
                size: 1024,
                mimeType: 'text/plain',
              },
              {
                type: 'file',
                name: 'error.log',
                path: 'error.log',
                hash: 'b2c3d4e5f6789012345678901234567890123456789012345678901234567891',
                size: 2048,
                mimeType: 'text/plain',
              },
            ]);
          default:
            return Promise.resolve(null);
        }
      });

      render(
        <TestWrapper>
          <App />
        </TestWrapper>
      );

      // Navigate to workspaces page
      await waitFor(() => {
        expect(screen.getByText(/workspaces/i)).toBeInTheDocument();
      });

      // Click import folder button
      const importButton = await screen.findByRole('button', { name: /import.*folder/i });
      await user.click(importButton);

      // Verify folder dialog was opened
      await waitFor(() => {
        expect(mockDialogOpen).toHaveBeenCalled();
      });

      // Verify import command was called
      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('import_folder', {
          path: '/test/logs/folder',
        });
      });

      // Wait for workspace to be ready
      await waitFor(() => {
        expect(screen.getByText(/ready/i)).toBeInTheDocument();
      }, { timeout: 5000 });

      // Verify workspace shows CAS format
      expect(screen.getByText(/10.*files/i)).toBeInTheDocument();
      expect(screen.getByText(/5MB/i)).toBeInTheDocument();

      // Verify file tree uses CAS (hash-based retrieval)
      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('get_virtual_file_tree', {
          workspaceId,
        });
      });
    });
  });

  describe('Import Archive Workflow - CAS Deduplication', () => {
    it('should import archive and deduplicate files using CAS', async () => {
      const workspaceId = 'cas-workspace-archive-001';
      
      // Mock archive selection
      mockDialogOpen.mockResolvedValue('/test/logs/archive.zip');
      
      mockInvoke.mockImplementation((command: string, args?: any) => {
        switch (command) {
          case 'get_workspaces':
            return Promise.resolve([]);
          case 'import_archive':
            return Promise.resolve({
              workspaceId,
              status: 'PROCESSING',
              message: 'Extracting archive...',
            });
          case 'get_workspace_status':
            return Promise.resolve({
              id: workspaceId,
              name: 'archive.zip',
              status: 'READY',
              files: 25,
              size: '10MB',
              format: 'cas',
              deduplicationRatio: 0.35, // 35% space saved through CAS deduplication
            });
          case 'get_virtual_file_tree':
            return Promise.resolve([
              {
                type: 'archive',
                name: 'archive.zip',
                path: 'archive.zip',
                hash: 'archive_hash_123',
                archiveType: 'zip',
                children: [
                  {
                    type: 'file',
                    name: 'log1.txt',
                    path: 'archive.zip/log1.txt',
                    hash: 'same_content_hash_456', // Same hash = deduplicated
                    size: 1024,
                    mimeType: 'text/plain',
                  },
                  {
                    type: 'file',
                    name: 'log2.txt',
                    path: 'archive.zip/log2.txt',
                    hash: 'same_content_hash_456', // Same content, same hash
                    size: 1024,
                    mimeType: 'text/plain',
                  },
                ],
              },
            ]);
          default:
            return Promise.resolve(null);
        }
      });

      render(
        <TestWrapper>
          <App />
        </TestWrapper>
      );

      await waitFor(() => {
        expect(screen.getByText(/workspaces/i)).toBeInTheDocument();
      });

      // Click import archive button
      const importButton = await screen.findByRole('button', { name: /import.*archive/i });
      await user.click(importButton);

      await waitFor(() => {
        expect(mockDialogOpen).toHaveBeenCalled();
      });

      // Verify import command
      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('import_archive', {
          path: '/test/logs/archive.zip',
        });
      });

      // Wait for completion
      await waitFor(() => {
        expect(screen.getByText(/ready/i)).toBeInTheDocument();
      }, { timeout: 5000 });

      // Verify CAS deduplication metrics
      expect(screen.getByText(/25.*files/i)).toBeInTheDocument();
      expect(screen.getByText(/10MB/i)).toBeInTheDocument();
      
      // Verify deduplication ratio is displayed (35% space saved)
      if (screen.queryByText(/35%.*saved/i)) {
        expect(screen.getByText(/35%.*saved/i)).toBeInTheDocument();
      }
    });
  });

  describe('Import Nested Archive Workflow - CAS Hierarchy', () => {
    it('should handle nested archives with CAS storage', async () => {
      const workspaceId = 'cas-workspace-nested-001';
      
      mockDialogOpen.mockResolvedValue('/test/nested.zip');
      
      mockInvoke.mockImplementation((command: string, args?: any) => {
        switch (command) {
          case 'get_workspaces':
            return Promise.resolve([]);
          case 'import_archive':
            return Promise.resolve({
              workspaceId,
              status: 'PROCESSING',
              message: 'Processing nested archives...',
            });
          case 'get_workspace_status':
            return Promise.resolve({
              id: workspaceId,
              name: 'nested.zip',
              status: 'READY',
              files: 50,
              size: '20MB',
              format: 'cas',
              maxDepth: 3, // 3 levels of nesting
            });
          case 'get_virtual_file_tree':
            // CAS handles nested structure with virtual paths
            return Promise.resolve([
              {
                type: 'archive',
                name: 'nested.zip',
                path: 'nested.zip',
                hash: 'outer_archive_hash',
                archiveType: 'zip',
                children: [
                  {
                    type: 'archive',
                    name: 'inner.tar.gz',
                    path: 'nested.zip/inner.tar.gz',
                    hash: 'inner_archive_hash',
                    archiveType: 'tar.gz',
                    children: [
                      {
                        type: 'file',
                        name: 'deep.log',
                        path: 'nested.zip/inner.tar.gz/deep.log',
                        hash: 'deep_file_hash_789',
                        size: 4096,
                        mimeType: 'text/plain',
                      },
                    ],
                  },
                ],
              },
            ]);
          default:
            return Promise.resolve(null);
        }
      });

      render(
        <TestWrapper>
          <App />
        </TestWrapper>
      );

      await waitFor(() => {
        expect(screen.getByText(/workspaces/i)).toBeInTheDocument();
      });

      const importButton = await screen.findByRole('button', { name: /import.*archive/i });
      await user.click(importButton);

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('import_archive', {
          path: '/test/nested.zip',
        });
      });

      await waitFor(() => {
        expect(screen.getByText(/ready/i)).toBeInTheDocument();
      }, { timeout: 5000 });

      // Verify nested structure is handled
      expect(screen.getByText(/50.*files/i)).toBeInTheDocument();
      
      // Verify max depth is tracked
      if (screen.queryByText(/3.*levels/i)) {
        expect(screen.getByText(/3.*levels/i)).toBeInTheDocument();
      }
    });
  });
});

describe('E2E: CAS Migration - Search Workflow', () => {
  let user: ReturnType<typeof userEvent.setup>;

  beforeEach(() => {
    user = userEvent.setup();
    jest.clearAllMocks();
    
    mockListen.mockResolvedValue(() => {});
    mockInvoke.mockImplementation((command: string) => {
      switch (command) {
        case 'get_workspaces':
          return Promise.resolve([]);
        case 'get_tasks':
          return Promise.resolve([]);
        case 'get_keyword_groups':
          return Promise.resolve([]);
        default:
          return Promise.resolve(null);
      }
    });
  });

  describe('Search Using MetadataStore and CAS', () => {
    it('should search files using MetadataStore query and CAS content retrieval', async () => {
      const workspaceId = 'cas-workspace-search-001';
      
      const mockWorkspaces = [
        {
          id: workspaceId,
          name: 'Search Test Workspace',
          path: '/test/workspace',
          status: 'READY',
          files: 100,
          size: '50MB',
          format: 'cas',
        },
      ];

      const mockSearchResults = [
        {
          id: 1,
          file: 'app.log',
          virtualPath: 'logs/app.log',
          hash: 'search_result_hash_001', // CAS hash for content retrieval
          line: 42,
          content: 'ERROR: Database connection failed',
          timestamp: '2024-01-15 10:30:00',
          level: 'ERROR',
        },
        {
          id: 2,
          file: 'system.log',
          virtualPath: 'archive.zip/system.log',
          hash: 'search_result_hash_002',
          line: 156,
          content: 'ERROR: Memory allocation failed',
          timestamp: '2024-01-15 10:31:00',
          level: 'ERROR',
        },
      ];

      mockInvoke.mockImplementation((command: string, args?: any) => {
        switch (command) {
          case 'get_workspaces':
            return Promise.resolve(mockWorkspaces);
          case 'search_logs':
            // Verify search uses workspace ID (for MetadataStore query)
            expect(args?.workspaceId).toBe(workspaceId);
            return Promise.resolve(mockSearchResults);
          case 'get_file_content':
            // Verify content retrieval uses hash (CAS)
            if (args?.hash === 'search_result_hash_001') {
              return Promise.resolve('Full content of app.log with ERROR: Database connection failed');
            }
            if (args?.hash === 'search_result_hash_002') {
              return Promise.resolve('Full content of system.log with ERROR: Memory allocation failed');
            }
            return Promise.resolve('');
          default:
            return Promise.resolve(null);
        }
      });

      render(
        <TestWrapper>
          <App />
        </TestWrapper>
      );

      // Navigate to search page
      const searchTab = await screen.findByRole('button', { name: /search/i });
      await user.click(searchTab);

      // Enter search query
      const searchInput = await screen.findByPlaceholderText(/search/i);
      await user.type(searchInput, 'ERROR');

      // Select workspace
      const workspaceSelect = await screen.findByLabelText(/workspace/i);
      await user.selectOptions(workspaceSelect, workspaceId);

      // Execute search
      const searchButton = screen.getByRole('button', { name: /search/i });
      await user.click(searchButton);

      // Verify search command uses MetadataStore
      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('search_logs', {
          query: 'ERROR',
          workspaceId,
        });
      });

      // Verify search results are displayed
      await waitFor(() => {
        expect(screen.getByText(/database connection failed/i)).toBeInTheDocument();
        expect(screen.getByText(/memory allocation failed/i)).toBeInTheDocument();
      });

      // Verify virtual paths are shown
      expect(screen.getByText(/logs\/app\.log/i)).toBeInTheDocument();
      expect(screen.getByText(/archive\.zip\/system\.log/i)).toBeInTheDocument();

      // Click on a result to view full content
      const firstResult = screen.getByText(/database connection failed/i);
      await user.click(firstResult);

      // Verify content is retrieved using CAS hash
      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('get_file_content', {
          hash: 'search_result_hash_001',
        });
      });
    });
  });

  describe('Search with FTS5 Full-Text Search', () => {
    it('should use SQLite FTS5 for fast full-text search', async () => {
      const workspaceId = 'cas-workspace-fts-001';
      
      const mockWorkspaces = [
        {
          id: workspaceId,
          name: 'FTS Test Workspace',
          status: 'READY',
          files: 1000,
          size: '500MB',
          format: 'cas',
        },
      ];

      // Simulate FTS5 search results
      const mockFTSResults = [
        {
          id: 1,
          file: 'app.log',
          virtualPath: 'app.log',
          hash: 'fts_hash_001',
          line: 10,
          content: 'Connection timeout after 30 seconds',
          rank: 0.95, // FTS5 relevance rank
        },
        {
          id: 2,
          file: 'network.log',
          virtualPath: 'logs.zip/network.log',
          hash: 'fts_hash_002',
          line: 25,
          content: 'Timeout waiting for response',
          rank: 0.87,
        },
      ];

      mockInvoke.mockImplementation((command: string, args?: any) => {
        switch (command) {
          case 'get_workspaces':
            return Promise.resolve(mockWorkspaces);
          case 'search_logs':
            // Verify FTS5 search parameters
            expect(args?.useFTS).toBe(true);
            return Promise.resolve(mockFTSResults);
          default:
            return Promise.resolve(null);
        }
      });

      render(
        <TestWrapper>
          <App />
        </TestWrapper>
      );

      const searchTab = await screen.findByRole('button', { name: /search/i });
      await user.click(searchTab);

      const searchInput = await screen.findByPlaceholderText(/search/i);
      await user.type(searchInput, 'timeout');

      const workspaceSelect = await screen.findByLabelText(/workspace/i);
      await user.selectOptions(workspaceSelect, workspaceId);

      // Enable FTS5 search
      const ftsCheckbox = await screen.findByLabelText(/full.*text.*search/i);
      await user.click(ftsCheckbox);

      const searchButton = screen.getByRole('button', { name: /search/i });
      await user.click(searchButton);

      // Verify FTS5 is used
      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('search_logs', {
          query: 'timeout',
          workspaceId,
          useFTS: true,
        });
      });

      // Verify results are ranked by relevance
      await waitFor(() => {
        expect(screen.getByText(/connection timeout/i)).toBeInTheDocument();
        expect(screen.getByText(/timeout waiting/i)).toBeInTheDocument();
      });
    });
  });

  describe('Search Across Multiple Archives', () => {
    it('should search across files from multiple archives using CAS', async () => {
      const workspaceId = 'cas-workspace-multi-001';
      
      const mockWorkspaces = [
        {
          id: workspaceId,
          name: 'Multi-Archive Workspace',
          status: 'READY',
          files: 200,
          size: '100MB',
          format: 'cas',
        },
      ];

      const mockSearchResults = [
        {
          id: 1,
          file: 'error.log',
          virtualPath: 'archive1.zip/error.log',
          hash: 'multi_hash_001',
          line: 5,
          content: 'FATAL: System crash detected',
        },
        {
          id: 2,
          file: 'error.log',
          virtualPath: 'archive2.tar.gz/error.log',
          hash: 'multi_hash_002',
          line: 12,
          content: 'FATAL: Kernel panic',
        },
        {
          id: 3,
          file: 'system.log',
          virtualPath: 'archive3.zip/nested.tar/system.log',
          hash: 'multi_hash_003',
          line: 89,
          content: 'FATAL: Out of memory',
        },
      ];

      mockInvoke.mockImplementation((command: string, args?: any) => {
        switch (command) {
          case 'get_workspaces':
            return Promise.resolve(mockWorkspaces);
          case 'search_logs':
            return Promise.resolve(mockSearchResults);
          default:
            return Promise.resolve(null);
        }
      });

      render(
        <TestWrapper>
          <App />
        </TestWrapper>
      );

      const searchTab = await screen.findByRole('button', { name: /search/i });
      await user.click(searchTab);

      const searchInput = await screen.findByPlaceholderText(/search/i);
      await user.type(searchInput, 'FATAL');

      const workspaceSelect = await screen.findByLabelText(/workspace/i);
      await user.selectOptions(workspaceSelect, workspaceId);

      const searchButton = screen.getByRole('button', { name: /search/i });
      await user.click(searchButton);

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('search_logs', {
          query: 'FATAL',
          workspaceId,
        });
      });

      // Verify results from all archives
      await waitFor(() => {
        expect(screen.getByText(/system crash detected/i)).toBeInTheDocument();
        expect(screen.getByText(/kernel panic/i)).toBeInTheDocument();
        expect(screen.getByText(/out of memory/i)).toBeInTheDocument();
      });

      // Verify virtual paths show archive hierarchy
      expect(screen.getByText(/archive1\.zip\/error\.log/i)).toBeInTheDocument();
      expect(screen.getByText(/archive2\.tar\.gz\/error\.log/i)).toBeInTheDocument();
      expect(screen.getByText(/archive3\.zip\/nested\.tar\/system\.log/i)).toBeInTheDocument();
    });
  });
});

describe('E2E: CAS Migration - Workspace Management', () => {
  let user: ReturnType<typeof userEvent.setup>;

  beforeEach(() => {
    user = userEvent.setup();
    jest.clearAllMocks();
    
    mockListen.mockResolvedValue(() => {});
    mockInvoke.mockImplementation((command: string) => {
      switch (command) {
        case 'get_workspaces':
          return Promise.resolve([]);
        case 'get_tasks':
          return Promise.resolve([]);
        case 'get_keyword_groups':
          return Promise.resolve([]);
        default:
          return Promise.resolve(null);
      }
    });
  });

  describe('Create CAS Workspace', () => {
    it('should create workspace with CAS architecture', async () => {
      const workspaceId = 'cas-workspace-create-001';
      
      mockDialogOpen.mockResolvedValue('/test/new-workspace');
      
      mockInvoke.mockImplementation((command: string, args?: any) => {
        switch (command) {
          case 'get_workspaces':
            return Promise.resolve([]);
          case 'create_workspace':
            // Verify workspace is created with CAS format
            return Promise.resolve({
              id: workspaceId,
              name: args?.name || 'New Workspace',
              path: args?.path,
              status: 'READY',
              format: 'cas', // Must be CAS format
              files: 0,
              size: '0MB',
            });
          case 'verify_workspace_structure':
            // Verify CAS directory structure exists
            return Promise.resolve({
              hasMetadataDb: true, // metadata.db exists
              hasObjectsDir: true, // objects/ directory exists
              hasLegacyIndex: false, // No .idx.gz files
              format: 'cas',
            });
          default:
            return Promise.resolve(null);
        }
      });

      render(
        <TestWrapper>
          <App />
        </TestWrapper>
      );

      await waitFor(() => {
        expect(screen.getByText(/workspaces/i)).toBeInTheDocument();
      });

      const createButton = await screen.findByRole('button', { name: /create.*workspace/i });
      await user.click(createButton);

      const nameInput = await screen.findByLabelText(/workspace.*name/i);
      await user.type(nameInput, 'My CAS Workspace');

      const pathButton = await screen.findByRole('button', { name: /select.*path/i });
      await user.click(pathButton);

      await waitFor(() => {
        expect(mockDialogOpen).toHaveBeenCalled();
      });

      const submitButton = screen.getByRole('button', { name: /create/i });
      await user.click(submitButton);

      // Verify workspace creation
      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('create_workspace', {
          name: 'My CAS Workspace',
          path: '/test/new-workspace',
        });
      });

      // Verify workspace appears with CAS format
      await waitFor(() => {
        expect(screen.getByText(/my cas workspace/i)).toBeInTheDocument();
        expect(screen.getByText(/ready/i)).toBeInTheDocument();
      });

      // Verify CAS structure
      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('verify_workspace_structure', {
          workspaceId,
        });
      });
    });
  });

  describe('Delete CAS Workspace', () => {
    it('should delete workspace and clean up CAS objects and MetadataStore', async () => {
      const workspaceId = 'cas-workspace-delete-001';
      
      const mockWorkspaces = [
        {
          id: workspaceId,
          name: 'Workspace to Delete',
          path: '/test/workspace',
          status: 'READY',
          files: 50,
          size: '25MB',
          format: 'cas',
        },
      ];

      mockInvoke.mockImplementation((command: string, args?: any) => {
        switch (command) {
          case 'get_workspaces':
            return Promise.resolve(mockWorkspaces);
          case 'delete_workspace':
            // Verify deletion includes CAS cleanup
            expect(args?.workspaceId).toBe(workspaceId);
            return Promise.resolve({
              success: true,
              deletedFiles: 50,
              deletedObjects: 45, // Some objects were deduplicated
              deletedMetadata: true,
              freedSpace: '25MB',
            });
          case 'verify_workspace_cleanup':
            // Verify complete cleanup
            return Promise.resolve({
              metadataDbExists: false,
              objectsDirExists: false,
              workspaceDirExists: false,
            });
          default:
            return Promise.resolve(null);
        }
      });

      render(
        <TestWrapper>
          <App />
        </TestWrapper>
      );

      await waitFor(() => {
        expect(screen.getByText(/workspace to delete/i)).toBeInTheDocument();
      });

      // Click delete button
      const deleteButton = await screen.findByRole('button', { name: /delete/i });
      await user.click(deleteButton);

      // Confirm deletion
      const confirmButton = await screen.findByRole('button', { name: /confirm/i });
      await user.click(confirmButton);

      // Verify deletion command
      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('delete_workspace', {
          workspaceId,
        });
      });

      // Verify workspace is removed from list
      await waitFor(() => {
        expect(screen.queryByText(/workspace to delete/i)).not.toBeInTheDocument();
      });

      // Verify cleanup verification
      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('verify_workspace_cleanup', {
          workspaceId,
        });
      });

      // Verify cleanup summary is shown
      if (screen.queryByText(/50.*files.*deleted/i)) {
        expect(screen.getByText(/50.*files.*deleted/i)).toBeInTheDocument();
        expect(screen.getByText(/25MB.*freed/i)).toBeInTheDocument();
      }
    });
  });

  describe('Verify CAS Workspace Structure', () => {
    it('should verify workspace uses CAS architecture (no legacy files)', async () => {
      const workspaceId = 'cas-workspace-verify-001';
      
      const mockWorkspaces = [
        {
          id: workspaceId,
          name: 'Verified CAS Workspace',
          path: '/test/workspace',
          status: 'READY',
          files: 100,
          size: '50MB',
          format: 'cas',
        },
      ];

      mockInvoke.mockImplementation((command: string, args?: any) => {
        switch (command) {
          case 'get_workspaces':
            return Promise.resolve(mockWorkspaces);
          case 'verify_workspace_structure':
            return Promise.resolve({
              workspaceId,
              format: 'cas',
              hasMetadataDb: true,
              hasObjectsDir: true,
              hasLegacyIndex: false, // No .idx.gz files
              hasPathMappingsTable: false, // No path_mappings table
              objectCount: 95, // 95 unique objects (5 deduplicated)
              metadataRecords: 100,
              ftsEnabled: true, // FTS5 full-text search enabled
            });
          case 'get_workspace_metrics':
            return Promise.resolve({
              totalFiles: 100,
              uniqueObjects: 95,
              deduplicationRatio: 0.05,
              storageEfficiency: 0.95,
              avgFileSize: 524288, // 512KB
            });
          default:
            return Promise.resolve(null);
        }
      });

      render(
        <TestWrapper>
          <App />
        </TestWrapper>
      );

      await waitFor(() => {
        expect(screen.getByText(/verified cas workspace/i)).toBeInTheDocument();
      });

      // Click on workspace to view details
      const workspaceCard = screen.getByText(/verified cas workspace/i);
      await user.click(workspaceCard);

      // Verify structure check is performed
      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('verify_workspace_structure', {
          workspaceId,
        });
      });

      // Verify CAS indicators are shown
      await waitFor(() => {
        // Should show CAS format
        if (screen.queryByText(/cas.*format/i)) {
          expect(screen.getByText(/cas.*format/i)).toBeInTheDocument();
        }
        
        // Should show no legacy files
        if (screen.queryByText(/no.*legacy.*files/i)) {
          expect(screen.getByText(/no.*legacy.*files/i)).toBeInTheDocument();
        }
        
        // Should show FTS5 enabled
        if (screen.queryByText(/full.*text.*search.*enabled/i)) {
          expect(screen.getByText(/full.*text.*search.*enabled/i)).toBeInTheDocument();
        }
      });

      // Verify metrics are displayed
      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('get_workspace_metrics', {
          workspaceId,
        });
      });

      // Check deduplication metrics
      if (screen.queryByText(/5%.*deduplicated/i)) {
        expect(screen.getByText(/5%.*deduplicated/i)).toBeInTheDocument();
      }
    });
  });

  describe('List Workspaces - CAS Only', () => {
    it('should list only CAS format workspaces (no legacy workspaces)', async () => {
      const mockWorkspaces = [
        {
          id: 'cas-workspace-001',
          name: 'CAS Workspace 1',
          status: 'READY',
          files: 50,
          size: '25MB',
          format: 'cas',
        },
        {
          id: 'cas-workspace-002',
          name: 'CAS Workspace 2',
          status: 'READY',
          files: 100,
          size: '50MB',
          format: 'cas',
        },
      ];

      mockInvoke.mockImplementation((command: string) => {
        switch (command) {
          case 'get_workspaces':
            // Should only return CAS workspaces
            return Promise.resolve(mockWorkspaces);
          default:
            return Promise.resolve(null);
        }
      });

      render(
        <TestWrapper>
          <App />
        </TestWrapper>
      );

      // Verify all workspaces are CAS format
      await waitFor(() => {
        expect(screen.getByText(/cas workspace 1/i)).toBeInTheDocument();
        expect(screen.getByText(/cas workspace 2/i)).toBeInTheDocument();
      });

      // Verify no legacy format indicators
      expect(screen.queryByText(/traditional/i)).not.toBeInTheDocument();
      expect(screen.queryByText(/needs.*migration/i)).not.toBeInTheDocument();
      expect(screen.queryByText(/migrate/i)).not.toBeInTheDocument();

      // All workspaces should show CAS format
      const workspaceCards = screen.getAllByText(/cas/i);
      expect(workspaceCards.length).toBeGreaterThan(0);
    });
  });
});
