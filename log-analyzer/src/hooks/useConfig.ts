/**
 * 配置管理 Hook
 *
 * 管理应用配置，包括搜索配置和任务管理器配置
 *
 * @module hooks/useConfig
 */

import { useState, useCallback } from 'react';
import { api, type SearchConfig, type TaskManagerConfig } from '../services/api';
import { getFullErrorMessage } from '../services/errors';

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
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // 配置状态
  const [searchConfig, setSearchConfig] = useState<SearchConfig | null>(null);
  const [taskManagerConfig, setTaskManagerConfig] = useState<TaskManagerConfig | null>(null);

  // ========================================================================
  // 加载配置
  // ========================================================================

  /**
   * 加载搜索配置
   */
  const loadSearchConfig = useCallback(async () => {
    setIsLoading(true);
    setError(null);

    try {
      const config = await api.getSearchConfig();
      setSearchConfig(config);
      return config;
    } catch (err) {
      setError(getFullErrorMessage(err));
      throw err;
    } finally {
      setIsLoading(false);
    }
  }, []);

  /**
   * 加载任务管理器配置
   */
  const loadTaskManagerConfig = useCallback(async () => {
    setIsLoading(true);
    setError(null);

    try {
      const config = await api.getTaskManagerConfig();
      setTaskManagerConfig(config);
      return config;
    } catch (err) {
      setError(getFullErrorMessage(err));
      throw err;
    } finally {
      setIsLoading(false);
    }
  }, []);

  /**
   * 加载所有配置
   */
  const loadAllConfigs = useCallback(async () => {
    setIsLoading(true);
    setError(null);

    try {
      const [search, taskManager] = await Promise.all([
        api.getSearchConfig(),
        api.getTaskManagerConfig(),
      ]);

      setSearchConfig(search);
      setTaskManagerConfig(taskManager);

      return { search, task_manager: taskManager };
    } catch (err) {
      setError(getFullErrorMessage(err));
      throw err;
    } finally {
      setIsLoading(false);
    }
  }, []);

  // ========================================================================
  // 保存配置
  // ========================================================================

  /**
   * 保存搜索配置
   */
  const saveSearchConfig = useCallback(async (config: SearchConfig) => {
    setIsLoading(true);
    setError(null);

    try {
      await api.saveSearchConfig(config);
      setSearchConfig(config);
    } catch (err) {
      setError(getFullErrorMessage(err));
      throw err;
    } finally {
      setIsLoading(false);
    }
  }, []);

  /**
   * 保存任务管理器配置
   */
  const saveTaskManagerConfig = useCallback(async (config: TaskManagerConfig) => {
    setIsLoading(true);
    setError(null);

    try {
      await api.saveTaskManagerConfig(config);
      setTaskManagerConfig(config);
    } catch (err) {
      setError(getFullErrorMessage(err));
      throw err;
    } finally {
      setIsLoading(false);
    }
  }, []);

  /**
   * 保存所有配置
   */
  const saveAllConfigs = useCallback(async (configs: AllConfigs) => {
    setIsLoading(true);
    setError(null);

    try {
      await Promise.all([
        api.saveSearchConfig(configs.search),
        api.saveTaskManagerConfig(configs.task_manager),
      ]);

      setSearchConfig(configs.search);
      setTaskManagerConfig(configs.task_manager);
    } catch (err) {
      setError(getFullErrorMessage(err));
      throw err;
    } finally {
      setIsLoading(false);
    }
  }, []);

  // ========================================================================
  // 重置配置
  // ========================================================================

  /**
   * 重置为默认配置
   */
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
