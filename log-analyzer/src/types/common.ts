// 从独立 stores 导入类型定义
import type { Toast } from '../stores/appStore';
import type { Workspace } from '../stores/workspaceStore';
import type { Task } from '../stores/taskStore';
import type { KeywordGroup } from '../stores/keywordStore';
import type { FileFilterConfig } from './api-responses';

// 从 api-responses 重新导出 LogEntry 和 MatchDetail 类型
// 这些类型使用 Zod Schema 定义，提供运行时类型安全
export type { LogEntry, MatchDetail } from './api-responses';

// 高级过滤器类型
export interface FilterOptions {
  timeRange: { start: string | null; end: string | null };
  levels: string[];
  filePattern: string;
}

// 重新导出 Store 类型供外部使用
export type { Toast, Workspace, Task, KeywordGroup };
export type { ToastType } from '../stores/appStore';
export type { KeywordPattern, ColorKey } from '../stores/keywordStore';

// ========== 文件过滤配置类型 ==========
// FilterMode 和 FileFilterConfig 以 api-responses.ts 为权威来源（含 Zod Schema），
// 此处 re-export 以保持向后兼容的 import 路径。
export { FilterMode } from './api-responses';
export type { FileFilterConfig } from './api-responses';

/**
 * 应用配置
 */
export interface AppConfig {
  /** 关键词分组配置 */
  keyword_groups: KeywordGroup[];

  /** 工作区配置 */
  workspaces: Workspace[];

  /** 文件类型过滤配置 */
  file_filter: FileFilterConfig;
}

// ========== 文件过滤配置类型结束 ==========

// ========== 性能监控类型 ==========
// PerformanceMetrics 以 api-responses.ts 为权威来源（含 Zod Schema），此处 re-export。
export type { PerformanceMetrics } from './api-responses';

// ========== 性能监控历史数据类型 ==========

/**
 * 时间范围类型（与后端一致）
 */
export type TimeRangeDto = 'LastHour' | 'Last6Hours' | 'Last24Hours' | 'Last7Days' | 'Last30Days';

/**
 * 指标快照数据结构（与后端 MetricsSnapshot 一致）
 */
export interface MetricsSnapshot {
  timestamp: number;
  search_latency_current: number;
  search_latency_average: number;
  search_latency_p95: number;
  search_latency_p99: number;
  throughput_current: number;
  throughput_average: number;
  throughput_peak: number;
  cache_hit_rate: number;
  cache_hit_count: number;
  cache_miss_count: number;
  cache_size: number;
  cache_capacity: number;
  memory_used: number;
  memory_total: number;
  task_total: number;
  task_running: number;
  task_completed: number;
  task_failed: number;
  index_total_files: number;
  index_indexed_files: number;
}

/**
 * 搜索事件数据结构
 */
export interface SearchEvent {
  id?: number;
  timestamp: number;
  workspace_id?: string;
  query: string;
  results_count: number;
  duration_ms: number;
  cache_hit: boolean;
}

/**
 * 指标存储统计信息
 */
export interface MetricsStoreStats {
  snapshot_count: number;
  event_count: number;
  latest_timestamp?: number;
  oldest_timestamp?: number;
}

/**
 * 历史指标数据响应
 */
export interface HistoricalMetricsData {
  snapshots: MetricsSnapshot[];
  stats: MetricsStoreStats;
}
