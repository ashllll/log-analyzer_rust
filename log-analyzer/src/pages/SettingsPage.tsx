import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { Card } from '../components/ui/Card';
import { Button } from '../components/ui/Button';
import { Input } from '../components/ui/Input';
import { FormField } from '../components/ui/FormField';
import { useToastManager } from '../hooks/useToastManager';

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
  performance: {
    temp_dir_ttl_hours: number;
    log_retention_days: number;
    enable_streaming: boolean;
    directory_batch_size: number;
    parallel_files_per_archive: number;
  };
  audit: {
    enable_audit_logging: boolean;
    log_format: string;
    log_level: string;
    log_security_events: boolean;
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
  performance: {
    temp_dir_ttl_hours: 24,
    log_retention_days: 90,
    enable_streaming: true,
    directory_batch_size: 10,
    parallel_files_per_archive: 4,
  },
  audit: {
    enable_audit_logging: true,
    log_format: 'json',
    log_level: 'info',
    log_security_events: true,
  },
};

export function SettingsPage() {
  const { t } = useTranslation();
  const { showToast } = useToastManager();
  const [policy, setPolicy] = useState<ExtractionPolicy>(defaultPolicy);
  const [loading, setLoading] = useState(false);
  const [errors, setErrors] = useState<Record<string, string>>({});

  // Load current policy on mount
  useEffect(() => {
    // Use default policy for now
  }, []);

  const validatePolicy = (): boolean => {
    const newErrors: Record<string, string> = {};

    // Validate max_depth
    if (policy.extraction.max_depth < 1 || policy.extraction.max_depth > 20) {
      newErrors.max_depth = t('settings.errors.max_depth_range');
    }

    // Validate positive sizes
    if (policy.extraction.max_file_size <= 0) {
      newErrors.max_file_size = t('settings.errors.must_be_positive');
    }

    if (policy.extraction.max_total_size <= 0) {
      newErrors.max_total_size = t('settings.errors.must_be_positive');
    }

    if (policy.extraction.max_workspace_size <= 0) {
      newErrors.max_workspace_size = t('settings.errors.must_be_positive');
    }

    // Validate shortening threshold
    if (
      policy.paths.shortening_threshold <= 0 ||
      policy.paths.shortening_threshold > 1.0
    ) {
      newErrors.shortening_threshold = t('settings.errors.threshold_range');
    }

    // Validate hash length
    if (policy.paths.hash_length < 8 || policy.paths.hash_length > 32) {
      newErrors.hash_length = t('settings.errors.hash_length_range');
    }

    // Validate parallel files
    if (
      policy.performance.parallel_files_per_archive < 1 ||
      policy.performance.parallel_files_per_archive > 8
    ) {
      newErrors.parallel_files_per_archive = t(
        'settings.errors.parallel_files_range'
      );
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const handleSave = async () => {
    if (!validatePolicy()) {
      showToast('error', t('settings.validation_failed'));
      return;
    }

    setLoading(true);
    try {
      // Policy saving will be implemented when backend API is ready
      showToast('success', t('settings.save_success'));
    } catch (error) {
      showToast(
        'error',
        t('settings.save_error', { error: String(error) })
      );
    } finally {
      setLoading(false);
    }
  };

  const handleReset = () => {
    setPolicy(defaultPolicy);
    setErrors({});
    showToast('info', t('settings.reset_success'));
  };

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold">{t('settings.title')}</h1>
        <div className="flex gap-2">
          <Button onClick={handleReset} variant="secondary">
            {t('settings.reset')}
          </Button>
          <Button onClick={handleSave} disabled={loading}>
            {loading ? t('settings.saving') : t('settings.save')}
          </Button>
        </div>
      </div>

      {/* Extraction Settings */}
      <Card>
        <h2 className="text-xl font-semibold mb-4">
          {t('settings.extraction.title')}
        </h2>
        <div className="space-y-4">
          <FormField
            label={t('settings.extraction.use_enhanced')}
            error={errors.use_enhanced_extraction}
          >
            <label className="flex items-center gap-2">
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
                className="w-4 h-4"
              />
              <span className="text-sm text-gray-600">
                {t('settings.extraction.use_enhanced_description')}
              </span>
            </label>
          </FormField>

          <FormField
            label={t('settings.extraction.max_depth')}
            error={errors.max_depth}
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
            label={t('settings.extraction.max_file_size')}
            error={errors.max_file_size}
          >
            <Input
              type="number"
              value={policy.extraction.max_file_size}
              onChange={(e) =>
                setPolicy({
                  ...policy,
                  extraction: {
                    ...policy.extraction,
                    max_file_size: parseInt(e.target.value) || 104857600,
                  },
                })
              }
            />
            <span className="text-sm text-gray-500">
              {(policy.extraction.max_file_size / 1024 / 1024).toFixed(2)} MB
            </span>
          </FormField>

          <FormField
            label={t('settings.extraction.concurrent_extractions')}
            error={errors.concurrent_extractions}
          >
            <Input
              type="number"
              value={policy.extraction.concurrent_extractions}
              onChange={(e) =>
                setPolicy({
                  ...policy,
                  extraction: {
                    ...policy.extraction,
                    concurrent_extractions: parseInt(e.target.value) || 0,
                  },
                })
              }
              min={0}
            />
            <span className="text-sm text-gray-500">
              {t('settings.extraction.concurrent_auto')}
            </span>
          </FormField>
        </div>
      </Card>

      {/* Security Settings */}
      <Card>
        <h2 className="text-xl font-semibold mb-4">
          {t('settings.security.title')}
        </h2>
        <div className="space-y-4">
          <FormField label={t('settings.security.enable_zip_bomb_detection')}>
            <label className="flex items-center gap-2">
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
                className="w-4 h-4"
              />
              <span className="text-sm text-gray-600">
                {t('settings.security.zip_bomb_description')}
              </span>
            </label>
          </FormField>

          <FormField
            label={t('settings.security.compression_ratio_threshold')}
            error={errors.compression_ratio_threshold}
          >
            <Input
              type="number"
              value={policy.security.compression_ratio_threshold}
              onChange={(e) =>
                setPolicy({
                  ...policy,
                  security: {
                    ...policy.security,
                    compression_ratio_threshold:
                      parseFloat(e.target.value) || 100.0,
                  },
                })
              }
              step={0.1}
            />
          </FormField>
        </div>
      </Card>

      {/* Path Settings */}
      <Card>
        <h2 className="text-xl font-semibold mb-4">
          {t('settings.paths.title')}
        </h2>
        <div className="space-y-4">
          <FormField label={t('settings.paths.enable_long_paths')}>
            <label className="flex items-center gap-2">
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
                className="w-4 h-4"
              />
              <span className="text-sm text-gray-600">
                {t('settings.paths.long_paths_description')}
              </span>
            </label>
          </FormField>

          <FormField
            label={t('settings.paths.shortening_threshold')}
            error={errors.shortening_threshold}
          >
            <Input
              type="number"
              value={policy.paths.shortening_threshold}
              onChange={(e) =>
                setPolicy({
                  ...policy,
                  paths: {
                    ...policy.paths,
                    shortening_threshold: parseFloat(e.target.value) || 0.8,
                  },
                })
              }
              step={0.1}
              min={0}
              max={1}
            />
          </FormField>

          <FormField
            label={t('settings.paths.hash_length')}
            error={errors.hash_length}
          >
            <Input
              type="number"
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
              min={8}
              max={32}
            />
          </FormField>
        </div>
      </Card>

      {/* Performance Settings */}
      <Card>
        <h2 className="text-xl font-semibold mb-4">
          {t('settings.performance.title')}
        </h2>
        <div className="space-y-4">
          <FormField label={t('settings.performance.enable_streaming')}>
            <label className="flex items-center gap-2">
              <input
                type="checkbox"
                checked={policy.performance.enable_streaming}
                onChange={(e) =>
                  setPolicy({
                    ...policy,
                    performance: {
                      ...policy.performance,
                      enable_streaming: e.target.checked,
                    },
                  })
                }
                className="w-4 h-4"
              />
              <span className="text-sm text-gray-600">
                {t('settings.performance.streaming_description')}
              </span>
            </label>
          </FormField>

          <FormField
            label={t('settings.performance.parallel_files_per_archive')}
            error={errors.parallel_files_per_archive}
          >
            <Input
              type="number"
              value={policy.performance.parallel_files_per_archive}
              onChange={(e) =>
                setPolicy({
                  ...policy,
                  performance: {
                    ...policy.performance,
                    parallel_files_per_archive:
                      parseInt(e.target.value) || 4,
                  },
                })
              }
              min={1}
              max={8}
            />
          </FormField>
        </div>
      </Card>
    </div>
  );
}
