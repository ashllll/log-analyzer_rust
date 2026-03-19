/**
 * useSearchState Hook 单元测试
 * 验证搜索状态 reducer 和 hook 的行为
 */

import { renderHook, act } from '@testing-library/react';
import { useSearchState, searchExecReducer, searchExecInitial, SearchExecState } from '../useSearchState';

// Mock keywordColors for tests
const mockKeywordColors = ['#3B82F6', '#8B5CF6', '#22C55E', '#F59E0B', '#EC4899', '#06B6D4'];

// Mock SearchResultSummary
const createMockSummary = (overrides = {}) => ({
  totalMatches: 100,
  keywordStats: [
    { keyword: 'error', matchCount: 50, matchPercentage: 50 },
    { keyword: 'warning', matchCount: 30, matchPercentage: 30 },
  ],
  searchDurationMs: 150,
  truncated: false,
  ...overrides,
});

describe('useSearchState', () => {
  describe('searchExecReducer', () => {
    it('should handle START action - set isSearching true', () => {
      const initialState: SearchExecState = {
        isSearching: false,
        searchSummary: null,
        keywordStats: [],
      };

      const action = { type: 'START' as const };
      const newState = searchExecReducer(initialState, action);

      expect(newState.isSearching).toBe(true);
      expect(newState.searchSummary).toBe(null);
      expect(newState.keywordStats).toEqual([]);
    });

    it('should handle START action - reset existing state', () => {
      const existingState: SearchExecState = {
        isSearching: false,
        searchSummary: createMockSummary(),
        keywordStats: [
          { keyword: 'error', matchCount: 50, matchPercentage: 50, color: '#3B82F6' },
        ],
      };

      const action = { type: 'START' as const };
      const newState = searchExecReducer(existingState, action);

      expect(newState.isSearching).toBe(true);
      expect(newState.searchSummary).toBe(null);
      expect(newState.keywordStats).toEqual([]);
    });

    it('should handle SUMMARY action - update summary and stats with colors', () => {
      const initialState: SearchExecState = {
        isSearching: true,
        searchSummary: null,
        keywordStats: [],
      };

      const summary = createMockSummary();
      const action = {
        type: 'SUMMARY' as const,
        summary,
        keywordColors: mockKeywordColors,
      };

      const newState = searchExecReducer(initialState, action);

      expect(newState.isSearching).toBe(true);
      expect(newState.searchSummary).toEqual(summary);
      expect(newState.keywordStats).toHaveLength(2);
      expect(newState.keywordStats[0].keyword).toBe('error');
      expect(newState.keywordStats[0].color).toBe('#3B82F6'); // First color
      expect(newState.keywordStats[1].keyword).toBe('warning');
      expect(newState.keywordStats[1].color).toBe('#8B5CF6'); // Second color
    });

    it('should handle SUMMARY action - cycle colors when more stats than colors', () => {
      const initialState: SearchExecState = {
        isSearching: true,
        searchSummary: null,
        keywordStats: [],
      };

      const summary = createMockSummary({
        keywordStats: [
          { keyword: 'a', matchCount: 10, matchPercentage: 10 },
          { keyword: 'b', matchCount: 10, matchPercentage: 10 },
          { keyword: 'c', matchCount: 10, matchPercentage: 10 },
          { keyword: 'd', matchCount: 10, matchPercentage: 10 },
          { keyword: 'e', matchCount: 10, matchPercentage: 10 },
          { keyword: 'f', matchCount: 10, matchPercentage: 10 },
          { keyword: 'g', matchCount: 10, matchPercentage: 10 },
        ],
      });

      const shortColors = ['#FF0000', '#00FF00', '#0000FF'];
      const action = {
        type: 'SUMMARY' as const,
        summary,
        keywordColors: shortColors,
      };

      const newState = searchExecReducer(initialState, action);

      // 7 items with 3 colors: index 0->0, 1->1, 2->2, 3->0, 4->1, 5->2, 6->0
      expect(newState.keywordStats[0].color).toBe('#FF0000');
      expect(newState.keywordStats[3].color).toBe('#FF0000');
      expect(newState.keywordStats[6].color).toBe('#FF0000');
    });

    it('should handle COMPLETE action - set isSearching false', () => {
      const searchingState: SearchExecState = {
        isSearching: true,
        searchSummary: createMockSummary(),
        keywordStats: [],
      };

      const action = { type: 'COMPLETE' as const };
      const newState = searchExecReducer(searchingState, action);

      expect(newState.isSearching).toBe(false);
      expect(newState.searchSummary).toEqual(searchingState.searchSummary); // Preserved
      expect(newState.keywordStats).toEqual(searchingState.keywordStats); // Preserved
    });

    it('should handle ERROR action - set isSearching false', () => {
      const searchingState: SearchExecState = {
        isSearching: true,
        searchSummary: createMockSummary(),
        keywordStats: [],
      };

      const action = { type: 'ERROR' as const };
      const newState = searchExecReducer(searchingState, action);

      expect(newState.isSearching).toBe(false);
      expect(newState.searchSummary).toEqual(searchingState.searchSummary); // Preserved
    });

    it('should handle RESET action - return initial state', () => {
      const modifiedState: SearchExecState = {
        isSearching: false,
        searchSummary: createMockSummary(),
        keywordStats: [
          { keyword: 'error', matchCount: 50, matchPercentage: 50, color: '#3B82F6' },
        ],
      };

      const action = { type: 'RESET' as const };
      const newState = searchExecReducer(modifiedState, action);

      expect(newState).toEqual(searchExecInitial);
      expect(newState.isSearching).toBe(false);
      expect(newState.searchSummary).toBe(null);
      expect(newState.keywordStats).toEqual([]);
    });

    it('should return undefined for unknown action type', () => {
      const currentState: SearchExecState = {
        isSearching: true,
        searchSummary: createMockSummary(),
        keywordStats: [],
      };

      // @ts-expect-error - testing with invalid action type
      const action = { type: 'UNKNOWN' as const };
      const newState = searchExecReducer(currentState, action);

      // Without a default case, reducer returns undefined
      expect(newState).toBeUndefined();
    });
  });

  describe('initial state', () => {
    it('should have correct default values', () => {
      expect(searchExecInitial.isSearching).toBe(false);
      expect(searchExecInitial.searchSummary).toBe(null);
      expect(searchExecInitial.keywordStats).toEqual([]);
    });

    it('should be immutable when reducer runs', () => {
      const action = { type: 'START' as const };
      const newState = searchExecReducer(searchExecInitial, action);

      // Initial state should remain unchanged
      expect(searchExecInitial.isSearching).toBe(false);
      expect(searchExecInitial.searchSummary).toBe(null);

      // New state should be different
      expect(newState.isSearching).toBe(true);
    });
  });

  describe('useSearchState hook', () => {
    it('should return initial state on mount', () => {
      const { result } = renderHook(() => useSearchState());

      expect(result.current[0]).toEqual(searchExecInitial);
      expect(typeof result.current[1]).toBe('function'); // dispatch function
    });

    it('should update state when dispatching START', () => {
      const { result } = renderHook(() => useSearchState());
      const [, dispatch] = result.current;

      act(() => {
        dispatch({ type: 'START' });
      });

      expect(result.current[0].isSearching).toBe(true);
      expect(result.current[0].searchSummary).toBe(null);
    });

    it('should update state when dispatching SUMMARY', () => {
      const { result } = renderHook(() => useSearchState());
      const [, dispatch] = result.current;

      const summary = createMockSummary();

      act(() => {
        dispatch({ type: 'SUMMARY', summary, keywordColors: mockKeywordColors });
      });

      expect(result.current[0].searchSummary).toEqual(summary);
      expect(result.current[0].keywordStats).toHaveLength(2);
      expect(result.current[0].keywordStats[0].color).toBe('#3B82F6');
    });

    it('should update state when dispatching COMPLETE', () => {
      const { result } = renderHook(() => useSearchState());
      const [, dispatch] = result.current;

      // First set searching state
      act(() => {
        dispatch({ type: 'START' });
      });

      expect(result.current[0].isSearching).toBe(true);

      // Then complete
      act(() => {
        dispatch({ type: 'COMPLETE' });
      });

      expect(result.current[0].isSearching).toBe(false);
    });

    it('should update state when dispatching ERROR', () => {
      const { result } = renderHook(() => useSearchState());
      const [, dispatch] = result.current;

      act(() => {
        dispatch({ type: 'START' });
      });

      expect(result.current[0].isSearching).toBe(true);

      act(() => {
        dispatch({ type: 'ERROR' });
      });

      expect(result.current[0].isSearching).toBe(false);
    });

    it('should reset state when dispatching RESET', () => {
      const { result } = renderHook(() => useSearchState());
      const [, dispatch] = result.current;

      // Set some state
      act(() => {
        dispatch({ type: 'START' });
        dispatch({
          type: 'SUMMARY',
          summary: createMockSummary(),
          keywordColors: mockKeywordColors,
        });
      });

      expect(result.current[0].isSearching).toBe(true);
      expect(result.current[0].searchSummary).not.toBeNull();

      // Reset
      act(() => {
        dispatch({ type: 'RESET' });
      });

      expect(result.current[0]).toEqual(searchExecInitial);
    });
  });
});
