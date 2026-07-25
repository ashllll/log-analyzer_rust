import { useState, useCallback } from "react";
import { useTranslation } from "react-i18next";

export interface ExtractionPolicy {
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

export const defaultPolicy: ExtractionPolicy = {
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

export function useExtractionPolicy() {
  const { t } = useTranslation();
  const [policy, setPolicy] = useState<ExtractionPolicy>(defaultPolicy);
  const [policyErrors, setPolicyErrors] = useState<Record<string, string>>({});

  const validatePolicy = useCallback((): boolean => {
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
  }, [policy, t]);

  const resetPolicy = useCallback(() => {
    setPolicy(defaultPolicy);
    setPolicyErrors({});
  }, []);

  return {
    policy,
    setPolicy,
    policyErrors,
    setPolicyErrors,
    validatePolicy,
    resetPolicy,
    defaultPolicy,
  };
}
