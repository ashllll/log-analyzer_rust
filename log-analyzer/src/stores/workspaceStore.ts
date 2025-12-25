/**
 * 工作区状态 Store - 使用 Zustand + Immer
 */

import { create } from 'zustand';
import { devtools, subscribeWithSelector } from 'zustand/middleware';
import { immer } from 'zustand/middleware/immer';

// ============================================================================
// Types
// ============================================================================

export interface Workspace {
  id: string;
  name: string;
  path: string;
  status: 'READY' | 'SCANNING' | 'OFFLINE' | 'PROCESSING';
  size: string;
  files: number;
  watching?: boolean;
  format?: 'traditional' | 'cas' | 'unknown';
  needsMigration?: boolean;
}

interface WorkspaceState {
  // State
  workspaces: Workspace[];
  loading: boolean;
  error: string | null;
  
  // Actions
  setWorkspaces: (workspaces: Workspace[]) => void;
  addWorkspace: (workspace: Workspace) => void;
  updateWorkspace: (id: string, updates: Partial<Workspace>) => void;
  deleteWorkspace: (id: string) => void;
  setLoading: (loading: boolean) => void;
  setError: (error: string | null) => void;
}

// ============================================================================
// Store
// ============================================================================

export const useWorkspaceStore = create<WorkspaceState>()(
  devtools(
    subscribeWithSelector(
      immer((set) => ({
        // Initial State
        workspaces: [],
        loading: false,
        error: null,
        
        // Actions
        setWorkspaces: (workspaces) => set((state) => {
          state.workspaces = workspaces;
        }),
        
        addWorkspace: (workspace) => set((state) => {
          state.workspaces.push(workspace);
        }),
        
        updateWorkspace: (id, updates) => set((state) => {
          const index = state.workspaces.findIndex(w => w.id === id);
          if (index !== -1) {
            Object.assign(state.workspaces[index], updates);
          }
        }),
        
        deleteWorkspace: (id) => set((state) => {
          state.workspaces = state.workspaces.filter(w => w.id !== id);
        }),
        
        setLoading: (loading) => set((state) => {
          state.loading = loading;
        }),
        
        setError: (error) => set((state) => {
          state.error = error;
        }),
      }))
    ),
    { name: 'workspace-store' }
  )
);

