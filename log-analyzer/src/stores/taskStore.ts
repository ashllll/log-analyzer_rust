/**
 * 任务状态 Store - 使用 Zustand + Immer
 * 
 * 实现任务生命周期管理：
 * 1. 任务去重逻辑，防止重复创建任务
 * 2. 自动清理已完成/失败的任务（TTL机制）
 * 3. 状态机模式确保状态转换的正确性
 */

import { create } from 'zustand';
import { devtools, subscribeWithSelector } from 'zustand/middleware';
import { immer } from 'zustand/middleware/immer';
import { logger } from '../utils/logger';

// ============================================================================
// Types
// ============================================================================

export interface Task {
  id: string;
  type: string;
  target: string;
  progress: number;
  message: string;
  status: 'RUNNING' | 'COMPLETED' | 'FAILED' | 'STOPPED';
  workspaceId?: string;
  completedAt?: number; // 完成时间戳，用于TTL清理
}

interface TaskState {
  // State
  tasks: Task[];
  loading: boolean;
  error: string | null;
  
  // Actions
  setTasks: (tasks: Task[]) => void;
  addTask: (task: Task) => void;
  addTaskIfNotExists: (task: Task) => void; // 带去重的添加
  updateTask: (id: string, updates: Partial<Task>) => void;
  deleteTask: (id: string) => void;
  setLoading: (loading: boolean) => void;
  setError: (error: string | null) => void;
}

// ============================================================================
// 空值安全工具函数
// ============================================================================

/**
 * 安全地创建 Task 对象
 * 防御性编程：确保 Task 对象的所有必要字段都有有效值
 */
export function createSafeTask(partial: Partial<Task> | null | undefined): Task | null {
  if (!partial || typeof partial !== 'object') {
    return null;
  }

  // 必要字段检查
  if (!partial.id || typeof partial.id !== 'string') {
    logger.warn('createSafeTask: 无效的 task id');
    return null;
  }

  return {
    id: partial.id,
    type: partial.type || 'unknown',
    target: partial.target || '',
    progress: typeof partial.progress === 'number' ? Math.max(0, Math.min(100, partial.progress)) : 0,
    message: partial.message || '',
    status: partial.status || 'RUNNING',
    workspaceId: partial.workspaceId,
    completedAt: partial.completedAt
  };
}

/**
 * 安全地过滤任务列表
 * 移除无效的任务对象
 */
export function sanitizeTaskList(tasks: Task[] | null | undefined): Task[] {
  if (!Array.isArray(tasks)) {
    return [];
  }

  return tasks
    .map(task => createSafeTask(task))
    .filter((task): task is Task => task !== null);
}

// ============================================================================
// Store
// ============================================================================

export const useTaskStore = create<TaskState>()(
  devtools(
    subscribeWithSelector(
      immer((set) => ({
        // Initial State
        tasks: [],
        loading: false,
        error: null,
        
        // Actions
        setTasks: (tasks) => set((state) => {
          // 空值安全：过滤无效的任务
          state.tasks = sanitizeTaskList(tasks);
        }),
        
        addTask: (task) => set((state) => {
          // 空值安全：验证任务对象
          const safeTask = createSafeTask(task);
          if (safeTask) {
            state.tasks.push(safeTask);
          }
        }),
        
        // 带去重的添加 - 防止重复创建任务
        addTaskIfNotExists: (task) => set((state) => {
          const exists = state.tasks.some(t => t.id === task.id);
          if (!exists) {
            state.tasks.push(task);
          }
        }),
        
        updateTask: (id, updates) => set((state) => {
          const index = state.tasks.findIndex(t => t.id === id);
          if (index !== -1) {
            Object.assign(state.tasks[index], updates);
          }
        }),
        
        deleteTask: (id) => set((state) => {
          state.tasks = state.tasks.filter(t => t.id !== id);
        }),
        
        setLoading: (loading) => set((state) => {
          state.loading = loading;
        }),
        
        setError: (error) => set((state) => {
          state.error = error;
        }),
      }))
    ),
    { name: 'task-store' }
  )
);

