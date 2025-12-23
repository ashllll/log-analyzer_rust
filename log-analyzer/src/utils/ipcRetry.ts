/**
 * IPC 命令重试机制
 * 
 * 业内成熟方案：
 * 1. 指数退避算法（Exponential Backoff）
 * 2. 抖动（Jitter）避免雷鸣群效应
 * 3. 断路器模式（Circuit Breaker）
 * 4. 超时控制
 */

import { invoke } from '@tauri-apps/api/core';
import { logger } from './logger';
import { getIPCHealthChecker } from './ipcHealthCheck';
import { getIPCMetricsCollector } from './ipcMetrics';

export interface RetryOptions {
  maxRetries?: number;
  initialDelayMs?: number;
  maxDelayMs?: number;
  backoffMultiplier?: number;
  timeoutMs?: number;
  jitter?: boolean;
}

export interface RetryResult<T> {
  success: boolean;
  data?: T;
  error?: string;
  attempts: number;
  totalDuration: number;
}

/**
 * 断路器状态
 */
enum CircuitState {
  CLOSED = 'CLOSED',     // 正常状态
  OPEN = 'OPEN',         // 断开状态（快速失败）
  HALF_OPEN = 'HALF_OPEN' // 半开状态（尝试恢复）
}

/**
 * 断路器实现
 */
class CircuitBreaker {
  private state: CircuitState = CircuitState.CLOSED;
  private failureCount: number = 0;
  private lastFailureTime: number = 0;
  private readonly failureThreshold: number = 5;
  private readonly resetTimeoutMs: number = 60000; // 1分钟后尝试恢复
  
  public canExecute(): boolean {
    if (this.state === CircuitState.CLOSED) {
      return true;
    }
    
    if (this.state === CircuitState.OPEN) {
      // 检查是否可以进入半开状态
      if (Date.now() - this.lastFailureTime >= this.resetTimeoutMs) {
        logger.debug('[CircuitBreaker] Transitioning to HALF_OPEN state');
        this.state = CircuitState.HALF_OPEN;
        return true;
      }
      return false;
    }
    
    // HALF_OPEN 状态允许执行
    return true;
  }
  
  public recordSuccess(): void {
    this.failureCount = 0;
    if (this.state === CircuitState.HALF_OPEN) {
      logger.debug('[CircuitBreaker] Transitioning to CLOSED state');
      this.state = CircuitState.CLOSED;
      
      // 记录状态变化
      const metricsCollector = getIPCMetricsCollector();
      metricsCollector.recordCircuitBreakerStateChange(this.state);
    }
  }
  
  public recordFailure(): void {
    this.failureCount++;
    this.lastFailureTime = Date.now();
    
    if (this.failureCount >= this.failureThreshold) {
      if (this.state !== CircuitState.OPEN) {
        logger.error('[CircuitBreaker] Transitioning to OPEN state');
        this.state = CircuitState.OPEN;
        
        // 记录状态变化
        const metricsCollector = getIPCMetricsCollector();
        metricsCollector.recordCircuitBreakerStateChange(this.state);
      }
    }
  }
  
  public getState(): CircuitState {
    return this.state;
  }
  
  public reset(): void {
    this.state = CircuitState.CLOSED;
    this.failureCount = 0;
    this.lastFailureTime = 0;
  }
}

// 全局断路器实例
const circuitBreaker = new CircuitBreaker();

/**
 * 计算重试延迟（指数退避 + 抖动）
 */
function calculateDelay(
  attempt: number,
  initialDelayMs: number,
  maxDelayMs: number,
  backoffMultiplier: number,
  jitter: boolean
): number {
  // 指数退避
  let delay = Math.min(
    initialDelayMs * Math.pow(backoffMultiplier, attempt),
    maxDelayMs
  );
  
  // 添加抖动（±25%）
  if (jitter) {
    const jitterRange = delay * 0.25;
    delay = delay + (Math.random() * 2 - 1) * jitterRange;
  }
  
  return Math.floor(delay);
}

/**
 * 带重试的 IPC 命令调用
 * 
 * @param command Tauri 命令名称
 * @param args 命令参数
 * @param options 重试选项
 */
