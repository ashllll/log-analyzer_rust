/**
 * 统一 API 层
 *
 * 封装所有 Tauri 命令调用，提供类型安全的接口和统一的错误处理。
 *
 * @module api
 */

import { invoke, type InvokeArgs } from '@tauri-apps/api/core';
import { z } from 'zod';
import { createApiError } from './errors';
import { logger } from '../utils/logger';
import type { FilterOptions } from '../types/common';
import type { SearchQuery } from '../types/search';
import {
  RarSupportInfoSchema,
  FileFilterConfigSchema,
  WorkspaceLoadResponseSchema,
  WorkspaceStatusResponseSchema,
  WorkspaceTimeRangeSchema,
  AppConfigSchema,
  SearchIdSchema,
  type RarSupportInfo,
  type FileFilterConfig,
  type WorkspaceStatusResponseValidated,
  type AppConfigValidated as AppConfig,
} from '../types/api-responses';

// ============================================================================
// 空值安全工具函数（原 nullSafeApi）
// ============================================================================

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
export function sanitizeArgs(args: Record<string, unknown>): Record<string, unknown> {
  const sanitized: Record<string, unknown> = {};

  for (const [key, value] of Object.entries(args)) {
    if (isEmpty(value)) {
      continue;
    }
    if (Array.isArray(value)) {
      sanitized[key] = value.map((item) => {
        if (item && typeof item === 'object' && !Array.isArray(item)) {
          return sanitizeArgs(item as Record<string, unknown>);
        }
        return item;
      });
    } else if (typeof value === 'object' && value !== null) {
      const sanitizedNested = sanitizeArgs(value as Record<string, unknown>);
      // 始终保留对象，即使所有字段都是 null/undefined
      // 避免后端将 "filter 已设置为空" 误解为 "无 filter"
      sanitized[key] = sanitizedNested;
    } else {
      sanitized[key] = value;
    }
  }

  return sanitized;
}

/**
 * API 调用参数类型
 */
export type ApiArgs = Record<string, unknown>;

/**
 * 带超时的 IPC 调用包装器
 *
 * 注意：Tauri v2 的 invoke 不支持 AbortController 取消，
 * 超时后底层请求仍会继续执行。此包装器仅用于前端超时检测，
 * 避免 UI 无限等待。如需真正取消，需在 Rust 端实现取消机制。
 */
export async function invokeWithTimeout<T>(
  command: string,
  args: ApiArgs,
  timeoutMs: number = 30000
): Promise<T> {
  const sanitizedArgs = sanitizeArgs(args);

  let isTimedOut = false;
  const timeoutId = setTimeout(() => {
    isTimedOut = true;
  }, timeoutMs);

  try {
    const result = await invoke<T>(command, sanitizedArgs);
    logger.debug('IPC 调用成功:', { command, hasResult: !!result });
    return result;
  } catch (error) {
    if (isTimedOut) {
      throw new Error(`操作超时（${timeoutMs}ms）: ${command}`);
    }
    logger.error('IPC 调用失败:', { command, error });
    throw error;
  } finally {
    clearTimeout(timeoutId);
  }
}

/**
 * 空值安全的 API 调用
 * 包装 invokeWithTimeout，提供更友好的错误处理
 */
export async function safeInvoke<T>(
  command: string,
  args: ApiArgs = {},
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

    if (fallback !== undefined) {
      return fallback;
    }

    throw err;
  }
}

/**
 * 空值安全的列表 API 调用
 * 确保返回数组类型，错误会向上传播给调用者处理
 */
export async function safeInvokeList<T>(
  command: string,
  args: ApiArgs = {}
): Promise<T[]> {
  const result = await safeInvoke<T[]>(command, args);
  return Array.isArray(result) ? result : [];
}

/**
 * 空值安全的单值 API 调用
 * 确保返回对象而不是 null，错误会向上传播给调用者处理
 */
export async function safeInvokeObject<T extends object>(
  command: string,
  args: ApiArgs = {},
  defaultValue: T
): Promise<T> {
  const result = await safeInvoke<T>(command, args);
  return result && typeof result === 'object' ? result : defaultValue;
}

// ============================================================================
// 类型定义
// ============================================================================

/**
 * 工作区加载响应
 */
export interface WorkspaceLoadResponse {
  success: boolean;
  fileCount: number;
}

export type WorkspaceStatusResponse = WorkspaceStatusResponseValidated;

/**
 * 搜索参数
 */
export interface SearchParams {
  query: string;
  structuredQuery?: SearchQuery;
  workspaceId?: string;
  maxResults?: number;
  filters?: FilterOptions;
}

// SearchFilters 统一使用 types/common.ts 中的 FilterOptions

/**
 * 导出结果条目
 */
export interface ExportResultEntry {
  id?: number;
  timestamp?: string;
  level?: string;
  content?: string;
  file?: string;
  line?: number;
  [key: string]: unknown;
}

/**
 * 导出参数
 */
export interface ExportParams {
  results: ExportResultEntry[];
  format: 'csv' | 'json';
  savePath: string;
}

