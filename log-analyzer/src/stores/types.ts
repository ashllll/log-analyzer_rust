/**
 * Store 类型定义
 *
 * 统一从 Zod Schema 导出类型，避免手写 interface 与运行时验证不同步。
 * 这个文件是 pure type definitions，不包含任何实现逻辑。
 */

// ============================================================================
// Toast 类型（前端专用，无对应后端 Schema）
// ============================================================================

export type ToastType = 'success' | 'error' | 'info';

export interface Toast {
  id: number;
  type: ToastType;
  message: string;
}

// ============================================================================
// Workspace / Keyword 类型（从 Zod Schema 导出）
// ============================================================================

export type {
  Workspace,
  KeywordGroup,
  KeywordPattern,
  ColorKey,
} from '../types/api-responses';

// ============================================================================
// Task 类型（从 events/types.ts Zod Schema 导出）
// ============================================================================

export type { TaskStatus, TaskType } from '../events/types';

/**
 * Task 类型 - 基于 TaskUpdateEventSchema 的字段，前端 store 使用
 * 注意：completedAt 是前端计算字段，不在事件 Schema 中
 */
export interface Task {
  id: string;
  type: string;
  target: string;
  progress: number;
  message: string;
  status: 'RUNNING' | 'COMPLETED' | 'FAILED' | 'STOPPED';
  workspaceId?: string;
  completedAt?: number;
}
