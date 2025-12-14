import { invoke } from '@tauri-apps/api/core';
import { SearchQuery } from '../types/search';

/**
 * 带超时的 IPC 调用包装器
 */
async function invokeWithTimeout<T>(
  command: string,
  args: Record<string, any>,
  timeoutMs: number = 30000
): Promise<T> {
  return new Promise<T>((resolve, reject) => {
    const timeoutId = setTimeout(() => {
      reject(new Error(`操作超时（${timeoutMs}ms）: ${command}`));
    }, timeoutMs);

    invoke<T>(command, args)
      .then((result) => {
        clearTimeout(timeoutId);
        resolve(result);
      })
      .catch((error) => {
        clearTimeout(timeoutId);
        reject(error);
      });
  });
}

/**
 * 执行结构化查询（带超时控制）
 */
export async function executeStructuredQuery(
  query: SearchQuery,
  logs: string[]
): Promise<string[]> {
  try {
    const result = await invokeWithTimeout<string[]>('execute_structured_query', {
      query,
      logs
    }, 30000); // 30秒超时
    return result;
  } catch (error: any) {
    console.error('Failed to execute query:', error);
    throw new Error(`查询执行失败: ${error}`);
  }
}

/**
 * 验证查询（带超时控制）
 */
export async function validateQuery(query: SearchQuery): Promise<boolean> {
  try {
    const result = await invokeWithTimeout<boolean>('validate_query', { query }, 5000); // 5秒超时
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
