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
