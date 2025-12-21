import { create } from 'zustand';
import { subscribeWithSelector, devtools } from 'zustand/middleware';
import { immer } from 'zustand/middleware/immer';
import { invoke } from '@tauri-apps/api/core';

import { logger } from '../utils/logger';

// ============================================================================
// Types (imported from original AppContext)
// ============================================================================

export type Page = 'search' | 'keywords' | 'workspaces' | 'tasks' | 'settings';
export type ColorKey = 'blue' | 'green' | 'red' | 'orange' | 'purple';
export type ToastType = 'success' | 'error' | 'info';

export interface KeywordPattern {
  regex: string;
  comment: string;
}

export interface KeywordGroup {
  id: string;
  name: string;
  color: ColorKey;
  patterns: KeywordPattern[];
  enabled: boolean;
}

export interface Workspace {
  id: string;
  name: string;
  path: string;
  status: 'READY' | 'SCANNING' | 'OFFLINE' | 'PROCESSING';
  size: string;
  files: number;
  watching?: boolean;
}

export interface Task {
  id: string;
  type: string;
  target: string;
  progress: number;
  message: string;
  status: 'RUNNING' | 'COMPLETED' | 'FAILED' | 'STOPPED';
  workspaceId?: string;
}

export interface Toast {
  id: number;
  type: ToastType;
  message: string;
}

// ============================================================================
// Store Interface
// ============================================================================

interface AppStore {
  // State
  page: Page;
  toasts: Toast[];
  activeWorkspaceId: string | null;
  
  // Workspace State
  workspaces: Workspace[];
  workspacesLoading: boolean;
  workspacesError: string | null;
  
  // Keyword State
  keywordGroups: KeywordGroup[];
  keywordsLoading: boolean;
  keywordsError: string | null;
  
  // Task State
  tasks: Task[];
  tasksLoading: boolean;
  tasksError: string | null;
  
  // App Actions
  setPage: (page: Page) => void;
  addToast: (type: ToastType, message: string) => number;
  removeToast: (id: number) => void;
  setActiveWorkspace: (id: string | null) => void;
  
  // Workspace Actions with built-in deduplication
  setWorkspaces: (workspaces: Workspace[]) => void;
  addWorkspace: (workspace: Workspace) => void;
  updateWorkspace: (id: string, updates: Partial<Workspace>) => void;
  deleteWorkspace: (id: string) => void;
  setWorkspacesLoading: (loading: boolean) => void;
  setWorkspacesError: (error: string | null) => void;
  
  // Keyword Actions
  setKeywordGroups: (groups: KeywordGroup[]) => void;
  addKeywordGroup: (group: KeywordGroup) => void;
  updateKeywordGroup: (group: KeywordGroup) => void;
  deleteKeywordGroup: (id: string) => void;
  toggleKeywordGroup: (id: string) => void;
  setKeywordsLoading: (loading: boolean) => void;
  setKeywordsError: (error: string | null) => void;
  
  // Task Actions with deduplication
  setTasks: (tasks: Task[]) => void;
  addTaskIfNotExists: (task: Task) => void;
  updateTask: (id: string, updates: Partial<Task>) => void;
  deleteTask: (id: string) => void;
  setTasksLoading: (loading: boolean) => void;
  setTasksError: (error: string | null) => void;
  
  // Async actions
  loadConfig: () => Promise<void>;
  saveConfig: () => Promise<void>;
}

// ============================================================================
// Store Implementation
// ============================================================================

// Toast ID counter for generating unique IDs
let toastIdCounter = 0;

// Toast timers map for auto-removal
const toastTimers = new Map<number, NodeJS.Timeout>();

