import { createContext, useContext, useReducer, useCallback, ReactNode, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { logger } from '../utils/logger';

// ============================================================================
// Types
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
  workspaceId?: string;  // 工作区 ID，用于匹配任务与工作区
}

export interface Toast {
  id: number;
  type: ToastType;
  message: string;
}

// ============================================================================
// Global App State
// ============================================================================

export interface AppState {
  page: Page;
  toasts: Toast[];
  activeWorkspaceId: string | null;
}

export type AppAction =
  | { type: 'SET_PAGE'; payload: Page }
  | { type: 'ADD_TOAST'; payload: { type: ToastType; message: string } }
  | { type: 'REMOVE_TOAST'; payload: number }
  | { type: 'SET_ACTIVE_WORKSPACE'; payload: string | null };

const appReducer = (state: AppState, action: AppAction): AppState => {
  switch (action.type) {
    case 'SET_PAGE':
      return { ...state, page: action.payload };
    
    case 'ADD_TOAST': {
      const id = Date.now();
      return { ...state, toasts: [...state.toasts, { id, ...action.payload }] };
    }
    
    case 'REMOVE_TOAST':
      return { ...state, toasts: state.toasts.filter(t => t.id !== action.payload) };
    
    case 'SET_ACTIVE_WORKSPACE':
      return { ...state, activeWorkspaceId: action.payload };
    
    default:
      return state;
  }
};

// ============================================================================
// Workspace State
// ============================================================================

export interface WorkspaceState {
  workspaces: Workspace[];
  loading: boolean;
  error: string | null;
}

export type WorkspaceAction =
  | { type: 'SET_WORKSPACES'; payload: Workspace[] }
  | { type: 'ADD_WORKSPACE'; payload: Workspace }
  | { type: 'UPDATE_WORKSPACE'; payload: { id: string; updates: Partial<Workspace> } }
  | { type: 'DELETE_WORKSPACE'; payload: string }
  | { type: 'SET_LOADING'; payload: boolean }
  | { type: 'SET_ERROR'; payload: string | null };

const workspaceReducer = (state: WorkspaceState, action: WorkspaceAction): WorkspaceState => {
  switch (action.type) {
    case 'SET_WORKSPACES':
      return { ...state, workspaces: action.payload };
    
    case 'ADD_WORKSPACE':
      return { ...state, workspaces: [...state.workspaces, action.payload] };
    
    case 'UPDATE_WORKSPACE':
      return {
        ...state,
        workspaces: state.workspaces.map(w =>
          w.id === action.payload.id ? { ...w, ...action.payload.updates } : w
        )
      };
    
    case 'DELETE_WORKSPACE':
      return {
        ...state,
        workspaces: state.workspaces.filter(w => w.id !== action.payload)
      };
    
    case 'SET_LOADING':
      return { ...state, loading: action.payload };
    
    case 'SET_ERROR':
      return { ...state, error: action.payload };
    
    default:
      return state;
  }
};

// ============================================================================
// Keyword State
// ============================================================================

export interface KeywordState {
  keywordGroups: KeywordGroup[];
  loading: boolean;
  error: string | null;
}

export type KeywordAction =
  | { type: 'SET_KEYWORD_GROUPS'; payload: KeywordGroup[] }
  | { type: 'ADD_KEYWORD_GROUP'; payload: KeywordGroup }
  | { type: 'UPDATE_KEYWORD_GROUP'; payload: KeywordGroup }
  | { type: 'DELETE_KEYWORD_GROUP'; payload: string }
  | { type: 'TOGGLE_KEYWORD_GROUP'; payload: string }
  | { type: 'SET_LOADING'; payload: boolean }
  | { type: 'SET_ERROR'; payload: string | null };

