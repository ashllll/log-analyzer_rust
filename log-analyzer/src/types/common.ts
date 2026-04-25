// 从 stores/types.ts 导入类型定义（单向依赖，打破循环）
import type { Toast, Workspace, Task, KeywordGroup, ToastType, KeywordPattern, ColorKey } from '../stores/types';
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
export type { Toast, Workspace, Task, KeywordGroup, ToastType, KeywordPattern, ColorKey };

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


