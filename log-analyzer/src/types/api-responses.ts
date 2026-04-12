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
  compiled: z.boolean(),
  available: z.boolean(),
  reason: z.string().nullable().optional(),
});

/**
 * RAR 支持信息类型
 */
export type RarSupportInfo = z.infer<typeof RarSupportInfoSchema>;

// ============================================================================
// 虚拟文件节点（与后端 VirtualTreeNode tagged enum 对齐）
// ============================================================================

/**
 * 文件节点类型
 */
export type VirtualFileNode = {
  type: 'file';
  name: string;
  path: string;
  hash: string;
  size: number;
  mimeType?: string;
};

/**
 * 归档节点类型
 */
export type VirtualArchiveNode = {
  type: 'archive';
  name: string;
  path: string;
  hash: string;
  archiveType: string;
  children: VirtualTreeNode[];
};

/**
 * 虚拟树节点联合类型
 */
export type VirtualTreeNode = VirtualFileNode | VirtualArchiveNode;

/**
 * 虚拟文件节点 Schema
 */
export const VirtualFileNodeSchema: z.ZodType<VirtualFileNode> = z.object({
  type: z.literal('file'),
  name: z.string(),
  path: z.string(),
  hash: z.string(),
  size: z.number(),
  mimeType: z.string().optional(),
});

/**
 * 虚拟归档节点 Schema（递归）
 */
export const VirtualArchiveNodeSchema: z.ZodType<VirtualArchiveNode> = z.object({
  type: z.literal('archive'),
  name: z.string(),
  path: z.string(),
  hash: z.string(),
  archiveType: z.string(),
  children: z.lazy(() => VirtualTreeNodeSchema.array()),
});

/**
 * 虚拟树节点联合 Schema
 */
export const VirtualTreeNodeSchema: z.ZodType<VirtualTreeNode> = z.union([
  VirtualFileNodeSchema,
  VirtualArchiveNodeSchema,
]);

// ============================================================================
// 工作区状态
// ============================================================================

/**
 * 工作区状态 Schema
 */
export const WorkspaceStatusSchema = z.enum(['READY', 'PROCESSING', 'OFFLINE', 'ERROR']);

/**
 * 工作区状态枚举类型
 */
export type WorkspaceStatus = z.infer<typeof WorkspaceStatusSchema>;

/**
 * 工作区状态 Schema
 */
export const WorkspaceStateSchema = z.object({
  id: z.string(),
  name: z.string(),
  path: z.string(),
  status: WorkspaceStatusSchema,
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
// 搜索相关
// ============================================================================

/**
 * 搜索 ID Schema
 * 验证搜索返回的 ID 是有效的 UUID 字符串
 */
export const SearchIdSchema = z.string().uuid();

/**
 * 搜索 ID 类型
 */
export type SearchId = z.infer<typeof SearchIdSchema>;

/**
 * 命令错误 Schema
 * 验证后端返回的结构化错误信息
 */
export const CommandErrorSchema = z.object({
  code: z.string(),
  message: z.string(),
  help: z.string().optional(),
  details: z.unknown().optional(),
});

/**
 * 命令错误类型
 */
export type CommandError = z.infer<typeof CommandErrorSchema>;

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

// ============================================================================
// LogEntry - 日志条目
// ============================================================================

/**
 * 匹配详情 Schema（与后端 services::MatchDetail 一致）
 */
export const MatchDetailSchema = z.object({
  term_id: z.string(),
  term_value: z.string(),
  priority: z.number(),
  match_position: z.tuple([z.number(), z.number()]).optional(),
});

/**
 * 匹配详情类型
 */
export type MatchDetail = z.infer<typeof MatchDetailSchema>;

/**
 * LogEntry Schema（与后端 models::LogEntry 一致）
 *
 * 注意：
 * - id: number（后端为 usize，序列化为 number）
 * - timestamp: string（后端为 Arc<str>）
 * - tags: string[]（后端为 Vec<String>）
 * - match_details: 可选数组
 * - matched_keywords: 可选数组
 */
export const LogEntrySchema = z.object({
  id: z.number(),
  timestamp: z.string(),
  level: z.string(),
  file: z.string(),
  real_path: z.string(),
  line: z.number(),
  content: z.string(),
  tags: z.array(z.string()),
  match_details: z.array(MatchDetailSchema).optional(),
  matched_keywords: z.array(z.string()).optional(),
});

/**
 * LogEntry 类型
 */
export type LogEntry = z.infer<typeof LogEntrySchema>;

// ============================================================================
// 工作区加载响应
// ============================================================================

/**
 * 工作区加载响应 Schema（对应 api.ts WorkspaceLoadResponse）
 */
export const WorkspaceLoadResponseSchema = z.object({
  success: z.boolean(),
  fileCount: z.number().int().nonnegative(),
});

/**
 * 工作区加载响应类型
 */
export type WorkspaceLoadResponseValidated = z.infer<typeof WorkspaceLoadResponseSchema>;

/**
 * 工作区状态响应 Schema
 */
export const WorkspaceStatusResponseSchema = z.object({
  id: z.string(),
  name: z.string(),
  status: WorkspaceStatusSchema,
  size: z.string(),
  files: z.number().int().nonnegative(),
});

/**
 * 工作区状态响应类型
 */
export type WorkspaceStatusResponseValidated = z.infer<typeof WorkspaceStatusResponseSchema>;

/**
 * 工作区时间范围响应 Schema
 */
export const WorkspaceTimeRangeSchema = z.object({
  minTimestamp: z.string().nullable(),
  maxTimestamp: z.string().nullable(),
  totalLogs: z.number().int().nonnegative(),
});

/**
 * 工作区时间范围响应类型
 */
export type WorkspaceTimeRangeValidated = z.infer<typeof WorkspaceTimeRangeSchema>;

// ============================================================================
// 应用配置
// ============================================================================

/**
 * 文件过滤器内嵌配置 Schema（AppConfig 内部）
 */
const AppConfigFileFilterSchema = z.object({
  enabled: z.boolean(),
  binary_detection_enabled: z.boolean(),
  mode: z.enum(['whitelist', 'blacklist']),
  filename_patterns: z.array(z.string()),
  allowed_extensions: z.array(z.string()),
  forbidden_extensions: z.array(z.string()),
});

/**
 * 关键词模式 Schema
 */
const KeywordPatternSchema = z.object({
  regex: z.string(),
  comment: z.string(),
});

/**
 * 颜色键类型
 */
const ColorKeySchema = z.enum(['blue', 'green', 'red', 'orange', 'purple']);

/**
 * 关键词组 Schema
 */
const KeywordGroupSchema = z.object({
  id: z.string(),
  name: z.string(),
  color: ColorKeySchema,
  patterns: z.array(KeywordPatternSchema),
  enabled: z.boolean(),
});

/**
 * 工作区 Schema
 */
const WorkspaceSchema = z.object({
  id: z.string(),
  name: z.string(),
  path: z.string(),
  status: z.enum(['READY', 'OFFLINE', 'PROCESSING', 'ERROR']),
  size: z.string(),
  files: z.number(),
  watching: z.boolean().optional(),
});

/**
 * 应用配置 Schema（对应 api.ts AppConfig）
 */
export const AppConfigSchema = z.object({
  keyword_groups: z.array(KeywordGroupSchema),
  workspaces: z.array(WorkspaceSchema),
  file_filter: AppConfigFileFilterSchema,
});

/**
 * 应用配置类型（Zod 验证后）
 */
export type AppConfigValidated = z.infer<typeof AppConfigSchema>;
