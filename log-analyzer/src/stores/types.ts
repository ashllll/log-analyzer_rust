/**
 * Store 类型定义
 *
 * 所有 Store 共享的类型定义，集中管理以避免循环依赖。
 * 这个文件是 pure type definitions，不包含任何实现逻辑。
 */

// ============================================================================
// Toast 类型
// ============================================================================

export type ToastType = 'success' | 'error' | 'info';

export interface Toast {
  id: number;
  type: ToastType;
  message: string;
}

// ============================================================================
// Workspace 类型
// ============================================================================

export interface Workspace {
  id: string;
  name: string;
  path: string;
  status: 'READY' | 'OFFLINE' | 'PROCESSING' | 'ERROR' | 'PARTIAL';
  size: string;
  files: number;
  ready_files?: number;
  watching?: boolean;
}

// ============================================================================
// Task 类型
// ============================================================================

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

// ============================================================================
// Keyword 类型
// ============================================================================

export type ColorKey = 'blue' | 'green' | 'red' | 'orange' | 'purple';

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
