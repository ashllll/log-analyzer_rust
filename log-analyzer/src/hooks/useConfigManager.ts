import { useEffect, useCallback, useRef } from 'react';
import { useWorkspaceStore } from '../stores/workspaceStore';
import { useKeywordStore } from '../stores/keywordStore';
import { useConfigMutation } from './useServerQueries';
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
  const keywordGroups = useKeywordStore((state) => state.keywordGroups);
  const workspaces = useWorkspaceStore((state) => state.workspaces);
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
  useEffect(() => {
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
  }, [saveConfig]);

  return {
    isLoading: isPending,
    error: error,
    lastSaved: isSuccess ? new Date() : null,
  };
};