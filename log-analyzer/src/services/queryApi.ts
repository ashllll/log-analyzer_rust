import { invoke } from '@tauri-apps/api/core';
import { SearchQuery } from '../types/search';

/**
 * 执行结构化查询
 */
export async function executeStructuredQuery(
  query: SearchQuery,
  logs: string[]
): Promise<string[]> {
  try {
    const result = await invoke<string[]>('execute_structured_query', {
      query,
      logs
    });
    return result;
  } catch (error: any) {
    console.error('Failed to execute query:', error);
    throw new Error(`查询执行失败: ${error}`);
  }
}

/**
 * 验证查询
 */
export async function validateQuery(query: SearchQuery): Promise<boolean> {
  try {
    const result = await invoke<boolean>('validate_query', { query });
    return result;
  } catch (error: any) {
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
