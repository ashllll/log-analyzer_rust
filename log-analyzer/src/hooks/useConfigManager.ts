import { useEffect, useCallback, useMemo, useRef } from 'react';
import { useShallow } from 'zustand/shallow';
import { useWorkspaceStore } from '../stores/workspaceStore';
import { useKeywordStore } from '../stores/keywordStore';
import { useAppStore } from '../stores/appStore';
import { useConfigMutation } from './useServerQueries';
import { logger } from '../utils/logger';
import type { KeywordGroup } from '../types/common';

/**
 * Compute a fingerprint that captures the full content of keyword groups and workspaces.
 * Must include name, color, patterns (not just id/enabled) so edits trigger persistence.
 */
export const computeConfigFingerprint = (
  keywordGroups: KeywordGroup[],
  workspaces: {
    id: string;
    name?: string;
    path?: string;
    status: string;
    size?: string;
    files?: number;
    watching?: boolean;
    ready_files?: number;
  }[],
): string => {
  return JSON.stringify({
    keywords: keywordGroups.map((g) => ({
      id: g.id,
      name: g.name,
      color: g.color,
      enabled: g.enabled,
      patterns: g.patterns.map((p) => p.regex),
    })),
    workspaces: workspaces.map((w) => ({
      id: w.id,
      name: w.name,
      path: w.path,
      status: w.status,
      size: w.size,
      files: w.files,
      watching: w.watching,
      ready_files: w.ready_files,
    })),
  });
};

/**
 * Hook for managing configuration with debounced saving using React patterns
 */
export const useConfigManager = () => {
  const keywordGroups = useKeywordStore(useShallow((state) => state.keywordGroups));
  const workspaces = useWorkspaceStore(useShallow((state) => state.workspaces));
  const initPhase = useAppStore((state) => state.initPhase);
  const { mutate: saveConfigMutate, isPending, error, isSuccess } = useConfigMutation();

  // Track last saved fingerprint to avoid duplicate saves
  const lastFingerprintRef = useRef<string>('');
  const saveTimeoutRef = useRef<NodeJS.Timeout | undefined>(undefined);

  // Use ref for mutate to avoid unstable configMutation reference in deps
  const mutateRef = useRef(saveConfigMutate);
  mutateRef.current = saveConfigMutate;
  const configFingerprint = useMemo(
    () => computeConfigFingerprint(keywordGroups, workspaces),
    [keywordGroups, workspaces]
  );

  // Stable save function — only depends on data, not on mutation object
  const saveConfig = useCallback(() => {
    // Skip saving if no data
    if (keywordGroups.length === 0 && workspaces.length === 0) {
      return;
    }

    // Skip if config hasn't changed
    if (configFingerprint === lastFingerprintRef.current) {
      logger.debug('[CONFIG_MANAGER] Configuration unchanged, skipping save');
      return;
    }

    lastFingerprintRef.current = configFingerprint;

    // Save using React Query mutation via stable ref
    mutateRef.current({
      keyword_groups: keywordGroups,
      workspaces: workspaces
    });

    logger.debug('[CONFIG_MANAGER] Configuration saved with fingerprint:', configFingerprint);
  }, [configFingerprint, keywordGroups, workspaces]);

  // Watch for changes and trigger debounced save
  // 仅在初始化完成后才启用自动保存，避免加载阶段的无意义请求
  // configFingerprint 仅在首次设置基线时读取，无需加入 deps
  useEffect(() => {
    if (initPhase !== 'ready') {
      return;
    }

    // 设置基线指纹，避免初始化后对已加载配置做无意义保存
    if (!lastFingerprintRef.current) {
      lastFingerprintRef.current = configFingerprint;
      return;
    }

    // Clear existing timeout
    if (saveTimeoutRef.current) {
      clearTimeout(saveTimeoutRef.current);
    }

    // Debounce save by 1000ms
    saveTimeoutRef.current = setTimeout(() => {
      saveConfig();
    }, 1000);

    // Cleanup on unmount
    return () => {
      if (saveTimeoutRef.current) {
        clearTimeout(saveTimeoutRef.current);
      }
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [saveConfig, initPhase]);

  return {
    isLoading: isPending,
    error: error,
    lastSaved: isSuccess ? new Date() : null,
  };
};
