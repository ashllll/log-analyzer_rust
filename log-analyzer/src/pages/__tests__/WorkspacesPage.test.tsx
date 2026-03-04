/**
 * WorkspacesPage 单元测试
 *
 * 测试工作区管理页面的核心功能:
 * 1. 显示工作区列表
 * 2. 导入文件/文件夹
 * 3. 刷新、删除、切换监听状态
 * 4. 切换活跃工作区
 */

// 设置所有 mock，必须在 import 之前完成
jest.mock('lucide-react', () => ({
  FileText: () => null,
  Plus: () => null,
  RefreshCw: () => null,
  Trash2: () => null,
  Eye: () => null,
  EyeOff: () => null,
  CheckCircle2: () => null,
  Settings: () => null,
}));

jest.mock('../../components/ui', () => ({
  Button: ({ children, onClick, 'data-testid': dataTestId }: any) => (
    <button onClick={onClick} data-testid={dataTestId || 'button'}>
      {children}
    </button>
  ),
  Card: ({ children, className, onClick, 'data-testid': dataTestId }: any) => (
    <div onClick={onClick} className={className} data-testid={dataTestId}>
      {children}
    </div>
  ),
}));

jest.mock('../../components/modals/FileFilterSettings', () => ({
  FileFilterSettings: ({ isOpen, onClose }: { isOpen: boolean; onClose: () => void }) => (
    isOpen ? (
      <div data-testid="file-filter-settings-modal">
        <button onClick={onClose}>Close</button>
      </div>
    ) : null
  ),
}));

jest.mock('../../services/api', () => ({
  api: {
    importFolder: jest.fn(),
    refreshWorkspace: jest.fn(),
    deleteWorkspace: jest.fn(),
  },
}));

jest.mock('../../utils/logger', () => ({
  logger: {
    debug: jest.fn(),
    error: jest.fn(),
    info: jest.fn(),
    warn: jest.fn(),
  },
}));

import React from 'react';
import { render, screen, fireEvent, waitFor, act } from '@testing-library/react';
import { useWorkspaceStore, Workspace } from '../../stores/workspaceStore';
import { useAppStore } from '../../stores/appStore';
import WorkspacesPage from '../WorkspacesPage';

const mockApi = require('../../services/api').api;

describe('WorkspacesPage', () => {
  const mockWorkspaces: Workspace[] = [
    {
      id: 'ws-1',
      name: 'Test Workspace 1',
      path: '/path/to/workspace1',
      status: 'READY',
      size: '100MB',
      files: 100,
      watching: false,
    },
    {
      id: 'ws-2',
      name: 'Test Workspace 2',
      path: '/path/to/workspace2',
      status: 'PROCESSING',
      size: '50MB',
      files: 50,
      watching: true,
    },
  ];

  beforeEach(() => {
    jest.clearAllMocks();
    mockApi.importFolder.mockResolvedValue('task-1');
    mockApi.refreshWorkspace.mockResolvedValue(undefined);
    mockApi.deleteWorkspace.mockResolvedValue(undefined);

    // 使用 setState 设置 Zustand store 状态
    act(() => {
      useWorkspaceStore.setState({
        workspaces: mockWorkspaces,
        loading: false,
        error: null,
      });
      useAppStore.setState({
        activeWorkspaceId: 'ws-1',
        page: 'workspaces',
        toasts: [],
        isInitialized: true,
        initializationError: null,
      });
    });
  });

  describe('初始渲染', () => {
    it('应该渲染页面标题', () => {
      render(<WorkspacesPage />);
      expect(screen.getByText('Workspaces')).toBeInTheDocument();
    });

    it('应该渲染所有工作区卡片', () => {
      render(<WorkspacesPage />);
      expect(screen.getByTestId('workspace-card-ws-1')).toBeInTheDocument();
      expect(screen.getByTestId('workspace-card-ws-2')).toBeInTheDocument();
    });

    it('应该显示工作区名称', () => {
      render(<WorkspacesPage />);
      expect(screen.getByText('Test Workspace 1')).toBeInTheDocument();
      expect(screen.getByText('Test Workspace 2')).toBeInTheDocument();
    });

    it('应该显示工作区路径', () => {
      render(<WorkspacesPage />);
      expect(screen.getByText('/path/to/workspace1')).toBeInTheDocument();
      expect(screen.getByText('/path/to/workspace2')).toBeInTheDocument();
    });

    it('应该显示操作按钮', () => {
      render(<WorkspacesPage />);
      expect(screen.getByTestId('file-filter-settings-button')).toBeInTheDocument();
      expect(screen.getByTestId('import-file-button')).toBeInTheDocument();
      expect(screen.getByTestId('import-folder-button')).toBeInTheDocument();
    });

    it('应该高亮活跃工作区', () => {
      render(<WorkspacesPage />);

      const activeCard = screen.getByTestId('workspace-card-ws-1');
      expect(activeCard).toHaveClass('border-primary');
      expect(activeCard).toHaveClass('ring-1');
    });
  });

  describe('工作区操作', () => {
    it('点击卡片应该切换工作区', async () => {
      render(<WorkspacesPage />);

      const card = screen.getByTestId('workspace-card-ws-1');
      fireEvent.click(card);

      await waitFor(() => {
        expect(useAppStore.getState().activeWorkspaceId).toBe('ws-1');
      });
    });

    it('点击刷新按钮应该调用 API', async () => {
      render(<WorkspacesPage />);

      const refreshButton = screen.getByTestId('workspace-refresh-ws-1');
      fireEvent.click(refreshButton);

      await waitFor(() => {
        expect(mockApi.refreshWorkspace).toHaveBeenCalledWith({
          workspaceId: 'ws-1',
        });
      });
    });

    it('点击删除按钮应该调用 API', async () => {
      render(<WorkspacesPage />);

      const deleteButton = screen.getByTestId('workspace-delete-ws-1');
      fireEvent.click(deleteButton);

      await waitFor(() => {
        expect(mockApi.deleteWorkspace).toHaveBeenCalledWith('ws-1');
      });
    });
  });

  describe('空状态', () => {
    it('没有工作区时应该显示空状态', () => {
      act(() => {
        useWorkspaceStore.setState({
          workspaces: [],
          loading: false,
          error: null,
        });
      });

      render(<WorkspacesPage />);

      expect(screen.queryByTestId('workspace-card-ws-1')).not.toBeInTheDocument();
      expect(screen.queryByTestId('workspace-card-ws-2')).not.toBeInTheDocument();
    });
  });
});