export async function invokeWithRetry<T = unknown>(
  command: string,
  args?: Record<string, unknown>,
  options: RetryOptions = {}
): Promise<RetryResult<T>> {
  const {
    maxRetries = 3,
    initialDelayMs = 1000,
    maxDelayMs = 10000,
    backoffMultiplier = 2,
    timeoutMs = 30000,
    jitter = true,
  } = options;
  
  const startTime = Date.now();
  let lastError: string = '';
  const metricsCollector = getIPCMetricsCollector();
  
  // 检查断路器状态
  if (!circuitBreaker.canExecute()) {
    logger.error(`[invokeWithRetry] Circuit breaker is OPEN, fast-failing command: ${command}`);
    
    const result = {
      success: false,
      error: 'IPC connection is temporarily unavailable (circuit breaker open)',
      attempts: 0,
      totalDuration: 0,
    };
    
    // 记录指标
    metricsCollector.recordCall({
      command,
      success: false,
      duration: 0,
      attempts: 0,
      timestamp: startTime,
      error: result.error,
    });
    
    return result;
  }
  
  for (let attempt = 0; attempt <= maxRetries; attempt++) {
    try {
      logger.debug(`[invokeWithRetry] Attempt ${attempt + 1}/${maxRetries + 1} for command: ${command}`);
      
      // 设置超时
      const timeoutPromise = new Promise<never>((_, reject) => {
        setTimeout(() => reject(new Error('Command timeout')), timeoutMs);
      });
      
      // 执行命令
      const resultPromise = invoke<T>(command, args);
      
      // 竞速：命令执行 vs 超时
      const result = await Promise.race([resultPromise, timeoutPromise]);
      
      // 成功
      circuitBreaker.recordSuccess();
      
      const totalDuration = Date.now() - startTime;
      logger.debug(`[invokeWithRetry] Command succeeded: ${command} (${totalDuration}ms)`);
      
      // 记录成功指标
      metricsCollector.recordCall({
        command,
        success: true,
        duration: totalDuration,
        attempts: attempt + 1,
        timestamp: startTime,
      });
      
      return {
        success: true,
        data: result,
        attempts: attempt + 1,
        totalDuration,
      };
    } catch (error) {
      lastError = String(error);
      logger.error(`[invokeWithRetry] Attempt ${attempt + 1} failed for command: ${command}`, error);
      
      // 如果是最后一次尝试，不再重试
      if (attempt === maxRetries) {
        circuitBreaker.recordFailure();
        break;
      }
      
      // 计算延迟
      const delay = calculateDelay(attempt, initialDelayMs, maxDelayMs, backoffMultiplier, jitter);
      logger.debug(`[invokeWithRetry] Retrying in ${delay}ms...`);
      
      // 等待后重试
      await new Promise(resolve => setTimeout(resolve, delay));
      
      // 在重试前检查 IPC 健康状态
      const healthChecker = getIPCHealthChecker();
      const isHealthy = await healthChecker.checkNow();
      
      if (!isHealthy) {
        logger.warn('[invokeWithRetry] IPC connection unhealthy, waiting for recovery...');
        const recovered = await healthChecker.waitForHealthy(5000, 500);
        
        if (!recovered) {
          logger.error('[invokeWithRetry] IPC connection failed to recover');
          circuitBreaker.recordFailure();
          break;
        }
      }
    }
  }
  
  // 所有重试都失败
  const totalDuration = Date.now() - startTime;
  
  // 记录失败指标
  metricsCollector.recordCall({
    command,
    success: false,
    duration: totalDuration,
    attempts: maxRetries + 1,
    timestamp: startTime,
    error: lastError,
  });
  
  return {
    success: false,
    error: lastError,
    attempts: maxRetries + 1,
    totalDuration,
  };
}

/**
 * 重置断路器（用于测试或手动恢复）
 */
export function resetCircuitBreaker(): void {
  circuitBreaker.reset();
  logger.debug('[CircuitBreaker] Manually reset');
}

/**
 * 获取断路器状态
 */
export function getCircuitBreakerState(): string {
  return circuitBreaker.getState();
}
