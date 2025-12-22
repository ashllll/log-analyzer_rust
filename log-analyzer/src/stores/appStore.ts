/**
 * 应用全局状态 Store - 使用 Zustand + Immer
 * 
 * 替换原有的 Context + Reducer 模式,提供更好的性能和开发体验
 */

import { create } from 'zustand';
import { devtools, subscribeWithSelector } from 'zustand/middleware';
import { immer } from 'zustand/middleware/immer';
import toast from 'react-hot-toast';

// ============================================================================
// Types
// ============================================================================

export type Page = 'search' | 'keywords' | 'workspaces' | 'tasks' | 'settings' | 'performance-monitoring';
export type ToastType = 'success' | 'error' | 'info';

export interface Toast {
  id: number;
  type: ToastType;
  message: string;
}

interface AppState {
  // State
  page: Page;
  toasts: Toast[];
  activeWorkspaceId: string | null;
  
  // Actions
  setPage: (page: Page) => void;
  addToast: (type: ToastType, message: string) => void;
  removeToast: (id: number) => void;
  setActiveWorkspace: (id: string | null) => void;
}

// ============================================================================
// Store
// ============================================================================

export const useAppStore = create<AppState>()(
  devtools(
    subscribeWithSelector(
      immer((set) => ({
        // Initial State
        page: 'workspaces',
        toasts: [],
        activeWorkspaceId: null,
        
        // Actions
        setPage: (page) => set((state) => {
          state.page = page;
        }),
        
        addToast: (type, message) => {
          // 使用 react-hot-toast 替代自定义 Toast
          const duration = type === 'error' ? 4000 : 3000;
          
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
        },
        
        removeToast: (id) => set((state) => {
          state.toasts = state.toasts.filter(t => t.id !== id);
        }),
        
        setActiveWorkspace: (id) => set((state) => {
          state.activeWorkspaceId = id;
        }),
      }))
    ),
    { name: 'app-store' }
  )
);

