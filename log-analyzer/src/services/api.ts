/**
 * 统一 API 层
 *
 * 封装所有 Tauri 命令调用，提供类型安全的接口和统一的错误处理。
 *
 * @module api
 */

import { invoke } from '@tauri-apps/api/core';
import { createApiError } from './errors';

// ============================================================================
// 类型定义
// ============================================================================

/**
 * 工作区加载响应
 */
export interface WorkspaceLoadResponse {
  id: string;
  name: string;
  path: string;
  status: 'READY' | 'PROCESSING' | 'OFFLINE';
  fileCount?: number;
  totalSize?: number;
}

/**
 * 工作区状态响应
 */
export interface WorkspaceStatusResponse {
  id: string;
  name: string;
  status: 'READY' | 'PROCESSING' | 'OFFLINE';
  fileCount?: number;
  totalSize?: number;
  watching?: boolean;
}

/**
 * 搜索参数
 */
export interface SearchParams extends Record<string, unknown> {
  query: string;
  workspaceId?: string;
  maxResults?: number;
  filters?: SearchFilters;
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
 * 导出参数
 */
export interface ExportParams extends Record<string, unknown> {
  results: any[];
  format: 'csv' | 'json';
  savePath: string;
}

/**
 * 文件监听参数
 */
export interface WatchParams extends Record<string, unknown> {
  workspaceId: string;
  autoSearch?: boolean;
}

/**
 * 应用配置
 */
export interface AppConfig {
  keyword_groups: any[];
  workspaces: any[];
  advanced_features: {
    enable_filter_engine: boolean;
    enable_regex_engine: boolean;
    enable_time_partition: boolean;
    enable_autocomplete: boolean;
    regex_cache_size: number;
    autocomplete_limit: number;
    time_partition_size_secs: number;
  };
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
      return await invoke('load_workspace', { workspaceId });
    } catch (error) {
      throw createApiError('load_workspace', error);
    }
  }

  /**
   * 刷新工作区
   *
   * @param workspaceId - 工作区 ID
   * @returns 工作区 ID
   */
  async refreshWorkspace(workspaceId: string): Promise<string> {
    try {
      return await invoke('refresh_workspace', { workspaceId });
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
      return await invoke('get_workspace_status', { workspaceId });
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
      return await invoke('search_logs', params);
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
  async asyncSearchLogs(params: SearchParams): Promise<string> {
    try {
      return await invoke('async_search_logs', params);
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
  async checkRarSupport(): Promise<Record<string, unknown>> {
    try {
      return await invoke('check_rar_support');
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
      await invoke('start_watch', params);
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
      return await invoke('load_config');
    } catch (error) {
      throw createApiError('load_config', error);
    }
  }

  /**
   * 获取文件过滤器配置
   *
   * @returns 文件过滤器配置
   */
  async getFileFilterConfig(): Promise<any> {
    try {
      return await invoke('get_file_filter_config');
    } catch (error) {
      throw createApiError('get_file_filter_config', error);
    }
  }

  /**
   * 保存文件过滤器配置
   *
   * @param filterConfig - 过滤器配置
   */
  async saveFileFilterConfig(filterConfig: any): Promise<void> {
    try {
      await invoke('save_file_filter_config', { filterConfig });
    } catch (error) {
      throw createApiError('save_file_filter_config', error);
    }
  }

  // ========================================================================
  // 性能监控
  // ========================================================================

  /**
   * 获取性能指标
   *
   * @returns 性能指标数据
   */
  async getPerformanceMetrics(): Promise<any> {
    try {
      return await invoke('get_performance_metrics');
    } catch (error) {
      throw createApiError('get_performance_metrics', error);
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
      return await invoke('export_results', params);
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
   * @returns 文件内容响应
   */
  async readFileByHash(params: {
    workspaceId: string;
    hash: string;
    maxLength?: number;
  }): Promise<any> {
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
  async getVirtualFileTree(workspaceId: string): Promise<any[]> {
    try {
      return await invoke('get_virtual_file_tree', { workspaceId });
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
  async getWorkspaceState(workspaceId: string): Promise<any> {
    try {
      return await invoke('get_workspace_state', { workspaceId });
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
  }): Promise<any[]> {
    try {
      return await invoke('get_event_history', params);
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
