/**
 * 任务事件总线 (TaskEventBus)
 *
 * 专注处理任务生命周期相关事件，提供 Schema 验证和幂等性保证。
 * 不处理 import-complete/import-error/validation-report 等简单通知事件，
 * 那些事件由 useTauriEventListeners 直接处理，避免不必要的验证开销。
 *
 * 职责：
 * 1. 事件验证（Zod Schema）
 * 2. 幂等性保证（版本号，仅 task-update）
 * 3. 事件分发
 *
 * 处理的事件类型：
 * - task-update: 任务状态更新（带版本号幂等性检查）
 * - task-removed: 任务移除通知
 * - workspace-event: 工作区状态变更
 */

import { z } from "zod";
import { logger } from "../utils/logger";
import {
  TaskUpdateEventSchema,
  TaskRemovedEventSchema,
  WorkspaceEventSchema,
  EventValidationError,
} from "./types";
import type { TaskUpdateEvent } from "./types";

// ============================================================================
// 事件处理器类型
// ============================================================================

export type EventHandler<T = unknown> = (event: T) => void | Promise<void>;

export interface EventBusConfig {
  enableValidation?: boolean; // 是否启用Schema验证
  enableIdempotency?: boolean; // 是否启用幂等性检查
  enableLogging?: boolean; // 是否启用日志
  logLevel?: "debug" | "info" | "warn" | "error"; // 日志级别
}

// ============================================================================
// 幂等性管理器
// ============================================================================

interface ProcessedEvent {
  taskId: string;
  version: number;
  timestamp: number;
}

/**
 * 幂等性管理器
 * 使用LRU缓存防止内存泄漏
 */
class IdempotencyManager {
  private processed = new Map<string, ProcessedEvent>();
  private maxSize = 1000;

  /**
   * 检查事件是否已处理
   *
   * 边界条件处理：
   * - version === undefined: 事件无版本号，跳过幂等性检查（允许处理）
   * - version < 0: 非法版本号，视为未处理（让验证层拒绝）
   * - version === 0: 合法版本号，正常比较
   *
   * 注意：后端 version 从 1 开始递增，使用 u64 存储，实际运行中不可能达到上限，
   * 因此不需要处理版本号环绕重置的复杂逻辑。
   */
  isProcessed(taskId: string, version: number | undefined): boolean {
    // 无版本号的事件跳过幂等性检查（由调用方决定是否处理）
    if (version === undefined) {
      return false;
    }

    // 拒绝非法版本号（负数），但让验证层处理错误
    if (version < 0) {
      return false;
    }

    const processed = this.processed.get(taskId);
    if (!processed) return false;

    // 只有严格相同版本才视为重复
    // 版本号递增的事件应该被处理
    if (processed.version === version) {
      return true;
    }

    // 如果新版本小于已处理版本，视为旧事件延迟到达，跳过处理
    if (processed.version > version) {
      logger.warn(
        {
          taskId,
          processedVersion: processed.version,
          incomingVersion: version,
        },
        "Skipping stale event with older version"
      );
      return true;
    }

    // 新版本事件，允许处理
    return false;
  }

  /**
   * 标记事件已处理
   * 使用 Map 插入顺序实现 O(1) LRU 淘汰
   */
  markProcessed(taskId: string, version: number): void {
    // 删除后重新插入，更新 LRU 顺序
    this.processed.delete(taskId);
    this.processed.set(taskId, {
      taskId,
      version,
      timestamp: Date.now(),
    });

    // O(1) LRU 淘汰：利用 Map 的插入顺序，淘汰最旧的条目
    if (this.processed.size > this.maxSize) {
      const firstKey = this.processed.keys().next().value;
      if (firstKey) {
        this.processed.delete(firstKey);
      }
    }
  }

  /**
   * 清空缓存
   */
  clear(): void {
    this.processed.clear();
  }

  /**
   * 获取缓存大小
   */
  size(): number {
    return this.processed.size;
  }
}

