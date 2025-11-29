import { SearchQuery } from '../types/search';

const STORAGE_KEY = 'log_analyzer_current_query';

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
    
    return JSON.parse(stored) as SearchQuery;
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
