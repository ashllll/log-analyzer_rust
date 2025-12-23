/**
 * IPC 监控和指标收集
 * 
 * 业内成熟方案：
 * 1. Metrics 收集（成功率、延迟、重试次数）
 * 2. 告警规则（连续失败、断路器打开）
 * 3. 性能监控（P50、P95、P99 延迟）
 * 4. 仪表板展示
 */

import { logger } from './logger';

/**
 * IPC 调用指标
 */
export interface IPCCallMetrics {
  command: string;
  success: boolean;
  duration: number;
  attempts: number;
  timestamp: number;
  error?: string;
}

/**
 * IPC 聚合指标
 */
export interface IPCAggregatedMetrics {
  totalCalls: number;
  successfulCalls: number;
  failedCalls: number;
  successRate: number;
  totalRetries: number;
  averageDuration: number;
  p50Duration: number;
  p95Duration: number;
  p99Duration: number;
  commandStats: Map<string, CommandStats>;
}

/**
 * 单个命令的统计信息
 */
export interface CommandStats {
  command: string;
  totalCalls: number;
  successfulCalls: number;
  failedCalls: number;
  successRate: number;
  totalRetries: number;
  averageDuration: number;
  lastError?: string;
  lastErrorTime?: number;
}

/**
 * 告警类型
 */
export enum AlertType {
  HIGH_FAILURE_RATE = 'HIGH_FAILURE_RATE',
  CIRCUIT_BREAKER_OPEN = 'CIRCUIT_BREAKER_OPEN',
  HIGH_LATENCY = 'HIGH_LATENCY',
  CONSECUTIVE_FAILURES = 'CONSECUTIVE_FAILURES',
}

/**
 * 告警信息
 */
export interface Alert {
  type: AlertType;
  message: string;
  severity: 'critical' | 'warning' | 'info';
  timestamp: number;
  metadata?: Record<string, unknown>;
}

/**
 * IPC 指标收集器
 */
export class IPCMetricsCollector {
  private static instance: IPCMetricsCollector;
  private metrics: IPCCallMetrics[] = [];
  private alerts: Alert[] = [];
  private readonly maxMetricsSize = 1000; // 保留最近 1000 条记录
  private readonly maxAlertsSize = 100; // 保留最近 100 条告警
  
  // 告警阈值配置
  private readonly alertThresholds = {
    failureRateThreshold: 0.5, // 50% 失败率触发告警
    consecutiveFailuresThreshold: 3, // 连续 3 次失败触发告警
    highLatencyThreshold: 5000, // 5秒延迟触发告警
  };
  
  private consecutiveFailures = 0;
  private lastCircuitBreakerState: string = 'CLOSED';
  
  private constructor() {
    // 私有构造函数，单例模式
  }
  
  public static getInstance(): IPCMetricsCollector {
    if (!IPCMetricsCollector.instance) {
      IPCMetricsCollector.instance = new IPCMetricsCollector();
    }
    return IPCMetricsCollector.instance;
  }
  
  /**
   * 记录 IPC 调用指标
   */
  public recordCall(metrics: IPCCallMetrics): void {
    // 添加到指标列表
    this.metrics.push(metrics);
    
    // 限制指标数量
    if (this.metrics.length > this.maxMetricsSize) {
      this.metrics.shift();
    }
    
    // 结构化日志记录
    if (metrics.success) {
      logger.info(
        `[IPCMetrics] Command succeeded: ${metrics.command}`,
        {
          duration: `${metrics.duration}ms`,
          attempts: metrics.attempts,
          timestamp: new Date(metrics.timestamp).toISOString(),
        }
      );
      
      // 重置连续失败计数
      this.consecutiveFailures = 0;
    } else {
      logger.error(
        `[IPCMetrics] Command failed: ${metrics.command}`,
        {
          error: metrics.error,
          duration: `${metrics.duration}ms`,
          attempts: metrics.attempts,
          timestamp: new Date(metrics.timestamp).toISOString(),
        }
      );
      
      // 增加连续失败计数
      this.consecutiveFailures++;
    }
    
    // 检查告警条件
    this.checkAlertConditions(metrics);
  }
  
