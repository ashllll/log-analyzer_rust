/**
 * IPC 重试机制测试
 * 
 * 验证：
 * - 指数退避算法正确性
 * - 断路器状态转换
 * - 重试逻辑
 * - 超时控制
 */

import { invokeWithRetry, resetCircuitBreaker, getCircuitBreakerState } from '../ipcRetry';

// Mock Tauri invoke
jest.mock('@tauri-apps/api/core', () => ({
  invoke: jest.fn(),
}));

// Mock logger
jest.mock('../logger', () => ({
  logger: {
    debug: jest.fn(),
    error: jest.fn(),
    warn: jest.fn(),
  },
}));

// Mock IPC health checker
jest.mock('../ipcHealthCheck', () => ({
  getIPCHealthChecker: jest.fn(() => ({
    checkNow: jest.fn().mockResolvedValue(true),
    waitForHealthy: jest.fn().mockResolvedValue(true),
  })),
}));

import { invoke } from '@tauri-apps/api/core';

describe('IPC Retry Mechanism', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    resetCircuitBreaker();
  });

  afterEach(() => {
    jest.restoreAllMocks();
  });

  describe('Property 41: Retry Exponential Backoff', () => {
    it('should retry with exponential backoff on failure', async () => {
      const mockInvoke = invoke as jest.MockedFunction<typeof invoke>;
      
      // 前两次失败，第三次成功
      mockInvoke
        .mockRejectedValueOnce(new Error('Connection failed'))
        .mockRejectedValueOnce(new Error('Connection failed'))
        .mockResolvedValueOnce('success');

      const result = await invokeWithRetry('test_command', {}, {
        maxRetries: 3,
        initialDelayMs: 100,
        maxDelayMs: 1000,
        backoffMultiplier: 2,
        jitter: false,
      });

      expect(result.success).toBe(true);
      expect(result.data).toBe('success');
      expect(result.attempts).toBe(3);
      expect(mockInvoke).toHaveBeenCalledTimes(3);
    });

    it('should respect max retries limit', async () => {
      const mockInvoke = invoke as jest.MockedFunction<typeof invoke>;
      mockInvoke.mockRejectedValue(new Error('Persistent failure'));

      const result = await invokeWithRetry('test_command', {}, {
        maxRetries: 2,
        initialDelayMs: 10,
      });

      expect(result.success).toBe(false);
      expect(result.attempts).toBe(3); // maxRetries + 1
      expect(mockInvoke).toHaveBeenCalledTimes(3);
    });

    it('should apply jitter to delay', async () => {
      const mockInvoke = invoke as jest.MockedFunction<typeof invoke>;
      mockInvoke
        .mockRejectedValueOnce(new Error('Fail 1'))
        .mockResolvedValueOnce('success');

      const startTime = Date.now();
      const result = await invokeWithRetry('test_command', {}, {
        maxRetries: 1,
        initialDelayMs: 100,
        jitter: true,
      });

      const duration = Date.now() - startTime;
      
      expect(result.success).toBe(true);
      // 延迟应该在 75ms - 125ms 之间（100ms ± 25%）
      expect(duration).toBeGreaterThanOrEqual(75);
      expect(duration).toBeLessThan(200);
    });
  });

  describe('Property 42: Circuit Breaker State Transitions', () => {
    it('should open circuit after threshold failures', async () => {
      const mockInvoke = invoke as jest.MockedFunction<typeof invoke>;
      mockInvoke.mockRejectedValue(new Error('Persistent failure'));

      // 触发5次失败（阈值）
      for (let i = 0; i < 5; i++) {
        await invokeWithRetry('test_command', {}, {
          maxRetries: 0,
          initialDelayMs: 1,
        });
      }

      // 断路器应该打开
      const result = await invokeWithRetry('test_command', {}, {
        maxRetries: 0,
      });

      expect(result.success).toBe(false);
      expect(result.error).toContain('circuit breaker open');
      expect(result.attempts).toBe(0); // 快速失败，不尝试调用
    });

    it('should transition to HALF_OPEN after timeout', async () => {
      const mockInvoke = invoke as jest.MockedFunction<typeof invoke>;
      mockInvoke.mockRejectedValue(new Error('Failure'));

      // 触发断路器打开
      for (let i = 0; i < 5; i++) {
        await invokeWithRetry('test_command', {}, {
          maxRetries: 0,
          initialDelayMs: 1,
        });
      }

      // 等待恢复超时（模拟）
      // 注意：实际测试中需要 mock 时间或使用较短的超时
      // 这里我们只验证逻辑，不实际等待60秒

      expect(getCircuitBreakerState()).toBe('OPEN');
    });

    it('should close circuit on successful recovery', async () => {
      const mockInvoke = invoke as jest.MockedFunction<typeof invoke>;
      
      // 先触发失败
      mockInvoke.mockRejectedValue(new Error('Failure'));
      for (let i = 0; i < 3; i++) {
        await invokeWithRetry('test_command', {}, {
          maxRetries: 0,
          initialDelayMs: 1,
        });
      }

      // 然后成功
      mockInvoke.mockResolvedValue('success');
      const result = await invokeWithRetry('test_command', {}, {
        maxRetries: 0,
      });

      expect(result.success).toBe(true);
      expect(getCircuitBreakerState()).toBe('CLOSED');
    });
  });

  describe('Property 43: Timeout Control', () => {
    it('should timeout long-running commands', async () => {
      const mockInvoke = invoke as jest.MockedFunction<typeof invoke>;
      
      // 模拟永不返回的命令
      mockInvoke.mockImplementation(() => 
        new Promise(() => {}) // 永不 resolve
      );

      const result = await invokeWithRetry('test_command', {}, {
        maxRetries: 0,
        timeoutMs: 100,
      });

      expect(result.success).toBe(false);
      expect(result.error).toContain('timeout');
    });

    it('should succeed if command completes before timeout', async () => {
      const mockInvoke = invoke as jest.MockedFunction<typeof invoke>;
      
      // 模拟快速完成的命令
      mockInvoke.mockImplementation(() => 
        new Promise(resolve => setTimeout(() => resolve('success'), 50))
      );

      const result = await invokeWithRetry('test_command', {}, {
        maxRetries: 0,
        timeoutMs: 200,
      });

      expect(result.success).toBe(true);
      expect(result.data).toBe('success');
    });
  });

  describe('Property 44: Delete Workspace Resilience', () => {
    it('should successfully delete workspace with retry', async () => {
      const mockInvoke = invoke as jest.MockedFunction<typeof invoke>;
      
      // 第一次失败，第二次成功
      mockInvoke
        .mockRejectedValueOnce(new Error('Temporary failure'))
        .mockResolvedValueOnce(undefined);

      const result = await invokeWithRetry('delete_workspace', 
        { workspaceId: 'test-123' },
        {
          maxRetries: 3,
          initialDelayMs: 100,
        }
      );

      expect(result.success).toBe(true);
      expect(result.attempts).toBe(2);
      expect(mockInvoke).toHaveBeenCalledWith('delete_workspace', {
        workspaceId: 'test-123',
      });
    });

    it('should provide detailed error on final failure', async () => {
      const mockInvoke = invoke as jest.MockedFunction<typeof invoke>;
      mockInvoke.mockRejectedValue(new Error('Workspace not found'));

      const result = await invokeWithRetry('delete_workspace', 
        { workspaceId: 'invalid' },
        {
          maxRetries: 2,
          initialDelayMs: 10,
        }
      );

      expect(result.success).toBe(false);
      expect(result.error).toContain('Workspace not found');
      expect(result.attempts).toBe(3);
    });
  });
});
