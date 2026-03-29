/**
 * 应用全局状态 Store - 使用 Zustand + Immer
 *
 * 替换原有的 Context + Reducer 模式,提供更好的性能和开发体验
 */

import { create } from 'zustand';
import { devtools, subscribeWithSelector } from 'zustand/middleware';
import { immer } from 'zustand/middleware/immer';

// ============================================================================
// Types
// ============================================================================

export interface AppState {
  // State
  activeWorkspaceId: string | null;
  isInitialized: boolean;
  initializationError: string | null;

  // Actions
  setActiveWorkspace: (id: string | null) => void;
  setInitialized: (initialized: boolean, error?: string | null) => void;
}

// ============================================================================
// Store
// ============================================================================

export const useAppStore = create<AppState>()(
  devtools(
    subscribeWithSelector(
      immer((set) => ({
        // Initial State
        activeWorkspaceId: null,
        isInitialized: false,
        initializationError: null,

        // Actions
        setActiveWorkspace: (id) => set((state) => {
          state.activeWorkspaceId = id;
        }),

        setInitialized: (initialized, error = null) => set((state) => {
          state.isInitialized = initialized;
          state.initializationError = error;
        }),
      }))
    ),
    { name: 'app-store' }
  )
);
