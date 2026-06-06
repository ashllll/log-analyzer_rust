/**
 * 配置管理 Hook
 *
 * 管理应用配置，包括搜索配置和任务管理器配置
 *
 * @module hooks/useConfig
 */

import { useState, useCallback } from 'react';
import { api, type SearchConfig, type TaskManagerConfig } from '../services/api';
import { useAsyncAction } from './useAsyncAction';

// ============================================================================
// 类型定义
// ============================================================================

export type { SearchConfig, TaskManagerConfig } from '../services/api';

/**
 * 所有配置
 */
export interface AllConfigs {
  search: SearchConfig;
  task_manager: TaskManagerConfig;
}

// ============================================================================
// 配置 Hook
// ============================================================================

/**
 * 配置管理 Hook
 *
 * 提供配置的加载、保存和验证功能
 */
export function useConfig() {
  const { execute, isLoading, error } = useAsyncAction();

  // 配置状态
  const [searchConfig, setSearchConfig] = useState<SearchConfig | null>(null);
  const [taskManagerConfig, setTaskManagerConfig] = useState<TaskManagerConfig | null>(null);

  // ========================================================================
  // 加载配置
  // ========================================================================

  const loadSearchConfig = useCallback(async () => {
    return execute(
      () => api.getSearchConfig(),
      { rethrow: true, onSuccess: (c) => setSearchConfig(c) },
    );
  }, [execute]);

  const loadTaskManagerConfig = useCallback(async () => {
    return execute(
      () => api.getTaskManagerConfig(),
      { rethrow: true, onSuccess: (c) => setTaskManagerConfig(c) },
    );
  }, [execute]);

  const loadAllConfigs = useCallback(async () => {
    const result = await execute(
      async () => {
        const [search, taskManager] = await Promise.all([
          api.getSearchConfig(),
          api.getTaskManagerConfig(),
        ]);
        return { search, task_manager: taskManager } as AllConfigs;
      },
      { rethrow: true },
    );
    if (result) {
      setSearchConfig(result.search);
      setTaskManagerConfig(result.task_manager);
    }
    return result;
  }, [execute]);

  // ========================================================================
  // 保存配置
  // ========================================================================

  const saveSearchConfig = useCallback(async (config: SearchConfig) => {
    return execute(
      () => api.saveSearchConfig(config),
      { rethrow: true, onSuccess: () => setSearchConfig(config) },
    );
  }, [execute]);

  const saveTaskManagerConfig = useCallback(async (config: TaskManagerConfig) => {
    return execute(
      () => api.saveTaskManagerConfig(config),
      { rethrow: true, onSuccess: () => setTaskManagerConfig(config) },
    );
  }, [execute]);

  const saveAllConfigs = useCallback(async (configs: AllConfigs) => {
    return execute(
      async () => {
        await Promise.all([
          api.saveSearchConfig(configs.search),
          api.saveTaskManagerConfig(configs.task_manager),
        ]);
      },
      {
        rethrow: true,
        onSuccess: () => {
          setSearchConfig(configs.search);
          setTaskManagerConfig(configs.task_manager);
        },
      },
    );
  }, [execute]);

  // ========================================================================
  // 重置配置
  // ========================================================================

  const resetToDefaults = useCallback(async () => {
    const defaults: AllConfigs = {
      search: {
        max_results: 1000,
        timeout_seconds: 10,
        max_concurrent_searches: 10,
        fuzzy_search_enabled: true,
        case_sensitive: false,
        regex_enabled: true,
        regex_cache_size: 1000,
      },
      task_manager: {
        max_concurrent_tasks: 10,
        completed_task_ttl: 300,
        failed_task_ttl: 1800,
        cleanup_interval: 60,
        operation_timeout: 30,
      },
    };

    await saveAllConfigs(defaults);
    return defaults;
  }, [saveAllConfigs]);

  return {
    // 状态
    isLoading,
    error,
    searchConfig,
    taskManagerConfig,

    // 加载配置
    loadSearchConfig,
    loadTaskManagerConfig,
    loadAllConfigs,

    // 保存配置
    saveSearchConfig,
    saveTaskManagerConfig,
    saveAllConfigs,

    // 重置
    resetToDefaults,
  };
}
