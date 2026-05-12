import { useEffect, useCallback, useRef } from 'react';
import { useShallow } from 'zustand/shallow';
import { useWorkspaceStore } from '../stores/workspaceStore';
import { useKeywordStore } from '../stores/keywordStore';
import { useAppStore } from '../stores/appStore';
import { useConfigMutation } from './useServerQueries';
import { api } from '../services/api';
import { logger } from '../utils/logger';
import type { KeywordGroup } from '../types/common';

/**
 * Compute a fingerprint that captures the full content of keyword groups and workspaces.
 * Must include name, color, patterns (not just id/enabled) so edits trigger persistence.
 */
export const computeConfigFingerprint = (
  keywordGroups: KeywordGroup[],
  workspaces: { id: string; status: string }[],
): string => {
  return JSON.stringify({
    keywords: keywordGroups.map((g) => ({
      id: g.id,
      name: g.name,
      color: g.color,
      enabled: g.enabled,
      patterns: g.patterns.map((p) => p.regex),
    })),
    workspaces: workspaces.map((w) => ({ id: w.id, status: w.status })),
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

  // Stable save function — only depends on data, not on mutation object
  const saveConfig = useCallback(() => {
    // Skip saving if no data
    if (keywordGroups.length === 0 && workspaces.length === 0) {
      return;
    }

    // Generate config fingerprint to avoid unnecessary saves
    const configFingerprint = computeConfigFingerprint(keywordGroups, workspaces);

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
  }, [keywordGroups, workspaces]);

  // Watch for changes and trigger debounced save
  // 仅在初始化完成后才启用自动保存，避免加载阶段的无意义请求
  useEffect(() => {
    if (initPhase !== 'ready') {
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
  }, [saveConfig, initPhase]);

  // 配置一致性校验：保存成功后重新加载并比对指纹
  useEffect(() => {
    if (!isSuccess || initPhase !== 'ready') {
      return;
    }

    const verifyConsistency = async () => {
      try {
        const diskConfig = await api.loadConfig();
        const diskFingerprint = computeConfigFingerprint(
          diskConfig.keyword_groups as KeywordGroup[],
          diskConfig.workspaces as { id: string; status: string }[]
        );
        const memoryFingerprint = computeConfigFingerprint(keywordGroups, workspaces);

        if (diskFingerprint !== memoryFingerprint) {
          logger.warn(
            { diskFingerprint, memoryFingerprint },
            '[CONFIG_MANAGER] 配置一致性校验失败：内存与磁盘数据不匹配'
          );
        } else {
          logger.debug('[CONFIG_MANAGER] 配置一致性校验通过');
        }
      } catch (error) {
        logger.error({ error }, '[CONFIG_MANAGER] 配置一致性校验出错');
      }
    };

    // 延迟校验，确保后端写入完成
    const verifyTimer = setTimeout(verifyConsistency, 500);
    return () => clearTimeout(verifyTimer);
  }, [isSuccess, initPhase, keywordGroups, workspaces]);

  return {
    isLoading: isPending,
    error: error,
    lastSaved: isSuccess ? new Date() : null,
  };
};