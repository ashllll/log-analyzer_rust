/**
 * 应用全局状态 Store - 使用 Zustand + Immer
 * 
 * 替换原有的 Context + Reducer 模式,提供更好的性能和开发体验
 */

import { create } from 'zustand';
import { devtools, subscribeWithSelector } from 'zustand/middleware';
import { immer } from 'zustand/middleware/immer';
import toast from 'react-hot-toast';

import type { Toast, ToastType } from './types';

// ============================================================================
// Types
// ============================================================================

export type { Toast, ToastType } from './types';

export interface AppState {
  // State
  toasts: Toast[];
  activeWorkspaceId: string | null;
  isInitialized: boolean;
  initializationError: string | null;
  
  // Actions
  addToast: (type: ToastType, message: string) => void;
  removeToast: (id: number) => void;
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
        toasts: [],
        activeWorkspaceId: null,
        isInitialized: false,
        initializationError: null,
        
        // Actions
        addToast: (type, message) => {
          const id = Date.now();
          const duration = type === 'error' ? 4000 : 3000;

          // 写入 Zustand 状态，使 removeToast 和订阅者能正确感知
          set((state) => {
            state.toasts.push({ id, type, message });
          });

          // 显示 react-hot-toast UI（需在 immer set 外执行）
          switch (type) {
            case 'success':
              toast.success(message, { duration });
              break;
            case 'error':
              toast.error(message, { duration });
              break;
            case 'info':
              toast(message, { duration, icon: 'ℹ️' });
              break;
          }

          // TTL 到期后自动从 Zustand 状态移除
          setTimeout(() => {
            set((state) => {
              state.toasts = state.toasts.filter((t) => t.id !== id);
            });
          }, duration);
        },
        
        removeToast: (id) => set((state) => {
          state.toasts = state.toasts.filter(t => t.id !== id);
        }),
        
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
