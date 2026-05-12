/**
 * 应用全局状态 Store - 使用 Zustand + Immer
 *
 * 替换原有的 Context + Reducer 模式,提供更好的性能和开发体验
 */

import { create } from 'zustand';
import { devtools, persist, subscribeWithSelector } from 'zustand/middleware';
import { immer } from 'zustand/middleware/immer';

// ============================================================================
// Types
// ============================================================================

export type InitPhase = 'idle' | 'loading' | 'ready' | 'error';

export interface AppState {
  // State
  activeWorkspaceId: string | null;
  isInitialized: boolean;
  initializationError: string | null;
  initPhase: InitPhase;

  // Actions
  setActiveWorkspace: (id: string | null) => void;
  setInitialized: (initialized: boolean, error?: string | null) => void;
  setInitPhase: (phase: InitPhase) => void;
}

// ============================================================================
// Store
// ============================================================================

export const useAppStore = create<AppState>()(
  devtools(
    persist(
      subscribeWithSelector(
        immer((set) => ({
          // Initial State
          activeWorkspaceId: null,
          isInitialized: false,
          initializationError: null,
          initPhase: 'idle',

          // Actions
          setActiveWorkspace: (id) => set((state) => {
            state.activeWorkspaceId = id;
          }),

          setInitialized: (initialized, error = null) => set((state) => {
            state.isInitialized = initialized;
            state.initializationError = error;
          }),

          setInitPhase: (phase) => set((state) => {
            state.initPhase = phase;
          }),
        }))
      ),
      {
        name: 'log-analyzer-app',
        version: 1,
        // 仅持久化用户选择的工作区ID，不持久化临时状态
        partialize: (state) => ({
          activeWorkspaceId: state.activeWorkspaceId,
        }),
      }
    ),
    { name: 'app-store' }
  )
);
