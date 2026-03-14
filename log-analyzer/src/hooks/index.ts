// 导出所有自定义Hooks
export { useWorkspaceOperations } from './useWorkspaceOperations';
export { useTaskManager } from './useTaskManager';
export { useKeywordManager } from './useKeywordManager';

// 流式无限搜索
export {
  useInfiniteSearch,
  registerSearchSession,
  removeSearchSession,
  getSearchSessionInfo,
  getVirtualSearchStats,
  searchQueryKeys,
} from './useInfiniteSearch';
export type {
  SearchPage,
  UseInfiniteSearchOptions,
  SearchContext,
} from './useInfiniteSearch';

// 性能监控查询
export {
  usePerformanceMetrics,
  useAutoRefreshPerformanceMetrics,
  useHistoricalMetrics,
  useAggregatedMetrics,
  useSearchEvents,
  useMetricsStats,
  performanceQueryKeys,
  DEFAULT_PERFORMANCE_METRICS,
} from './usePerformanceQueries';
