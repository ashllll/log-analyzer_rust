/**
 * API 响应类型定义
 *
 * 使用 Zod 为所有 API 响应添加 Schema 验证，提供运行时类型安全
 *
 * @module api-responses
 */

import { z } from 'zod';

// ============================================================================
// RAR 支持信息
// ============================================================================

/**
 * RAR 支持信息 Schema
 */
export const RarSupportInfoSchema = z.object({
  supported: z.boolean(),
  version: z.string().optional(),
});

/**
 * RAR 支持信息类型
 */
export type RarSupportInfo = z.infer<typeof RarSupportInfoSchema>;

// ============================================================================
// 虚拟文件节点
// ============================================================================

/**
 * 虚拟文件节点类型
 * 类型定义必须在 Schema 之前，避免循环引用问题
 */
export type VirtualFileNode = {
  name: string;
  path: string;
  is_directory: boolean;
  size?: number;
  children?: VirtualFileNode[];
};

/**
 * 虚拟文件节点 Schema
 * 使用 lazy 处理递归结构
 */
export const VirtualFileNodeSchema: z.ZodType<VirtualFileNode> = z.object({
  name: z.string(),
  path: z.string(),
  is_directory: z.boolean(),
  size: z.number().optional(),
  children: z.lazy(() => VirtualFileNodeSchema.array()).optional(),
});

// ============================================================================
// 工作区状态
// ============================================================================

/**
 * 工作区状态 Schema
 */
export const WorkspaceStateSchema = z.object({
  id: z.string(),
  name: z.string(),
  path: z.string(),
  status: z.enum(['READY', 'PROCESSING', 'OFFLINE', 'ERROR']),
  last_accessed: z.number().optional(),
});

/**
 * 工作区状态类型
 */
export type WorkspaceState = z.infer<typeof WorkspaceStateSchema>;

// ============================================================================
// 事件记录
// ============================================================================

/**
 * 事件记录 Schema
 * 使用 record 定义任意键值对，值为 unknown 类型
 */
export const EventRecordSchema = z.object({
  id: z.string(),
  timestamp: z.number(),
  type: z.string(),
  payload: z.record(z.string(), z.unknown()),
});

/**
 * 事件记录类型
 */
export type EventRecord = z.infer<typeof EventRecordSchema>;

// ============================================================================
// 文件过滤模式
// ============================================================================

/**
 * 文件过滤模式枚举
 */
export enum FilterMode {
  Whitelist = 'whitelist',
  Blacklist = 'blacklist'
}

// ============================================================================
// 文件过滤器配置
// ============================================================================

/**
 * 文件过滤器配置类型
 * 与 common.ts 中的 FileFilterConfig 保持一致
 */
export type FileFilterConfig = {
  /** 是否启用第2层智能过滤 */
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
};

/**
 * 文件过滤器配置 Schema
 * 使用 passthrough 保持向后兼容，允许额外字段
 */
export const FileFilterConfigSchema: z.ZodType<FileFilterConfig> = z.object({
  enabled: z.boolean(),
  binary_detection_enabled: z.boolean(),
  mode: z.nativeEnum(FilterMode),
  filename_patterns: z.array(z.string()),
  allowed_extensions: z.array(z.string()),
  forbidden_extensions: z.array(z.string()),
}).passthrough();

// ============================================================================
// 性能指标 - 使用 passthrough 允许向后兼容
// ============================================================================

/**
 * 性能指标类型
 * 与 common.ts 中的 PerformanceMetrics 保持一致
 */
export type PerformanceMetrics = {
  /** 搜索延迟指标 */
  searchLatency: {
    current: number;
    average: number;
    p95: number;
    p99: number;
  };
  /** 搜索吞吐量指标 */
  searchThroughput: {
    current: number;
    average: number;
    peak: number;
  };
  /** 缓存性能指标 */
  cacheMetrics: {
    hitRate: number;
    missCount: number;
    hitCount: number;
    size: number;
    capacity: number;
    evictions: number;
  };
  /** 内存使用指标 */
  memoryMetrics: {
    used: number;
    total: number;
    heapUsed: number;
    external: number;
  };
  /** 任务执行指标 */
  taskMetrics: {
    total: number;
    running: number;
    completed: number;
    failed: number;
    averageDuration: number;
  };
  /** 索引指标 */
  indexMetrics: {
    totalFiles: number;
    indexedFiles: number;
    totalSize: number;
    indexSize: number;
  };
};

/**
 * 性能指标 Schema
 * 使用 passthrough 保持向后兼容，允许额外字段
 */
export const PerformanceMetricsSchema: z.ZodType<PerformanceMetrics> = z.object({
  searchLatency: z.object({
    current: z.number(),
    average: z.number(),
    p95: z.number(),
    p99: z.number(),
  }),
  searchThroughput: z.object({
    current: z.number(),
    average: z.number(),
    peak: z.number(),
  }),
  cacheMetrics: z.object({
    hitRate: z.number(),
    missCount: z.number(),
    hitCount: z.number(),
    size: z.number(),
    capacity: z.number(),
    evictions: z.number(),
  }),
  memoryMetrics: z.object({
    used: z.number(),
    total: z.number(),
    heapUsed: z.number(),
    external: z.number(),
  }),
  taskMetrics: z.object({
    total: z.number(),
    running: z.number(),
    completed: z.number(),
    failed: z.number(),
    averageDuration: z.number(),
  }),
  indexMetrics: z.object({
    totalFiles: z.number(),
    indexedFiles: z.number(),
    totalSize: z.number(),
    indexSize: z.number(),
  }),
}).passthrough();

// ============================================================================
// 文件读取响应
// ============================================================================

/**
 * 文件读取响应 Schema
 */
export const ReadFileResponseSchema = z.object({
  content: z.string(),
  truncated: z.boolean().optional(),
  totalLines: z.number().optional(),
});

/**
 * 文件读取响应类型
 */
export type ReadFileResponse = z.infer<typeof ReadFileResponseSchema>;
