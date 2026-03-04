/**
 * useConfig Hook 单元测试
 *
 * 测试配置管理 Hook 的加载、保存和重置功能
 */

import { renderHook, act, waitFor } from '@testing-library/react';
import { useConfig, CacheConfig, SearchConfig, TaskManagerConfig } from '../useConfig';

// Mock Tauri invoke
jest.mock('@tauri-apps/api/core', () => ({
  invoke: jest.fn(),
}));

// Get the mocked invoke function
const mockInvoke = require('@tauri-apps/api/core').invoke;

// Mock errors service
jest.mock('../../services/errors', () => ({
  createApiError: (action: string, err: unknown) => {
    // Return a real Error that can be thrown
    const error = new Error(`Error: ${action} - ${err}`) as any;
    error.getUserMessage = () => `Error: ${action} - ${err}`;
    return error;
  },
}));

describe('useConfig Hook', () => {
  const mockCacheConfig: CacheConfig = {
    max_capacity: 1000,
    ttl: 300,
    l2_enabled: false,
    l2_capacity: 10000,
  };

  const mockSearchConfig: SearchConfig = {
    default_max_results: 1000,
    cache_enabled: true,
    cache_size: 1000,
    search_timeout: 30,
    enable_regex_engine: true,
    enable_filter_engine: true,
  };

  const mockTaskManagerConfig: TaskManagerConfig = {
    max_concurrent_tasks: 10,
    task_timeout: 300,
    data_dir: '/app/data',
    log_level: 'info',
    debug_mode: false,
    enable_profiling: false,
  };

  beforeEach(() => {
    jest.clearAllMocks();
    mockInvoke.mockResolvedValue(undefined);
  });

  describe('初始状态', () => {
    it('应该初始化为非加载状态', () => {
      const { result } = renderHook(() => useConfig());

      expect(result.current.isLoading).toBe(false);
    });

    it('应该初始化为无错误状态', () => {
      const { result } = renderHook(() => useConfig());

      expect(result.current.error).toBe(null);
    });

    it('应该初始化配置为 null', () => {
      const { result } = renderHook(() => useConfig());

      expect(result.current.cacheConfig).toBe(null);
      expect(result.current.searchConfig).toBe(null);
      expect(result.current.taskManagerConfig).toBe(null);
    });

    it('应该提供所有配置方法', () => {
      const { result } = renderHook(() => useConfig());

      expect(typeof result.current.loadCacheConfig).toBe('function');
      expect(typeof result.current.loadSearchConfig).toBe('function');
      expect(typeof result.current.loadTaskManagerConfig).toBe('function');
      expect(typeof result.current.loadAllConfigs).toBe('function');
      expect(typeof result.current.saveCacheConfig).toBe('function');
      expect(typeof result.current.saveSearchConfig).toBe('function');
      expect(typeof result.current.saveTaskManagerConfig).toBe('function');
      expect(typeof result.current.saveAllConfigs).toBe('function');
      expect(typeof result.current.resetToDefaults).toBe('function');
    });
  });

  describe('loadCacheConfig', () => {
    it('应该成功加载缓存配置', async () => {
      mockInvoke.mockResolvedValueOnce(mockCacheConfig);

      const { result } = renderHook(() => useConfig());

      await act(async () => {
        const config = await result.current.loadCacheConfig();
        expect(config).toEqual(mockCacheConfig);
      });

      expect(mockInvoke).toHaveBeenCalledWith('get_cache_config');
      expect(result.current.cacheConfig).toEqual(mockCacheConfig);
      expect(result.current.isLoading).toBe(false);
      expect(result.current.error).toBe(null);
    });

    it('加载时应该设置 loading 状态', async () => {
      mockInvoke.mockImplementation(() => new Promise(resolve => setTimeout(() => resolve(mockCacheConfig), 100)));

      const { result } = renderHook(() => useConfig());

      act(() => {
        result.current.loadCacheConfig();
      });

      expect(result.current.isLoading).toBe(true);

      await waitFor(() => {
        expect(result.current.isLoading).toBe(false);
      });
    });

    it('加载失败应该设置错误', async () => {
      const error = new Error('Load failed');
      mockInvoke.mockRejectedValueOnce(error);

      const { result } = renderHook(() => useConfig());

      // 使用 expect().rejects.toThrow() 来验证抛出错误
      await expect(result.current.loadCacheConfig()).rejects.toThrow();

      // 由于 Hook 内部会 catch 错误并设置状态，我们需要等待下次渲染
      await act(async () => {
        await new Promise(resolve => setTimeout(resolve, 0));
      });

      expect(result.current.error).toBeTruthy();
      expect(result.current.error).toContain('Load failed');
    });
  });

  describe('saveAllConfigs', () => {
    it('应该并行保存所有配置', async () => {
      mockInvoke.mockResolvedValue(undefined);

      const { result } = renderHook(() => useConfig());

      const configs = {
        cache: mockCacheConfig,
        search: mockSearchConfig,
        task_manager: mockTaskManagerConfig,
      };

      await act(async () => {
        await result.current.saveAllConfigs(configs);
      });

      expect(mockInvoke).toHaveBeenCalledWith('save_cache_config', {
        cacheConfig: mockCacheConfig,
      });
      expect(mockInvoke).toHaveBeenCalledWith('save_search_config', {
        searchConfig: mockSearchConfig,
      });
      expect(mockInvoke).toHaveBeenCalledWith('save_task_manager_config', {
        taskManagerConfig: mockTaskManagerConfig,
      });

      expect(result.current.cacheConfig).toEqual(mockCacheConfig);
      expect(result.current.searchConfig).toEqual(mockSearchConfig);
      expect(result.current.taskManagerConfig).toEqual(mockTaskManagerConfig);
    });
  });

  describe('resetToDefaults', () => {
    it('应该重置为默认配置', async () => {
      mockInvoke.mockResolvedValue(undefined);

      const { result } = renderHook(() => useConfig());

      await act(async () => {
        const defaults = await result.current.resetToDefaults();
        expect(defaults.cache.max_capacity).toBe(1000);
        expect(defaults.cache.ttl).toBe(300);
        expect(defaults.search.default_max_results).toBe(1000);
        expect(defaults.task_manager.max_concurrent_tasks).toBe(10);
      });

      expect(result.current.cacheConfig).not.toBe(null);
      expect(result.current.searchConfig).not.toBe(null);
      expect(result.current.taskManagerConfig).not.toBe(null);
    });

    it('默认配置应该有正确的值', async () => {
      mockInvoke.mockResolvedValue(undefined);

      const { result } = renderHook(() => useConfig());

      await act(async () => {
        await result.current.resetToDefaults();
      });

      // Cache 默认值
      expect(result.current.cacheConfig?.max_capacity).toBe(1000);
      expect(result.current.cacheConfig?.ttl).toBe(300);
      expect(result.current.cacheConfig?.l2_enabled).toBe(false);
      expect(result.current.cacheConfig?.l2_capacity).toBe(10000);

      // Search 默认值
      expect(result.current.searchConfig?.default_max_results).toBe(1000);
      expect(result.current.searchConfig?.cache_enabled).toBe(true);
      expect(result.current.searchConfig?.cache_size).toBe(1000);
      expect(result.current.searchConfig?.search_timeout).toBe(30);
      expect(result.current.searchConfig?.enable_regex_engine).toBe(true);
      expect(result.current.searchConfig?.enable_filter_engine).toBe(true);

      // TaskManager 默认值
      expect(result.current.taskManagerConfig?.max_concurrent_tasks).toBe(10);
      expect(result.current.taskManagerConfig?.task_timeout).toBe(300);
      expect(result.current.taskManagerConfig?.log_level).toBe('info');
      expect(result.current.taskManagerConfig?.debug_mode).toBe(false);
      expect(result.current.taskManagerConfig?.enable_profiling).toBe(false);
    });
  });
});
