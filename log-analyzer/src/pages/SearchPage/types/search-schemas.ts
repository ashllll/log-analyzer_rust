/**
 * Zod Schema 验证 — 确保 API 响应类型安全
 * 对应 Rust 后端数据结构
 */
import { z } from "zod";

/**
 * 匹配详情 Schema
 * 对应 Rust 后端 MatchDetail 结构
 */
const MatchDetailSchema = z.object({
  term_id: z.string(),
  term_value: z.string(),
  priority: z.number(),
  match_position: z
    .tuple([z.number().int().nonnegative(), z.number().int().nonnegative()])
    .optional(),
});

/**
 * 日志条目 Schema
 * 对应 Rust 后端 LogEntry 结构
 * 注意：后端使用 snake_case，序列化后保持 snake_case
 */
const LogEntrySchema = z.object({
  id: z.number().int().nonnegative(),
  timestamp: z.string(),
  level: z.string(),
  file: z.string(),
  real_path: z.string(),
  line: z.number().int().nonnegative(),
  content: z.string(),
  tags: z.array(z.string()),
  match_details: z.array(MatchDetailSchema).optional(),
  matched_keywords: z.array(z.string()).optional(),
});

/**
 * 关键词统计 Schema
 * 对应 Rust 后端 KeywordStatistics 结构（使用 serde rename）
 */
const KeywordStatisticsSchema = z.object({
  keyword: z.string(),
  matchCount: z.number().int().nonnegative(),
  matchPercentage: z.number().min(0).max(100),
});

/**
 * 搜索结果摘要 Schema
 * 对应 Rust 后端 SearchResultSummary 结构（使用 serde rename）
 */
const SearchResultSummarySchema = z.object({
  totalMatches: z.number().int().nonnegative(),
  keywordStats: z.array(KeywordStatisticsSchema),
  searchDurationMs: z.number().int().nonnegative(),
  truncated: z.boolean(),
});

/**
 * 分页搜索结果 Schema
 * 对应历史 PagedSearchResult 结构。
 * 当前后端主链路已收敛到 `search_logs + fetch_search_page`，此 Schema 仅保留兼容测试。
 */
const PagedSearchResultSchema = z.object({
  results: z.array(LogEntrySchema),
  total_count: z.number().int().nonnegative(),
  page_index: z.number().int().nonnegative(),
  page_size: z.number().int().positive(),
  total_pages: z.number().int().nonnegative(),
  has_more: z.boolean(),
  summary: SearchResultSummarySchema,
  query: z.string(),
  search_id: z.string().min(1),
});

export {
  MatchDetailSchema,
  LogEntrySchema,
  KeywordStatisticsSchema,
  SearchResultSummarySchema,
  PagedSearchResultSchema,
};
