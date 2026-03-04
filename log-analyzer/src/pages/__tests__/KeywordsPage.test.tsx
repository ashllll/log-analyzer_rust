/**
 * KeywordsPage 单元测试
 *
 * 测试关键词配置页面的核心功能:
 * 1. 显示关键词组列表
 * 2. 新建关键词组
 * 3. 编辑关键词组
 * 4. 删除关键词组
 */

import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { useKeywordStore, KeywordGroup } from '../../stores/keywordStore';
import KeywordsPage from '../KeywordsPage';

// Mock lucide-react icons
jest.mock('lucide-react', () => ({
  Plus: () => <span data-testid="icon-plus" />,
  Edit2: () => <span data-testid="icon-edit" />,
  Trash2: () => <span data-testid="icon-trash" />,
}));

// Mock hooks
jest.mock('../../hooks/useKeywordManager', () => ({
  useKeywordManager: jest.fn(),
}));

// Mock store
jest.mock('../../stores/keywordStore');

// Mock modals
jest.mock('../../components/modals/KeywordModal', () => ({
  KeywordModal: ({
    isOpen,
    onClose,
    onSave,
    initialData
  }: {
    isOpen: boolean;
    onClose: () => void;
    onSave: (group: KeywordGroup) => void;
    initialData: KeywordGroup | null;
  }) => (
    isOpen ? (
      <div data-testid="keyword-modal">
        <div data-testid="initial-data">
          {initialData ? JSON.stringify(initialData) : 'null'}
        </div>
        <button
          onClick={() =>
            onSave({
              id: initialData?.id || 'new-1',
              name: 'Test Group',
              color: 'blue',
              patterns: [{ regex: 'test', comment: '' }],
              enabled: true,
            })
          }
        >
          Save
        </button>
        <button onClick={onClose}>Cancel</button>
      </div>
    ) : null
  ),
}));

// Mock UI components
jest.mock('../../components/ui', () => ({
  Button: ({ children, onClick, className, variant }: any) => (
    <button onClick={onClick} className={className} data-variant={variant}>
      {children}
    </button>
  ),
  Card: ({ children, className }: any) => (
    <div className={className} data-testid="keyword-card">
      {children}
    </div>
  ),
}));

// Mock constants
jest.mock('../../constants/colors', () => ({
  COLOR_STYLES: {
    blue: { dot: 'bg-blue-500', badge: 'bg-blue-100 text-blue-700' },
    green: { dot: 'bg-green-500', badge: 'bg-green-100 text-green-700' },
    red: { dot: 'bg-red-500', badge: 'bg-red-100 text-red-700' },
    orange: { dot: 'bg-orange-500', badge: 'bg-orange-100 text-orange-700' },
    purple: { dot: 'bg-purple-500', badge: 'bg-purple-100 text-purple-700' },
  },
}));

const mockUseKeywordManager = require('../../hooks/useKeywordManager').useKeywordManager;

