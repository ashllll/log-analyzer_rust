import { useEffect, useCallback, useRef } from 'react';
import { useAppStore } from '../stores/appStore';
import { useConfigMutation } from './useServerQueries';
import { logger } from '../utils/logger';

/**
 * Hook for managing configuration with debounced saving using React patterns
 */
export const useConfigManager = () => {
  const keywordGroups = useAppStore((state) => state.keywordGroups);
  const workspaces = useAppStore((state) => state.workspaces);
  const configMutation = useConfigMutation();
  
  // Track last saved fingerprint to avoid duplicate saves
  const lastFingerprintRef = useRef<string>('');
  const saveTimeoutRef = useRef<NodeJS.Timeout | undefined>(undefined);

  // Stable save function using useCallback
  const saveConfig = useCallback(() => {
    // Skip saving if no data
    if (keywordGroups.length === 0 && workspaces.length === 0) {
      return;
    }

    // Generate config fingerprint to avoid unnecessary saves
    const configFingerprint = JSON.stringify({
      keywords: keywordGroups.map(g => ({ id: g.id, enabled: g.enabled })),
      workspaces: workspaces.map(w => ({ id: w.id, status: w.status }))
    });

    // Skip if config hasn't changed
    if (configFingerprint === lastFingerprintRef.current) {
      logger.debug('[CONFIG_MANAGER] Configuration unchanged, skipping save');
      return;
    }

    lastFingerprintRef.current = configFingerprint;

    // Save using React Query mutation
    configMutation.mutate({
      keyword_groups: keywordGroups,
      workspaces: workspaces
    });

    logger.debug('[CONFIG_MANAGER] Configuration saved with fingerprint:', configFingerprint);
  }, [keywordGroups, workspaces, configMutation]);

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
    isLoading: configMutation.isPending,
    error: configMutation.error,
    lastSaved: configMutation.isSuccess ? new Date() : null,
  };
};