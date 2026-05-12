// 从 stores/types.ts 导入类型定义（单向依赖，打破循环）
import type { Toast, Workspace, Task, KeywordGroup, ToastType, KeywordPattern, ColorKey } from '../stores/types';

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

// AppConfig 类型统一以 Zod Schema 为唯一数据源
// 使用 api-responses.ts 中导出的 AppConfigValidated，避免手写 interface 与 Schema 不同步
export type { AppConfigValidated as AppConfig } from './api-responses';

// ========== 文件过滤配置类型结束 ==========


