/**
 * EventBus 单元测试
 *
 * 测试覆盖：
 * 1. 单例模式
 * 2. Schema验证
 * 3. 幂等性检查
 * 4. 事件分发
 * 5. 错误处理
 * 6. 指标收集
 * 7. 配置管理
 *
 * @author Claude (老王)
 * @created 2025-12-27
 */

// Mock logger before importing EventBus
jest.mock('../../utils/logger', () => ({
  logger: {
    debug: jest.fn(),
    info: jest.fn(),
    warn: jest.fn(),
    error: jest.fn((...args: any[]) => {
      // 老王备注：确保mock logger不吞掉错误
      console.log('[Mock Logger Error]', ...args);
    }),
    setLevel: jest.fn(),
    getLevel: jest.fn(() => 'info'),
  },
}));

import { EventBus, eventBus } from '../EventBus';
import { EventValidationError } from '../types';

describe('EventBus', () => {
  let testEventBus: EventBus;

  // 每个测试前创建新的EventBus实例
  beforeEach(() => {
    // 创建新实例（不是全局单例）
    testEventBus = new EventBus({ enableLogging: false });
  });

  afterEach(() => {
    // 清理
    testEventBus.clearCache();
  });

  describe('单例模式', () => {
    it('应该返回相同的实例', () => {
      const instance1 = EventBus.getInstance();
      const instance2 = EventBus.getInstance();
      expect(instance1).toBe(instance2);
    });

    it('全局导出应该是EventBus实例', () => {
      expect(eventBus).toBeInstanceOf(EventBus);
    });
  });

  describe('Schema验证', () => {
    it('应该接受有效的task-update事件', async () => {
      const validEvent = {
        task_id: 'test-task-1',
        task_type: 'Import',
        target: '/path/to/file.log',
        progress: 50,
        message: 'Processing...',
        status: 'RUNNING',
        workspace_id: 'workspace-1',
        version: 1,
        timestamp: Date.now(),
      };

      // 应该不抛出错误
      await expect(
        testEventBus.processEvent('task-update', validEvent)
      ).resolves.not.toThrow();
    });

    it('应该拒绝无效的task-update事件', async () => {
      const invalidEvent = {
        task_id: '', // 空字符串，应该失败
        task_type: 'Import',
        target: '/path/to/file.log',
        progress: 150, // 超出范围（0-100）
        message: 'Test',
        status: 'INVALID_STATUS', // 无效状态
      };

      // 应该抛出验证错误
      await expect(
        testEventBus.processEvent('task-update', invalidEvent)
      ).rejects.toThrow(EventValidationError);
    });

    it('应该接受有效的task-removed事件', async () => {
      const validEvent = {
        task_id: 'test-task-1',
        version: 1,
        timestamp: Date.now(),
      };

      await expect(
        testEventBus.processEvent('task-removed', validEvent)
      ).resolves.not.toThrow();
    });

    it('应该拒绝task_id为空的task-removed事件', async () => {
      const invalidEvent = {
        task_id: '', // 空字符串
      };

      await expect(
        testEventBus.processEvent('task-removed', invalidEvent)
      ).rejects.toThrow(EventValidationError);
    });
  });

  describe('幂等性检查', () => {
    it('应该跳过重复的事件（相同task_id和version）', async () => {
      const event = {
        task_id: 'test-task-1',
        task_type: 'Import' as const,
        target: '/path/to/file.log',
        progress: 50,
        message: 'Processing...',
        status: 'RUNNING' as const,
        version: 1,
      };

      let callCount = 0;
      const handler = jest.fn(() => {
        callCount++;
      });

      // 注册处理器
      testEventBus.on('task-update', handler);

      // 第一次处理
      await testEventBus.processEvent('task-update', event);
      expect(callCount).toBe(1);

      // 第二次处理相同版本的事件（应该被跳过）
      await testEventBus.processEvent('task-update', event);
      expect(callCount).toBe(1); // 不应该再次调用
    });

    it('应该处理更高版本的事件', async () => {
      let callCount = 0;
      const handler = jest.fn(() => {
        callCount++;
      });

      testEventBus.on('task-update', handler);

      // 版本1
      await testEventBus.processEvent('task-update', {
        task_id: 'test-task-2',
        task_type: 'Import',
        target: '/path/to/file.log',
        progress: 50,
        message: 'Processing...',
        status: 'RUNNING',
        version: 1,
      });
      expect(callCount).toBe(1);

      // 版本2（应该处理）
      await testEventBus.processEvent('task-update', {
        task_id: 'test-task-2',
        task_type: 'Import',
        target: '/path/to/file.log',
        progress: 75,
        message: 'Still processing...',
        status: 'RUNNING',
        version: 2,
      });
      expect(callCount).toBe(2);
    });

    it('应该正确追踪幂等性跳过次数', async () => {
      const event = {
        task_id: 'test-task-3',
        task_type: 'Import' as const,
        target: '/path/to/file.log',
        progress: 50,
        message: 'Test',
        status: 'RUNNING' as const,
        version: 1,
      };

      testEventBus.on('task-update', jest.fn());

      // 第一次处理
      await testEventBus.processEvent('task-update', event);
      expect(testEventBus.getMetrics().idempotencySkips).toBe(0);

      // 第二次处理（应该跳过）
      await testEventBus.processEvent('task-update', event);
      expect(testEventBus.getMetrics().idempotencySkips).toBe(1);

      // 第三次处理（应该跳过）
      await testEventBus.processEvent('task-update', event);
      expect(testEventBus.getMetrics().idempotencySkips).toBe(2);
    });
  });

  describe('事件分发', () => {
    it('应该将事件分发给所有注册的处理器', async () => {
      const handler1 = jest.fn();
      const handler2 = jest.fn();
      const handler3 = jest.fn();

      testEventBus.on('task-update', handler1);
      testEventBus.on('task-update', handler2);
      testEventBus.on('task-update', handler3);

      const event = {
        task_id: 'test-task-4',
        task_type: 'Import',
        target: '/path/to/file.log',
        progress: 50,
        message: 'Test',
        status: 'RUNNING',
        version: 1,
      };

      await testEventBus.processEvent('task-update', event);

      // 所有处理器都应该被调用
      expect(handler1).toHaveBeenCalled();
      expect(handler2).toHaveBeenCalled();
      expect(handler3).toHaveBeenCalled();
    });

    it('应该只调用对应事件类型的处理器', async () => {
      const taskUpdateHandler = jest.fn();
      const taskRemovedHandler = jest.fn();

      testEventBus.on('task-update', taskUpdateHandler);
      testEventBus.on('task-removed', taskRemovedHandler);

      const event = {
        task_id: 'test-task-5',
        task_type: 'Import',
        target: '/path/to/file.log',
        progress: 50,
        message: 'Test',
        status: 'RUNNING',
        version: 1,
      };

      await testEventBus.processEvent('task-update', event);

      // 只有task-update处理器应该被调用
      expect(taskUpdateHandler).toHaveBeenCalled();
      expect(taskRemovedHandler).not.toHaveBeenCalled();
    });

    it('应该支持取消订阅', async () => {
      const handler = jest.fn();

      const unsubscribe = testEventBus.on('task-update', handler);

      // 取消订阅
      unsubscribe();

      const event = {
        task_id: 'test-task-6',
        task_type: 'Import',
        target: '/path/to/file.log',
        progress: 50,
        message: 'Test',
        status: 'RUNNING',
        version: 1,
      };

      await testEventBus.processEvent('task-update', event);

      // 处理器不应该被调用
      expect(handler).not.toHaveBeenCalled();
    });

    it('应该处理处理器抛出的错误不影响其他处理器', async () => {
      const errorHandler = jest.fn(() => {
        throw new Error('Handler error');
      });
      const successHandler = jest.fn();

      testEventBus.on('task-update', errorHandler);
      testEventBus.on('task-update', successHandler);

      const event = {
        task_id: 'test-task-7',
        task_type: 'Import',
        target: '/path/to/file.log',
        progress: 50,
        message: 'Test',
        status: 'RUNNING',
        version: 1,
      };

      // 应该不抛出错误（错误被捕获）
      await expect(
        testEventBus.processEvent('task-update', event)
      ).resolves.not.toThrow();

      // 两个处理器都应该被调用（即使一个抛出错误）
      expect(errorHandler).toHaveBeenCalled();
      expect(successHandler).toHaveBeenCalled();
    });
  });

  describe('指标收集', () => {
    it('应该正确追踪总事件数', async () => {
      testEventBus.on('task-update', jest.fn());

      await testEventBus.processEvent('task-update', {
        task_id: 'test-task-8',
        task_type: 'Import',
        target: '/path/to/file.log',
        progress: 50,
        message: 'Test',
        status: 'RUNNING',
        version: 1,
      });

      await testEventBus.processEvent('task-update', {
        task_id: 'test-task-9',
        task_type: 'Import',
        target: '/path/to/file.log',
        progress: 75,
        message: 'Test',
        status: 'RUNNING',
        version: 1,
      });

      const metrics = testEventBus.getMetrics();
      expect(metrics.totalEvents).toBe(2);
    });

    it('应该正确追踪验证错误', async () => {
      testEventBus.on('task-update', jest.fn());

      // 发送无效事件
      try {
        await testEventBus.processEvent('task-update', {
          task_id: '', // 无效
          task_type: 'Import',
          target: '/path/to/file.log',
          progress: 50,
          message: 'Test',
          status: 'RUNNING',
        });
      } catch {
        // 预期的错误
      }

      const metrics = testEventBus.getMetrics();
      expect(metrics.validationErrors).toBe(1);
    });

    it('应该正确追踪处理错误', async () => {
      testEventBus.on('task-update', () => {
        throw new Error('Handler error');
      });

      await testEventBus.processEvent('task-update', {
        task_id: 'test-task-10',
        task_type: 'Import',
        target: '/path/to/file.log',
        progress: 50,
        message: 'Test',
        status: 'RUNNING',
        version: 1,
      });

      const metrics = testEventBus.getMetrics();
      expect(metrics.processingErrors).toBe(1);
    });

    it('应该追踪最后事件时间', async () => {
      testEventBus.on('task-update', jest.fn());

      const beforeTime = Date.now();

      await testEventBus.processEvent('task-update', {
        task_id: 'test-task-11',
        task_type: 'Import',
        target: '/path/to/file.log',
        progress: 50,
        message: 'Test',
        status: 'RUNNING',
        version: 1,
      });

      const metrics = testEventBus.getMetrics();
      expect(metrics.lastEventTime).toBeGreaterThanOrEqual(beforeTime);
      expect(metrics.lastEventTime).toBeLessThanOrEqual(Date.now());
    });

    it('应该支持重置指标', async () => {
      testEventBus.on('task-update', jest.fn());

      await testEventBus.processEvent('task-update', {
        task_id: 'test-task-12',
        task_type: 'Import',
        target: '/path/to/file.log',
        progress: 50,
        message: 'Test',
        status: 'RUNNING',
        version: 1,
      });

      expect(testEventBus.getMetrics().totalEvents).toBe(1);

      testEventBus.resetMetrics();

      expect(testEventBus.getMetrics().totalEvents).toBe(0);
      expect(testEventBus.getMetrics().validationErrors).toBe(0);
      expect(testEventBus.getMetrics().idempotencySkips).toBe(0);
      expect(testEventBus.getMetrics().processingErrors).toBe(0);
    });
  });

  describe('配置管理', () => {
    it('应该支持禁用验证', async () => {
      const customBus = new EventBus({ enableValidation: false, enableLogging: false });
      customBus.on('task-update', jest.fn());

      // 无效事件应该被接受（因为验证被禁用）
      await expect(
        customBus.processEvent('task-update', {
          task_id: '', // 无效
          task_type: 'Import',
          target: '/path/to/file.log',
          progress: 150, // 无效
          message: 'Test',
          status: 'INVALID_STATUS',
        })
      ).resolves.not.toThrow();
    });

    it('应该支持禁用幂等性', async () => {
      const customBus = new EventBus({ enableIdempotency: false, enableLogging: false });
      let callCount = 0;
      const handler = jest.fn(() => {
        callCount++;
      });

      customBus.on('task-update', handler);

      const event = {
        task_id: 'test-task-13',
        task_type: 'Import',
        target: '/path/to/file.log',
        progress: 50,
        message: 'Test',
        status: 'RUNNING',
        version: 1,
      };

      // 两次处理相同事件
      await customBus.processEvent('task-update', event);
      await customBus.processEvent('task-update', event);

      // 幂等性禁用，应该调用两次
      expect(callCount).toBe(2);
    });

    it('应该支持更新配置', async () => {
      testEventBus.on('task-update', jest.fn());

      // 禁用验证
      testEventBus.updateConfig({ enableValidation: false });

      // 应该不抛出验证错误
      await expect(
        testEventBus.processEvent('task-update', {
          task_id: '',
          task_type: 'InvalidType',
          target: '/path',
          progress: 999,
          message: 'Test',
          status: 'INVALID',
        })
      ).resolves.not.toThrow();
    });

    it('应该支持清空幂等性缓存', async () => {
      const event = {
        task_id: 'test-task-14',
        task_type: 'Import',
        target: '/path/to/file.log',
        progress: 50,
        message: 'Test',
        status: 'RUNNING',
        version: 1,
      };

      testEventBus.on('task-update', jest.fn());

      await testEventBus.processEvent('task-update', event);
      expect(testEventBus.getMetrics().idempotencyCacheSize).toBe(1);

      // 清空缓存
      testEventBus.clearCache();
      expect(testEventBus.getMetrics().idempotencyCacheSize).toBe(0);

      // 现在应该可以再次处理相同事件
      await testEventBus.processEvent('task-update', event);
      expect(testEventBus.getMetrics().idempotencyCacheSize).toBe(1);
    });
  });

  describe('边界情况', () => {
    it('应该处理没有处理器的事件', async () => {
      // 不注册任何处理器
      const event = {
        task_id: 'test-task-15',
        task_type: 'Import',
        target: '/path/to/file.log',
        progress: 50,
        message: 'Test',
        status: 'RUNNING',
        version: 1,
      };

      // 应该不抛出错误
      await expect(
        testEventBus.processEvent('task-update', event)
      ).resolves.not.toThrow();
    });

    it('应该处理缺少可选字段的事件', async () => {
      testEventBus.on('task-update', jest.fn());

      const event = {
        task_id: 'test-task-16',
        task_type: 'Import',
        target: '/path/to/file.log',
        progress: 50,
        message: 'Test',
        status: 'RUNNING',
        // workspace_id、version、timestamp 都是可选的
      };

      await expect(
        testEventBus.processEvent('task-update', event)
      ).resolves.not.toThrow();
    });

    it('应该使用默认version为1', async () => {
      let receivedVersion = 0;
      testEventBus.on('task-update', (event: any) => {
        receivedVersion = event.version;
      });

      const event = {
        task_id: 'test-task-17',
        task_type: 'Import',
        target: '/path/to/file.log',
        progress: 50,
        message: 'Test',
        status: 'RUNNING',
        // 不提供version
      };

      await testEventBus.processEvent('task-update', event);
      expect(receivedVersion).toBe(1);
    });
  });
});