const keywordReducer = (state: KeywordState, action: KeywordAction): KeywordState => {
  switch (action.type) {
    case 'SET_KEYWORD_GROUPS':
      return { ...state, keywordGroups: action.payload };
    
    case 'ADD_KEYWORD_GROUP':
      return { ...state, keywordGroups: [...state.keywordGroups, action.payload] };
    
    case 'UPDATE_KEYWORD_GROUP':
      return {
        ...state,
        keywordGroups: state.keywordGroups.map(g =>
          g.id === action.payload.id ? action.payload : g
        )
      };
    
    case 'DELETE_KEYWORD_GROUP':
      return {
        ...state,
        keywordGroups: state.keywordGroups.filter(g => g.id !== action.payload)
      };
    
    case 'TOGGLE_KEYWORD_GROUP':
      return {
        ...state,
        keywordGroups: state.keywordGroups.map(g =>
          g.id === action.payload ? { ...g, enabled: !g.enabled } : g
        )
      };
    
    case 'SET_LOADING':
      return { ...state, loading: action.payload };
    
    case 'SET_ERROR':
      return { ...state, error: action.payload };
    
    default:
      return state;
  }
};

// ============================================================================
// Task State
// ============================================================================

export interface TaskState {
  tasks: Task[];
  loading: boolean;
  error: string | null;
}

export type TaskAction =
  | { type: 'SET_TASKS'; payload: Task[] }
  | { type: 'ADD_TASK'; payload: Task }
  | { type: 'UPDATE_TASK'; payload: { id: string; updates: Partial<Task> } }
  | { type: 'DELETE_TASK'; payload: string }
  | { type: 'SET_LOADING'; payload: boolean }
  | { type: 'SET_ERROR'; payload: string | null };

const taskReducer = (state: TaskState, action: TaskAction): TaskState => {
  switch (action.type) {
    case 'SET_TASKS':
      return { ...state, tasks: action.payload };
    
    case 'ADD_TASK':
      return { ...state, tasks: [...state.tasks, action.payload] };
    
    case 'UPDATE_TASK':
      return {
        ...state,
        tasks: state.tasks.map(t =>
          t.id === action.payload.id ? { ...t, ...action.payload.updates } : t
        )
      };
    
    case 'DELETE_TASK':
      return {
        ...state,
        tasks: state.tasks.filter(t => t.id !== action.payload)
      };
    
    case 'SET_LOADING':
      return { ...state, loading: action.payload };
    
    case 'SET_ERROR':
      return { ...state, error: action.payload };
    
    default:
      return state;
  }
};

// ============================================================================
// Context Definitions
// ============================================================================

interface AppContextType {
  state: AppState;
  setPage: (page: Page) => void;
  addToast: (type: ToastType, message: string) => void;
  removeToast: (id: number) => void;
  setActiveWorkspace: (id: string | null) => void;
}

interface WorkspaceContextType {
  state: WorkspaceState;
  dispatch: React.Dispatch<WorkspaceAction>;
}

interface KeywordContextType {
  state: KeywordState;
  dispatch: React.Dispatch<KeywordAction>;
}

interface TaskContextType {
  state: TaskState;
  dispatch: React.Dispatch<TaskAction>;
}

const AppContext = createContext<AppContextType | undefined>(undefined);
const WorkspaceContext = createContext<WorkspaceContextType | undefined>(undefined);
const KeywordContext = createContext<KeywordContextType | undefined>(undefined);
const TaskContext = createContext<TaskContextType | undefined>(undefined);

// ============================================================================
// Provider Component
// ============================================================================

