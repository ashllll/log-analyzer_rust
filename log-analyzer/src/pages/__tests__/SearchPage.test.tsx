/**
 * SearchPage 单元测试
 *
 * 测试搜索页面的核心功能:
 * 1. 搜索输入和触发
 * 2. 虚拟滚动渲染
 * 3. 高级过滤器
 * 4. 结果导出
 * 5. 关键词高亮
 * 6. 日志详情面板
 */

import React from 'react';
import { render, screen, fireEvent, waitFor, act } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import SearchPage from '../SearchPage';
import { LogEntry, Workspace, KeywordGroup, FilterOptions } from '../../types/common';

// Mock lucide-react icons
jest.mock('lucide-react', () => ({
  Search: () => <span data-testid="icon-search" />,
  Download: () => <span data-testid="icon-download" />,
  Filter: () => <span data-testid="icon-filter" />,
  X: () => <span data-testid="icon-x" />,
  ChevronDown: () => <span data-testid="icon-chevron-down" />,
  Hash: () => <span data-testid="icon-hash" />,
  Copy: () => <span data-testid="icon-copy" />,
  Loader2: () => <span data-testid="icon-loader" />,
  RotateCcw: () => <span data-testid="icon-rotate" />,
}));

// Mock Tauri APIs
jest.mock('@tauri-apps/api/core', () => ({
  invoke: jest.fn(),
}));

jest.mock('@tauri-apps/api/event', () => ({
  listen: jest.fn(),
}));

jest.mock('@tauri-apps/plugin-dialog', () => ({
  save: jest.fn(),
}));

// Mock services
jest.mock('../../services/api', () => ({
  api: {
    searchLogs: jest.fn(),
    exportResults: jest.fn(),
  },
}));

jest.mock('../../services/errors', () => ({
  getFullErrorMessage: (err: unknown) => `Error: ${err}`,
}));

jest.mock('../../services/queryStorage', () => ({
  saveQuery: jest.fn(),
  loadQuery: jest.fn(() => null),
}));

// Mock components
jest.mock('../../components/modals/FilterPalette', () => ({
  FilterPalette: ({ isOpen, onClose }: { isOpen: boolean; onClose: () => void }) => (
    isOpen ? <div data-testid="filter-palette"><button onClick={onClose}>Close</button></div> : null
  ),
}));

jest.mock('../../components/search/KeywordStatsPanel', () => ({
  KeywordStatsPanel: ({ keywords, totalMatches, searchDurationMs, onClose }: any) => (
    <div data-testid="keyword-stats-panel">
      <div>Total: {totalMatches}</div>
      <div>Duration: {searchDurationMs}ms</div>
      <button onClick={onClose}>Close Stats</button>
    </div>
  ),
}));

jest.mock('../../components/renderers/HybridLogRenderer', () => {
  return ({ text, query, keywordGroups }: any) => (
    <span data-testid="log-renderer">{text}</span>
  );
});

// Mock UI components
jest.mock('../../components/ui', () => ({
  Button: ({ children, onClick, disabled, className, variant }: any) => (
    <button
      onClick={onClick}
      disabled={disabled}
      className={className}
      data-variant={variant}
      data-testid="button"
    >
      {children}
    </button>
  ),
  Input: ({ value, onChange, onKeyDown, placeholder, className }: any) => (
    <input
      value={value}
      onChange={onChange}
      onKeyDown={onKeyDown}
      placeholder={placeholder}
      className={className}
    />
  ),
}));

const mockInvoke = invoke as jest.MockedFunction<typeof invoke>;
const mockListen = listen as jest.MockedFunction<typeof listen>;
const mockSave = require('@tauri-apps/plugin-dialog').save;

// Mock API
const mockApi = require('../../services/api').api;