// ============================================================================
// 事件总线实现
// ============================================================================

class TaskEventBus {
  private static instance: TaskEventBus;

  private config: Required<EventBusConfig>;
  private idempotencyManager: IdempotencyManager;
  private handlers = new Map<string, Set<EventHandler>>();
  private metrics = {
    totalEvents: 0,
    validationErrors: 0,
    idempotencySkips: 0,
    processingErrors: 0,
    lastEventTime: 0,
  };

  constructor(config: EventBusConfig = {}) {
    this.config = {
      enableValidation: config.enableValidation ?? true,
      enableIdempotency: config.enableIdempotency ?? true,
      enableLogging: config.enableLogging ?? true,
      logLevel: config.logLevel ?? "info",
    };

    this.idempotencyManager = new IdempotencyManager();

    if (this.config.enableLogging) {
      logger.setLevel(this.config.logLevel);
      logger.info({ component: "TaskEventBus" }, "TaskEventBus initialized");
    }
  }

  static getInstance(config?: EventBusConfig): TaskEventBus {
    if (!TaskEventBus.instance) {
      TaskEventBus.instance = new TaskEventBus(config);
    }
    return TaskEventBus.instance;
  }

  // ========================================================================
  // 事件注册
  // ========================================================================

  /**
   * 注册事件处理器
   *
   * @param eventType - 事件类型
   * @param handler - 事件处理函数
   * @returns 取消订阅函数
   */
  on<T = unknown>(eventType: string, handler: EventHandler<T>): () => void {
    if (!this.handlers.has(eventType)) {
      this.handlers.set(eventType, new Set());
    }

    const handlers = this.handlers.get(eventType)!;
    handlers.add(handler as EventHandler);

    if (this.config.enableLogging) {
      logger.debug(
        { eventType, handlerCount: handlers.size },
        "Handler registered"
      );
    }

    // 返回取消订阅函数
    return () => {
      handlers.delete(handler as EventHandler);
      if (this.config.enableLogging) {
        logger.debug({ eventType }, "Handler unregistered");
      }
      if (handlers.size === 0) {
        this.handlers.delete(eventType);
      }
    };
  }

  // ========================================================================
  // 事件处理核心逻辑（公开API，供测试使用）
  // ========================================================================

  /**
   * 处理事件（公开API，供外部调用）
   *
   * @param eventType - 事件类型
   * @param rawData - 原始事件数据
   */
  async processEvent(
    eventType: "task-update" | "task-removed" | "workspace-event",
    rawData: unknown
  ): Promise<void> {
    this.metrics.totalEvents++;
    this.metrics.lastEventTime = Date.now();

    if (this.config.enableLogging) {
      logger.debug({ eventType, rawData }, "Event received");
    }

    try {
      // Step 1: Schema验证
      const validatedEvent = this.validateEvent(eventType, rawData);

      // Step 2: 幂等性检查（仅task-update）
      if (eventType === "task-update" && this.config.enableIdempotency) {
        const event = validatedEvent as TaskUpdateEvent;

        // 严格模式：拒绝无 version 字段的事件（非 TaskManager 发送）
        // 使用 === undefined 明确区分 "未定义" 和 "值为0"
        if (event.version === undefined) {
          if (this.config.enableLogging) {
            logger.warn(
              {
                taskId: event.task_id,
                rawEvent: rawData,
              },
              "Ignoring event without version field (not from TaskManager)"
            );
          }
          this.metrics.validationErrors++;
          return; // 直接丢弃，不处理
        }

        const version = event.version;

        if (this.idempotencyManager.isProcessed(event.task_id, version)) {
          if (this.config.enableLogging) {
            logger.info(
              {
                taskId: event.task_id,
                version,
              },
              "Stale event skipped (idempotency)"
            );
          }
          this.metrics.idempotencySkips++;
          return;
        }

        // 标记事件已处理
        this.idempotencyManager.markProcessed(event.task_id, version);
      }

      // Step 3: 分发事件
      await this.dispatchEvent(eventType, validatedEvent);
    } catch (err: unknown) {
      // 验证错误单独统计，其他错误算处理错误
      const isValidationError = (err as Error)?.name === "EventValidationError";

      if (isValidationError) {
        this.metrics.validationErrors++;
      } else {
        this.metrics.processingErrors++;
      }

      logger.error(
        {
          eventType,
          error: err instanceof Error ? err.message : String(err),
          rawData,
        },
        "Event processing failed"
      );

      // 验证错误应该抛出，让调用者能感知到
      if (isValidationError) {
        throw err;
      }
    }
  }