/**
 * 文件监听参数
 */
export interface WatchParams {
  workspaceId: string;
  autoSearch?: boolean;
}

// ============================================================================
// 统一 API 类
// ============================================================================

/**
 * 日志分析器统一 API
 *
 * 提供所有 Tauri 命令的类型安全封装
 */
class LogAnalyzerApi {
  // ========================================================================
  // 内部辅助方法
  // ========================================================================

  /**
   * 统一包装 IPC 调用，自动处理错误转换
   */
  private async invokeWithErrorHandling<T>(
    command: string,
    args: InvokeArgs,
    parser: (raw: unknown) => T
  ): Promise<T> {
    try {
      const raw = await invoke(command, args);
      return parser(raw);
    } catch (error) {
      throw createApiError(command, error);
    }
  }

  // ========================================================================
  // 工作区操作
  // ========================================================================

  /**
   * 加载工作区
   *
   * @param workspaceId - 工作区 ID
   * @returns 工作区加载响应
   */
  async loadWorkspace(workspaceId: string): Promise<WorkspaceLoadResponse> {
    return this.invokeWithErrorHandling(
      'load_workspace',
      { workspaceId },
      (raw) => WorkspaceLoadResponseSchema.parse(raw) as WorkspaceLoadResponse
    );
  }

  /**
   * 刷新工作区
   *
   * @param workspaceId - 工作区 ID
   * @param path - 工作区原始路径
   * @returns 工作区 ID
   */
  async refreshWorkspace(workspaceId: string, path?: string): Promise<string> {
    const resolvedPath = path && path.trim().length > 0
      ? path
      : (await this.loadConfig()).workspaces.find((workspace) => workspace.id === workspaceId)?.path;

    const args = resolvedPath && resolvedPath.trim().length > 0
      ? { workspaceId, path: resolvedPath }
      : { workspaceId };

    return this.invokeWithErrorHandling(
      'refresh_workspace',
      args as InvokeArgs,
      (raw) => SearchIdSchema.parse(raw)
    );
  }

  /**
   * 删除工作区
   *
   * @param workspaceId - 工作区 ID
   */
  async deleteWorkspace(workspaceId: string): Promise<void> {
    return this.invokeWithErrorHandling(
      'delete_workspace',
      { workspaceId },
      () => undefined
    );
  }

  /**
   * 获取工作区状态
   *
   * @param workspaceId - 工作区 ID
   * @returns 工作区状态响应
   */
  async getWorkspaceStatus(workspaceId: string): Promise<WorkspaceStatusResponse> {
    return this.invokeWithErrorHandling(
      'get_workspace_status',
      { workspaceId },
      (raw) => WorkspaceStatusResponseSchema.parse(raw)
    );
  }

  /**
   * 创建工作区
   *
   * @param name - 工作区名称
   * @param path - 工作区路径
   * @returns 工作区 ID
   */
  async createWorkspace(name: string, path: string): Promise<string> {
    return this.invokeWithErrorHandling(
      'create_workspace',
      { name, path },
      (raw) => z.string().parse(raw)
    );
  }

  /**
   * 获取工作区日志时间范围
   *
   * @param workspaceId - 工作区 ID
   * @returns 时间范围信息 { minTimestamp, maxTimestamp, totalLogs }
   */
  async getWorkspaceTimeRange(workspaceId: string): Promise<{
    minTimestamp: string | null;
    maxTimestamp: string | null;
    totalLogs: number;
  }> {
    return this.invokeWithErrorHandling(
      'get_workspace_time_range',
      { workspaceId },
      (raw) => WorkspaceTimeRangeSchema.parse(raw)
    );
  }

  // ========================================================================
  // 搜索操作
  // ========================================================================

  /**
   * 搜索日志
   *
   * @param params - 搜索参数
   * @returns 搜索 ID
   */
  async searchLogs(params: SearchParams): Promise<string> {
    return this.invokeWithErrorHandling(
      'search_logs',
      params as unknown as InvokeArgs,
      (raw) => SearchIdSchema.parse(raw)
    );
  }

  /**
   * 取消搜索
   *
   * @param searchId - 搜索 ID
   */
  async cancelSearch(searchId: string): Promise<void> {
    return this.invokeWithErrorHandling(
      'cancel_search',
      { searchId },
      () => undefined
    );
  }

  // ========================================================================
  // 导入操作
  // =====================================================================

  /**
   * 导入文件夹
   *
   * @param path - 文件夹路径
   * @param workspaceId - 工作区 ID
   * @returns 任务 ID
   */
  async importFolder(path: string, workspaceId: string): Promise<string> {
    return this.invokeWithErrorHandling(
      'import_folder',
      { path, workspaceId },
      (raw) => z.string().parse(raw)
    );
  }

  /**
   * 检查 RAR 支持
   *
   * @returns RAR 支持信息
   */
  async checkRarSupport(): Promise<RarSupportInfo> {
    return this.invokeWithErrorHandling(
      'check_rar_support',
      {},
      (raw) => RarSupportInfoSchema.parse(raw)
    );
  }

