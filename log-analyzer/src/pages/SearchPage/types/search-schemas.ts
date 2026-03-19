/**
 * Zod Schema 验证 — 确保 API 响应类型安全
 * 对应 Rust 后端数据结构
 */
import { z } from 'zod';

/**
 * 匹配详情 Schema
 * 对应 Rust 后端 MatchDetail 结构
 */
const MatchDetailSchema = z.object({
  term_id: z.string(),
  term_value: z.string(),
  priority: z.number(),
  match_position: z.tuple([z.number(), z.number()]).optional(),
});

/**
 * 日志条目 Schema
 * 对应 Rust 后端 LogEntry 结构
 * 注意：后端使用 snake_case，序列化后保持 snake_case
 */
const LogEntrySchema = z.object({
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
 * 关键词统计 Schema
 * 对应 Rust 后端 KeywordStatistics 结构（使用 serde rename）
 */
const KeywordStatisticsSchema = z.object({
  keyword: z.string(),
  matchCount: z.number(),
  matchPercentage: z.number(),
});

/**
 * 搜索结果摘要 Schema
 * 对应 Rust 后端 SearchResultSummary 结构（使用 serde rename）
 */
const SearchResultSummarySchema = z.object({
  totalMatches: z.number(),
  keywordStats: z.array(KeywordStatisticsSchema),
  searchDurationMs: z.number(),
  truncated: z.boolean(),
});

/**
 * 分页搜索结果 Schema
 * 对应 Rust 后端 PagedSearchResult 结构
 * 用于验证 search_logs_paged 命令的返回值
 */
const PagedSearchResultSchema = z.object({
  results: z.array(LogEntrySchema),
  total_count: z.number(),
  page_index: z.number(),
  page_size: z.number(),
  total_pages: z.number(),
  has_more: z.boolean(),
  summary: SearchResultSummarySchema,
  query: z.string(),
  search_id: z.string(),
});

export {
  MatchDetailSchema,
  LogEntrySchema,
  KeywordStatisticsSchema,
  SearchResultSummarySchema,
  PagedSearchResultSchema,
};