  // ========================================================================
  // Schema验证
  // ========================================================================

  private validateEvent(eventType: string, rawData: unknown): unknown | null {
    if (!this.config.enableValidation) {
      return rawData;
    }

    try {
      switch (eventType) {
        case "task-update":
          return TaskUpdateEventSchema.parse(rawData);
        case "task-removed":
          return TaskRemovedEventSchema.parse(rawData);
        case "workspace-event":
          return WorkspaceEventSchema.parse(rawData);
        default:
          logger.warn({ eventType }, "Unknown event type");
          return rawData;
      }
    } catch (err: unknown) {
      if (err instanceof z.ZodError) {
        const zodErrors = err.issues || [];
        if (this.config.enableLogging) {
          logger.error(
            {
              eventType,
              errors: zodErrors,
              rawData,
            },
            "Event validation failed"
          );
        }
        throw new EventValidationError(eventType, err, rawData);
      }
      return null;
    }
  }

  // ========================================================================
  // 事件分发
  // ========================================================================

  private async dispatchEvent(
    eventType: string,
    event: unknown
  ): Promise<void> {
    const handlers = this.handlers.get(eventType);
    if (!handlers || handlers.size === 0) {
      if (this.config.enableLogging) {
        logger.warn({ eventType }, "No handlers registered");
      }
      return;
    }

    // 并发调用所有处理器
    const promises = Array.from(handlers).map(async (handler) => {
      try {
        await handler(event);
      } catch (err: unknown) {
        // 老王备注：handler抛出的错误要统计到processingErrors
        this.metrics.processingErrors++;

        logger.error(
          {
            eventType,
            error: err instanceof Error ? err.message : String(err),
            handler: handler.name || "anonymous",
          },
          "Handler error"
        );
      }
    });

    await Promise.allSettled(promises);
  }

  // ========================================================================
  // 公共API
  // ========================================================================

  /**
   * 获取指标
   */
  getMetrics() {
    return {
      ...this.metrics,
      handlersCount: this.handlers.size,
      idempotencyCacheSize: this.idempotencyManager.size(),
    };
  }

  /**
   * 重置指标
   */
  resetMetrics(): void {
    this.metrics = {
      totalEvents: 0,
      validationErrors: 0,
      idempotencySkips: 0,
      processingErrors: 0,
      lastEventTime: 0,
    };

    if (this.config.enableLogging) {
      logger.info({ component: "TaskEventBus" }, "Metrics reset");
    }
  }

  /** 清理幂等性缓存 */
  clearCache(): void {
    this.idempotencyManager.clear();

    if (this.config.enableLogging) {
      logger.info({ component: "TaskEventBus" }, "Idempotency cache cleared");
    }
  }

  /** 更新配置 */
  updateConfig(config: Partial<EventBusConfig>): void {
    this.config = { ...this.config, ...config };

    if (this.config.enableLogging) {
      logger.setLevel(this.config.logLevel);
      logger.info(
        { component: "TaskEventBus", newConfig: config },
        "Config updated"
      );
    }
  }
}

// ============================================================================
// 单例导出
// ============================================================================

/** 全局 TaskEventBus 实例 */
export const eventBus = TaskEventBus.getInstance();

/** TaskEventBus 类（供测试使用） */
export { TaskEventBus as EventBus };
