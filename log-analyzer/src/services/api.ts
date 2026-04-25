/**
 * 统一 API 层
 *
 * 封装所有 Tauri 命令调用，提供类型安全的接口和统一的错误处理。
 * 同时整合空值安全调用（原 nullSafeApi）和查询 API（原 queryApi）。
 *
 * @module api
 */

import { invoke, type InvokeArgs } from '@tauri-apps/api/core';
import { z } from 'zod';
import { createApiError } from './errors';
import { logger } from '../utils/logger';
import type { KeywordGroup, Workspace } from '../types/common';
import type { SearchQuery } from '../types/search';
import {
  RarSupportInfoSchema,
  FileFilterConfigSchema,
  VirtualTreeNodeSchema,  WorkspaceStateSchema,
  EventRecordSchema,
  WorkspaceLoadResponseSchema,
  WorkspaceStatusResponseSchema,
  WorkspaceTimeRangeSchema,
  AppConfigSchema,
  SearchIdSchema,
  type RarSupportInfo,
  type FileFilterConfig,
  type VirtualTreeNode,
  type WorkspaceState,
  type WorkspaceStatusResponseValidated,
  type EventRecord,
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
 * API 调用参数类型
 */
export type ApiArgs = Record<string, unknown>;

/**
 * 带超时的 IPC 调用包装器
 */
export async function invokeWithTimeout<T>(
  command: string,
  args: ApiArgs,
  timeoutMs: number = 30000
): Promise<T> {
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
 * 确保返回空数组而不是 null
 */
export async function safeInvokeList<T>(
  command: string,
  args: ApiArgs = {}
): Promise<T[]> {
  try {
    const result = await safeInvoke<T[]>(command, args, { fallback: [] });
    return Array.isArray(result) ? result : [];
  } catch {
    return [];
  }
}

/**
 * 空值安全的单值 API 调用
 * 确保返回对象而不是 null
 */
export async function safeInvokeObject<T extends object>(
  command: string,
  args: ApiArgs = {},
  defaultValue: T
): Promise<T> {
  try {
    const result = await safeInvoke<T>(command, args, { fallback: defaultValue });
    return result && typeof result === 'object' ? result : defaultValue;
  } catch {
    return defaultValue;
  }
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
  filters?: SearchFilters;
}

/**
 * 异步搜索参数
 */
export interface AsyncSearchParams {
  query: string;
  structuredQuery?: SearchQuery;
  workspaceId?: string;
  maxResults?: number;
  timeoutSeconds?: number;
}

/**
 * 搜索过滤器
 */
export interface SearchFilters {
  levels?: string[];
  timeRange?: { start?: string; end?: string };
  filePattern?: string;
}

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

/**
 * 应用配置
 */
export interface AppConfig {
  keyword_groups: KeywordGroup[];
  workspaces: Workspace[];
  file_filter: {
    enabled: boolean;
    binary_detection_enabled: boolean;
    mode: 'whitelist' | 'blacklist';
    filename_patterns: string[];
    allowed_extensions: string[];
    forbidden_extensions: string[];
  };
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
  // 工作区操作
  // ========================================================================

  /**
   * 加载工作区
   *
   * @param workspaceId - 工作区 ID
   * @returns 工作区加载响应
   */
  async loadWorkspace(workspaceId: string): Promise<WorkspaceLoadResponse> {
    try {
      const raw = await invoke('load_workspace', { workspaceId });
      return WorkspaceLoadResponseSchema.parse(raw) as WorkspaceLoadResponse;
    } catch (error) {
      throw createApiError('load_workspace', error);
    }
  }

  /**
   * 刷新工作区
   *
   * @param workspaceId - 工作区 ID
   * @param path - 工作区原始路径
   * @returns 工作区 ID
   */
  async refreshWorkspace(workspaceId: string, path?: string): Promise<string> {
    try {
      const resolvedPath = path && path.trim().length > 0
        ? path
        : (await this.loadConfig()).workspaces.find((workspace) => workspace.id === workspaceId)?.path;

      const args = resolvedPath && resolvedPath.trim().length > 0
        ? { workspaceId, path: resolvedPath }
        : { workspaceId };

      const result = await invoke('refresh_workspace', args as InvokeArgs);
      return SearchIdSchema.parse(result);
    } catch (error) {
      throw createApiError('refresh_workspace', error);
    }
  }

  /**
   * 删除工作区
   *
   * @param workspaceId - 工作区 ID
   */
  async deleteWorkspace(workspaceId: string): Promise<void> {
    try {
      await invoke('delete_workspace', { workspaceId });
    } catch (error) {
      throw createApiError('delete_workspace', error);
    }
  }

  /**
   * 获取工作区状态
   *
   * @param workspaceId - 工作区 ID
   * @returns 工作区状态响应
   */
  async getWorkspaceStatus(workspaceId: string): Promise<WorkspaceStatusResponse> {
    try {
      const result = await invoke('get_workspace_status', { workspaceId });
      return WorkspaceStatusResponseSchema.parse(result);
    } catch (error) {
      throw createApiError('get_workspace_status', error);
    }
  }

  /**
   * 创建工作区
   *
   * @param name - 工作区名称
   * @param path - 工作区路径
   * @returns 工作区 ID
   */
  async createWorkspace(name: string, path: string): Promise<string> {
    try {
      return await invoke('create_workspace', { name, path });
    } catch (error) {
      throw createApiError('create_workspace', error);
    }
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
    try {
      const result = await invoke('get_workspace_time_range', { workspaceId });
      return WorkspaceTimeRangeSchema.parse(result);
    } catch (error) {
      throw createApiError('get_workspace_time_range', error);
    }
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
    try {
      const result = await invoke('search_logs', params as unknown as InvokeArgs);
      // 使用 Zod 验证返回的搜索 ID
      return SearchIdSchema.parse(result);
    } catch (error) {
      throw createApiError('search_logs', error);
    }
  }

  /**
   * 取消搜索
   *
   * @param searchId - 搜索 ID
   */
  async cancelSearch(searchId: string): Promise<void> {
    try {
      await invoke('cancel_search', { searchId });
    } catch (error) {
      throw createApiError('cancel_search', error);
    }
  }

  /**
   * 异步搜索日志
   *
   * @param params - 搜索参数
   * @returns 搜索 ID
   */
  async asyncSearchLogs(params: AsyncSearchParams): Promise<string> {
    try {
      const result = await invoke('async_search_logs', params as unknown as InvokeArgs);
      // 使用 Zod 验证返回的搜索 ID
      return SearchIdSchema.parse(result);
    } catch (error) {
      throw createApiError('async_search_logs', error);
    }
  }

  /**
   * 取消异步搜索
   *
   * @param searchId - 搜索 ID
   */
  async cancelAsyncSearch(searchId: string): Promise<void> {
    try {
      await invoke('cancel_async_search', { searchId });
    } catch (error) {
      throw createApiError('cancel_async_search', error);
    }
  }

  // ========================================================================
  // 导入操作
  // ========================================================================

  /**
   * 导入文件夹
   *
   * @param path - 文件夹路径
   * @param workspaceId - 工作区 ID
   * @returns 任务 ID
   */
  async importFolder(path: string, workspaceId: string): Promise<string> {
    try {
      return await invoke('import_folder', { path, workspaceId });
    } catch (error) {
      throw createApiError('import_folder', error);
    }
  }

  /**
   * 检查 RAR 支持
   *
   * @returns RAR 支持信息
   */
  async checkRarSupport(): Promise<RarSupportInfo> {
    try {
      const result = await invoke('check_rar_support');
      return RarSupportInfoSchema.parse(result);
    } catch (error) {
      throw createApiError('check_rar_support', error);
    }
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
    try {
      await invoke('start_watch', params as unknown as InvokeArgs);
    } catch (error) {
      throw createApiError('start_watch', error);
    }
  }

  /**
   * 停止文件监听
   *
   * @param workspaceId - 工作区 ID
   */
  async stopWatch(workspaceId: string): Promise<void> {
    try {
      await invoke('stop_watch', { workspaceId });
    } catch (error) {
      throw createApiError('stop_watch', error);
    }
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
    try {
      await invoke('cancel_task', { taskId });
    } catch (error) {
      throw createApiError('cancel_task', error);
    }
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
    try {
      await invoke('save_config', { config });
    } catch (error) {
      throw createApiError('save_config', error);
    }
  }

  /**
   * 加载配置
   *
   * @returns 应用配置
   */
  async loadConfig(): Promise<AppConfig> {
    try {
      const raw = await invoke('load_config');
      return AppConfigSchema.parse(raw) as AppConfig;
    } catch (error) {
      throw createApiError('load_config', error);
    }
  }

  /**
   * 获取文件过滤器配置
   *
   * @returns 文件过滤器配置
   */
  async getFileFilterConfig(): Promise<FileFilterConfig> {
    try {
      const result = await invoke('get_file_filter_config');
      return FileFilterConfigSchema.parse(result);
    } catch (error) {
      throw createApiError('get_file_filter_config', error);
    }
  }

  /**
   * 保存文件过滤器配置
   *
   * @param filterConfig - 过滤器配置
   */
  async saveFileFilterConfig(filterConfig: FileFilterConfig): Promise<FileFilterConfig> {
    try {
      // 验证输入参数
      const validatedConfig = FileFilterConfigSchema.parse(filterConfig);
      await invoke('save_file_filter_config', { filter_config: validatedConfig });
      return validatedConfig;
    } catch (error) {
      throw createApiError('save_file_filter_config', error);
    }
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
    try {
      return await invoke('export_results', params as unknown as InvokeArgs);
    } catch (error) {
      throw createApiError('export_results', error);
    }
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
    try {
      return await invoke('read_file_by_hash', params);
    } catch (error) {
      throw createApiError('read_file_by_hash', error);
    }
  }

  /**
   * 获取虚拟文件树
   *
   * @param workspaceId - 工作区 ID
   * @returns 文件树节点数组
   */
  async getVirtualFileTree(workspaceId: string): Promise<VirtualTreeNode[]> {
    try {
      const result = await invoke('get_virtual_file_tree', { workspaceId });
      return z.array(VirtualTreeNodeSchema).parse(result);
    } catch (error) {
      throw createApiError('get_virtual_file_tree', error);
    }
  }

  // ========================================================================
  // 状态同步
  // ========================================================================

  /**
   * 初始化状态同步
   */
  async initStateSync(): Promise<void> {
    try {
      await invoke('init_state_sync');
    } catch (error) {
      throw createApiError('init_state_sync', error);
    }
  }

  /**
   * 获取工作区状态
   *
   * @param workspaceId - 工作区 ID
   * @returns 工作区状态
   */
  async getWorkspaceState(workspaceId: string): Promise<WorkspaceState> {
    try {
      const result = await invoke('get_workspace_state', { workspaceId });
      return WorkspaceStateSchema.parse(result);
    } catch (error) {
      throw createApiError('get_workspace_state', error);
    }
  }

  /**
   * 获取事件历史
   *
   * @param params - 查询参数
   * @returns 事件数组
   */
  async getEventHistory(params: {
    workspaceId: string;
    limit?: number;
  }): Promise<EventRecord[]> {
    try {
      const result = await invoke('get_event_history', params);
      return z.array(EventRecordSchema).parse(result);
    } catch (error) {
      throw createApiError('get_event_history', error);
    }
  }

  // ========================================================================
  // 缓存管理
  // ========================================================================

  /**
   * 清理工作区缓存
   *
   * @param workspaceId - 工作区 ID
   * @returns 清理的缓存条目数量
   */
  async invalidateWorkspaceCache(workspaceId: string): Promise<number> {
    try {
      return await invoke('invalidate_workspace_cache', { workspaceId });
    } catch (error) {
      throw createApiError('invalidate_workspace_cache', error);
    }
  }

  // ========================================================================
  // 结构化查询（原 queryApi）
  // ========================================================================

  /**
   * 执行结构化查询（带超时控制 + 空值保护）
   *
   * @param query - 搜索查询结构
   * @param logs - 待查询的日志行
   * @returns 匹配的日志行
   */
  async executeStructuredQuery(query: SearchQuery, logs: string[]): Promise<string[]> {
    try {
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
   *
   * @param query - 搜索查询结构
   * @returns 查询是否有效
   */
  async validateQuery(query: SearchQuery): Promise<boolean> {
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
// 查询 API 便捷对象（向后兼容）
// ============================================================================

/**
 * 查询 API 便捷对象
 *
 * @example
 * import { queryApi } from '@/services/api';
 * const results = await queryApi.execute(query, logs);
 * const valid = await queryApi.validate(query);
 */
export const queryApi = {
  execute: (query: SearchQuery, logs: string[]) => api.executeStructuredQuery(query, logs),
  validate: (query: SearchQuery) => api.validateQuery(query),
};

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
    throw new Error(`Failed to read file: ${error}`);
  }
}

/**
 * 文件 API 便捷对象
 */
export const fileApi = {
  readByHash: readFileByHash
};
