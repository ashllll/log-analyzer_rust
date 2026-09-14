import { useEffect, useCallback, useMemo, useRef } from "react";
import { useShallow } from "zustand/shallow";
import { useWorkspaceStore } from "../stores/workspaceStore";
import { useKeywordStore } from "../stores/keywordStore";
import { useAppStore } from "../stores/appStore";
import { useConfigMutation } from "./useServerQueries";
import { logger } from "../utils/logger";
import {
  computeConfigFingerprint,
  hasPersistableConfig,
} from "../services/configSync";

export { computeConfigFingerprint } from "../services/configSync";

/**
 * Hook for managing configuration with debounced saving using React patterns
 */
export const useConfigManager = () => {
  const keywordGroups = useKeywordStore(
    useShallow((state) => state.keywordGroups)
  );
  const workspaces = useWorkspaceStore(useShallow((state) => state.workspaces));
  const initPhase = useAppStore((state) => state.initPhase);
  const {
    mutate: saveConfigMutate,
    isPending,
    error,
    isSuccess,
  } = useConfigMutation();

  // Track last saved fingerprint to avoid duplicate saves
  const lastFingerprintRef = useRef<string>("");
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
    if (!hasPersistableConfig({ keyword_groups: keywordGroups, workspaces })) {
      return;
    }

    // Skip if config hasn't changed
    if (configFingerprint === lastFingerprintRef.current) {
      logger.debug("[CONFIG_MANAGER] Configuration unchanged, skipping save");
      return;
    }

    // P3 修复：指纹只在保存成功后标记。此前在发起 mutate 之前就写入，
    // 保存失败时指纹已被标记，后续不再重试 → 配置静默丢失。
    mutateRef.current(
      {
        keyword_groups: keywordGroups,
        workspaces: workspaces,
      },
      {
        onSuccess: () => {
          lastFingerprintRef.current = configFingerprint;
          logger.debug(
            "[CONFIG_MANAGER] Configuration saved with fingerprint:",
            configFingerprint
          );
        },
        onError: (saveError: unknown) => {
          // 指纹保持旧值：下次触发保存时会重试，而非静默丢弃。
          logger.error(
            "[CONFIG_MANAGER] Configuration save failed, will retry on next change:",
            saveError
          );
        },
      }
    );
  }, [configFingerprint, keywordGroups, workspaces]);

  // Watch for changes and trigger debounced save
  // 仅在初始化完成后才启用自动保存，避免加载阶段的无意义请求
  useEffect(() => {
    if (initPhase !== "ready") {
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
  }, [saveConfig, initPhase, configFingerprint]);

  return {
    isLoading: isPending,
    error: error,
    lastSaved: isSuccess ? new Date() : null,
  };
};
