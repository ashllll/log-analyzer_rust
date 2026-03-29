/**
 * 关键词状态 Store - 使用 Zustand + Immer
 */

import { create } from 'zustand';
import { devtools, persist, subscribeWithSelector } from 'zustand/middleware';
import { immer } from 'zustand/middleware/immer';

import type { KeywordGroup } from './types';

// ============================================================================
// Types
// ============================================================================

export type { ColorKey, KeywordGroup, KeywordPattern } from './types';

interface KeywordState {
  // State
  keywordGroups: KeywordGroup[];
  loading: boolean;
  error: string | null;
  
  // Actions
  setKeywordGroups: (groups: KeywordGroup[]) => void;
  addKeywordGroup: (group: KeywordGroup) => void;
  updateKeywordGroup: (group: KeywordGroup) => void;
  deleteKeywordGroup: (id: string) => void;
  toggleKeywordGroup: (id: string) => void;
  setLoading: (loading: boolean) => void;
  setError: (error: string | null) => void;
}

// ============================================================================
// Store
// ============================================================================

export const useKeywordStore = create<KeywordState>()(
  devtools(
    persist(
      subscribeWithSelector(
        immer((set) => ({
          // Initial State
          keywordGroups: [],
          loading: false,
          error: null,

          // Actions
          setKeywordGroups: (groups) => set((state) => {
            state.keywordGroups = groups;
          }),

          addKeywordGroup: (group) => set((state) => {
            state.keywordGroups.push(group);
          }),

          updateKeywordGroup: (group) => set((state) => {
            const index = state.keywordGroups.findIndex(g => g.id === group.id);
            if (index !== -1) {
              state.keywordGroups[index] = group;
            }
          }),

          deleteKeywordGroup: (id) => set((state) => {
            state.keywordGroups = state.keywordGroups.filter(g => g.id !== id);
          }),

          toggleKeywordGroup: (id) => set((state) => {
            const group = state.keywordGroups.find(g => g.id === id);
            if (group) {
              group.enabled = !group.enabled;
            }
          }),

          setLoading: (loading) => set((state) => {
            state.loading = loading;
          }),

          setError: (error) => set((state) => {
            state.error = error;
          }),
        }))
      ),
      {
        name: 'log-analyzer-keywords',
        // 仅持久化关键词数据，不持久化临时状态（loading/error）
        partialize: (state) => ({
          keywordGroups: state.keywordGroups,
        }),
      }
    ),
    { name: 'keyword-store' }
  )
);

