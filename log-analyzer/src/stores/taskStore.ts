/**
 * 任务状态 Store - 使用 Zustand + Immer
 * 
 * 实现任务去重逻辑,防止重复创建任务
 */

import { create } from 'zustand';
import { devtools, subscribeWithSelector } from 'zustand/middleware';
import { immer } from 'zustand/middleware/immer';

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
          state.tasks = tasks;
        }),
        
        addTask: (task) => set((state) => {
          state.tasks.push(task);
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

