/**
 * 配置管理 Hook
 *
 * 管理应用配置，包括缓存配置、搜索配置和任务管理器配置
 *
 * @module hooks/useConfig
 */

import { useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { createApiError } from '../services/errors';

// ============================================================================
// 类型定义
// ============================================================================

/**
 * 缓存配置
 */
export interface CacheConfig {
  regex_cache_size: number;
  autocomplete_limit: number;
  max_cache_capacity: number;
  cache_ttl_seconds: number;
  cache_tti_seconds: number;
  compression_threshold: number;
  compression_enabled: boolean;
  access_window_size: number;
  preload_threshold: number;
  min_hit_rate_threshold: number;
  max_avg_access_time_ms: number;
  max_avg_load_time_ms: number;
  max_eviction_rate_per_min: number;
}

/**
 * 搜索配置
 */
export interface SearchConfig {
  max_results: number;
  timeout_seconds: number;
  max_concurrent_searches: number;
  fuzzy_search_enabled: boolean;
  case_sensitive: boolean;
  regex_enabled: boolean;
}

/**
 * 任务管理器配置
 */
export interface TaskManagerConfig {
  max_concurrent_tasks: number;
  completed_task_ttl: number;
  failed_task_ttl: number;
  cleanup_interval: number;
  operation_timeout: number;
}

/**
 * 所有配置
 */
export interface AllConfigs {
  cache: CacheConfig;
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
 *
 * @example
 * ```typescript
 * const {
 *   cacheConfig, searchConfig, taskManagerConfig,
 *   loadAllConfigs, saveCacheConfig, saveSearchConfig,
 *   isLoading, error
 * } = useConfig();
 * ```
 */
export function useConfig() {
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // 配置状态
  const [cacheConfig, setCacheConfig] = useState<CacheConfig | null>(null);
  const [searchConfig, setSearchConfig] = useState<SearchConfig | null>(null);
  const [taskManagerConfig, setTaskManagerConfig] = useState<TaskManagerConfig | null>(null);

  // ========================================================================
  // 加载配置
  // ========================================================================

  /**
   * 加载缓存配置
   */
  const loadCacheConfig = useCallback(async () => {
    setIsLoading(true);
    setError(null);

    try {
      const config = await invoke<CacheConfig>('get_cache_config');
      setCacheConfig(config);
      return config;
    } catch (err) {
      const apiError = createApiError('get_cache_config', err);
      setError(apiError.getUserMessage());
      throw apiError;
    } finally {
      setIsLoading(false);
    }
  }, []);

  /**
   * 加载搜索配置
   */
  const loadSearchConfig = useCallback(async () => {
    setIsLoading(true);
    setError(null);

    try {
      const config = await invoke<SearchConfig>('get_search_config');
      setSearchConfig(config);
      return config;
    } catch (err) {
      const apiError = createApiError('get_search_config', err);
      setError(apiError.getUserMessage());
      throw apiError;
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
      const config = await invoke<TaskManagerConfig>('get_task_manager_config');
      setTaskManagerConfig(config);
      return config;
    } catch (err) {
      const apiError = createApiError('get_task_manager_config', err);
      setError(apiError.getUserMessage());
      throw apiError;
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
      const [cache, search, taskManager] = await Promise.all([
        invoke<CacheConfig>('get_cache_config'),
        invoke<SearchConfig>('get_search_config'),
        invoke<TaskManagerConfig>('get_task_manager_config'),
      ]);

      setCacheConfig(cache);
      setSearchConfig(search);
      setTaskManagerConfig(taskManager);

      return { cache, search, task_manager: taskManager };
    } catch (err) {
      const apiError = createApiError('load_configs', err);
      setError(apiError.getUserMessage());
      throw apiError;
    } finally {
      setIsLoading(false);
    }
  }, []);

  // ========================================================================
  // 保存配置
  // ========================================================================

  /**
   * 保存缓存配置
   */
  const saveCacheConfig = useCallback(async (config: CacheConfig) => {
    setIsLoading(true);
    setError(null);

    try {
      await invoke('save_cache_config', { cacheConfig: config });
      setCacheConfig(config);
    } catch (err) {
      const apiError = createApiError('save_cache_config', err);
      setError(apiError.getUserMessage());
      throw apiError;
    } finally {
      setIsLoading(false);
    }
  }, []);

  /**
   * 保存搜索配置
   */
  const saveSearchConfig = useCallback(async (config: SearchConfig) => {
    setIsLoading(true);
    setError(null);

    try {
      await invoke('save_search_config', { searchConfig: config });
      setSearchConfig(config);
    } catch (err) {
      const apiError = createApiError('save_search_config', err);
      setError(apiError.getUserMessage());
      throw apiError;
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
      await invoke('save_task_manager_config', { taskManagerConfig: config });
      setTaskManagerConfig(config);
    } catch (err) {
      const apiError = createApiError('save_task_manager_config', err);
      setError(apiError.getUserMessage());
      throw apiError;
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
        invoke('save_cache_config', { cacheConfig: configs.cache }),
        invoke('save_search_config', { searchConfig: configs.search }),
        invoke('save_task_manager_config', { taskManagerConfig: configs.task_manager }),
      ]);

      setCacheConfig(configs.cache);
      setSearchConfig(configs.search);
      setTaskManagerConfig(configs.task_manager);
    } catch (err) {
      const apiError = createApiError('save_configs', err);
      setError(apiError.getUserMessage());
      throw apiError;
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
      cache: {
        regex_cache_size: 1000,
        autocomplete_limit: 100,
        max_cache_capacity: 100,
        cache_ttl_seconds: 300,
        cache_tti_seconds: 60,
        compression_threshold: 10 * 1024,
        compression_enabled: true,
        access_window_size: 1000,
        preload_threshold: 5,
        min_hit_rate_threshold: 0.7,
        max_avg_access_time_ms: 10,
        max_avg_load_time_ms: 100,
        max_eviction_rate_per_min: 10,
      },
      search: {
        max_results: 1000,
        timeout_seconds: 10,
        max_concurrent_searches: 10,
        fuzzy_search_enabled: true,
        case_sensitive: false,
        regex_enabled: true,
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
    cacheConfig,
    searchConfig,
    taskManagerConfig,

    // 加载配置
    loadCacheConfig,
    loadSearchConfig,
    loadTaskManagerConfig,
    loadAllConfigs,

    // 保存配置
    saveCacheConfig,
    saveSearchConfig,
    saveTaskManagerConfig,
    saveAllConfigs,

    // 重置
    resetToDefaults,
  };
}
