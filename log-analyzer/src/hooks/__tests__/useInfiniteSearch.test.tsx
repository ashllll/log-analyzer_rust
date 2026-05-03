/**
 * useInfiniteSearch 单元测试
 * 测试流式无限搜索 Hook 的功能
 */

import { renderHook } from '@testing-library/react';
import { useInfiniteSearch } from '../useInfiniteSearch';

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

      const _result = renderHook(() =>
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

  describe('searchQueryKeys', () => {
    it('should export searchQueryKeys factory', async () => {
      const { searchQueryKeys } = require('../useInfiniteSearch');

      const key = searchQueryKeys.infinite('search-1', 'error', 'workspace-1');
      expect(key).toEqual(['search', 'infinite', 'search-1', 'error', 'workspace-1']);
    });
  });
});