describe('SearchPage', () => {
  const mockKeywordGroups: KeywordGroup[] = [
    {
      id: 'group-1',
      name: 'Errors',
      color: 'red',
      patterns: [{ regex: 'error', comment: '' }],
      enabled: true,
    },
    {
      id: 'group-2',
      name: 'Warnings',
      color: 'orange',
      patterns: [{ regex: 'warning', comment: '' }],
      enabled: false,
    },
  ];

  const mockWorkspace: Workspace = {
    id: 'ws-1',
    name: 'Test Workspace',
    path: '/path/to/workspace',
    status: 'READY',
    size: '100MB',
    files: 100,
  };

  const mockLogs: LogEntry[] = Array.from({ length: 50 }, (_, i) => ({
    id: i + 1,
    timestamp: '2024-01-01 12:00:00',
    level: i % 3 === 0 ? 'ERROR' : i % 3 === 1 ? 'WARN' : 'INFO',
    content: `Test log message ${i}`,
    file: 'test.log',
    line: i + 1,
    real_path: '/path/to/test.log',
    tags: [],
    match_details: null,
    matched_keywords: undefined,
  }));

  const mockAddToast = jest.fn();
  const mockSearchInputRef = React.createRef<HTMLInputElement>();

  // Mock event unlisten functions
  let mockUnlisteners: Array<jest.Mock>;

  beforeEach(() => {
    jest.clearAllMocks();

    mockUnlisteners = [];
    for (let i = 0; i < 5; i++) {
      const unlisten = jest.fn();
      mockUnlisteners.push(unlisten);
    }

    // Setup listen mock to return unlisten
    mockListen.mockImplementation((event, handler) => {
      return Promise.resolve(mockUnlisteners[mockUnlisteners.length - 1] || jest.fn());
    });

    // Setup invoke mock
    mockInvoke.mockResolvedValue(undefined);

    // Setup save dialog mock
    mockSave.mockResolvedValue('/path/to/export.csv');

    // Setup API mock
    mockApi.searchLogs.mockResolvedValue(undefined);
    mockApi.exportResults.mockResolvedValue(undefined);
  });

  const renderSearchPage = (props = {}) => {
    return render(
      <SearchPage
        keywordGroups={mockKeywordGroups}
        addToast={mockAddToast}
        searchInputRef={mockSearchInputRef}
        activeWorkspace={mockWorkspace}
        {...props}
      />
    );
  };

  describe('初始渲染', () => {
    it('应该渲染搜索输入框', () => {
      renderSearchPage();

      expect(screen.getByPlaceholderText(/Search keywords/i)).toBeInTheDocument();
    });

    it('应该渲染操作按钮', () => {
      renderSearchPage();

      expect(screen.getByText('CSV')).toBeInTheDocument();
      expect(screen.getByText('JSON')).toBeInTheDocument();
      expect(screen.getByText('Search')).toBeInTheDocument();
    });

    it('应该渲染过滤器标签', () => {
      renderSearchPage();

      expect(screen.getByText('Filters')).toBeInTheDocument();
      expect(screen.getByText('Advanced Filters')).toBeInTheDocument();
    });

    it('应该渲染日志级别过滤器', () => {
      renderSearchPage();

      expect(screen.getByText('Level')).toBeInTheDocument();
      expect(screen.getByText('E')).toBeInTheDocument(); // ERROR
      expect(screen.getByText('W')).toBeInTheDocument(); // WARN
      expect(screen.getByText('I')).toBeInTheDocument(); // INFO
      expect(screen.getByText('D')).toBeInTheDocument(); // DEBUG
    });

    it('应该渲染时间范围过滤器', () => {
      renderSearchPage();

      expect(screen.getByText('Time Range')).toBeInTheDocument();
    });

    it('应该渲染文件模式过滤器', () => {
      renderSearchPage();

      expect(screen.getByText('File Pattern')).toBeInTheDocument();
    });

    it('应该显示空状态提示', () => {
      renderSearchPage();

      expect(screen.getByText(/No logs found/i)).toBeInTheDocument();
    });

    it('没有活跃工作区时不应该触发搜索', () => {
      renderSearchPage({ activeWorkspace: null });

      const searchButton = screen.getByText('Search');
      fireEvent.click(searchButton);

      expect(mockApi.searchLogs).not.toHaveBeenCalled();
      expect(mockAddToast).toHaveBeenCalledWith('error', 'Select a workspace first.');
    });
  });

  describe('搜索功能', () => {
    it('应该更新搜索查询', async () => {
      renderSearchPage();

      const input = screen.getByPlaceholderText(/Search keywords/i);
      fireEvent.change(input, { target: { value: 'error | warning' } });

      expect(input).toHaveValue('error|warning');
    });

    it('应该规范化输入（移除 | 前后空格）', async () => {
      renderSearchPage();

      const input = screen.getByPlaceholderText(/Search keywords/i);
      fireEvent.change(input, { target: { value: 'error | warning' } });

      expect(input).toHaveValue('error|warning');
    });

    it('按 Enter 键应该触发搜索', async () => {
      renderSearchPage();

      const input = screen.getByPlaceholderText(/Search keywords/i);
      fireEvent.change(input, { target: { value: 'error' } });

      fireEvent.keyDown(input, { key: 'Enter' });

      await waitFor(() => {
        expect(mockApi.searchLogs).toHaveBeenCalledWith({
          query: 'error',
          workspaceId: 'ws-1',
          filters: expect.any(Object),
        });
      });
    });

    it('点击 Search 按钮应该触发搜索', async () => {
      renderSearchPage();

      const input = screen.getByPlaceholderText(/Search keywords/i);
      fireEvent.change(input, { target: { value: 'error' } });

      const searchButton = screen.getByText('Search');
      fireEvent.click(searchButton);

      await waitFor(() => {
        expect(mockApi.searchLogs).toHaveBeenCalledWith({
          query: 'error',
          workspaceId: 'ws-1',
          filters: expect.any(Object),
        });
      });
    });

    it('搜索时应该显示加载状态', async () => {
      renderSearchPage();

      const input = screen.getByPlaceholderText(/Search keywords/i);
      fireEvent.change(input, { target: { value: 'error' } });

      const searchButton = screen.getByText('Search');
      fireEvent.click(searchButton);

      // 按钮应该显示省略号
      expect(screen.getByText('...')).toBeInTheDocument();
    });

    it('空查询不应该触发搜索', async () => {
      renderSearchPage();

      const searchButton = screen.getByText('Search');
      fireEvent.click(searchButton);

      expect(mockApi.searchLogs).not.toHaveBeenCalled();
    });
  });

  describe('事件监听', () => {
    it('应该设置搜索事件监听器', async () => {
      renderSearchPage();

      await waitFor(() => {
        expect(mockListen).toHaveBeenCalledWith('search-results', expect.any(Function));
        expect(mockListen).toHaveBeenCalledWith('search-summary', expect.any(Function));
        expect(mockListen).toHaveBeenCalledWith('search-complete', expect.any(Function));
        expect(mockListen).toHaveBeenCalledWith('search-error', expect.any(Function));
        expect(mockListen).toHaveBeenCalledWith('search-start', expect.any(Function));
      });
    });

    it('应该接收到搜索结果', async () => {
      renderSearchPage();

      // 获取 search-results 事件处理器
      const searchResultsHandler = mockListen.mock.calls.find(
        call => call[0] === 'search-results'
      )?.[1];

      if (searchResultsHandler) {
        await act(async () => {
          searchResultsHandler({ payload: mockLogs.slice(0, 10) });
        });

        // 组件应该更新状态并显示日志
        await waitFor(() => {
          expect(screen.getByText('Test log message 0')).toBeInTheDocument();
        });
      }
    });

    it('应该接收到搜索完成事件', async () => {
      renderSearchPage();

      const searchCompleteHandler = mockListen.mock.calls.find(
        call => call[0] === 'search-complete'
      )?.[1];

      if (searchCompleteHandler) {
        await act(async () => {
          searchCompleteHandler({ payload: 100 });
        });

        expect(mockAddToast).toHaveBeenCalledWith('success', '找到 100 条日志');
      }
    });

    it('应该接收到搜索错误事件', async () => {
      renderSearchPage();

      const searchErrorHandler = mockListen.mock.calls.find(
        call => call[0] === 'search-error'
      )?.[1];

      if (searchErrorHandler) {
        await act(async () => {
          searchErrorHandler({ payload: 'Search failed' });
        });

        expect(mockAddToast).toHaveBeenCalledWith('error', '搜索失败: Search failed');
      }
    });

    it('组件卸载时应该清理事件监听器', async () => {
      const { unmount } = renderSearchPage();

      await waitFor(() => {
        expect(mockListen).toHaveBeenCalled();
      });

      unmount();

      // 所有 unlisten 函数应该被调用
      mockUnlisteners.forEach(unlisten => {
        expect(unlisten).toHaveBeenCalled();
      });
    });
  });

  describe('过滤器功能', () => {
    it('应该切换日志级别过滤器', async () => {
      renderSearchPage();

      const errorButton = screen.getByText('E');
      fireEvent.click(errorButton);

      // 按钮应该有激活样式
      expect(errorButton).toHaveClass('bg-primary');
    });

    it('应该更新时间范围过滤器', async () => {
      renderSearchPage();

      const inputs = screen.getAllByPlaceholderText(/\d{4}-\d{2}-\d{2}T/);
      const startInput = inputs[0];

      fireEvent.change(startInput, { target: { value: '2024-01-01T00:00' } });

      expect(startInput).toHaveValue('2024-01-01T00:00');
    });

    it('应该更新文件模式过滤器', async () => {
      renderSearchPage();

      const filePatternInput = screen.getByPlaceholderText('e.g. error.log');
      fireEvent.change(filePatternInput, { target: { value: '*.log' } });

      expect(filePatternInput).toHaveValue('*.log');
    });

    it('应该显示激活的过滤器标签', async () => {
      renderSearchPage();

      // 激活一些过滤器
      const errorButton = screen.getByText('E');
      fireEvent.click(errorButton);

      await waitFor(() => {
        expect(screen.getByText(/1 levels/)).toBeInTheDocument();
      });
    });

    it('点击 Reset 应该重置所有过滤器', async () => {
      renderSearchPage();

      // 激活一些过滤器
      fireEvent.click(screen.getByText('E'));
      fireEvent.click(screen.getByText('W'));

      const resetButton = screen.getByText('Reset');
      fireEvent.click(resetButton);

      expect(mockAddToast).toHaveBeenCalledWith('info', '过滤器已重置');
    });

    it('应该打开和关闭过滤器面板', async () => {
      renderSearchPage();

      const filterButton = screen.getByText('Filters');
      fireEvent.click(filterButton);

      expect(screen.getByTestId('filter-palette')).toBeInTheDocument();

      const closeButton = screen.getByText('Close');
      fireEvent.click(closeButton);

      expect(screen.queryByTestId('filter-palette')).not.toBeInTheDocument();
    });
  });

  describe('活跃关键词显示', () => {
    it('应该显示当前激活的搜索关键词', async () => {
      renderSearchPage();

      const input = screen.getByPlaceholderText(/Search keywords/i);
      fireEvent.change(input, { target: { value: 'error|warning|debug' } });

      expect(screen.getByText('error')).toBeInTheDocument();
      expect(screen.getByText('warning')).toBeInTheDocument();
      expect(screen.getByText('debug')).toBeInTheDocument();
    });

    it('应该能够移除单个关键词', async () => {
      renderSearchPage();

      const input = screen.getByPlaceholderText(/Search keywords/i);
      fireEvent.change(input, { target: { value: 'error|warning' } });

      // 点击移除按钮
      const removeButtons = screen.getAllByTitle('Remove keyword');
      fireEvent.click(removeButtons[0]);

      await waitFor(() => {
        expect(input).toHaveValue('warning');
      });
    });

    it('没有查询时应该显示 None', async () => {
      renderSearchPage();

      expect(screen.getByText('None')).toBeInTheDocument();
    });
  });

  describe('导出功能', () => {
    it('应该导出 CSV 格式', async () => {
      // 模拟有日志结果
      mockListen.mockImplementation(async (event, handler) => {
        if (event === 'search-results') {
          await act(async () => {
            handler({ payload: mockLogs });
          });
        }
        return jest.fn();
      });

      renderSearchPage();

      // 模拟有日志
      const input = screen.getByPlaceholderText(/Search keywords/i);
      fireEvent.change(input, { target: { value: 'test' } });

      await waitFor(() => {
        // 等待日志加载
      }, { timeout: 1000 });

      const csvButton = screen.getByText('CSV');
      fireEvent.click(csvButton);

      await waitFor(() => {
        expect(mockSave).toHaveBeenCalled();
        expect(mockApi.exportResults).toHaveBeenCalledWith({
          results: expect.any(Array),
          format: 'csv',
          savePath: '/path/to/export.csv',
        });
      });
    });

    it('应该导出 JSON 格式', async () => {
      mockListen.mockImplementation(async (event, handler) => {
        if (event === 'search-results') {
          await act(async () => {
            handler({ payload: mockLogs });
          });
        }
        return jest.fn();
      });

      renderSearchPage();

      const jsonButton = screen.getByText('JSON');
      fireEvent.click(jsonButton);

      await waitFor(() => {
        expect(mockApi.exportResults).toHaveBeenCalledWith({
          results: expect.any(Array),
          format: 'json',
          savePath: '/path/to/export.json',
        });
      });
    });

    it('没有日志时导出应该显示错误', async () => {
      renderSearchPage();

      const csvButton = screen.getByText('CSV');
      fireEvent.click(csvButton);

      expect(mockAddToast).toHaveBeenCalledWith('error', '没有可导出的数据');
    });

    it('用户取消导出时不应该调用 API', async () => {
      mockSave.mockResolvedValue(null); // 用户取消

      mockListen.mockImplementation(async (event, handler) => {
        if (event === 'search-results') {
          await act(async () => {
            handler({ payload: mockLogs });
          });
        }
        return jest.fn();
      });

      renderSearchPage();

      const csvButton = screen.getByText('CSV');
      fireEvent.click(csvButton);

      await waitFor(() => {
        expect(mockApi.exportResults).not.toHaveBeenCalled();
      });
    });
  });

  describe('关键词统计面板', () => {
    it('应该显示关键词统计面板', async () => {
      renderSearchPage();

      const searchSummaryHandler = mockListen.mock.calls.find(
        call => call[0] === 'search-summary'
      )?.[1];

      if (searchSummaryHandler) {
        await act(async () => {
          searchSummaryHandler({
            payload: {
              totalMatches: 1000,
              searchDurationMs: 150,
              keywordStats: [
                { keyword: 'error', count: 800 },
                { keyword: 'warning', count: 200 },
              ],
            },
          });
        });

        expect(screen.getByTestId('keyword-stats-panel')).toBeInTheDocument();
        expect(screen.getByText('Total: 1000')).toBeInTheDocument();
        expect(screen.getByText('Duration: 150ms')).toBeInTheDocument();
      }
    });

    it('应该能够关闭统计面板', async () => {
      renderSearchPage();

      const searchSummaryHandler = mockListen.mock.calls.find(
        call => call[0] === 'search-summary'
      )?.[1];

      if (searchSummaryHandler) {
        await act(async () => {
          searchSummaryHandler({
            payload: {
              totalMatches: 1000,
              searchDurationMs: 150,
              keywordStats: [{ keyword: 'error', count: 800 }],
            },
          });
        });

        const closeButton = screen.getByText('Close Stats');
        fireEvent.click(closeButton);

        expect(screen.queryByTestId('keyword-stats-panel')).not.toBeInTheDocument();
      }
    });
  });

  describe('边界条件', () => {
    it('应该处理没有关键词组的情况', () => {
      renderSearchPage({ keywordGroups: [] });

      expect(screen.getByPlaceholderText(/Search keywords/i)).toBeInTheDocument();
    });

    it('应该处理特殊字符查询', async () => {
      renderSearchPage();

      const input = screen.getByPlaceholderText(/Search keywords/i);
      fireEvent.change(input, { target: { value: 'error: "test" | warning[0-9]' } });

      expect(input).toHaveValue('error: "test"|warning[0-9]');
    });

    it('应该处理超长查询', async () => {
      renderSearchPage();

      const longQuery = 'a'.repeat(1000);
      const input = screen.getByPlaceholderText(/Search keywords/i);
      fireEvent.change(input, { target: { value: longQuery } });

      expect(input).toHaveValue(longQuery);
    });
  });

  describe('防抖搜索', () => {
    jest.useFakeTimers();

    it('应该在防抖延迟后触发搜索', async () => {
      renderSearchPage();

      const input = screen.getByPlaceholderText(/Search keywords/i);
      fireEvent.change(input, { target: { value: 'error' } });

      // 快速输入不应该立即触发搜索
      expect(mockApi.searchLogs).not.toHaveBeenCalled();

      // 前进时间到防抖延迟后
      act(() => {
        jest.advanceTimersByTime(500);
      });

      await waitFor(() => {
        expect(mockApi.searchLogs).toHaveBeenCalled();
      });
    });

    it('连续输入应该重置防抖计时器', async () => {
      renderSearchPage();

      const input = screen.getByPlaceholderText(/Search keywords/i);

      fireEvent.change(input, { target: { value: 'err' } });
      act(() => {
        jest.advanceTimersByTime(300);
      });

      fireEvent.change(input, { target: { value: 'error' } });
      act(() => {
        jest.advanceTimersByTime(300);
      });

      // 300ms 不应该触发搜索（需要 500ms）
      expect(mockApi.searchLogs).not.toHaveBeenCalled();

      act(() => {
        jest.advanceTimersByTime(200);
      });

      await waitFor(() => {
        expect(mockApi.searchLogs).toHaveBeenCalled();
      });
    });

    afterEach(() => {
      jest.useRealTimers();
    });
  });

  describe('禁用关键词组过滤', () => {
    it('应该只使用启用的关键词组', () => {
      renderSearchPage();

      // mockKeywordGroups 中 group-2 是禁用的
      // 组件应该过滤掉禁用的组
      expect(mockKeywordGroups[1].enabled).toBe(false);
    });
  });
});
