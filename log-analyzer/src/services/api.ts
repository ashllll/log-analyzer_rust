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
import {
  RarSupportInfoSchema,
  FileFilterConfigSchema,
  WorkspaceLoadResponseSchema,
  WorkspaceStatusResponseSchema,
  WorkspaceTimeRangeSchema,
  AppConfigSchema,
  SearchIdSchema,
  SearchParamsSchema,
  ExportParamsSchema,
  WatchParamsSchema,
  SearchConfigSchema,
  TaskManagerConfigSchema,
  type RarSupportInfo,
  type FileFilterConfig,
  type WorkspaceLoadResponseValidated,
  type WorkspaceStatusResponseValidated,
  type SearchParamsValidated,
  type ExportParamsValidated,
  type WatchParamsValidated,
  type SearchConfigValidated,
  type TaskManagerConfigValidated,
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

/**
 * API 调用参数空值处理
 * 移除 null/undefined 值，防止 Rust 后端解析错误
 */
export function sanitizeArgs(args: Record<string, unknown>): Record<string, unknown> {
  const sanitized: Record<string, unknown> = {};

  for (const [key, value] of Object.entries(args)) {
    const sanitizedValue = sanitizeValue(value);
    if (sanitizedValue === undefined) {
      continue;
    }
    sanitized[key] = sanitizedValue;
  }

  return sanitized;
}

function sanitizeValue(value: unknown): unknown | undefined {
  if (isEmpty(value)) {
    return undefined;
  }

  if (Array.isArray(value)) {
    return value
      .map((item) => {
        if (item && typeof item === 'object') {
          return sanitizeValue(item);
        }
        return item;
      })
      .filter((item) => item !== undefined);
  }

  if (typeof value === 'object') {
    const sanitizedNested = sanitizeArgs(value as Record<string, unknown>);
    return Object.keys(sanitizedNested).length > 0 ? sanitizedNested : undefined;
  }

  return value;
}

/**
 * API 调用参数类型
 */
export type ApiArgs = Record<string, unknown>;

/**
 * 带超时的 IPC 调用包装器
 *
 * 注意：Tauri v2 的 invoke 不支持 AbortController 取消。
 * 超时后前端会忽略后续结果，避免迟到响应继续触发日志或状态更新；
 * 需要真正取消的长任务必须使用后端 cancellation token 命令。
 */
export async function invokeWithTimeout<T>(
  command: string,
  args: ApiArgs,
  timeoutMs: number = 30000
): Promise<T> {
  const sanitizedArgs = sanitizeArgs(args);

  return new Promise<T>((resolve, reject) => {
    let settled = false;
    const timeoutId = setTimeout(() => {
      settled = true;
      reject(new Error(`操作超时（${timeoutMs}ms）: ${command}`));
    }, timeoutMs);

    invoke<T>(command, sanitizedArgs)
      .then((result) => {
        if (settled) return;
        settled = true;
        clearTimeout(timeoutId);
        logger.debug('IPC 调用成功:', { command, hasResult: !!result });
        resolve(result);
      })
      .catch((error) => {
        if (settled) return;
        settled = true;
        clearTimeout(timeoutId);
        logger.error('IPC 调用失败:', { command, error });
        reject(error);
      });
  });
}

// ============================================================================
// 类型定义
// ============================================================================

export type SearchParams = SearchParamsValidated;
export type SearchConfig = SearchConfigValidated;
export type TaskManagerConfig = TaskManagerConfigValidated;

// SearchFilters 统一使用 types/common.ts 中的 FilterOptions

export type ExportParams = ExportParamsValidated;

export type WatchParams = WatchParamsValidated;

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
  async loadWorkspace(workspaceId: string): Promise<WorkspaceLoadResponseValidated> {
    return this.invokeWithErrorHandling(
      'load_workspace',
      { workspaceId },
      (raw) => WorkspaceLoadResponseSchema.parse(raw)
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
      : undefined;

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
  async getWorkspaceStatus(workspaceId: string): Promise<WorkspaceStatusResponseValidated> {
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
    const validatedParams = SearchParamsSchema.parse(params);
    return this.invokeWithErrorHandling(
      'search_logs',
      validatedParams as unknown as InvokeArgs,
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
    const validatedParams = WatchParamsSchema.parse(params);
    return this.invokeWithErrorHandling(
      'start_watch',
      validatedParams as unknown as InvokeArgs,
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
      (raw) => AppConfigSchema.parse(raw)
    );
  }

  async getSearchConfig(): Promise<SearchConfig> {
    return this.invokeWithErrorHandling(
      'get_search_config',
      {},
      (raw) => SearchConfigSchema.parse(raw)
    );
  }

  async saveSearchConfig(searchConfig: SearchConfig): Promise<void> {
    const validatedConfig = SearchConfigSchema.parse(searchConfig);
    return this.invokeWithErrorHandling(
      'save_search_config',
      { searchConfig: validatedConfig },
      () => undefined
    );
  }

  async getTaskManagerConfig(): Promise<TaskManagerConfig> {
    return this.invokeWithErrorHandling(
      'get_task_manager_config',
      {},
      (raw) => TaskManagerConfigSchema.parse(raw)
    );
  }

  async saveTaskManagerConfig(taskManagerConfig: TaskManagerConfig): Promise<void> {
    const validatedConfig = TaskManagerConfigSchema.parse(taskManagerConfig);
    return this.invokeWithErrorHandling(
      'save_task_manager_config',
      { taskManagerConfig: validatedConfig },
      () => undefined
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
    const validatedParams = ExportParamsSchema.parse(params);
    return this.invokeWithErrorHandling(
      'export_results',
      validatedParams as unknown as InvokeArgs,
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
