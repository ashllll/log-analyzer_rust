import { SearchQuery } from '../types/search';
import { safeInvoke, isEmptyArray } from './nullSafeApi';
import { logger } from '../utils/logger';

/**
 * 执行结构化查询（带超时控制 + 空值保护）
 */
export async function executeStructuredQuery(
  query: SearchQuery,
  logs: string[]
): Promise<string[]> {
  try {
    // 空值检查
    if (isEmptyArray(logs)) {
      logger.warn('executeStructuredQuery: logs 数组为空');
      return [];
    }

    const result = await safeInvoke<string[]>('execute_structured_query', {
      query,
      logs
    }, { timeoutMs: 30000 });

    return Array.isArray(result) ? result : [];
  } catch (error: unknown) {
    console.error('Failed to execute query:', error);
    const errorMessage = error instanceof Error ? error.message : String(error);
    throw new Error(`查询执行失败: ${errorMessage}`);
  }
}

/**
 * 验证查询（带超时控制 + 空值保护）
 */
export async function validateQuery(query: SearchQuery): Promise<boolean> {
  try {
    if (!query || typeof query !== 'object') {
      logger.warn('validateQuery: 无效的 query 参数');
      return false;
    }

    return await safeInvoke<boolean>('validate_query', { query }, {
      timeoutMs: 5000,
      fallback: false
    });
  } catch (error: unknown) {
    console.error('Failed to validate query:', error);
    return false;
  }
}

/**
 * 查询 API
 */
export const queryApi = {
  execute: executeStructuredQuery,
  validate: validateQuery
};
