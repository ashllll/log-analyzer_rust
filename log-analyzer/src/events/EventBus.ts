/**
 * 企业级事件总线
 *
 * 职责：
 * 1. 事件验证（Zod Schema）
 * 2. 幂等性保证（版本号）
 * 3. 事件分发
 * 4. 错误处理
 * 5. 可观测性（日志、指标）
 *
 * 设计模式：
 * - 单例模式
 * - 观察者模式
 * - 依赖倒置（依赖注入）
 *
 * @module events/EventBus
 * @author Claude (老王)
 * @created 2025-12-27
 */

import { z } from 'zod';
import { logger } from '../utils/logger';
import { TaskUpdateEventSchema, TaskRemovedEventSchema, EventValidationError } from './types';
import type { TaskUpdateEvent } from './types';

// ============================================================================
// 事件处理器类型
// ============================================================================

export type EventHandler<T = any> = (event: T) => void | Promise<void>;

export interface EventBusConfig {
  enableValidation?: boolean;      // 是否启用Schema验证
  enableIdempotency?: boolean;      // 是否启用幂等性检查
  enableLogging?: boolean;          // 是否启用日志
  logLevel?: 'debug' | 'info' | 'warn' | 'error';     // 日志级别
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
  private maxSize = 100;

  /**
   * 检查事件是否已处理
   */
  isProcessed(taskId: string, version: number): boolean {
    const processed = this.processed.get(taskId);
    if (!processed) return false;
    return processed.version >= version;
  }

  /**
   * 标记事件已处理
   */
  markProcessed(taskId: string, version: number): void {
    this.processed.set(taskId, {
      taskId,
      version,
      timestamp: Date.now(),
    });

    // LRU 淘汰
    if (this.processed.size > this.maxSize) {
      const oldest = Array.from(this.processed.entries())
        .sort((a, b) => a[1].timestamp - b[1].timestamp)[0];
      this.processed.delete(oldest[0]);
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

class EventBus {
  private static instance: EventBus;

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

  private constructor(config: EventBusConfig = {}) {
    this.config = {
      enableValidation: config.enableValidation ?? true,
      enableIdempotency: config.enableIdempotency ?? true,
      enableLogging: config.enableLogging ?? true,
      logLevel: config.logLevel ?? 'info',
    };

    this.idempotencyManager = new IdempotencyManager();

    if (this.config.enableLogging) {
      logger.setLevel(this.config.logLevel);
      logger.info({ component: 'EventBus' }, 'EventBus initialized');
    }
  }

  static getInstance(config?: EventBusConfig): EventBus {
    if (!EventBus.instance) {
      EventBus.instance = new EventBus(config);
    }
    return EventBus.instance;
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
  on<T = any>(
    eventType: string,
    handler: EventHandler<T>
  ): () => void {
    if (!this.handlers.has(eventType)) {
      this.handlers.set(eventType, new Set());
    }

    const handlers = this.handlers.get(eventType)!;
    handlers.add(handler as EventHandler);

    if (this.config.enableLogging) {
      logger.debug(
        { eventType, handlerCount: handlers.size },
        'Handler registered'
      );
    }

    // 返回取消订阅函数
    return () => {
      handlers.delete(handler as EventHandler);
      if (this.config.enableLogging) {
        logger.debug({ eventType }, 'Handler unregistered');
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
  async processEvent(eventType: 'task-update' | 'task-removed', rawData: any): Promise<void> {
    this.metrics.totalEvents++;
    this.metrics.lastEventTime = Date.now();

    if (this.config.enableLogging) {
      logger.debug({ eventType, rawData }, 'Event received');
    }

    try {
      // Step 1: Schema验证
      const validatedEvent = this.validateEvent(eventType, rawData);

      // Step 2: 幂等性检查（仅task-update）
      if (eventType === 'task-update' && this.config.enableIdempotency) {
        const event = validatedEvent as TaskUpdateEvent;

        // 确保有version字段
        const version = event.version || 1;

        if (this.idempotencyManager.isProcessed(event.task_id, version)) {
          if (this.config.enableLogging) {
            logger.info(
              {
                taskId: event.task_id,
                version,
              },
              'Stale event skipped (idempotency)'
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
      // 老王备注：验证错误单独统计，其他错误算处理错误
      const isValidationError = (err as Error)?.name === 'EventValidationError';

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
        'Event processing failed'
      );

      // 老王备注：验证错误应该抛出，让调用者能感知到
      if (isValidationError) {
        throw err;
      }
    }
  }

  // ========================================================================
  // Schema验证
  // ========================================================================

  private validateEvent(eventType: string, rawData: any): any | null {
    if (!this.config.enableValidation) {
      return rawData;
    }

    try {
      switch (eventType) {
        case 'task-update':
          return TaskUpdateEventSchema.parse(rawData);
        case 'task-removed':
          return TaskRemovedEventSchema.parse(rawData);
        default:
          logger.warn({ eventType }, 'Unknown event type');
          return rawData;
      }
    } catch (err: unknown) {
      if (err instanceof z.ZodError) {
        const zodErrors = (err as any).issues || [];
        if (this.config.enableLogging) {
          logger.error(
            {
              eventType,
              errors: zodErrors,
              rawData,
            },
            'Event validation failed'
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

  private async dispatchEvent(eventType: string, event: any): Promise<void> {
    const handlers = this.handlers.get(eventType);
    if (!handlers || handlers.size === 0) {
      if (this.config.enableLogging) {
        logger.warn({ eventType }, 'No handlers registered');
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
            handler: handler.name || 'anonymous',
          },
          'Handler error'
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
      logger.info({ component: 'EventBus' }, 'Metrics reset');
    }
  }

  /**
   * 清理幂等性缓存
   */
  clearCache(): void {
    this.idempotencyManager.clear();

    if (this.config.enableLogging) {
      logger.info({ component: 'EventBus' }, 'Idempotency cache cleared');
    }
  }

  /**
   * 更新配置
   */
  updateConfig(config: Partial<EventBusConfig>): void {
    this.config = { ...this.config, ...config };

    if (this.config.enableLogging) {
      logger.setLevel(this.config.logLevel);
      logger.info({ component: 'EventBus', newConfig: config }, 'Config updated');
    }
  }
}

// ============================================================================
// 单例导出
// ============================================================================

/**
 * 全局EventBus实例
 */
export const eventBus = EventBus.getInstance();

/**
 * EventBus类（供测试使用）
 */
export { EventBus };
