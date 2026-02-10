/**
 * 性能监控查询 Hooks
 *
 * 使用 React Query 管理性能指标数据
 * 符合项目"必须使用业内成熟方案"原则
 */

import { useQuery } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { logger } from '../utils/logger';
import type {
  PerformanceMetrics,
  TimeRangeDto,
  MetricsSnapshot,
  SearchEvent,
  MetricsStoreStats,
  HistoricalMetricsData,
} from '../types/common';

// ============================================================================
// API 封装（保持向后兼容）
// ============================================================================

const api = {
  getPerformanceMetrics: async (): Promise<PerformanceMetrics> => {
    return invoke<PerformanceMetrics>('get_performance_metrics');
  },
};

// ============================================================================
// Query Keys
// ============================================================================

export const performanceQueryKeys = {
  metrics: ['performanceMetrics'] as const,
  historical: (range: TimeRangeDto) => ['performanceHistorical', range] as const,
  aggregated: (range: TimeRangeDto, interval: number) =>
    ['performanceAggregated', range, interval] as const,
  searchEvents: (range: TimeRangeDto, workspaceId?: string) =>
    ['performanceSearchEvents', range, workspaceId] as const,
  stats: ['performanceStats'] as const,
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

// ============================================================================
// 历史数据查询 Hooks
// ============================================================================

/**
 * 获取历史指标数据
 *
 * @param range - 时间范围
 * @param options - 查询选项
 * @returns React Query 结果
 */
export const useHistoricalMetrics = (
  range: TimeRangeDto,
  options?: {
    enabled?: boolean;
    refetchInterval?: number | false;
  }
) => {
  return useQuery<HistoricalMetricsData, Error>({
    queryKey: performanceQueryKeys.historical(range),
    queryFn: async () => {
      logger.debug('[QUERY] Loading historical metrics', { range });
      const data = await invoke<HistoricalMetricsData>('get_historical_metrics', {
        range,
      });
      return data;
    },
    enabled: options?.enabled ?? true,
    refetchInterval: options?.refetchInterval ?? false,
    staleTime: 30000, // 30秒后数据过期
    gcTime: 300000, // 5分钟后垃圾回收
    retry: 1,
  });
};

/**
 * 获取聚合指标数据
 *
 * @param range - 时间范围
 * @param intervalSeconds - 聚合间隔（秒）
 * @param options - 查询选项
 * @returns React Query 结果
 */
export const useAggregatedMetrics = (
  range: TimeRangeDto,
  intervalSeconds: number,
  options?: {
    enabled?: boolean;
  }
) => {
  return useQuery<MetricsSnapshot[], Error>({
    queryKey: performanceQueryKeys.aggregated(range, intervalSeconds),
    queryFn: async () => {
      logger.debug('[QUERY] Loading aggregated metrics', { range, intervalSeconds });
      const data = await invoke<MetricsSnapshot[]>('get_aggregated_metrics', {
        range,
        intervalSeconds,
      });
      return data;
    },
    enabled: options?.enabled ?? true,
    staleTime: 60000, // 1分钟后数据过期
    gcTime: 300000,
    retry: 1,
  });
};

/**
 * 获取搜索事件
 *
 * @param range - 时间范围
 * @param workspaceId - 可选的工作区 ID 过滤
 * @param options - 查询选项
 * @returns React Query 结果
 */
export const useSearchEvents = (
  range: TimeRangeDto,
  workspaceId?: string,
  options?: {
    enabled?: boolean;
  }
) => {
  return useQuery<SearchEvent[], Error>({
    queryKey: performanceQueryKeys.searchEvents(range, workspaceId),
    queryFn: async () => {
      logger.debug('[QUERY] Loading search events', { range, workspaceId });
      const data = await invoke<SearchEvent[]>('get_search_events', {
        range,
        workspaceId,
      });
      return data;
    },
    enabled: options?.enabled ?? true,
    staleTime: 60000,
    gcTime: 300000,
    retry: 1,
  });
};

/**
 * 获取指标存储统计信息
 *
 * @param options - 查询选项
 * @returns React Query 结果
 */
export const useMetricsStats = (options?: {
  enabled?: boolean;
  refetchInterval?: number | false;
}) => {
  return useQuery<MetricsStoreStats, Error>({
    queryKey: performanceQueryKeys.stats,
    queryFn: async () => {
      logger.debug('[QUERY] Loading metrics stats');
      const data = await invoke<MetricsStoreStats>('get_metrics_stats');
      return data;
    },
    enabled: options?.enabled ?? true,
    refetchInterval: options?.refetchInterval ?? false,
    staleTime: 60000,
    gcTime: 300000,
    retry: 1,
  });
};
