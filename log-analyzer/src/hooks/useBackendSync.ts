import { useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useToast } from './useToast';

/**
 * 后端状态同步 Hook
 * 在应用挂载时调用 init_state_sync 命令，建立前后端状态同步通道
 */
export function useBackendSync() {
  const { showToast } = useToast();

  useEffect(() => {
    let isMounted = true;

    const setupStateSync = async () => {
      try {
        await invoke('init_state_sync');
        if (!isMounted) return;

        if (import.meta.env.DEV) {
          console.log('[StateSync] State sync initialized');
        }
      } catch (error) {
        if (!isMounted) return;
        if (import.meta.env.DEV) {
          console.error('[StateSync] Failed to initialize:', error);
        }
        showToast('error', 'Failed to initialize state sync');
      }
    };

    setupStateSync();

    return () => {
      isMounted = false;
    };
  }, [showToast]);
}