export const useAppStore = create<AppStore>()(
  devtools(
    subscribeWithSelector(
      immer((set, get) => ({
        // Initial State
        page: 'workspaces' as Page,
        toasts: [],
        activeWorkspaceId: null,
        
        workspaces: [],
        workspacesLoading: false,
        workspacesError: null,
        
        keywordGroups: [],
        keywordsLoading: false,
        keywordsError: null,
        
        tasks: [],
        tasksLoading: false,
        tasksError: null,
        
        // App Actions
        setPage: (page) => set((state) => {
          state.page = page;
        }),
        
        addToast: (type, message) => {
          // Use monotonically increasing counter for unique IDs
          const id = ++toastIdCounter;
          set((state) => {
            state.toasts.push({ id, type, message });
          });
          
          // Auto-remove toast after 3 seconds
          const timer = setTimeout(() => {
            toastTimers.delete(id);
            get().removeToast(id);
          }, 3000);
          
          toastTimers.set(id, timer);
          
          // Return the ID so it can be used for removal
          return id;
        },
        
        removeToast: (id) => {
          // Clear the timer if it exists
          const timer = toastTimers.get(id);
          if (timer) {
            clearTimeout(timer);
            toastTimers.delete(id);
          }
          
          set((state) => {
            const index = state.toasts.findIndex(t => t.id === id);
            if (index !== -1) {
              state.toasts.splice(index, 1);
            }
          });
        },
        
        setActiveWorkspace: (id) => set((state) => {
          state.activeWorkspaceId = id;
        }),
        
        // Workspace Actions
        setWorkspaces: (workspaces) => set((state) => {
          state.workspaces = workspaces;
        }),
        
        addWorkspace: (workspace) => set((state) => {
          // Check for duplicates before adding
          const exists = state.workspaces.some(w => w.id === workspace.id);
          if (!exists) {
            state.workspaces.push(workspace);
          }
        }),
        
        updateWorkspace: (id, updates) => set((state) => {
          const index = state.workspaces.findIndex(w => w.id === id);
          if (index !== -1) {
            Object.assign(state.workspaces[index], updates);
          }
        }),
        
        deleteWorkspace: (id) => set((state) => {
          const index = state.workspaces.findIndex(w => w.id === id);
          if (index !== -1) {
            state.workspaces.splice(index, 1);
          }
        }),
        
        setWorkspacesLoading: (loading) => set((state) => {
          state.workspacesLoading = loading;
        }),
        
        setWorkspacesError: (error) => set((state) => {
          state.workspacesError = error;
        }),
        
        // Keyword Actions
        setKeywordGroups: (groups) => set((state) => {
          state.keywordGroups = groups;
        }),
        
        addKeywordGroup: (group) => set((state) => {
          // Check for duplicates before adding
          const exists = state.keywordGroups.some(g => g.id === group.id);
          if (!exists) {
            state.keywordGroups.push(group);
          }
        }),
        
        updateKeywordGroup: (group) => set((state) => {
          const index = state.keywordGroups.findIndex(g => g.id === group.id);
          if (index !== -1) {
            state.keywordGroups[index] = group;
          }
        }),
        
        deleteKeywordGroup: (id) => set((state) => {
          const index = state.keywordGroups.findIndex(g => g.id === id);
          if (index !== -1) {
            state.keywordGroups.splice(index, 1);
          }
        }),
        
        toggleKeywordGroup: (id) => set((state) => {
          const index = state.keywordGroups.findIndex(g => g.id === id);
          if (index !== -1) {
            state.keywordGroups[index].enabled = !state.keywordGroups[index].enabled;
          }
        }),
        
        setKeywordsLoading: (loading) => set((state) => {
          state.keywordsLoading = loading;
        }),
        
        setKeywordsError: (error) => set((state) => {
          state.keywordsError = error;
        }),
        
        // Task Actions with deduplication
        setTasks: (tasks) => set((state) => {
          state.tasks = tasks;
        }),
        
        addTaskIfNotExists: (task) => set((state) => {
          const exists = state.tasks.some(t => t.id === task.id);
          if (!exists) {
            state.tasks.push(task);
            logger.debug('[STORE] Added new task:', task.id);
          } else {
            logger.debug('[STORE] Task already exists, skipping:', task.id);
          }
        }),
        
        updateTask: (id, updates) => set((state) => {
          const index = state.tasks.findIndex(t => t.id === id);
          if (index !== -1) {
            Object.assign(state.tasks[index], updates);
            logger.debug('[STORE] Updated task:', id, updates);
          }
        }),
        
        deleteTask: (id) => set((state) => {
          const index = state.tasks.findIndex(t => t.id === id);
          if (index !== -1) {
            state.tasks.splice(index, 1);
          }
        }),
        
        setTasksLoading: (loading) => set((state) => {
          state.tasksLoading = loading;
        }),
        
        setTasksError: (error) => set((state) => {
          state.tasksError = error;
        }),
        
        // Async Actions
        loadConfig: async () => {
          try {
            const config = await invoke<any>('load_config');
            set((state) => {
              if (config.keyword_groups) {
                state.keywordGroups = config.keyword_groups;
              }
              if (config.workspaces) {
                state.workspaces = config.workspaces;
              }
            });
          } catch (e) {
            logger.error('Failed to load config:', e);
            set((state) => {
              state.workspacesError = 'Failed to load configuration';
              state.keywordsError = 'Failed to load configuration';
            });
          }
        },
        
        saveConfig: async () => {
          const { keywordGroups, workspaces } = get();
          
          // Skip saving if no data
          if (keywordGroups.length === 0 && workspaces.length === 0) {
            return;
          }
          
          try {
            await invoke('save_config', {
              config: {
                keyword_groups: keywordGroups,
                workspaces: workspaces
              }
            });
            logger.debug('[STORE] Configuration saved successfully');
          } catch (e) {
            logger.error('Failed to save config:', e);
            set((state) => {
              state.addToast('error', 'Failed to save configuration');
            });
          }
        },
        
        // Note: Event listeners are now managed by the EventManager React component
        // This ensures proper cleanup using React's useEffect patterns
      }))
    ),
    { name: 'app-store' }
  )
);

// ============================================================================
// Note: Debounced config saving is now handled by useConfigManager hook
// using React's built-in patterns for better resource management
// ============================================================================

// ============================================================================
// Note: Event listeners are now managed by the EventManager React component
// Configuration loading is handled by React Query in useConfigQuery
// ============================================================================