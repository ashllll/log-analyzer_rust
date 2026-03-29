import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';

// 采用最稳健的物理隔离导入，彻底杜绝索引文件导致的循环依赖
import { Card } from '../components/ui/Card';
import { Button } from '../components/ui/Button';
import { Input } from '../components/ui/Input';
import { FormField } from '../components/ui/FormField';
import { useToast } from '../hooks/useToast';
import { useConfig, type CacheConfig, type SearchConfig, type TaskManagerConfig } from '../hooks/useConfig';

interface ExtractionPolicy {
  extraction: {
    max_depth: number;
    max_file_size: number;
    max_total_size: number;
    max_workspace_size: number;
    concurrent_extractions: number;
    buffer_size: number;
    use_enhanced_extraction: boolean;
  };
  security: {
    compression_ratio_threshold: number;
    exponential_backoff_threshold: number;
    enable_zip_bomb_detection: boolean;
  };
  paths: {
    enable_long_paths: boolean;
    shortening_threshold: number;
    hash_algorithm: string;
    hash_length: number;
  };
}

const defaultPolicy: ExtractionPolicy = {
  extraction: {
    max_depth: 10,
    max_file_size: 104857600,
    max_total_size: 10737418240,
    max_workspace_size: 53687091200,
    concurrent_extractions: 0,
    buffer_size: 65536,
    use_enhanced_extraction: false,
  },
  security: {
    compression_ratio_threshold: 100.0,
    exponential_backoff_threshold: 1000000.0,
    enable_zip_bomb_detection: true,
  },
  paths: {
    enable_long_paths: true,
    shortening_threshold: 0.8,
    hash_algorithm: 'SHA256',
    hash_length: 16,
  },
};

