/**
 * 搜索执行状态 Hook
 * 封装 useReducer 管理 isSearching / searchSummary / keywordStats
 */
import { useReducer } from 'react';
import type { SearchResultSummary, KeywordStat } from '../../../types/search';

export interface SearchExecState {
  isSearching: boolean;
  searchSummary: SearchResultSummary | null;
  keywordStats: KeywordStat[];
}

export type SearchExecAction =
  | { type: 'START' }
  | { type: 'SUMMARY'; summary: SearchResultSummary; keywordColors: string[] }
  | { type: 'COMPLETE' }
  | { type: 'ERROR' }
  | { type: 'RESET' };

export const searchExecInitial: SearchExecState = {
  isSearching: false,
  searchSummary: null,
  keywordStats: [],
};

export function searchExecReducer(state: SearchExecState, action: SearchExecAction): SearchExecState {
  switch (action.type) {
    case 'START':
      return { isSearching: true, searchSummary: null, keywordStats: [] };
    case 'SUMMARY': {
      const stats: KeywordStat[] = action.summary.keywordStats.map((stat, i) => ({
        ...stat,
        color: action.keywordColors[i % action.keywordColors.length],
      }));
      return { ...state, searchSummary: action.summary, keywordStats: stats };
    }
    case 'COMPLETE':
      return { ...state, isSearching: false };
    case 'ERROR':
      return { ...state, isSearching: false };
    case 'RESET':
      return searchExecInitial;
  }
}

export function useSearchState() {
  return useReducer(searchExecReducer, searchExecInitial);
}