export const AppProvider = ({ children }: { children: ReactNode }) => {
  // App State
  const [appState, appDispatch] = useReducer(appReducer, {
    page: 'workspaces' as Page,
    toasts: [],
    activeWorkspaceId: null
  });

  // Workspace State
  const [workspaceState, workspaceDispatch] = useReducer(workspaceReducer, {
    workspaces: [],
    loading: false,
    error: null
  });

  // Keyword State
  const [keywordState, keywordDispatch] = useReducer(keywordReducer, {
    keywordGroups: [],
    loading: false,
    error: null
  });

  // Task State
  const [taskState, taskDispatch] = useReducer(taskReducer, {
    tasks: [],
    loading: false,
    error: null
  });
  
  // 使用 ref 保存最新的 tasks 状态，避免闭包陷阱
  const tasksRef = useRef(taskState.tasks);
  useEffect(() => {
    tasksRef.current = taskState.tasks;
  }, [taskState.tasks]);

  // App Actions
  const setPage = useCallback((page: Page) => {
    appDispatch({ type: 'SET_PAGE', payload: page });
  }, []);

  const addToast = useCallback((type: ToastType, message: string) => {
    const action = { type: 'ADD_TOAST' as const, payload: { type, message } };
    appDispatch(action);
    
    // 自动移除Toast
    const id = Date.now();
    setTimeout(() => {
      appDispatch({ type: 'REMOVE_TOAST', payload: id });
    }, 3000);
  }, []);

  const removeToast = useCallback((id: number) => {
    appDispatch({ type: 'REMOVE_TOAST', payload: id });
  }, []);

  const setActiveWorkspace = useCallback((id: string | null) => {
    appDispatch({ type: 'SET_ACTIVE_WORKSPACE', payload: id });
  }, []);

  // 加载配置
  useEffect(() => {
    const loadConfig = async () => {
      try {
        const config = await invoke<any>('load_config');
        if (config.keyword_groups) {
          keywordDispatch({ type: 'SET_KEYWORD_GROUPS', payload: config.keyword_groups });
        }
        if (config.workspaces) {
          workspaceDispatch({ type: 'SET_WORKSPACES', payload: config.workspaces });
        }
      } catch (e) {
        console.error('Failed to load config:', e);
      }
    };
    loadConfig();
  }, []);

  // 保存配置（使用防抖避免频繁保存）
  const saveTimeoutRef = useRef<number | null>(null);
  const lastSavedRef = useRef<string>('');
  
  useEffect(() => {
    if (keywordState.keywordGroups.length === 0 && workspaceState.workspaces.length === 0) {
      return;
    }
    
    // 生成配置指纹，避免相同配置重复保存
    const configFingerprint = JSON.stringify({
      keywords: keywordState.keywordGroups.map(g => ({ id: g.id, enabled: g.enabled })),
      workspaces: workspaceState.workspaces.map(w => ({ id: w.id, status: w.status }))
    });
    
    if (configFingerprint === lastSavedRef.current) {
      return; // 配置未变化，跳过保存
    }
    
    // 清除之前的定时器
    if (saveTimeoutRef.current) {
      window.clearTimeout(saveTimeoutRef.current);
    }
    
    // 防抖：500ms 后保存
    saveTimeoutRef.current = window.setTimeout(() => {
      lastSavedRef.current = configFingerprint;
      invoke('save_config', {
        config: {
          keyword_groups: keywordState.keywordGroups,
          workspaces: workspaceState.workspaces
        }
      }).catch(e => console.error('Failed to save config:', e));
    }, 500);
    
    return () => {
      if (saveTimeoutRef.current) {
        window.clearTimeout(saveTimeoutRef.current);
      }
    };
  }, [keywordState.keywordGroups, workspaceState.workspaces]);

  // 监听后端任务事件
  useEffect(() => {
    // 使用 Set 跟踪已创建的任务 ID，避免重复创建
    const createdTaskIds = new Set<string>();
    
    // 监听任务更新事件
    const unlistenTaskUpdate = listen<any>('task-update', (event) => {
      const { task_id, task_type, target, status, message, progress, workspace_id } = event.payload;
      logger.debug('[EVENT] task-update:', event.payload);
      
      // 检查任务是否已存在（使用 ref 获取最新状态）
      const existingTask = tasksRef.current.find(t => t.id === task_id);
      
      if (!existingTask && !createdTaskIds.has(task_id)) {
        // 如果任务不存在且未被标记为已创建，先添加它
        logger.debug('[EVENT] Task not found, creating new task:', task_id);
        createdTaskIds.add(task_id);
        taskDispatch({
          type: 'ADD_TASK',
          payload: {
            id: task_id,
            type: task_type,
            target,
            progress,
            status: status as 'RUNNING' | 'COMPLETED' | 'FAILED' | 'STOPPED',
            message,
            workspaceId: workspace_id
          }
        });
      } else if (existingTask || createdTaskIds.has(task_id)) {
        // 任务已存在或已标记为已创建，更新它
        taskDispatch({
          type: 'UPDATE_TASK',
          payload: {
            id: task_id,
            updates: {
              type: task_type,
              target,
              status: status as 'RUNNING' | 'COMPLETED' | 'FAILED' | 'STOPPED',
              message,
              progress
            }
          }
        });
      }
      
      // 当任务完成时，更新工作区状态为 READY
      if (status === 'COMPLETED') {
        logger.debug('[EVENT] Task completed, workspace_id:', workspace_id);
        
        // 直接使用 workspace_id 更新状态，不需要先查找
        // 如果工作区不存在，reducer 会忽略这个操作
        if (workspace_id) {
          logger.debug('[EVENT] Updating workspace status to READY:', workspace_id);
          workspaceDispatch({
            type: 'UPDATE_WORKSPACE',
            payload: { id: workspace_id, updates: { status: 'READY' } }
          });
        } else {
          // 备选：如果没有 workspace_id，尝试通过任务 ID 或 name 查找
          const task = taskState.tasks.find(t => t.id === task_id);
          if (task?.workspaceId) {
            workspaceDispatch({
              type: 'UPDATE_WORKSPACE',
              payload: { id: task.workspaceId, updates: { status: 'READY' } }
            });
          } else {
            logger.debug('[EVENT] Could not find workspace_id for completed task:', { task_id, target });
          }
        }
      } else if (status === 'FAILED') {
        // 直接使用 workspace_id 更新状态
        if (workspace_id) {
          workspaceDispatch({
            type: 'UPDATE_WORKSPACE',
            payload: { id: workspace_id, updates: { status: 'OFFLINE' } }
          });
        }
      }
    });

    // 监听导入完成事件
    const unlistenImportComplete = listen<string>('import-complete', (event) => {
      logger.debug('[EVENT] import-complete:', event.payload);
      const taskId = event.payload;
      
      // 标记任务为完成
      taskDispatch({
        type: 'UPDATE_TASK',
        payload: {
          id: taskId,
          updates: { status: 'COMPLETED', progress: 100 }
        }
      });
    });

    // 监听导入错误事件
    const unlistenImportError = listen<string>('import-error', (event) => {
      logger.error('[EVENT] import-error:', event.payload);
      addToast('error', `导入失败: ${event.payload}`);
    });

    // 清理监听器
    return () => {
      unlistenTaskUpdate.then(f => f());
      unlistenImportComplete.then(f => f());
      unlistenImportError.then(f => f());
    };
  }, [addToast, taskDispatch, workspaceDispatch, taskState.tasks]);

  return (
    <AppContext.Provider value={{ state: appState, setPage, addToast, removeToast, setActiveWorkspace }}>
      <WorkspaceContext.Provider value={{ state: workspaceState, dispatch: workspaceDispatch }}>
        <KeywordContext.Provider value={{ state: keywordState, dispatch: keywordDispatch }}>
          <TaskContext.Provider value={{ state: taskState, dispatch: taskDispatch }}>
            {children}
          </TaskContext.Provider>
        </KeywordContext.Provider>
      </WorkspaceContext.Provider>
    </AppContext.Provider>
  );
};

// ============================================================================
// Hooks
// ============================================================================

export const useApp = () => {
  const context = useContext(AppContext);
  if (!context) {
    throw new Error('useApp must be used within AppProvider');
  }
  return context;
};

export const useWorkspaceState = () => {
  const context = useContext(WorkspaceContext);
  if (!context) {
    throw new Error('useWorkspaceState must be used within AppProvider');
  }
  return context;
};

export const useKeywordState = () => {
  const context = useContext(KeywordContext);
  if (!context) {
    throw new Error('useKeywordState must be used within AppProvider');
  }
  return context;
};

export const useTaskState = () => {
  const context = useContext(TaskContext);
  if (!context) {
    throw new Error('useTaskState must be used within AppProvider');
  }
  return context;
};
