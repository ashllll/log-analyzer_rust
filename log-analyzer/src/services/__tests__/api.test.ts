import { api } from '../api';

jest.mock('@tauri-apps/api/core', () => ({
  invoke: jest.fn(),
}));

const mockInvoke = require('@tauri-apps/api/core').invoke as jest.Mock;

const makeConfig = (workspacePath: string) => ({
  keyword_groups: [],
  workspaces: [
    {
      id: 'workspace-1',
      name: 'Test Workspace',
      path: workspacePath,
      status: 'READY' as const,
      size: '100MB',
      files: 10,
    },
  ],
  file_filter: {
    enabled: false,
    binary_detection_enabled: true,
    mode: 'blacklist' as const,
    filename_patterns: [],
    allowed_extensions: [],
    forbidden_extensions: [],
  },
});

describe('api.refreshWorkspace', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('should invoke refresh_workspace directly when path is provided', async () => {
    mockInvoke.mockResolvedValueOnce('550e8400-e29b-41d4-a716-446655440000');

    await expect(api.refreshWorkspace('workspace-1', '/logs/app')).resolves.toBe('550e8400-e29b-41d4-a716-446655440000');

    expect(mockInvoke).toHaveBeenCalledTimes(1);
    expect(mockInvoke).toHaveBeenCalledWith('refresh_workspace', {
      workspaceId: 'workspace-1',
      path: '/logs/app',
    });
  });

  it('should load config and recover path when it is missing', async () => {
    mockInvoke
      .mockResolvedValueOnce(makeConfig('/logs/from-config'))
      .mockResolvedValueOnce('550e8400-e29b-41d4-a716-446655440001');

    await expect(api.refreshWorkspace('workspace-1')).resolves.toBe('550e8400-e29b-41d4-a716-446655440001');

    expect(mockInvoke).toHaveBeenNthCalledWith(1, 'load_config');
    expect(mockInvoke).toHaveBeenNthCalledWith(2, 'refresh_workspace', {
      workspaceId: 'workspace-1',
      path: '/logs/from-config',
    });
  });
});

describe('api.loadWorkspace', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('should accept the backend load_workspace payload shape', async () => {
    mockInvoke.mockResolvedValueOnce({
      success: true,
      fileCount: 42,
    });

    await expect(api.loadWorkspace('workspace-1')).resolves.toEqual({
      success: true,
      fileCount: 42,
    });

    expect(mockInvoke).toHaveBeenCalledWith('load_workspace', {
      workspaceId: 'workspace-1',
    });
  });

  it('should reject malformed workspace payloads', async () => {
    mockInvoke.mockResolvedValueOnce({
      success: true,
    });

    await expect(api.loadWorkspace('workspace-1')).rejects.toThrow();
  });
});

describe('api.getWorkspaceStatus', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('should accept valid workspace statuses from the backend', async () => {
    mockInvoke.mockResolvedValueOnce({
      id: 'workspace-1',
      name: 'Test Workspace',
      status: 'ERROR',
      size: '100MB',
      files: 10,
    });

    await expect(api.getWorkspaceStatus('workspace-1')).resolves.toEqual({
      id: 'workspace-1',
      name: 'Test Workspace',
      status: 'ERROR',
      size: '100MB',
      files: 10,
    });
  });

  it('should reject unexpected workspace statuses', async () => {
    mockInvoke.mockResolvedValueOnce({
      id: 'workspace-1',
      name: 'Test Workspace',
      status: 'BROKEN',
      size: '100MB',
      files: 10,
    });

    await expect(api.getWorkspaceStatus('workspace-1')).rejects.toThrow();
  });
});
