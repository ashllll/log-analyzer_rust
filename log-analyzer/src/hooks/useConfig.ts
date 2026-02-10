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
  /** 最大缓存条目数 */
  max_capacity: number;

  /** TTL (秒) */
  ttl: number;

  /** 是否启用 L2 缓存 */
  l2_enabled: boolean;

  /** L2 缓存大小 */
  l2_capacity: number;
}

/**
 * 搜索配置
 */
export interface SearchConfig {
  /** 默认最大结果数 */
  default_max_results: number;

  /** 是否启用缓存 */
  cache_enabled: boolean;

  /** 缓存大小 */
  cache_size: number;

  /** 搜索超时 (秒) */
  search_timeout: number;

  /** 是否启用正则表达式引擎 */
  enable_regex_engine: boolean;

  /** 是否启用过滤器引擎 */
  enable_filter_engine: boolean;
}

/**
 * 任务管理器配置
 */
export interface TaskManagerConfig {
  /** 最大并发任务数 */
  max_concurrent_tasks: number;

  /** 任务超时 (秒) */
  task_timeout: number;

  /** 数据目录 */
  data_dir: string;

  /** 日志级别 */
  log_level: string;

  /** 是否启用调试模式 */
  debug_mode: boolean;

  /** 是否启用性能监控 */
  enable_profiling: boolean;
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

      return { cache, search, taskManager };
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
        max_capacity: 1000,
        ttl: 300,
        l2_enabled: false,
        l2_capacity: 10000,
      },
      search: {
        default_max_results: 1000,
        cache_enabled: true,
        cache_size: 1000,
        search_timeout: 30,
        enable_regex_engine: true,
        enable_filter_engine: true,
      },
      task_manager: {
        max_concurrent_tasks: 10,
        task_timeout: 300,
        data_dir: '',
        log_level: 'info',
        debug_mode: false,
        enable_profiling: false,
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
