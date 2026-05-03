import { SearchQuery } from '../types/search';
import { z } from 'zod';

const STORAGE_KEY = 'log_analyzer_current_query';

const SearchTermSchema = z.object({
  id: z.string(),
  value: z.string(),
  operator: z.string(),
  source: z.string(),
  presetGroupId: z.string().nullable().optional(),
  isRegex: z.boolean().optional(),
  priority: z.number().optional(),
  enabled: z.boolean().optional(),
  caseSensitive: z.boolean().optional(),
});

const SearchQuerySchema = z.object({
  id: z.string(),
  terms: z.array(SearchTermSchema),
  globalOperator: z.string(),
  filters: z.object({
    levels: z.array(z.string()).optional(),
    timeRange: z.object({
      start: z.number().optional(),
      end: z.number().optional(),
    }).optional(),
    filePattern: z.string().optional(),
  }).nullable().optional(),
  metadata: z.object({
    createdAt: z.number().optional(),
    lastModified: z.number().optional(),
    executionCount: z.number().optional(),
    label: z.string().nullable().optional(),
  }),
});

/**
 * 保存查询到 localStorage
 */
export function saveQuery(query: SearchQuery): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(query));
  } catch (error) {
    console.error('Failed to save query:', error);
  }
}

/**
 * 从 localStorage 加载查询
 */
export function loadQuery(): SearchQuery | null {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (!stored) return null;

    const parsed = JSON.parse(stored);
    return SearchQuerySchema.parse(parsed) as SearchQuery;
  } catch (error) {
    console.error('Failed to load query:', error);
    return null;
  }
}

/**
 * 清除保存的查询
 */
export function clearQuery(): void {
  localStorage.removeItem(STORAGE_KEY);
}
