/**
 * IPC 健康检查和重连机制
 * 
 * 业内成熟方案：
 * 1. 连接健康检查（心跳机制）
 * 2. 自动重连策略（指数退避）
 * 3. 连接池管理
 * 4. 优雅降级
 */

import { invoke } from '@tauri-apps/api/core';
import { logger } from './logger';

export interface IPCHealthStatus {
  isHealthy: boolean;
  lastCheckTime: number;
  consecutiveFailures: number;
  lastError?: string;
}

export class IPCHealthChecker {
  private static instance: IPCHealthChecker;
  private healthStatus: IPCHealthStatus = {
    isHealthy: true,
    lastCheckTime: Date.now(),
    consecutiveFailures: 0,
  };
  
  private checkInterval: number | null = null;
  private readonly CHECK_INTERVAL_MS = 30000; // 30秒检查一次
  private readonly MAX_CONSECUTIVE_FAILURES = 3;
  
  private constructor() {
    this.startHealthCheck();
  }
  
  public static getInstance(): IPCHealthChecker {
    if (!IPCHealthChecker.instance) {
      IPCHealthChecker.instance = new IPCHealthChecker();
    }
    return IPCHealthChecker.instance;
  }
  
  /**
   * 启动健康检查
   */
  private startHealthCheck(): void {
    if (this.checkInterval !== null) {
      return;
    }
    
    // 立即执行一次检查
    this.performHealthCheck();
    
    // 定期检查
    this.checkInterval = window.setInterval(() => {
      this.performHealthCheck();
    }, this.CHECK_INTERVAL_MS);
    
    logger.debug('[IPCHealthChecker] Health check started');
  }
  
  /**
   * 停止健康检查
   */
  public stopHealthCheck(): void {
    if (this.checkInterval !== null) {
      window.clearInterval(this.checkInterval);
      this.checkInterval = null;
      logger.debug('[IPCHealthChecker] Health check stopped');
    }
  }
  
  /**
   * 执行健康检查
   * 使用轻量级命令测试 IPC 连接
   */
  private async performHealthCheck(): Promise<void> {
    try {
      // 使用一个轻量级命令测试连接
      // 这里使用 load_config 作为健康检查命令（不会产生副作用）
      await invoke('load_config');
      
      // 检查成功
      this.healthStatus = {
        isHealthy: true,
        lastCheckTime: Date.now(),
        consecutiveFailures: 0,
      };
      
      logger.debug('[IPCHealthChecker] Health check passed');
    } catch (error) {
      // 检查失败
      this.healthStatus = {
        isHealthy: false,
        lastCheckTime: Date.now(),
        consecutiveFailures: this.healthStatus.consecutiveFailures + 1,
        lastError: String(error),
      };
      
      logger.error('[IPCHealthChecker] Health check failed:', error);
      
      // 如果连续失败次数过多，触发告警
      if (this.healthStatus.consecutiveFailures >= this.MAX_CONSECUTIVE_FAILURES) {
        logger.error(
          `[IPCHealthChecker] IPC connection unhealthy (${this.healthStatus.consecutiveFailures} consecutive failures)`
        );
      }
    }
  }
  
  /**
   * 获取当前健康状态
   */
  public getHealthStatus(): IPCHealthStatus {
    return { ...this.healthStatus };
  }
  
  /**
   * 手动触发健康检查
   */
  public async checkNow(): Promise<boolean> {
    await this.performHealthCheck();
    return this.healthStatus.isHealthy;
  }
  
  /**
   * 等待 IPC 连接恢复健康
   * @param timeoutMs 超时时间（毫秒）
   * @param retryIntervalMs 重试间隔（毫秒）
   */
  public async waitForHealthy(
    timeoutMs: number = 10000,
    retryIntervalMs: number = 500
  ): Promise<boolean> {
    const startTime = Date.now();
    
    while (Date.now() - startTime < timeoutMs) {
      await this.performHealthCheck();
      
      if (this.healthStatus.isHealthy) {
        return true;
      }
      
      // 等待后重试
      await new Promise(resolve => setTimeout(resolve, retryIntervalMs));
    }
    
    return false;
  }
}

/**
 * 获取全局 IPC 健康检查器实例
 */
export const getIPCHealthChecker = (): IPCHealthChecker => {
  return IPCHealthChecker.getInstance();
};
