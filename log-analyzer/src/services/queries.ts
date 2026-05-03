/**
 * React Query Query Options 工厂
 *
 * 提供纯数据获取配置（无副作用），供各组件/Hooks 组合使用。
 * 所有 queryFn 均通过 api 层调用 Tauri 命令，保持统一的错误处理和类型安全。
 */

import { api } from './api';
import type { VirtualTreeNode } from '../types/api-responses';

// ============================================================================
// Query Keys
// ============================================================================

export const queryKeys = {
  config: ['config'] as const,
  workspaces: ['workspaces'] as const,
  workspace: (id: string) => ['workspace', id] as const,
  keywordGroups: ['keywordGroups'] as const,
  tasks: ['tasks'] as const,
  virtualFileTree: (workspaceId: string) => ['virtualFileTree', workspaceId] as const,
  cacheConfig: ['cacheConfig'] as const,
  searchConfig: ['searchConfig'] as const,
  taskManagerConfig: ['taskManagerConfig'] as const,
} as const;

// ============================================================================
// Configuration Queries
// ============================================================================

/**
 * 应用配置查询选项
 * 后端命令: load_config
 */
export const configQuery = {
  queryKey: queryKeys.config,
  queryFn: () => api.loadConfig(),
  staleTime: 60_000, // 1 分钟内视为新鲜，避免频繁请求
  gcTime: 300_000,   // 5 分钟未使用则从缓存清除
};

// ============================================================================
// Virtual File Tree Queries
// ============================================================================

/**
 * 虚拟文件树查询选项
 * 后端命令: get_virtual_file_tree
 * @param workspaceId - 工作区 ID
 */
export const virtualFileTreeQuery = (workspaceId: string) => ({
  queryKey: queryKeys.virtualFileTree(workspaceId),
  queryFn: (): Promise<VirtualTreeNode[]> => api.getVirtualFileTree(workspaceId),
  staleTime: 30_000, // 30 秒内视为新鲜
  gcTime: 300_000,
  enabled: !!workspaceId, // workspaceId 为空时不自动请求
});
