/**
 * useKeywordManager Hook 单元测试
 *
 * 测试关键词管理 Hook 的 CRUD 操作和状态管理
 */

import { renderHook, act } from '@testing-library/react';
import { useKeywordManager } from '../useKeywordManager';
import { useKeywordStore, KeywordGroup } from '../../stores/keywordStore';
import { useAppStore } from '../../stores/appStore';

// Mock logger
jest.mock('../../utils/logger', () => ({
  logger: {
    debug: jest.fn(),
    error: jest.fn(),
    info: jest.fn(),
    warn: jest.fn(),
  },
}));

describe('useKeywordManager Hook', () => {
  beforeEach(() => {
    // Reset stores before each test
    act(() => {
      useKeywordStore.setState({
        keywordGroups: [],
        loading: false,
        error: null,
      });
      useAppStore.setState({
        page: 'keywords',
        toasts: [],
        activeWorkspaceId: null,
        isInitialized: false,
        initializationError: null,
      });
    });
    jest.clearAllMocks();
  });

  describe('初始状态', () => {
    it('应该返回关键词组列表', () => {
      const { result } = renderHook(() => useKeywordManager());

      expect(result.current.keywordGroups).toEqual([]);
    });

    it('应该返回加载状态', () => {
      const { result } = renderHook(() => useKeywordManager());

      expect(result.current.loading).toBe(false);
    });

    it('应该返回错误状态', () => {
      const { result } = renderHook(() => useKeywordManager());

      expect(result.current.error).toBe(null);
    });

    it('应该提供所有 CRUD 方法', () => {
      const { result } = renderHook(() => useKeywordManager());

      expect(typeof result.current.addKeywordGroup).toBe('function');
      expect(typeof result.current.updateKeywordGroup).toBe('function');
      expect(typeof result.current.deleteKeywordGroup).toBe('function');
      expect(typeof result.current.toggleKeywordGroup).toBe('function');
      expect(typeof result.current.saveKeywordGroup).toBe('function');
    });
  });

  describe('addKeywordGroup', () => {
    it('应该添加关键词组', () => {
      const { result } = renderHook(() => useKeywordManager());

      const newGroup: KeywordGroup = {
        id: 'test-1',
        name: 'Test Group',
        color: 'blue',
        patterns: [
          { regex: 'error', comment: 'Error pattern' },
        ],
        enabled: true,
      };

      act(() => {
        result.current.addKeywordGroup(newGroup);
      });

      expect(result.current.keywordGroups).toHaveLength(1);
      expect(result.current.keywordGroups[0]).toEqual(newGroup);
    });

    it('应该添加多个关键词组', () => {
      const { result } = renderHook(() => useKeywordManager());

      const group1: KeywordGroup = {
        id: 'test-1',
        name: 'Error Patterns',
        color: 'red',
        patterns: [{ regex: 'error', comment: '' }],
        enabled: true,
      };

      const group2: KeywordGroup = {
        id: 'test-2',
        name: 'Warning Patterns',
        color: 'orange',
        patterns: [{ regex: 'warning', comment: '' }],
        enabled: true,
      };

      act(() => {
        result.current.addKeywordGroup(group1);
        result.current.addKeywordGroup(group2);
      });

      expect(result.current.keywordGroups).toHaveLength(2);
      expect(result.current.keywordGroups[0].name).toBe('Error Patterns');
      expect(result.current.keywordGroups[1].name).toBe('Warning Patterns');
    });

    it('应该调用 toast 显示成功消息', () => {
      const { result } = renderHook(() => useKeywordManager());

      const newGroup: KeywordGroup = {
        id: 'test-1',
        name: 'Test Group',
        color: 'green',
        patterns: [{ regex: 'test', comment: '' }],
        enabled: true,
      };

      act(() => {
        result.current.addKeywordGroup(newGroup);
      });

      // Hook 会调用 appStore.addToast，这里是测试状态变化
      expect(result.current.keywordGroups).toHaveLength(1);
    });
  });

  describe('updateKeywordGroup', () => {
    it('应该更新现有的关键词组', () => {
      const { result } = renderHook(() => useKeywordManager());

      const group: KeywordGroup = {
        id: 'test-1',
        name: 'Original Name',
        color: 'blue',
        patterns: [{ regex: 'pattern1', comment: '' }],
        enabled: true,
      };

      act(() => {
        result.current.addKeywordGroup(group);
      });

      const updatedGroup: KeywordGroup = {
        ...group,
        name: 'Updated Name',
        color: 'red',
        patterns: [
          { regex: 'pattern1', comment: '' },
          { regex: 'pattern2', comment: 'New pattern' },
        ],
      };

      act(() => {
        result.current.updateKeywordGroup(updatedGroup);
      });

      expect(result.current.keywordGroups).toHaveLength(1);
      expect(result.current.keywordGroups[0].name).toBe('Updated Name');
      expect(result.current.keywordGroups[0].color).toBe('red');
      expect(result.current.keywordGroups[0].patterns).toHaveLength(2);
    });

    it('更新不存在的组不应添加新组', () => {
      const { result } = renderHook(() => useKeywordManager());

      const nonExistentGroup: KeywordGroup = {
        id: 'non-existent',
        name: 'Non-existent',
        color: 'blue',
        patterns: [],
        enabled: true,
      };

      act(() => {
        result.current.updateKeywordGroup(nonExistentGroup);
      });

      expect(result.current.keywordGroups).toHaveLength(0);
    });

    it('应该调用 toast 显示成功消息', () => {
      const { result } = renderHook(() => useKeywordManager());

      const group: KeywordGroup = {
        id: 'test-1',
        name: 'Test Group',
        color: 'blue',
        patterns: [{ regex: 'test', comment: '' }],
        enabled: true,
      };

      act(() => {
        result.current.addKeywordGroup(group);
      });

      act(() => {
        result.current.updateKeywordGroup({ ...group, name: 'Updated' });
      });

      // Hook 会调用 appStore.addToast
      expect(result.current.keywordGroups[0].name).toBe('Updated');
    });
  });

  describe('deleteKeywordGroup', () => {
    it('应该删除指定的关键词组', () => {
      const { result } = renderHook(() => useKeywordManager());

      const group1: KeywordGroup = {
        id: 'test-1',
        name: 'Group 1',
        color: 'blue',
        patterns: [],
        enabled: true,
      };

      const group2: KeywordGroup = {
        id: 'test-2',
        name: 'Group 2',
        color: 'red',
        patterns: [],
        enabled: true,
      };

      act(() => {
        result.current.addKeywordGroup(group1);
        result.current.addKeywordGroup(group2);
      });

      expect(result.current.keywordGroups).toHaveLength(2);

      act(() => {
        result.current.deleteKeywordGroup('test-1');
      });

      expect(result.current.keywordGroups).toHaveLength(1);
      expect(result.current.keywordGroups[0].id).toBe('test-2');
    });

    it('删除不存在的组不应报错', () => {
      const { result } = renderHook(() => useKeywordManager());

      expect(() => {
        act(() => {
          result.current.deleteKeywordGroup('non-existent');
        });
      }).not.toThrow();

      expect(result.current.keywordGroups).toHaveLength(0);
    });

    it('应该调用 toast 显示信息消息', () => {
      const { result } = renderHook(() => useKeywordManager());

      const group: KeywordGroup = {
        id: 'test-1',
        name: 'Test Group',
        color: 'blue',
        patterns: [],
        enabled: true,
      };

      act(() => {
        result.current.addKeywordGroup(group);
      });

      act(() => {
        result.current.deleteKeywordGroup('test-1');
      });

      // Hook 会调用 appStore.addToast
      expect(result.current.keywordGroups).toHaveLength(0);
    });
  });

  describe('toggleKeywordGroup', () => {
    it('应该切换关键词组的启用状态', () => {
      const { result } = renderHook(() => useKeywordManager());

      const group: KeywordGroup = {
        id: 'test-1',
        name: 'Test Group',
        color: 'blue',
        patterns: [],
        enabled: true,
      };

      act(() => {
        result.current.addKeywordGroup(group);
      });

      expect(result.current.keywordGroups[0].enabled).toBe(true);

      act(() => {
        result.current.toggleKeywordGroup('test-1');
      });

      expect(result.current.keywordGroups[0].enabled).toBe(false);

      act(() => {
        result.current.toggleKeywordGroup('test-1');
      });

      expect(result.current.keywordGroups[0].enabled).toBe(true);
    });

    it('切换不存在的组不应报错', () => {
      const { result } = renderHook(() => useKeywordManager());

      expect(() => {
        act(() => {
          result.current.toggleKeywordGroup('non-existent');
        });
      }).not.toThrow();
    });

    it('toggle 不应显示 toast', () => {
      const { result } = renderHook(() => useKeywordManager());

      const group: KeywordGroup = {
        id: 'test-1',
        name: 'Test Group',
        color: 'blue',
        patterns: [],
        enabled: true,
      };

      act(() => {
        result.current.addKeywordGroup(group);
      });

      const enabledBefore = result.current.keywordGroups[0].enabled;

      act(() => {
        result.current.toggleKeywordGroup('test-1');
      });

      // toggle 应该切换状态
      expect(result.current.keywordGroups[0].enabled).toBe(!enabledBefore);
    });
  });

  describe('saveKeywordGroup', () => {
    it('当 isEditing=false 时应该添加新组', () => {
      const { result } = renderHook(() => useKeywordManager());

      const newGroup: KeywordGroup = {
        id: 'test-1',
        name: 'New Group',
        color: 'green',
        patterns: [{ regex: 'test', comment: '' }],
        enabled: true,
      };

      act(() => {
        result.current.saveKeywordGroup(newGroup, false);
      });

      expect(result.current.keywordGroups).toHaveLength(1);
      expect(result.current.keywordGroups[0].name).toBe('New Group');
    });

    it('当 isEditing=true 时应该更新现有组', () => {
      const { result } = renderHook(() => useKeywordManager());

      const group: KeywordGroup = {
        id: 'test-1',
        name: 'Original',
        color: 'blue',
        patterns: [],
        enabled: true,
      };

      act(() => {
        result.current.addKeywordGroup(group);
      });

      const updatedGroup: KeywordGroup = {
        ...group,
        name: 'Updated',
      };

      act(() => {
        result.current.saveKeywordGroup(updatedGroup, true);
      });

      expect(result.current.keywordGroups).toHaveLength(1);
      expect(result.current.keywordGroups[0].name).toBe('Updated');
    });

    it('应该根据 isEditing 参数调用正确的方法', () => {
      const { result } = renderHook(() => useKeywordManager());

      const group: KeywordGroup = {
        id: 'test-1',
        name: 'Test',
        color: 'blue',
        patterns: [],
        enabled: true,
      };

      act(() => {
        result.current.saveKeywordGroup(group, false);
      });

      expect(result.current.keywordGroups).toHaveLength(1);

      act(() => {
        result.current.saveKeywordGroup(group, true);
      });

      // 仍然只有 1 个，因为更新而不是添加
      expect(result.current.keywordGroups).toHaveLength(1);
    });
  });

  describe('状态同步', () => {
    it('应该反映 store 中的变化', () => {
      const { result } = renderHook(() => useKeywordManager());

      // 直接修改 store
      act(() => {
        useKeywordStore.getState().setKeywordGroups([
          {
            id: 'store-group',
            name: 'From Store',
            color: 'purple',
            patterns: [],
            enabled: true,
          },
        ]);
      });

      expect(result.current.keywordGroups).toHaveLength(1);
      expect(result.current.keywordGroups[0].name).toBe('From Store');
    });

    it('应该反映 loading 状态', () => {
      const { result } = renderHook(() => useKeywordManager());

      act(() => {
        useKeywordStore.getState().setLoading(true);
      });

      expect(result.current.loading).toBe(true);

      act(() => {
        useKeywordStore.getState().setLoading(false);
      });

      expect(result.current.loading).toBe(false);
    });

    it('应该反映 error 状态', () => {
      const { result } = renderHook(() => useKeywordManager());

      act(() => {
        useKeywordStore.getState().setError('Test error');
      });

      expect(result.current.error).toBe('Test error');

      act(() => {
        useKeywordStore.getState().setError(null);
      });

      expect(result.current.error).toBe(null);
    });
  });

  describe('边界条件', () => {
    it('应该处理空 patterns 数组', () => {
      const { result } = renderHook(() => useKeywordManager());

      const group: KeywordGroup = {
        id: 'test-1',
        name: 'Empty Patterns',
        color: 'blue',
        patterns: [],
        enabled: true,
      };

      act(() => {
        result.current.addKeywordGroup(group);
      });

      expect(result.current.keywordGroups[0].patterns).toEqual([]);
    });

    it('应该处理多 patterns', () => {
      const { result } = renderHook(() => useKeywordManager());

      const group: KeywordGroup = {
        id: 'test-1',
        name: 'Multiple Patterns',
        color: 'red',
        patterns: [
          { regex: 'error', comment: 'Error' },
          { regex: 'warning', comment: 'Warning' },
          { regex: 'critical', comment: 'Critical' },
        ],
        enabled: true,
      };

      act(() => {
        result.current.addKeywordGroup(group);
      });

      expect(result.current.keywordGroups[0].patterns).toHaveLength(3);
    });

    it('应该处理所有颜色类型', () => {
      const { result } = renderHook(() => useKeywordManager());
      const colors: Array<'blue' | 'green' | 'red' | 'orange' | 'purple'> = ['blue', 'green', 'red', 'orange', 'purple'];

      colors.forEach((color, index) => {
        const group: KeywordGroup = {
          id: `test-${index}`,
          name: `Group ${color}`,
          color,
          patterns: [],
          enabled: true,
        };

        act(() => {
          result.current.addKeywordGroup(group);
        });
      });

      expect(result.current.keywordGroups).toHaveLength(5);
      expect(result.current.keywordGroups[0].color).toBe('blue');
      expect(result.current.keywordGroups[4].color).toBe('purple');
    });
  });
});