describe('KeywordsPage', () => {
  const mockKeywordGroups: KeywordGroup[] = [
    {
      id: 'group-1',
      name: 'Error Patterns',
      color: 'red',
      patterns: [
        { regex: 'error', comment: 'Error message' },
        { regex: 'fatal', comment: 'Fatal error' },
      ],
      enabled: true,
    },
    {
      id: 'group-2',
      name: 'Warning Patterns',
      color: 'orange',
      patterns: [
        { regex: 'warning', comment: '' },
        { regex: 'deprecated', comment: 'Deprecated feature' },
      ],
      enabled: true,
    },
    {
      id: 'group-3',
      name: 'Debug Patterns',
      color: 'blue',
      patterns: [{ regex: 'debug', comment: 'Debug info' }],
      enabled: false,
    },
  ];

  const mockManager = {
    keywordGroups: mockKeywordGroups,
    saveKeywordGroup: jest.fn(),
    deleteKeywordGroup: jest.fn(),
    addKeywordGroup: jest.fn(),
    updateKeywordGroup: jest.fn(),
    toggleKeywordGroup: jest.fn(),
  };

  beforeEach(() => {
    jest.clearAllMocks();

    // Setup store mock
    (useKeywordStore as unknown as jest.Mock).mockReturnValue({
      keywordGroups: mockKeywordGroups,
    });

    // Setup manager mock
    mockUseKeywordManager.mockReturnValue(mockManager);
  });

  describe('初始渲染', () => {
    it('应该渲染页面标题', () => {
      render(<KeywordsPage />);

      expect(screen.getByText('Keyword Configuration')).toBeInTheDocument();
    });

    it('应该渲染所有关键词组卡片', () => {
      render(<KeywordsPage />);

      const cards = screen.getAllByTestId('keyword-card');
      expect(cards).toHaveLength(3);
    });

    it('应该显示关键词组名称', () => {
      render(<KeywordsPage />);

      expect(screen.getByText('Error Patterns')).toBeInTheDocument();
      expect(screen.getByText('Warning Patterns')).toBeInTheDocument();
      expect(screen.getByText('Debug Patterns')).toBeInTheDocument();
    });

    it('应该显示 New Group 按钮', () => {
      render(<KeywordsPage />);

      expect(screen.getByText('New Group')).toBeInTheDocument();
    });
  });

  describe('关键词组内容显示', () => {
    it('应该显示所有关键词模式', () => {
      render(<KeywordsPage />);

      // Error Patterns group
      expect(screen.getByText('error')).toBeInTheDocument();
      expect(screen.getByText('fatal')).toBeInTheDocument();

      // Warning Patterns group
      expect(screen.getByText('warning')).toBeInTheDocument();
      expect(screen.getByText('deprecated')).toBeInTheDocument();

      // Debug Patterns group
      expect(screen.getByText('debug')).toBeInTheDocument();
    });

    it('应该显示模式注释', () => {
      render(<KeywordsPage />);

      expect(screen.getByText('Error message')).toBeInTheDocument();
      expect(screen.getByText('Fatal error')).toBeInTheDocument();
      expect(screen.getByText('Deprecated feature')).toBeInTheDocument();
      expect(screen.getByText('Debug info')).toBeInTheDocument();
    });

    it('应该显示颜色指示器', () => {
      const { container } = render(<KeywordsPage />);

      const dots = container.querySelectorAll('.rounded-full');
      expect(dots.length).toBeGreaterThanOrEqual(3);
    });
  });

  describe('新建关键词组', () => {
    it('点击 New Group 应该打开模态框', () => {
      render(<KeywordsPage />);

      const newGroupButton = screen.getByText('New Group');
      fireEvent.click(newGroupButton);

      expect(screen.getByTestId('keyword-modal')).toBeInTheDocument();
    });

    it('新建时初始数据应该为 null', () => {
      render(<KeywordsPage />);

      const newGroupButton = screen.getByText('New Group');
      fireEvent.click(newGroupButton);

      expect(screen.getByTestId('initial-data')).toHaveTextContent('null');
    });

    it('保存时应该调用 saveKeywordGroup', async () => {
      render(<KeywordsPage />);

      // 打开模态框
      fireEvent.click(screen.getByText('New Group'));

      // 点击保存
      const saveButton = screen.getByText('Save');
      fireEvent.click(saveButton);

      await waitFor(() => {
        expect(mockManager.saveKeywordGroup).toHaveBeenCalledWith(
          expect.objectContaining({
            id: 'new-1',
            name: 'Test Group',
          }),
          false // isEditing = false
        );
      });
    });

    it('点击 Cancel 应该关闭模态框', () => {
      render(<KeywordsPage />);

      fireEvent.click(screen.getByText('New Group'));
      expect(screen.getByTestId('keyword-modal')).toBeInTheDocument();

      fireEvent.click(screen.getByText('Cancel'));
      expect(screen.queryByTestId('keyword-modal')).not.toBeInTheDocument();
    });
  });

  describe('编辑关键词组', () => {
    it('点击 Edit 应该打开模态框', () => {
      render(<KeywordsPage />);

      const editButtons = screen.getAllByText('Edit');
      fireEvent.click(editButtons[0]);

      expect(screen.getByTestId('keyword-modal')).toBeInTheDocument();
    });

    it('编辑时应该加载初始数据', () => {
      render(<KeywordsPage />);

      const editButtons = screen.getAllByText('Edit');
      fireEvent.click(editButtons[0]);

      const initialData = screen.getByTestId('initial-data');
      expect(initialData.textContent).toContain('Error Patterns');
      expect(initialData.textContent).toContain('group-1');
    });

    it('保存时应该调用 saveKeywordGroup 并传入 isEditing=true', async () => {
      render(<KeywordsPage />);

      // 点击编辑第一个组
      const editButtons = screen.getAllByText('Edit');
      fireEvent.click(editButtons[0]);

      // 点击保存
      fireEvent.click(screen.getByText('Save'));

      await waitFor(() => {
        expect(mockManager.saveKeywordGroup).toHaveBeenCalledWith(
          expect.objectContaining({
            name: 'Test Group',
          }),
          true // isEditing = true
        );
      });
    });
  });

  describe('删除关键词组', () => {
    it('点击 Delete 应该调用 deleteKeywordGroup', async () => {
      render(<KeywordsPage />);

      const deleteButtons = screen.getAllByText('Delete');
      fireEvent.click(deleteButtons[0]);

      await waitFor(() => {
        expect(mockManager.deleteKeywordGroup).toHaveBeenCalledWith('group-1');
      });
    });

    it('应该删除对应的关键词组', async () => {
      render(<KeywordsPage />);

      expect(screen.getByText('Error Patterns')).toBeInTheDocument();

      const deleteButtons = screen.getAllByText('Delete');
      fireEvent.click(deleteButtons[0]);

      await waitFor(() => {
        expect(mockManager.deleteKeywordGroup).toHaveBeenCalledWith('group-1');
      });
    });
  });

  describe('空状态', () => {
    it('没有关键词组时应该显示空状态', () => {
      mockUseKeywordManager.mockReturnValue({
        ...mockManager,
        keywordGroups: [],
      });

      render(<KeywordsPage />);

      expect(screen.queryByTestId('keyword-card')).not.toBeInTheDocument();
    });

    it('空状态下点击 New Group 应该能正常工作', () => {
      mockUseKeywordManager.mockReturnValue({
        ...mockManager,
        keywordGroups: [],
      });

      render(<KeywordsPage />);

      const newGroupButton = screen.getByText('New Group');
      expect(newGroupButton).toBeInTheDocument();

      fireEvent.click(newGroupButton);
      expect(screen.getByTestId('keyword-modal')).toBeInTheDocument();
    });
  });

  describe('边界条件', () => {
    it('应该处理没有注释的模式', () => {
      render(<KeywordsPage />);

      // 'warning' 模式没有注释
      expect(screen.getByText('warning')).toBeInTheDocument();
    });

    it('应该处理空模式数组的关键词组', () => {
      const emptyGroup: KeywordGroup = {
        id: 'empty-1',
        name: 'Empty Group',
        color: 'blue',
        patterns: [],
        enabled: true,
      };

      mockUseKeywordManager.mockReturnValue({
        ...mockManager,
        keywordGroups: [...mockKeywordGroups, emptyGroup],
      });

      render(<KeywordsPage />);

      expect(screen.getByText('Empty Group')).toBeInTheDocument();
    });

    it('应该处理多个模式的关键词组', () => {
      const manyPatternsGroup: KeywordGroup = {
        id: 'many-1',
        name: 'Many Patterns',
        color: 'purple',
        patterns: [
          { regex: 'pattern1', comment: 'Comment 1' },
          { regex: 'pattern2', comment: 'Comment 2' },
          { regex: 'pattern3', comment: 'Comment 3' },
          { regex: 'pattern4', comment: 'Comment 4' },
          { regex: 'pattern5', comment: 'Comment 5' },
        ],
        enabled: true,
      };

      mockUseKeywordManager.mockReturnValue({
        ...mockManager,
        keywordGroups: [...mockKeywordGroups, manyPatternsGroup],
      });

      render(<KeywordsPage />);

      expect(screen.getByText('Many Patterns')).toBeInTheDocument();
      expect(screen.getByText('pattern1')).toBeInTheDocument();
      expect(screen.getByText('pattern5')).toBeInTheDocument();
    });

    it('应该处理所有颜色类型', () => {
      const colorGroups: KeywordGroup[] = ['blue', 'green', 'red', 'orange', 'purple'].map(
        (color, index) => ({
          id: `color-${index}`,
          name: `Color ${color}`,
          color,
          patterns: [{ regex: `test-${color}`, comment: '' }],
          enabled: true,
        })
      );

      mockUseKeywordManager.mockReturnValue({
        ...mockManager,
        keywordGroups: colorGroups,
      });

      render(<KeywordsPage />);

      expect(screen.getByText('Color blue')).toBeInTheDocument();
      expect(screen.getByText('Color purple')).toBeInTheDocument();
    });
  });

  describe('交互行为', () => {
    it('模态框应该保持打开直到保存或取消', () => {
      render(<KeywordsPage />);

      fireEvent.click(screen.getByText('New Group'));
      expect(screen.getByTestId('keyword-modal')).toBeInTheDocument();

      // 触发其他操作不应该关闭模态框
      const newGroupButton = screen.getByText('New Group');
      fireEvent.click(newGroupButton);

      // 模态框应该仍然存在
      expect(screen.getByTestId('keyword-modal')).toBeInTheDocument();
    });

    it('编辑不同的组应该更新模态框的初始数据', () => {
      render(<KeywordsPage />);

      // 编辑第一个组
      const editButtons = screen.getAllByText('Edit');
      fireEvent.click(editButtons[0]);

      let initialData = screen.getByTestId('initial-data');
      expect(initialData.textContent).toContain('Error Patterns');

      // 关闭模态框
      fireEvent.click(screen.getByText('Cancel'));

      // 编辑第二个组
      fireEvent.click(editButtons[1]);

      initialData = screen.getByTestId('initial-data');
      expect(initialData.textContent).toContain('Warning Patterns');
    });
  });

  describe('UI 组件集成', () => {
    it('应该使用正确的样式类', () => {
      const { container } = render(<KeywordsPage />);

      // 检查页面容器
      const pageContainer = container.querySelector('.max-w-6xl');
      expect(pageContainer).toBeInTheDocument();
    });

    it('应该渲染所有图标', () => {
      render(<KeywordsPage />);

      // New Group, Edit (x2), Delete (x2)
      expect(screen.getAllByTestId('icon-plus').length).toBeGreaterThanOrEqual(1);
      expect(screen.getAllByTestId('icon-edit').length).toBeGreaterThanOrEqual(2);
      expect(screen.getAllByTestId('icon-trash').length).toBeGreaterThanOrEqual(2);
    });
  });
});
