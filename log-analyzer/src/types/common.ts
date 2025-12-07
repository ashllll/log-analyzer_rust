// 从 App.tsx 迁移的通用类型定义
import { Toast, Workspace, Task, KeywordGroup } from '../contexts/AppContext';

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

// 性能指标类型
export interface PerformanceStats {
  memoryUsed: number;
  pathMapSize: number;
  cacheSize: number;
  lastSearchDuration: number;
  cacheHitRate: number;
  indexedFilesCount: number;
  indexFileSizeMb: number;
}

// 重新导出 Context 类型供外部使用
export type { Toast, Workspace, Task, KeywordGroup };
export type { ToastType, KeywordPattern, ColorKey } from '../contexts/AppContext';
