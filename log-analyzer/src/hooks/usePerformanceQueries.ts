/**
 * 性能监控查询 Hooks
 *
 * 使用 React Query 管理性能指标数据
 * 符合项目"必须使用业内成熟方案"原则
 */

import { useQuery } from '@tanstack/react-query';
import { logger } from '../utils/logger';
import { api } from '../services/api';
import type { PerformanceMetrics } from '../types/common';

// ============================================================================
// Query Keys
// ============================================================================

export const performanceQueryKeys = {
  metrics: ['performanceMetrics'] as const,
} as const;

/**
 * 默认性能指标数据
 * 用于初始加载或失败时的回退数据
 */
export const DEFAULT_PERFORMANCE_METRICS: PerformanceMetrics = {
  searchLatency: {
    current: 0,
    average: 0,
    p95: 0,
    p99: 0,
  },
  searchThroughput: {
    current: 0,
    average: 0,
    peak: 0,
  },
  cacheMetrics: {
    hitRate: 0,
    hitCount: 0,
    missCount: 0,
    size: 0,
    capacity: 1000,
    evictions: 0,
  },
  memoryMetrics: {
    used: 0,
    total: 100,
    heapUsed: 0,
    external: 0,
  },
  taskMetrics: {
    total: 0,
    running: 0,
    completed: 0,
    failed: 0,
    averageDuration: 0,
  },
  indexMetrics: {
    totalFiles: 0,
    indexedFiles: 0,
    totalSize: 0,
    indexSize: 0,
  },
};

// ============================================================================
// Performance Queries
// ============================================================================

/**
 * 获取性能指标
 *
 * @param options - 查询选项
 * @param options.enabled - 是否启用查询（默认 true）
 * @param options.refetchInterval - 自动刷新间隔（毫秒），false 表示禁用
 * @returns React Query 结果
 */
export const usePerformanceMetrics = (options?: {
  enabled?: boolean;
  refetchInterval?: number | false;
}) => {
  return useQuery<PerformanceMetrics, Error>({
    queryKey: performanceQueryKeys.metrics,
    queryFn: async () => {
      logger.debug('[QUERY] Loading performance metrics');
      const metrics = await api.getPerformanceMetrics();
      return metrics;
    },
    enabled: options?.enabled ?? true,
    refetchInterval: options?.refetchInterval ?? false,
    staleTime: 5000, // 5秒后数据过期
    gcTime: 60000, // 1分钟后垃圾回收
    retry: 2,
    retryDelay: (attemptIndex) => Math.min(1000 * 2 ** attemptIndex, 10000),
  });
};

/**
 * 性能指标查询 Hook（带自动刷新）
 *
 * @param autoRefresh - 是否自动刷新（默认 true）
 * @param refreshInterval - 刷新间隔（毫秒，默认 5000ms）
 * @returns React Query 结果
 */
export const useAutoRefreshPerformanceMetrics = (
  autoRefresh: boolean = true,
  refreshInterval: number = 5000
) => {
  return usePerformanceMetrics({
    enabled: true,
    refetchInterval: autoRefresh ? refreshInterval : false,
  });
};
