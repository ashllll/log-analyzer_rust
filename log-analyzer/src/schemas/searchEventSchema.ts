/**
 * 搜索事件 Zod Schema
 *
 * 将 Tauri 原生搜索事件纳入 EventBus 统一事件体系，
 * 获得运行时 Schema 验证与类型安全。
 */

import { z } from 'zod';

/**
 * search-progress 事件
 *
 * 后端每处理一批日志后推送的进度更新。
 */
export const SearchProgressEventSchema = z.object({
  search_id: z.string(),
  count: z.number(),
  disk_write_offset: z.number().optional(),
});

/**
 * search-complete 事件
 *
 * 搜索全部完成后推送的汇总事件。
 */
export const SearchCompleteEventSchema = z.object({
  search_id: z.string(),
  total_count: z.number(),
});

/**
 * search-error 事件
 *
 * 搜索过程中发生错误时推送。
 */
export const SearchErrorEventSchema = z.object({
  search_id: z.string(),
  error: z.string(),
});

export type SearchProgressEvent = z.infer<typeof SearchProgressEventSchema>;
export type SearchCompleteEvent = z.infer<typeof SearchCompleteEventSchema>;
export type SearchErrorEvent = z.infer<typeof SearchErrorEventSchema>;