export function SettingsPage() {
  const { t } = useTranslation();
  const { showToast } = useToast();

  // Extraction Policy State
  const [policy, setPolicy] = useState<ExtractionPolicy>(defaultPolicy);
  const [policyErrors, setPolicyErrors] = useState<Record<string, string>>({});

  // Local state for configurations
  const [localCacheConfig, setLocalCacheConfig] = useState<CacheConfig | null>(null);
  const [localSearchConfig, setLocalSearchConfig] = useState<SearchConfig | null>(null);
  const [localTaskConfig, setLocalTaskConfig] = useState<TaskManagerConfig | null>(null);

  // Use config hook for system configurations
  const {
    cacheConfig,
    searchConfig,
    taskManagerConfig,
    loadAllConfigs,
    saveCacheConfig,
    saveSearchConfig,
    saveTaskManagerConfig,
    isLoading: configLoading,
    error: configError,
  } = useConfig();

  const [loading, setLoading] = useState(false);
  const [activeTab, setActiveTab] = useState<'extraction' | 'cache' | 'search' | 'task'>('extraction');

  // Load configurations and sync local state
  useEffect(() => {
    const loadConfigs = async () => {
      try {
        await loadAllConfigs();
      } catch (error) {
        console.error('Failed to load configurations:', error);
        showToast('error', t('settings.load_config_error', { error: String(error) }));
      }
    };
    loadConfigs();
  }, [loadAllConfigs, showToast]);

  // Sync local state when configs change
  useEffect(() => {
    if (cacheConfig) setLocalCacheConfig({ ...cacheConfig });
  }, [cacheConfig]);

  useEffect(() => {
    if (searchConfig) setLocalSearchConfig({ ...searchConfig });
  }, [searchConfig]);

  useEffect(() => {
    if (taskManagerConfig) setLocalTaskConfig({ ...taskManagerConfig });
  }, [taskManagerConfig]);

  const validatePolicy = (): boolean => {
    const newErrors: Record<string, string> = {};
    if (policy.extraction.max_depth < 1 || policy.extraction.max_depth > 20) {
      newErrors.max_depth = t('settings.policyErrors.max_depth_range');
    }
    if (policy.extraction.max_file_size <= 0) {
      newErrors.max_file_size = t('settings.errors.must_be_positive');
    }
    if (policy.paths.shortening_threshold <= 0 || policy.paths.shortening_threshold > 1.0) {
      newErrors.shortening_threshold = t('settings.errors.threshold_range');
    }
    setPolicyErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const validateCacheConfig = (): boolean => {
    if (!cacheConfig) return false;
    return cacheConfig.max_capacity > 0 && cacheConfig.ttl > 0 && cacheConfig.l2_capacity > 0;
  };

  const validateSearchConfig = (config: SearchConfig | null): boolean => {
    if (!config) return false;
    return config.default_max_results > 0 && config.cache_size > 0 && config.search_timeout > 0;
  };

  const validateTaskManagerConfig = (): boolean => {
    if (!taskManagerConfig) return false;
    return taskManagerConfig.max_concurrent_tasks > 0 && taskManagerConfig.task_timeout > 0;
  };

  const handleSave = async () => {
    setLoading(true);
    try {
      switch (activeTab) {
        case 'extraction':
          if (!validatePolicy()) {
            showToast('error', t('settings.validation_failed'));
            return;
          }
          showToast('success', t('settings.save_success'));
          break;
        case 'cache':
          if (localCacheConfig && validateCacheConfig()) {
            await saveCacheConfig(localCacheConfig);
            showToast('success', t('settings.cache.save_success'));
          } else {
            showToast('error', t('settings.cache.invalid'));
            return;
          }
          break;
        case 'search':
          if (localSearchConfig && validateSearchConfig(localSearchConfig)) {
            await saveSearchConfig(localSearchConfig);
            showToast('success', t('settings.search_config.save_success'));
          } else {
            showToast('error', t('settings.search_config.invalid'));
            return;
          }
          break;
        case 'task':
          if (localTaskConfig && validateTaskManagerConfig()) {
            await saveTaskManagerConfig(localTaskConfig);
            showToast('success', t('settings.task_manager.save_success'));
          } else {
            showToast('error', t('settings.task_manager.invalid'));
            return;
          }
          break;
      }
    } catch (error) {
      showToast('error', t('settings.save_error', { error: String(error) }));
    } finally {
      setLoading(false);
    }
  };

  const handleReset = () => {
    setPolicy(defaultPolicy);
    setPolicyErrors({});
    showToast('info', t('settings.reset_success'));
  };

  return (
    <div className="p-6 space-y-6 overflow-y-auto h-full">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold text-text-main">{t('settings.title')}</h1>
        <div className="flex gap-2">
          <Button onClick={handleReset} variant="secondary">
            {t('settings.reset')}
          </Button>
          <Button onClick={handleSave} disabled={loading || configLoading}>
            {loading || configLoading ? t('settings.saving') : t('settings.save')}
          </Button>
        </div>
      </div>

      {/* Error Message */}
      {configError && (
        <div className="bg-red-500/10 border border-red-500/50 rounded-lg p-4">
          <p className="text-red-500 text-sm">{configError}</p>
        </div>
      )}

      {/* Tab Navigation */}
      <div className="flex border-b border-border-base">
        <button
          onClick={() => setActiveTab('extraction')}
          className={`px-6 py-2 font-medium text-sm transition-colors ${
            activeTab === 'extraction'
              ? 'text-primary border-b-2 border-primary'
              : 'text-text-muted hover:text-text-main'
          }`}
        >
          {t('settings.tabs.extraction')}
        </button>
        <button
          onClick={() => setActiveTab('cache')}
          className={`px-6 py-2 font-medium text-sm transition-colors ${
            activeTab === 'cache'
              ? 'text-primary border-b-2 border-primary'
              : 'text-text-muted hover:text-text-main'
          }`}
        >
          {t('settings.tabs.cache')}
        </button>
        <button
          onClick={() => setActiveTab('search')}
          className={`px-6 py-2 font-medium text-sm transition-colors ${
            activeTab === 'search'
              ? 'text-primary border-b-2 border-primary'
              : 'text-text-muted hover:text-text-main'
          }`}
        >
          {t('settings.tabs.search')}
        </button>
        <button
          onClick={() => setActiveTab('task')}
          className={`px-6 py-2 font-medium text-sm transition-colors ${
            activeTab === 'task'
              ? 'text-primary border-b-2 border-primary'
              : 'text-text-muted hover:text-text-main'
          }`}
        >
          {t('settings.tabs.task')}
        </button>
      </div>

      {/* Tab Content */}
      {activeTab === 'extraction' && (
        <div className="space-y-6">
          {/* Extraction Settings */}
          <Card className="p-6">
            <h2 className="text-xl font-semibold mb-4 text-text-main">
              {t('settings.extraction.title')}
            </h2>
            <div className="space-y-4">
              <FormField label={t('settings.extraction.use_enhanced')} error={policyErrors.use_enhanced_extraction}>
                <div className="flex items-center gap-2">
                  <input
                    type="checkbox"
                    checked={policy.extraction.use_enhanced_extraction}
                    onChange={(e) => setPolicy({
                      ...policy,
                      extraction: { ...policy.extraction, use_enhanced_extraction: e.target.checked }
                    })}
                    className="w-4 h-4 rounded border-border-base text-primary focus:ring-primary/50"
                  />
                  <span className="text-sm text-text-muted">
                    {t('settings.extraction.use_enhanced_description')}
                  </span>
                </div>
              </FormField>

              <FormField label={t('settings.extraction.max_depth')} error={policyErrors.max_depth}>
                <Input
                  type="number"
                  value={policy.extraction.max_depth}
                  onChange={(e) => setPolicy({
                    ...policy,
                    extraction: { ...policy.extraction, max_depth: parseInt(e.target.value) || 10 }
                  })}
                  min={1}
                  max={20}
                />
              </FormField>

              <FormField label={t('settings.extraction.max_file_size')} error={policyErrors.max_file_size}>
                <div className="flex flex-col gap-1">
                  <Input
                    type="number"
                    value={policy.extraction.max_file_size}
                    onChange={(e) => setPolicy({
                      ...policy,
                      extraction: { ...policy.extraction, max_file_size: parseInt(e.target.value) || 104857600 }
                    })}
                  />
                  <span className="text-xs text-text-dim">
                    {(policy.extraction.max_file_size / 1024 / 1024).toFixed(2)} MB
                  </span>
                </div>
              </FormField>
            </div>
          </Card>

          {/* Security Settings */}
          <Card className="p-6">
            <h2 className="text-xl font-semibold mb-4 text-text-main">
              {t('settings.security.title')}
            </h2>
            <div className="space-y-4">
              <FormField label={t('settings.security.enable_zip_bomb_detection')}>
                <div className="flex items-center gap-2">
                  <input
                    type="checkbox"
                    checked={policy.security.enable_zip_bomb_detection}
                    onChange={(e) => setPolicy({
                      ...policy,
                      security: { ...policy.security, enable_zip_bomb_detection: e.target.checked }
                    })}
                    className="w-4 h-4 rounded border-border-base text-primary focus:ring-primary/50"
                  />
                  <span className="text-sm text-text-muted">
                    {t('settings.security.zip_bomb_description')}
                  </span>
                </div>
              </FormField>
            </div>
          </Card>

          {/* Path Settings */}
          <Card className="p-6">
            <h2 className="text-xl font-semibold mb-4 text-text-main">
              {t('settings.paths.title')}
            </h2>
            <div className="space-y-4">
              <FormField label={t('settings.paths.enable_long_paths')}>
                <div className="flex items-center gap-2">
                  <input
                    type="checkbox"
                    checked={policy.paths.enable_long_paths}
                    onChange={(e) => setPolicy({
                      ...policy,
                      paths: { ...policy.paths, enable_long_paths: e.target.checked }
                    })}
                    className="w-4 h-4 rounded border-border-base text-primary focus:ring-primary/50"
                  />
                  <span className="text-sm text-text-muted">
                    {t('settings.paths.long_paths_description')}
                  </span>
                </div>
              </FormField>

              <FormField label={t('settings.paths.shortening_threshold')} error={policyErrors.shortening_threshold}>
                <Input
                  type="number"
                  step="0.1"
                  min="0.1"
                  max="1.0"
                  value={policy.paths.shortening_threshold}
                  onChange={(e) => setPolicy({
                    ...policy,
                    paths: { ...policy.paths, shortening_threshold: parseFloat(e.target.value) || 0.8 }
                  })}
                />
              </FormField>

              <FormField label={t('settings.paths.hash_length')}>
                <Input
                  type="number"
                  min={8}
                  max={32}
                  value={policy.paths.hash_length}
                  onChange={(e) => setPolicy({
                    ...policy,
                    paths: { ...policy.paths, hash_length: parseInt(e.target.value) || 16 }
                  })}
                />
              </FormField>
            </div>
          </Card>
        </div>
      )}

      {activeTab === 'cache' && localCacheConfig && (
        <Card className="p-6">
          <h2 className="text-xl font-semibold mb-4 text-text-main">{t('settings.cache.title')}</h2>
          <div className="space-y-4">
            <FormField label={t('settings.cache.max_capacity')}>
              <Input
                type="number"
                value={localCacheConfig.max_capacity}
                onChange={(e) => setLocalCacheConfig({ ...localCacheConfig, max_capacity: parseInt(e.target.value) || 1000 })}
                min={1}
              />
              <span className="text-xs text-text-dim mt-1">{t('settings.cache.max_capacity_hint')}</span>
            </FormField>

            <FormField label={t('settings.cache.ttl')}>
              <Input
                type="number"
                value={localCacheConfig.ttl}
                onChange={(e) => setLocalCacheConfig({ ...localCacheConfig, ttl: parseInt(e.target.value) || 300 })}
                min={1}
              />
              <span className="text-xs text-text-dim mt-1">{t('settings.cache.ttl_hint')}</span>
            </FormField>

            <FormField label={t('settings.cache.l2_enabled')}>
              <div className="flex items-center gap-2">
                <input
                  type="checkbox"
                  checked={localCacheConfig.l2_enabled}
                  onChange={(e) => setLocalCacheConfig({ ...localCacheConfig, l2_enabled: e.target.checked })}
                  className="w-4 h-4 rounded border-border-base text-primary focus:ring-primary/50"
                />
                <span className="text-sm text-text-muted">
                  {t('settings.cache.l2_enabled_hint')}
                </span>
              </div>
            </FormField>

            <FormField label={t('settings.cache.l2_capacity')}>
              <Input
                type="number"
                value={localCacheConfig.l2_capacity}
                onChange={(e) => setLocalCacheConfig({ ...localCacheConfig, l2_capacity: parseInt(e.target.value) || 10000 })}
                min={1}
                disabled={!localCacheConfig.l2_enabled}
              />
              <span className="text-xs text-text-dim mt-1">{t('settings.cache.l2_capacity_hint')}</span>
            </FormField>
          </div>
        </Card>
      )}

      {activeTab === 'search' && localSearchConfig && (
        <Card className="p-6">
          <h2 className="text-xl font-semibold mb-4 text-text-main">{t('settings.search_config.title')}</h2>
          <div className="space-y-4">
            <FormField label={t('settings.search_config.default_max_results')}>
              <Input
                type="number"
                value={localSearchConfig.default_max_results}
                onChange={(e) => setLocalSearchConfig({ ...localSearchConfig, default_max_results: parseInt(e.target.value) || 1000 })}
                min={1}
              />
              <span className="text-xs text-text-dim mt-1">{t('settings.search_config.default_max_results_hint')}</span>
            </FormField>

            <FormField label={t('settings.search_config.cache_enabled')}>
              <div className="flex items-center gap-2">
                <input
                  type="checkbox"
                  checked={localSearchConfig.cache_enabled}
                  onChange={(e) => setLocalSearchConfig({ ...localSearchConfig, cache_enabled: e.target.checked })}
                  className="w-4 h-4 rounded border-border-base text-primary focus:ring-primary/50"
                />
                <span className="text-sm text-text-muted">
                  {t('settings.search_config.cache_enabled_hint')}
                </span>
              </div>
            </FormField>

            <FormField label={t('settings.search_config.cache_size')}>
              <Input
                type="number"
                value={localSearchConfig.cache_size}
                onChange={(e) => setLocalSearchConfig({ ...localSearchConfig, cache_size: parseInt(e.target.value) || 1000 })}
                min={1}
                disabled={!localSearchConfig.cache_enabled}
              />
              <span className="text-xs text-text-dim mt-1">{t('settings.search_config.cache_size_hint')}</span>
            </FormField>

            <FormField label={t('settings.search_config.search_timeout')}>
              <Input
                type="number"
                value={localSearchConfig.search_timeout}
                onChange={(e) => setLocalSearchConfig({ ...localSearchConfig, search_timeout: parseInt(e.target.value) || 30 })}
                min={1}
              />
              <span className="text-xs text-text-dim mt-1">{t('settings.search_config.search_timeout_hint')}</span>
            </FormField>
          </div>
        </Card>
      )}

      {activeTab === 'task' && localTaskConfig && (
        <Card className="p-6">
          <h2 className="text-xl font-semibold mb-4 text-text-main">{t('settings.task_manager.title')}</h2>
          <div className="space-y-4">
            <FormField label={t('settings.task_manager.max_concurrent_tasks')}>
              <Input
                type="number"
                value={localTaskConfig.max_concurrent_tasks}
                onChange={(e) => setLocalTaskConfig({ ...localTaskConfig, max_concurrent_tasks: parseInt(e.target.value) || 10 })}
                min={1}
              />
              <span className="text-xs text-text-dim mt-1">{t('settings.task_manager.max_concurrent_tasks_hint')}</span>
            </FormField>

            <FormField label={t('settings.task_manager.task_timeout')}>
              <Input
                type="number"
                value={localTaskConfig.task_timeout}
                onChange={(e) => setLocalTaskConfig({ ...localTaskConfig, task_timeout: parseInt(e.target.value) || 300 })}
                min={1}
              />
              <span className="text-xs text-text-dim mt-1">{t('settings.task_manager.task_timeout_hint')}</span>
            </FormField>

            <FormField label={t('settings.task_manager.data_dir')}>
              <Input
                type="text"
                value={localTaskConfig.data_dir}
                onChange={(e) => setLocalTaskConfig({ ...localTaskConfig, data_dir: e.target.value })}
                placeholder={t('settings.task_manager.data_dir_placeholder')}
              />
              <span className="text-xs text-text-dim mt-1">{t('settings.task_manager.data_dir_hint')}</span>
            </FormField>

            <FormField label={t('settings.task_manager.log_level')}>
              <select
                value={localTaskConfig.log_level}
                onChange={(e) => setLocalTaskConfig({ ...localTaskConfig, log_level: e.target.value })}
                className="w-full px-3 py-2 bg-bg-base border border-border-base rounded-lg text-text-main focus:outline-none focus:ring-2 focus:ring-primary/50"
              >
                <option value="trace">Trace</option>
                <option value="debug">Debug</option>
                <option value="info">Info</option>
                <option value="warn">Warn</option>
                <option value="error">Error</option>
              </select>
              <span className="text-xs text-text-dim mt-1">{t('settings.task_manager.log_level_hint')}</span>
            </FormField>

            <FormField label={t('settings.task_manager.debug_mode')}>
              <div className="flex items-center gap-2">
                <input
                  type="checkbox"
                  checked={localTaskConfig.debug_mode}
                  onChange={(e) => setLocalTaskConfig({ ...localTaskConfig, debug_mode: e.target.checked })}
                  className="w-4 h-4 rounded border-border-base text-primary focus:ring-primary/50"
                />
                <span className="text-sm text-text-muted">
                  {t('settings.task_manager.debug_mode_hint')}
                </span>
              </div>
            </FormField>

            <FormField label={t('settings.task_manager.profiling')}>
              <div className="flex items-center gap-2">
                <input
                  type="checkbox"
                  checked={localTaskConfig.enable_profiling}
                  onChange={(e) => setLocalTaskConfig({ ...localTaskConfig, enable_profiling: e.target.checked })}
                  className="w-4 h-4 rounded border-border-base text-primary focus:ring-primary/50"
                />
                <span className="text-sm text-text-muted">
                  {t('settings.task_manager.profiling_hint')}
                </span>
              </div>
            </FormField>
          </div>
        </Card>
      )}
    </div>
  );
}

export default SettingsPage;
