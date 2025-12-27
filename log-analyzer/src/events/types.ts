/**
 * 企业级事件系统 - 类型定义
 *
 * 设计原则：
 * 1. 类型安全：编译时 + 运行时双重保证
 * 2. 自文档化：类型即文档
 * 3. 可扩展：易于添加新事件
 *
 * @module events/types
 * @author Claude (老王)
 * @created 2025-12-27
 */

import { z } from 'zod';

// ============================================================================
// 基础类型
// ============================================================================

/**
 * 任务状态枚举
 */
export const TaskStatusSchema = z.enum([
  'RUNNING',
  'COMPLETED',
  'FAILED',
  'STOPPED'
]);

export type TaskStatus = z.infer<typeof TaskStatusSchema>;

/**
 * 任务类型枚举
 */
export const TaskTypeSchema = z.enum([
  'Import',
  'Export',
  'Search',
  'Index'
]);

export type TaskType = z.infer<typeof TaskTypeSchema>;

// ============================================================================
// 事件Schema定义
// ============================================================================

/**
 * task-update 事件
 *
 * 验证规则：
 * - task_id: 非空字符串
 * - progress: 0-100
 * - version: 正整数
 */
export const TaskUpdateEventSchema = z.object({
  // 基本信息
  task_id: z.string().min(1, "task_id is required"),
  task_type: TaskTypeSchema,
  target: z.string().min(1, "target is required"),

  // 进度信息
  progress: z.number().int().min(0).max(100),
  message: z.string(),
  status: TaskStatusSchema,

  // 可选信息
  workspace_id: z.string().optional(),

  // 版本控制
  version: z.number().int().positive().default(1),

  // 时间戳
  timestamp: z.number().int().positive().optional(),
});

export type TaskUpdateEvent = z.infer<typeof TaskUpdateEventSchema>;

/**
 * task-removed 事件
 */
export const TaskRemovedEventSchema = z.object({
  task_id: z.string().min(1, "task_id is required"),
  version: z.number().int().positive().optional(),
  timestamp: z.number().int().positive().optional(),
});

export type TaskRemovedEvent = z.infer<typeof TaskRemovedEventSchema>;

// ============================================================================
// 错误类型
// ============================================================================

/**
 * 事件验证错误
 */
export class EventValidationError extends Error {
  constructor(
    public eventType: string,
    public errors: z.ZodError,
    public rawData: any
  ) {
    // 老王备注：防御性检查，避免errors为undefined
    const errorSummary = errors?.errors
      ? errors.errors.map(e => `${e.path.join('.')}: ${e.message}`).join('; ')
      : 'Unknown validation error';

    super(`Event validation failed for ${eventType}: ${errorSummary}`);
    this.name = 'EventValidationError';
  }
}

/**
 * 过期事件错误
 */
export class StaleEventError extends Error {
  constructor(
    public taskId: string,
    public eventVersion: number,
    public currentVersion: number
  ) {
    super(`Stale event for task ${taskId}: event version ${eventVersion} < current version ${currentVersion}`);
    this.name = 'StaleEventError';
  }
}
