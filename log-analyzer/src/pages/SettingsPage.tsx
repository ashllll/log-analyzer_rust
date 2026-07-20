import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";

// 采用最稳健的物理隔离导入，彻底杜绝索引文件导致的循环依赖
import { Card } from "../components/ui/Card";
import { Button } from "../components/ui/Button";
import { Input } from "../components/ui/Input";
import { FormField } from "../components/ui/FormField";
import { useToast } from "../hooks/useToast";
import {
  useConfig,
  type SearchConfig,
  type TaskManagerConfig,
} from "../hooks/useConfig";

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
    hash_algorithm: "SHA256",
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
  const [localSearchConfig, setLocalSearchConfig] =
    useState<SearchConfig | null>(null);
  const [localTaskConfig, setLocalTaskConfig] =
    useState<TaskManagerConfig | null>(null);

  // Use config hook for system configurations
  const {
    searchConfig,
    taskManagerConfig,
    loadAllConfigs,
    saveSearchConfig,
    saveTaskManagerConfig,
    isLoading: configLoading,
    error: configError,
  } = useConfig();

  const [loading, setLoading] = useState(false);
  const [saveStatus, setSaveStatus] = useState<"saved" | "error" | null>(null);
  const [activeTab, setActiveTab] = useState<"extraction" | "search" | "task">(
    "extraction"
  );

  // Load configurations and sync local state
  useEffect(() => {
    const loadConfigs = async () => {
      try {
        await loadAllConfigs();
      } catch (error) {
        showToast(
          "error",
          t("settings.load_config_error", { error: String(error) })
        );
      }
    };
    loadConfigs();
  }, [loadAllConfigs, showToast, t]);

  // Sync local state when configs change
  useEffect(() => {
    if (searchConfig) setLocalSearchConfig({ ...searchConfig });
  }, [searchConfig]);

  useEffect(() => {
    if (taskManagerConfig) setLocalTaskConfig({ ...taskManagerConfig });
  }, [taskManagerConfig]);

  const validatePolicy = (): boolean => {
    const newErrors: Record<string, string> = {};
    if (policy.extraction.max_depth < 1 || policy.extraction.max_depth > 20) {
      newErrors.max_depth = t("settings.policyErrors.max_depth_range");
    }
    if (policy.extraction.max_file_size <= 0) {
      newErrors.max_file_size = t("settings.errors.must_be_positive");
    }
    if (
      policy.paths.shortening_threshold <= 0 ||
      policy.paths.shortening_threshold > 1.0
    ) {
      newErrors.shortening_threshold = t("settings.errors.threshold_range");
    }
    setPolicyErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const validateSearchConfig = (config: SearchConfig | null): boolean => {
    if (!config) return false;
    return (
      config.max_results > 0 &&
      config.timeout_seconds > 0 &&
      config.max_concurrent_searches > 0 &&
      config.regex_cache_size > 0
    );
  };

  const validateTaskManagerConfig = (): boolean => {
    if (!localTaskConfig) return false;
    return (
      localTaskConfig.max_concurrent_tasks > 0 &&
      localTaskConfig.operation_timeout > 0 &&
      localTaskConfig.completed_task_ttl > 0 &&
      localTaskConfig.failed_task_ttl > 0 &&
      localTaskConfig.cleanup_interval > 0
    );
  };

  const handleSave = async () => {
    setLoading(true);
    setSaveStatus(null);
    try {
      switch (activeTab) {
        case "extraction":
          if (!validatePolicy()) {
            showToast("error", t("settings.validation_failed"));
            return;
          }
          setSaveStatus("saved");
          break;
        case "search":
          if (localSearchConfig && validateSearchConfig(localSearchConfig)) {
            await saveSearchConfig(localSearchConfig);
            setSaveStatus("saved");
          } else {
            showToast("error", t("settings.search_config.invalid"));
            return;
          }
          break;
        case "task":
          if (localTaskConfig && validateTaskManagerConfig()) {
            await saveTaskManagerConfig(localTaskConfig);
            setSaveStatus("saved");
          } else {
            showToast("error", t("settings.task_manager.invalid"));
            return;
          }
          break;
      }
    } catch (error) {
      setSaveStatus("error");
      showToast("error", t("settings.save_error", { error: String(error) }));
    } finally {
      setLoading(false);
    }
  };

  const handleReset = () => {
    setPolicy(defaultPolicy);
    setPolicyErrors({});
    setSaveStatus(null);
    showToast("info", t("settings.reset_success"));
  };

  return (
    <div className="mx-auto h-full max-w-5xl space-y-6 overflow-y-auto px-8 py-7">
      <div className="flex items-center justify-between">
        <h1 className="text-[28px] font-semibold tracking-[-0.02em] text-text-main">
          {t("settings.title")}
        </h1>
      </div>

      {/* Error Message */}
      {configError && (
        <div className="bg-status-error/10 border border-status-error/50 rounded-lg p-4">
          <p className="text-status-error text-sm">{configError}</p>
        </div>
      )}

      <div className="grid grid-cols-1 items-start gap-6 lg:grid-cols-[200px_minmax(0,1fr)]">
        {/* Settings group navigation */}
        <div
          className="sticky top-4 flex flex-col rounded-[12px] bg-bg-elevated p-1.5"
          role="group"
          aria-label="Settings section"
        >
          <button
            onClick={() => setActiveTab("extraction")}
            aria-pressed={activeTab === "extraction"}
            className={`rounded-[8px] px-3 py-2 text-left text-sm font-medium transition-[color,background-color,box-shadow] duration-150 ${
              activeTab === "extraction"
                ? "bg-bg-card text-text-main shadow-sm"
                : "text-text-muted hover:text-text-main"
            }`}
          >
            {t("settings.tabs.extraction")}
          </button>
          <button
            onClick={() => setActiveTab("search")}
            aria-pressed={activeTab === "search"}
            className={`rounded-[8px] px-3 py-2 text-left text-sm font-medium transition-[color,background-color,box-shadow] duration-150 ${
              activeTab === "search"
                ? "bg-bg-card text-text-main shadow-sm"
                : "text-text-muted hover:text-text-main"
            }`}
          >
            {t("settings.tabs.search")}
          </button>
          <button
            onClick={() => setActiveTab("task")}
            aria-pressed={activeTab === "task"}
            className={`rounded-[8px] px-3 py-2 text-left text-sm font-medium transition-[color,background-color,box-shadow] duration-150 ${
              activeTab === "task"
                ? "bg-bg-card text-text-main shadow-sm"
                : "text-text-muted hover:text-text-main"
            }`}
          >
            {t("settings.tabs.task")}
          </button>
        </div>

        {/* Tab Content */}
        <section className="min-w-0">
          {activeTab === "extraction" && (
            <div className="space-y-6">
              {/* Extraction Settings */}
              <Card className="p-6">
                <h2 className="text-xl font-semibold mb-4 text-text-main">
                  {t("settings.extraction.title")}
                </h2>
                <div className="space-y-4">
                  <FormField
                    label={t("settings.extraction.use_enhanced")}
                    error={policyErrors.use_enhanced_extraction}
                  >
                    <div className="flex items-center gap-2">
                      <input
                        type="checkbox"
                        checked={policy.extraction.use_enhanced_extraction}
                        onChange={(e) =>
                          setPolicy({
                            ...policy,
                            extraction: {
                              ...policy.extraction,
                              use_enhanced_extraction: e.target.checked,
                            },
                          })
                        }
                        className="w-4 h-4 rounded border-border-base text-primary focus:ring-primary/50"
                      />
                      <span className="text-sm text-text-muted">
                        {t("settings.extraction.use_enhanced_description")}
                      </span>
                    </div>
                  </FormField>

                  <FormField
                    label={t("settings.extraction.max_depth")}
                    error={policyErrors.max_depth}
                  >
                    <Input
                      type="number"
                      value={policy.extraction.max_depth}
                      onChange={(e) =>
                        setPolicy({
                          ...policy,
                          extraction: {
                            ...policy.extraction,
                            max_depth: parseInt(e.target.value) || 10,
                          },
                        })
                      }
                      min={1}
                      max={20}
                    />
                  </FormField>

                  <FormField
                    label={t("settings.extraction.max_file_size")}
                    error={policyErrors.max_file_size}
                  >
                    <div className="flex flex-col gap-1">
                      <Input
                        type="number"
                        value={policy.extraction.max_file_size}
                        onChange={(e) =>
                          setPolicy({
                            ...policy,
                            extraction: {
                              ...policy.extraction,
                              max_file_size:
                                parseInt(e.target.value) || 104857600,
                            },
                          })
                        }
                      />
                      <span className="text-xs text-text-dim">
                        {(
                          policy.extraction.max_file_size /
                          1024 /
                          1024
                        ).toFixed(2)}{" "}
                        MB
                      </span>
                    </div>
                  </FormField>
                </div>
              </Card>

              {/* Security Settings */}
              <Card className="p-6">
                <h2 className="text-xl font-semibold mb-4 text-text-main">
                  {t("settings.security.title")}
                </h2>
                <div className="space-y-4">
                  <FormField
                    label={t("settings.security.enable_zip_bomb_detection")}
                  >
                    <div className="flex items-center gap-2">
                      <input
                        type="checkbox"
                        checked={policy.security.enable_zip_bomb_detection}
                        onChange={(e) =>
                          setPolicy({
                            ...policy,
                            security: {
                              ...policy.security,
                              enable_zip_bomb_detection: e.target.checked,
                            },
                          })
                        }
                        className="w-4 h-4 rounded border-border-base text-primary focus:ring-primary/50"
                      />
                      <span className="text-sm text-text-muted">
                        {t("settings.security.zip_bomb_description")}
                      </span>
                    </div>
                  </FormField>
                </div>
              </Card>

              {/* Path Settings */}
              <Card className="p-6">
                <h2 className="text-xl font-semibold mb-4 text-text-main">
                  {t("settings.paths.title")}
                </h2>
                <div className="space-y-4">
                  <FormField label={t("settings.paths.enable_long_paths")}>
                    <div className="flex items-center gap-2">
                      <input
                        type="checkbox"
                        checked={policy.paths.enable_long_paths}
                        onChange={(e) =>
                          setPolicy({
                            ...policy,
                            paths: {
                              ...policy.paths,
                              enable_long_paths: e.target.checked,
                            },
                          })
                        }
                        className="w-4 h-4 rounded border-border-base text-primary focus:ring-primary/50"
                      />
                      <span className="text-sm text-text-muted">
                        {t("settings.paths.long_paths_description")}
                      </span>
                    </div>
                  </FormField>

                  <FormField
                    label={t("settings.paths.shortening_threshold")}
                    error={policyErrors.shortening_threshold}
                  >
                    <Input
                      type="number"
                      step="0.1"
                      min="0.1"
                      max="1.0"
                      value={policy.paths.shortening_threshold}
                      onChange={(e) =>
                        setPolicy({
                          ...policy,
                          paths: {
                            ...policy.paths,
                            shortening_threshold:
                              parseFloat(e.target.value) || 0.8,
                          },
                        })
                      }
                    />
                  </FormField>

                  <FormField label={t("settings.paths.hash_length")}>
                    <Input
                      type="number"
                      min={8}
                      max={32}
                      value={policy.paths.hash_length}
                      onChange={(e) =>
                        setPolicy({
                          ...policy,
                          paths: {
                            ...policy.paths,
                            hash_length: parseInt(e.target.value) || 16,
                          },
                        })
                      }
                    />
                  </FormField>
                </div>
              </Card>
            </div>
          )}

          {activeTab === "search" && localSearchConfig && (
            <Card className="p-6">
              <h2 className="text-xl font-semibold mb-4 text-text-main">
                {t("settings.search_config.title")}
              </h2>
              <div className="space-y-4">
                <FormField label={t("settings.search_config.max_results")}>
                  <Input
                    type="number"
                    value={localSearchConfig.max_results}
                    onChange={(e) =>
                      setLocalSearchConfig({
                        ...localSearchConfig,
                        max_results: parseInt(e.target.value, 10) || 1000,
                      })
                    }
                    min={1}
                  />
                  <span className="text-xs text-text-dim mt-1">
                    {t("settings.search_config.max_results_hint")}
                  </span>
                </FormField>

                <FormField label={t("settings.search_config.timeout_seconds")}>
                  <Input
                    type="number"
                    value={localSearchConfig.timeout_seconds}
                    onChange={(e) =>
                      setLocalSearchConfig({
                        ...localSearchConfig,
                        timeout_seconds: parseInt(e.target.value, 10) || 10,
                      })
                    }
                    min={1}
                  />
                  <span className="text-xs text-text-dim mt-1">
                    {t("settings.search_config.timeout_seconds_hint")}
                  </span>
                </FormField>

                <FormField
                  label={t("settings.search_config.max_concurrent_searches")}
                >
                  <Input
                    type="number"
                    value={localSearchConfig.max_concurrent_searches}
                    onChange={(e) =>
                      setLocalSearchConfig({
                        ...localSearchConfig,
                        max_concurrent_searches:
                          parseInt(e.target.value, 10) || 10,
                      })
                    }
                    min={1}
                  />
                  <span className="text-xs text-text-dim mt-1">
                    {t("settings.search_config.max_concurrent_searches_hint")}
                  </span>
                </FormField>

                <FormField label={t("settings.search_config.case_sensitive")}>
                  <div className="flex items-center gap-2">
                    <input
                      type="checkbox"
                      checked={localSearchConfig.case_sensitive}
                      onChange={(e) =>
                        setLocalSearchConfig({
                          ...localSearchConfig,
                          case_sensitive: e.target.checked,
                        })
                      }
                      className="w-4 h-4 rounded border-border-base text-primary focus:ring-primary/50"
                    />
                    <span className="text-sm text-text-muted">
                      {t("settings.search_config.case_sensitive_hint")}
                    </span>
                  </div>
                </FormField>

                <FormField label={t("settings.search_config.regex_enabled")}>
                  <div className="flex items-center gap-2">
                    <input
                      type="checkbox"
                      checked={localSearchConfig.regex_enabled}
                      onChange={(e) =>
                        setLocalSearchConfig({
                          ...localSearchConfig,
                          regex_enabled: e.target.checked,
                        })
                      }
                      className="w-4 h-4 rounded border-border-base text-primary focus:ring-primary/50"
                    />
                    <span className="text-sm text-text-muted">
                      {t("settings.search_config.regex_enabled_hint")}
                    </span>
                  </div>
                </FormField>

                <FormField label={t("settings.search_config.regex_cache_size")}>
                  <Input
                    type="number"
                    value={localSearchConfig.regex_cache_size}
                    onChange={(e) =>
                      setLocalSearchConfig({
                        ...localSearchConfig,
                        regex_cache_size: parseInt(e.target.value, 10) || 1000,
                      })
                    }
                    min={1}
                  />
                  <span className="text-xs text-text-dim mt-1">
                    {t("settings.search_config.regex_cache_size_hint")}
                  </span>
                </FormField>

                <FormField
                  label={t("settings.search_config.fuzzy_search_enabled")}
                >
                  <div className="flex items-center gap-2">
                    <input
                      type="checkbox"
                      checked={localSearchConfig.fuzzy_search_enabled}
                      onChange={(e) =>
                        setLocalSearchConfig({
                          ...localSearchConfig,
                          fuzzy_search_enabled: e.target.checked,
                        })
                      }
                      className="w-4 h-4 rounded border-border-base text-primary focus:ring-primary/50"
                    />
                    <span className="text-sm text-text-muted">
                      {t("settings.search_config.fuzzy_search_enabled_hint")}
                    </span>
                  </div>
                </FormField>
              </div>
            </Card>
          )}

          {activeTab === "task" && localTaskConfig && (
            <Card className="p-6">
              <h2 className="text-xl font-semibold mb-4 text-text-main">
                {t("settings.task_manager.title")}
              </h2>
              <div className="space-y-4">
                <FormField
                  label={t("settings.task_manager.max_concurrent_tasks")}
                >
                  <Input
                    type="number"
                    value={localTaskConfig.max_concurrent_tasks}
                    onChange={(e) =>
                      setLocalTaskConfig({
                        ...localTaskConfig,
                        max_concurrent_tasks:
                          parseInt(e.target.value, 10) || 10,
                      })
                    }
                    min={1}
                  />
                  <span className="text-xs text-text-dim mt-1">
                    {t("settings.task_manager.max_concurrent_tasks_hint")}
                  </span>
                </FormField>

                <FormField label={t("settings.task_manager.operation_timeout")}>
                  <Input
                    type="number"
                    value={localTaskConfig.operation_timeout}
                    onChange={(e) =>
                      setLocalTaskConfig({
                        ...localTaskConfig,
                        operation_timeout: parseInt(e.target.value, 10) || 30,
                      })
                    }
                    min={1}
                  />
                  <span className="text-xs text-text-dim mt-1">
                    {t("settings.task_manager.operation_timeout_hint")}
                  </span>
                </FormField>

                <FormField
                  label={t("settings.task_manager.completed_task_ttl")}
                >
                  <Input
                    type="number"
                    value={localTaskConfig.completed_task_ttl}
                    onChange={(e) =>
                      setLocalTaskConfig({
                        ...localTaskConfig,
                        completed_task_ttl: parseInt(e.target.value, 10) || 300,
                      })
                    }
                    min={1}
                  />
                  <span className="text-xs text-text-dim mt-1">
                    {t("settings.task_manager.completed_task_ttl_hint")}
                  </span>
                </FormField>

                <FormField label={t("settings.task_manager.failed_task_ttl")}>
                  <Input
                    type="number"
                    value={localTaskConfig.failed_task_ttl}
                    onChange={(e) =>
                      setLocalTaskConfig({
                        ...localTaskConfig,
                        failed_task_ttl: parseInt(e.target.value, 10) || 1800,
                      })
                    }
                    min={1}
                  />
                  <span className="text-xs text-text-dim mt-1">
                    {t("settings.task_manager.failed_task_ttl_hint")}
                  </span>
                </FormField>

                <FormField label={t("settings.task_manager.cleanup_interval")}>
                  <Input
                    type="number"
                    value={localTaskConfig.cleanup_interval}
                    onChange={(e) =>
                      setLocalTaskConfig({
                        ...localTaskConfig,
                        cleanup_interval: parseInt(e.target.value, 10) || 60,
                      })
                    }
                    min={1}
                  />
                  <span className="text-xs text-text-dim mt-1">
                    {t("settings.task_manager.cleanup_interval_hint")}
                  </span>
                </FormField>
              </div>
            </Card>
          )}
        </section>
      </div>

      <div
        className="apple-material sticky bottom-0 z-20 -mx-2 flex items-center justify-end gap-2 rounded-[14px] border border-border-subtle px-3 py-2 shadow-elevated"
        aria-live="polite"
      >
        {saveStatus && (
          <span
            className={`mr-auto text-sm ${saveStatus === "saved" ? "text-status-success" : "text-status-error"}`}
          >
            {saveStatus === "saved"
              ? t("settings.save_success")
              : t("settings.save_error", { error: "" })}
          </span>
        )}
        <Button onClick={handleReset} variant="secondary">
          {t("settings.reset")}
        </Button>
        <Button onClick={handleSave} disabled={loading || configLoading}>
          {loading || configLoading ? t("settings.saving") : t("settings.save")}
        </Button>
      </div>
    </div>
  );
}

export default SettingsPage;
