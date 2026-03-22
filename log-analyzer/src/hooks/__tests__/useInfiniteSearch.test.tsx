/**
 * useInfiniteSearch 单元测试
 * 测试流式无限搜索 Hook 的功能
 */

import { renderHook, waitFor } from '@testing-library/react';
import { useInfiniteSearch, registerSearchSession, removeSearchSession, getSearchSessionInfo, getVirtualSearchStats } from '../useInfiniteSearch';

// Mock @tanstack/react-query
jest.mock('@tanstack/react-query', () => ({
  useInfiniteQuery: jest.fn(),
  InfiniteData: {}
}));

// Mock @tauri-apps/api/core
jest.mock('@tauri-apps/api/core', () => ({
  invoke: jest.fn()
}));

// Mock logger
jest.mock('../../utils/logger', () => ({
  logger: {
    debug: jest.fn(),
    error: jest.fn(),
    info: jest.fn(),
    warn: jest.fn(),
  },
}));

import { useInfiniteQuery } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';

describe('useInfiniteSearch', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  describe('useInfiniteSearch hook', () => {
    it('should call useInfiniteQuery with correct parameters', () => {
      const mockUseInfiniteQuery = useInfiniteQuery as jest.Mock;
      mockUseInfiniteQuery.mockReturnValue({
        data: undefined,
        fetchNextPage: jest.fn(),
        hasNextPage: false,
        isFetchingNextPage: false,
        isLoading: false,
        error: null
      });

      const { result } = renderHook(() =>
        useInfiniteSearch({
          searchId: 'test-search',
          query: 'error',
          workspaceId: 'workspace-1',
          enabled: true,
          pageSize: 100,
          staleTime: 60000
        })
      );

      expect(mockUseInfiniteQuery).toHaveBeenCalled();
      const callArg = mockUseInfiniteQuery.mock.calls[0][0];
      expect(callArg.queryKey).toEqual(['search', 'infinite', 'test-search', 'error', 'workspace-1']);
      expect(callArg.getNextPageParam).toBeDefined();
      expect(callArg.initialPageParam).toBe(0);
    });

    it('should pass correct options to useInfiniteQuery', () => {
      const mockUseInfiniteQuery = useInfiniteQuery as jest.Mock;
      mockUseInfiniteQuery.mockReturnValue({
        data: undefined,
        fetchNextPage: jest.fn(),
        hasNextPage: false,
        isFetchingNextPage: false,
        isLoading: false,
        error: null
      });

      renderHook(() =>
        useInfiniteSearch({
          searchId: 'test-search',
          query: 'error',
          workspaceId: 'workspace-1',
          enabled: true,
          pageSize: 500,
          staleTime: 120000
        })
      );

      const callArg = mockUseInfiniteQuery.mock.calls[0][0];
      expect(callArg.staleTime).toBe(120000);
    });
  });

  describe('registerSearchSession', () => {
    it('should call invoke with correct parameters', async () => {
      (invoke as jest.Mock).mockResolvedValue('new-search-id');

      const result = await registerSearchSession('search-1', 'error', []);

      expect(invoke).toHaveBeenCalledWith('register_search_session', {
        searchId: 'search-1',
        query: 'error',
        entries: []
      });
      expect(result).toBe('new-search-id');
    });

    it('should pass entries to invoke', async () => {
      const mockEntries = [
        { id: '1', content: 'error line 1' },
        { id: '2', content: 'error line 2' }
      ];
      (invoke as jest.Mock).mockResolvedValue('search-id');

      await registerSearchSession('search-1', 'error', mockEntries as any);

      expect(invoke).toHaveBeenCalledWith('register_search_session', {
        searchId: 'search-1',
        query: 'error',
        entries: mockEntries
      });
    });
  });

  describe('removeSearchSession', () => {
    it('should call invoke with correct parameters', async () => {
      (invoke as jest.Mock).mockResolvedValue(true);

      const result = await removeSearchSession('search-1');

      expect(invoke).toHaveBeenCalledWith('remove_search_session', {
        searchId: 'search-1'
      });
      expect(result).toBe(true);
    });

    it('should return false when invoke returns false', async () => {
      (invoke as jest.Mock).mockResolvedValue(false);

      const result = await removeSearchSession('search-1');

      expect(result).toBe(false);
    });
  });

  describe('getSearchSessionInfo', () => {
    it('should call invoke with correct parameters', async () => {
      const mockInfo = {
        search_id: 'search-1',
        query: 'error',
        total_count: 100
      };
      (invoke as jest.Mock).mockResolvedValue(mockInfo);

      const result = await getSearchSessionInfo('search-1');

      expect(invoke).toHaveBeenCalledWith('get_search_session_info', {
        searchId: 'search-1'
      });
      expect(result).toEqual(mockInfo);
    });

    it('should return null when invoke returns null', async () => {
      (invoke as jest.Mock).mockResolvedValue(null);

      const result = await getSearchSessionInfo('search-1');

      expect(result).toBeNull();
    });
  });

  describe('getVirtualSearchStats', () => {
    it('should call invoke and return stats', async () => {
      const mockStats = {
        active_sessions: 5,
        total_cached_entries: 10000,
        max_sessions: 100,
        max_entries_per_session: 5000,
        session_ttl_seconds: 300
      };
      (invoke as jest.Mock).mockResolvedValue(mockStats);

      const result = await getVirtualSearchStats();

      expect(invoke).toHaveBeenCalledWith('get_virtual_search_stats');
      expect(result).toEqual(mockStats);
    });
  });

  describe('searchQueryKeys', () => {
    it('should export searchQueryKeys factory', async () => {
      const { searchQueryKeys } = require('../useInfiniteSearch');

      const key = searchQueryKeys.infinite('search-1', 'error', 'workspace-1');
      expect(key).toEqual(['search', 'infinite', 'search-1', 'error', 'workspace-1']);
    });
  });
});