  // ========================================================================
  // 文件监听
  // ========================================================================

  /**
   * 启动文件监听
   *
   * @param params - 监听参数
   */
  async startWatch(params: WatchParams): Promise<void> {
    return this.invokeWithErrorHandling(
      'start_watch',
      params as unknown as InvokeArgs,
      () => undefined
    );
  }

  /**
   * 停止文件监听
   *
   * @param workspaceId - 工作区 ID
   */
  async stopWatch(workspaceId: string): Promise<void> {
    return this.invokeWithErrorHandling(
      'stop_watch',
      { workspaceId },
      () => undefined
    );
  }

  // ========================================================================
  // 任务管理
  // ========================================================================

  /**
   * 取消任务
   *
   * @param taskId - 任务 ID
   */
  async cancelTask(taskId: string): Promise<void> {
    return this.invokeWithErrorHandling(
      'cancel_task',
      { taskId },
      () => undefined
    );
  }

  // ========================================================================
  // 配置管理
  // ========================================================================

  /**
   * 保存配置
   *
   * @param config - 应用配置
   */
  async saveConfig(config: AppConfig): Promise<void> {
    return this.invokeWithErrorHandling(
      'save_config',
      { config },
      () => undefined
    );
  }

  /**
   * 加载配置
   *
   * @returns 应用配置
   */
  async loadConfig(): Promise<AppConfig> {
    return this.invokeWithErrorHandling(
      'load_config',
      {},
      (raw) => AppConfigSchema.parse(raw) as AppConfig
    );
  }

  /**
   * 获取文件过滤器配置
   *
   * @returns 文件过滤器配置
   */
  async getFileFilterConfig(): Promise<FileFilterConfig> {
    return this.invokeWithErrorHandling(
      'get_file_filter_config',
      {},
      (raw) => FileFilterConfigSchema.parse(raw)
    );
  }

  /**
   * 保存文件过滤器配置
   *
   * @param filterConfig - 过滤器配置
   */
  async saveFileFilterConfig(filterConfig: FileFilterConfig): Promise<FileFilterConfig> {
    const validatedConfig = FileFilterConfigSchema.parse(filterConfig);
    return this.invokeWithErrorHandling(
      'save_file_filter_config',
      { filter_config: validatedConfig },
      () => validatedConfig
    );
  }

  // ========================================================================
  // 导出操作
  // ========================================================================

  /**
   * 导出结果
   *
   * @param params - 导出参数
   * @returns 导出文件路径
   */
  async exportResults(params: ExportParams): Promise<string> {
    return this.invokeWithErrorHandling(
      'export_results',
      params as unknown as InvokeArgs,
      (raw) => raw as string
    );
  }

  // ========================================================================
  // 虚拟文件树
  // ========================================================================

  /**
   * 通过哈希读取文件
   *
   * @param params - 文件参数
   * @returns 文件内容
   */
  async readFileByHash(params: {
    workspaceId: string;
    hash: string;
    maxLength?: number;
  }): Promise<string> {
    return this.invokeWithErrorHandling(
      'read_file_by_hash',
      params,
      (raw) => raw as string
    );
  }

}

// ============================================================================
// 导出单例
// ============================================================================

/**
 * 统一 API 实例
 *
 * @example
 * ```typescript
 * import { api } from '@/services/api';
 *
 * // 加载工作区
 * const workspace = await api.loadWorkspace('workspace-123');
 *
 * // 搜索日志
 * const searchId = await api.searchLogs({
 *   query: 'error timeout',
 *   workspaceId: 'workspace-123',
 *   maxResults: 1000
 * });
 * ```
 */
export const api = new LogAnalyzerApi();

// ============================================================================
// 文件内容响应类型（原 fileApi）
// ============================================================================

/**
 * 文件内容响应
 */
export interface FileContentResponse {
  content: string;
  hash: string;
  size: number;
}

/**
 * 空值安全的文件读取（增强版）
 */
export async function readFileByHash(
  workspaceId: string,
  hash: string
): Promise<FileContentResponse | null> {
  try {
    if (isEmptyString(workspaceId)) {
      logger.warn('readFileByHash: workspaceId 为空');
      return null;
    }
    if (isEmptyString(hash)) {
      logger.warn('readFileByHash: hash 为空');
      return null;
    }

    logger.debug('Reading file by hash:', { workspaceId, hash });

    const response = await safeInvoke<FileContentResponse | null>(
      'read_file_by_hash',
      { workspaceId, hash },
      {
        timeoutMs: 10000,
        fallback: null,
        onError: (err) => logger.error('读取文件失败', { error: err.message })
      }
    );

    if (response) {
      logger.debug('Successfully read file:', { hash, size: response.size });
    }

    return response;
  } catch (error) {
    logger.error('Failed to read file by hash:', error);
    throw createApiError('read_file_by_hash', error);
  }
}

/**
 * 文件 API 便捷对象
 */
export const fileApi = {
  readByHash: readFileByHash
};