  /**
   * 检查告警条件
   */
  private checkAlertConditions(metrics: IPCCallMetrics): void {
    // 1. 检查连续失败
    if (this.consecutiveFailures >= this.alertThresholds.consecutiveFailuresThreshold) {
      this.addAlert({
        type: AlertType.CONSECUTIVE_FAILURES,
        message: `IPC连续失败 ${this.consecutiveFailures} 次`,
        severity: 'critical',
        timestamp: Date.now(),
        metadata: {
          consecutiveFailures: this.consecutiveFailures,
          lastCommand: metrics.command,
          lastError: metrics.error,
        },
      });
    }
    
    // 2. 检查高延迟
    if (metrics.duration > this.alertThresholds.highLatencyThreshold) {
      this.addAlert({
        type: AlertType.HIGH_LATENCY,
        message: `IPC调用延迟过高: ${metrics.duration}ms`,
        severity: 'warning',
        timestamp: Date.now(),
        metadata: {
          command: metrics.command,
          duration: metrics.duration,
          threshold: this.alertThresholds.highLatencyThreshold,
        },
      });
    }
    
    // 3. 检查失败率（最近 10 次调用）
    const recentMetrics = this.metrics.slice(-10);
    if (recentMetrics.length >= 10) {
      const failureRate = recentMetrics.filter(m => !m.success).length / recentMetrics.length;
      
      if (failureRate >= this.alertThresholds.failureRateThreshold) {
        this.addAlert({
          type: AlertType.HIGH_FAILURE_RATE,
          message: `IPC失败率过高: ${(failureRate * 100).toFixed(1)}%`,
          severity: 'critical',
          timestamp: Date.now(),
          metadata: {
            failureRate: failureRate,
            threshold: this.alertThresholds.failureRateThreshold,
            recentCalls: recentMetrics.length,
          },
        });
      }
    }
  }
  
  /**
   * 记录断路器状态变化
   */
  public recordCircuitBreakerStateChange(newState: string): void {
    if (newState !== this.lastCircuitBreakerState) {
      logger.warn(`[IPCMetrics] Circuit breaker state changed: ${this.lastCircuitBreakerState} -> ${newState}`);
      
      if (newState === 'OPEN') {
        this.addAlert({
          type: AlertType.CIRCUIT_BREAKER_OPEN,
          message: '断路器已打开，IPC连接暂时不可用',
          severity: 'critical',
          timestamp: Date.now(),
          metadata: {
            previousState: this.lastCircuitBreakerState,
            newState: newState,
          },
        });
      }
      
      this.lastCircuitBreakerState = newState;
    }
  }
  
  /**
   * 添加告警
   */
  private addAlert(alert: Alert): void {
    this.alerts.push(alert);
    
    // 限制告警数量
    if (this.alerts.length > this.maxAlertsSize) {
      this.alerts.shift();
    }
    
    // 根据严重程度记录日志
    const logMessage = `[IPCAlert] ${alert.type}: ${alert.message}`;
    
    switch (alert.severity) {
      case 'critical':
        logger.error(logMessage, alert.metadata);
        break;
      case 'warning':
        logger.warn(logMessage, alert.metadata);
        break;
      case 'info':
        logger.info(logMessage, alert.metadata);
        break;
    }
  }
  
