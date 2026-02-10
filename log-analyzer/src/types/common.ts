// 从独立 stores 导入类型定义
import type { Toast } from '../stores/appStore';
import type { Workspace } from '../stores/workspaceStore';
import type { Task } from '../stores/taskStore';
import type { KeywordGroup } from '../stores/keywordStore';

// 日志条目类型
export interface LogEntry {
  id: number;
  timestamp: string;
  level: string;
  file: string;
  line: number;
  content: string;
  tags: any[];
  real_path?: string;
  /** 该行匹配的关键词列表 */
  matched_keywords?: string[];
}

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

/**
 * 文件过滤模式
 * - Whitelist: 白名单模式（仅允许指定文件）
 * - Blacklist: 黑名单模式（禁止指定文件）
 */
export enum FilterMode {
  Whitelist = 'whitelist',
  Blacklist = 'blacklist'
}

/**
 * 文件类型过滤配置（三层检测策略）
 *
 * 防御性设计：
 * - 默认禁用第2层智能过滤（enabled = false）
 * - 默认启用第1层二进制检测（binary_detection_enabled = true）
 * - 任何配置错误都会降级到默认行为（允许所有文件）
 */
export interface FileFilterConfig {
  /** 是否启用第2层智能过滤（第1层二进制检测始终启用） */
  enabled: boolean;

  /** 第1层：二进制文件检测（默认启用） */
  binary_detection_enabled: boolean;

  /** 第2层：过滤模式（whitelist 或 blacklist） */
  mode: FilterMode;

  /** 文件名 Glob 模式列表（支持无后缀日志） */
  filename_patterns: string[];

  /** 扩展名白名单 */
  allowed_extensions: string[];

  /** 扩展名黑名单 */
  forbidden_extensions: string[];
}

/**
 * 高级特性配置
 */
export interface AdvancedFeaturesConfig {
  /** 是否启用位图索引过滤器（RoaringBitmap） */
  enable_filter_engine: boolean;

  /** 是否启用正则表达式搜索引擎（LRU缓存） */
  enable_regex_engine: boolean;

  /** 是否启用时间分区索引（时序优化） */
  enable_time_partition: boolean;

  /** 是否启用自动补全引擎（Trie树） */
  enable_autocomplete: boolean;

  /** 正则表达式缓存大小（默认1000） */
  regex_cache_size: number;

  /** 自动补全建议数量（默认100） */
  autocomplete_limit: number;

  /** 时间分区大小（秒，默认3600 = 1小时） */
  time_partition_size_secs: number;
}

/**
 * 应用配置
 */
export interface AppConfig {
  /** 关键词分组配置 */
  keyword_groups: KeywordGroup[];

  /** 工作区配置 */
  workspaces: Workspace[];

  /** 高级搜索特性配置 */
  advanced_features: AdvancedFeaturesConfig;

  /** 文件类型过滤配置 */
  file_filter: FileFilterConfig;
}

// ========== 文件过滤配置类型结束 ==========

// ========== 性能监控类型 ==========

/**
 * 性能指标数据结构
 *
 * 用于展示系统性能、搜索性能、缓存命中率等信息
 */
export interface PerformanceMetrics {
  /** 搜索延迟指标 */
  searchLatency: {
    current: number;  // 当前延迟 (ms)
    average: number;  // 平均延迟 (ms)
    p95: number;      // 95分位延迟 (ms)
    p99: number;      // 99分位延迟 (ms)
  };
  /** 搜索吞吐量指标 */
  searchThroughput: {
    current: number;  // 当前吞吐量 (次/秒)
    average: number;  // 平均吞吐量 (次/秒)
    peak: number;     // 峰值吞吐量 (次/秒)
  };
  /** 缓存性能指标 */
  cacheMetrics: {
    hitRate: number;     // 命中率 (0-100)
    missCount: number;   // 未命中次数
    hitCount: number;    // 命中次数
    size: number;        // 当前缓存大小
    capacity: number;    // 缓存容量
    evictions: number;   // 驱逐次数
  };
  /** 内存使用指标 */
  memoryMetrics: {
    used: number;        // 已用内存 (MB)
    total: number;       // 总内存 (MB)
    heapUsed: number;    // 堆内存使用 (MB)
    external: number;    // 外部内存 (MB)
  };
  /** 任务执行指标 */
  taskMetrics: {
    total: number;       // 总任务数
    running: number;     // 运行中任务数
    completed: number;   // 已完成任务数
    failed: number;      // 失败任务数
    averageDuration: number; // 平均执行时间 (ms)
  };
  /** 索引指标 */
  indexMetrics: {
    totalFiles: number;     // 总文件数
    indexedFiles: number;   // 已索引文件数
    totalSize: number;      // 总大小 (bytes)
    indexSize: number;      // 索引大小 (bytes)
  };
}

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
