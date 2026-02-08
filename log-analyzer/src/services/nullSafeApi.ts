import { invoke } from '@tauri-apps/api/core';
import { logger } from '../utils/logger';

/**
 * 空值安全检查工具
 */
export function isEmpty<T>(value: T | null | undefined): value is null | undefined {
  return value === null || value === undefined;
}

export function isEmptyString(value: string | null | undefined): boolean {
  return value === null || value === undefined || value === '';
}

export function isEmptyArray<T>(value: T[] | null | undefined): boolean {
  return value === null || value === undefined || value.length === 0;
}

/**
 * API 调用参数空值处理
 * 移除 null/undefined 值，防止 Rust 后端解析错误
 */
export function sanitizeArgs(args: Record<string, any>): Record<string, any> {
  const sanitized: Record<string, any> = {};

  for (const [key, value] of Object.entries(args)) {
    if (isEmpty(value)) {
      // 空值不传递，让 Rust 使用默认值
      continue;
    }
    if (Array.isArray(value) && value.length === 0) {
      // 空数组根据情况决定是否传递
      // 对于 Rust Vec 参数，空数组可能需要传递
      sanitized[key] = value;
    } else if (typeof value === 'object' && value !== null) {
      // 递归处理嵌套对象
      const sanitizedNested = sanitizeArgs(value);
      if (Object.keys(sanitizedNested).length > 0) {
        sanitized[key] = sanitizedNested;
      }
    } else {
      sanitized[key] = value;
    }
  }

  return sanitized;
}

/**
 * 带超时的 IPC 调用包装器（增强版）
 */
export async function invokeWithTimeout<T>(
  command: string,
  args: Record<string, any>,
  timeoutMs: number = 30000
): Promise<T> {
  // 参数空值处理
  const sanitizedArgs = sanitizeArgs(args);

  return new Promise<T>((resolve, reject) => {
    const timeoutId = setTimeout(() => {
      reject(new Error(`操作超时（${timeoutMs}ms）: ${command}`));
    }, timeoutMs);

    invoke<T>(command, sanitizedArgs)
      .then((result) => {
        clearTimeout(timeoutId);
        logger.debug('IPC 调用成功:', { command, hasResult: !!result });
        resolve(result);
      })
      .catch((error) => {
        clearTimeout(timeoutId);
        logger.error('IPC 调用失败:', { command, error });
        reject(error);
      });
  });
}

/**
 * 空值安全的 API 调用
 * 包装 invokeWithTimeout，提供更友好的错误处理
 */
export async function safeInvoke<T>(
  command: string,
  args: Record<string, any> = {},
  options: { timeoutMs?: number; fallback?: T; onError?: (error: Error) => void } = {}
): Promise<T> {
  const { timeoutMs = 30000, fallback, onError } = options;

  try {
    const result = await invokeWithTimeout<T>(command, args, timeoutMs);
    return result;
  } catch (error) {
    const err = error instanceof Error ? error : new Error(String(error));

    if (onError) {
      onError(err);
    } else {
      logger.warn(`API 调用失败，使用默认值: ${command}`, { error: err.message });
    }

    // 返回默认值（如果有）
    if (fallback !== undefined) {
      return fallback;
    }

    // 重新抛出错误，让调用方处理
    throw err;
  }
}

/**
 * 空值安全的列表 API 调用
 * 确保返回空数组而不是 null
 */
export async function safeInvokeList<T>(
  command: string,
  args: Record<string, any> = {}
): Promise<T[]> {
  try {
    const result = await safeInvoke<T[]>(command, args, { fallback: [] });
    // 确保返回数组（防御性）
    return Array.isArray(result) ? result : [];
  } catch {
    // 返回空数组作为后备
    return [];
  }
}

/**
 * 空值安全的单值 API 调用
 * 确保返回对象而不是 null
 */
export async function safeInvokeObject<T extends object>(
  command: string,
  args: Record<string, any> = {},
  defaultValue: T
): Promise<T> {
  try {
    const result = await safeInvoke<T>(command, args, { fallback: defaultValue });
    // 确保返回对象（防御性）
    return result && typeof result === 'object' ? result : defaultValue;
  } catch {
    return defaultValue;
  }
}