  /**
   * 获取聚合指标
   */
  public getAggregatedMetrics(): IPCAggregatedMetrics {
    if (this.metrics.length === 0) {
      return {
        totalCalls: 0,
        successfulCalls: 0,
        failedCalls: 0,
        successRate: 0,
        totalRetries: 0,
        averageDuration: 0,
        p50Duration: 0,
        p95Duration: 0,
        p99Duration: 0,
        commandStats: new Map(),
      };
    }
    
    const totalCalls = this.metrics.length;
    const successfulCalls = this.metrics.filter(m => m.success).length;
    const failedCalls = totalCalls - successfulCalls;
    const successRate = successfulCalls / totalCalls;
    const totalRetries = this.metrics.reduce((sum, m) => sum + (m.attempts - 1), 0);
    
    // 计算延迟统计
    const durations = this.metrics.map(m => m.duration).sort((a, b) => a - b);
    const averageDuration = durations.reduce((sum, d) => sum + d, 0) / durations.length;
    const p50Duration = this.calculatePercentile(durations, 0.5);
    const p95Duration = this.calculatePercentile(durations, 0.95);
    const p99Duration = this.calculatePercentile(durations, 0.99);
    
    // 按命令统计
    const commandStats = this.calculateCommandStats();
    
    return {
      totalCalls,
      successfulCalls,
      failedCalls,
      successRate,
      totalRetries,
      averageDuration,
      p50Duration,
      p95Duration,
      p99Duration,
      commandStats,
    };
  }
  
  /**
   * 计算百分位数
   */
  private calculatePercentile(sortedValues: number[], percentile: number): number {
    if (sortedValues.length === 0) return 0;
    
    const index = Math.ceil(sortedValues.length * percentile) - 1;
    return sortedValues[Math.max(0, index)];
  }
  
  /**
   * 按命令计算统计信息
   */
  private calculateCommandStats(): Map<string, CommandStats> {
    const statsMap = new Map<string, CommandStats>();
    
    for (const metric of this.metrics) {
      const existing = statsMap.get(metric.command);
      
      if (existing) {
        existing.totalCalls++;
        if (metric.success) {
          existing.successfulCalls++;
        } else {
          existing.failedCalls++;
          existing.lastError = metric.error;
          existing.lastErrorTime = metric.timestamp;
        }
        existing.totalRetries += (metric.attempts - 1);
        existing.successRate = existing.successfulCalls / existing.totalCalls;
        
        // 更新平均延迟（增量计算）
        const totalDuration = existing.averageDuration * (existing.totalCalls - 1) + metric.duration;
        existing.averageDuration = totalDuration / existing.totalCalls;
      } else {
        statsMap.set(metric.command, {
          command: metric.command,
          totalCalls: 1,
          successfulCalls: metric.success ? 1 : 0,
          failedCalls: metric.success ? 0 : 1,
          successRate: metric.success ? 1 : 0,
          totalRetries: metric.attempts - 1,
          averageDuration: metric.duration,
          lastError: metric.error,
          lastErrorTime: metric.error ? metric.timestamp : undefined,
        });
      }
    }
    
    return statsMap;
  }
  
  /**
   * 获取最近的告警
   */
  public getRecentAlerts(limit: number = 10): Alert[] {
    return this.alerts.slice(-limit).reverse();
  }
  
  /**
   * 获取所有告警
   */
  public getAllAlerts(): Alert[] {
    return [...this.alerts].reverse();
  }
  
  /**
   * 清除告警
   */
  public clearAlerts(): void {
    this.alerts = [];
    logger.info('[IPCMetrics] Alerts cleared');
  }
  
  /**
   * 重置指标
   */
  public resetMetrics(): void {
    this.metrics = [];
    this.consecutiveFailures = 0;
    logger.info('[IPCMetrics] Metrics reset');
  }
  
  /**
   * 导出指标（用于外部监控系统）
   */
  public exportMetrics(): {
    metrics: IPCCallMetrics[];
    aggregated: IPCAggregatedMetrics;
    alerts: Alert[];
  } {
    return {
      metrics: [...this.metrics],
      aggregated: this.getAggregatedMetrics(),
      alerts: [...this.alerts],
    };
  }
}

/**
 * 获取全局 IPC 指标收集器实例
 */
export const getIPCMetricsCollector = (): IPCMetricsCollector => {
  return IPCMetricsCollector.getInstance();
};
