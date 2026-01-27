import { useState } from 'react';
import { useTranslation } from 'react-i18next';

// 采用最稳健的物理隔离导入，彻底杜绝索引文件导致的循环依赖
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
  const { showToast } = useToastManager();
  const [policy, setPolicy] = useState<ExtractionPolicy>(defaultPolicy);
  const [loading, setLoading] = useState(false);
  const [errors, setErrors] = useState<Record<string, string>>({});

  const validatePolicy = (): boolean => {
    const newErrors: Record<string, string> = {};
    if (policy.extraction.max_depth < 1 || policy.extraction.max_depth > 20) {
      newErrors.max_depth = t('settings.errors.max_depth_range');
    }
    if (policy.extraction.max_file_size <= 0) {
      newErrors.max_file_size = t('settings.errors.must_be_positive');
    }
    if (policy.paths.shortening_threshold <= 0 || policy.paths.shortening_threshold > 1.0) {
      newErrors.shortening_threshold = t('settings.errors.threshold_range');
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
      showToast('success', t('settings.save_success'));
    } catch (error) {
      showToast('error', t('settings.save_error', { error: String(error) }));
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
    <div className="p-6 space-y-6 overflow-y-auto h-full">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold text-text-main">{t('settings.title')}</h1>
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
      <Card className="p-6">
        <h2 className="text-xl font-semibold mb-4 text-text-main">
          {t('settings.extraction.title')}
        </h2>
        <div className="space-y-4">
          <FormField label={t('settings.extraction.use_enhanced')} error={errors.use_enhanced_extraction}>
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

          <FormField label={t('settings.extraction.max_depth')} error={errors.max_depth}>
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

          <FormField label={t('settings.extraction.max_file_size')} error={errors.max_file_size}>
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

          <FormField label={t('settings.paths.shortening_threshold')} error={errors.shortening_threshold}>
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
  );
}

export default SettingsPage;
