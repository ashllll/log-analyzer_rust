/**
 * 搜索事件处理 Hook
 * 封装 useSearchListeners 的回调配置，消除 SearchPage 中的 ref 滥用
 */
import { useCallback } from 'react';
import { useSearchListeners } from '../../../hooks/useSearchListeners';
import type { SearchResultSummary } from '../../../types/search';
import { logger } from '../../../utils/logger';

export interface UseSearchEventsOptions {
  currentSearchId: string;
  onStart: () => void;
  onProgress: (count: number) => void;
  onSummary: (summary: SearchResultSummary) => void;
  onComplete: (count: number) => void;
  onError: (errorMsg: string) => void;
  onRefetch: () => Promise<unknown>;
  onScrollToTop: () => void;
}

export function useSearchEvents({
  currentSearchId,
  onStart,
  onProgress,
  onSummary,
  onComplete,
  onError,
  onRefetch,
  onScrollToTop,
}: UseSearchEventsOptions): void {
  const handleProgress = useCallback(
    (count: number) => {
      onProgress(count);
    },
    [onProgress]
  );

  const handleSummary = useCallback(
    (summary: SearchResultSummary) => {
      onSummary(summary);
    },
    [onSummary]
  );

  const handleComplete = useCallback(
    (count: number) => {
      onComplete(count);
      onRefetch().catch((error) => {
        logger.error('Refetch search page after completion failed:', error);
      });
      setTimeout(() => {
        onScrollToTop();
      }, 50);
    },
    [onComplete, onRefetch, onScrollToTop]
  );

  const handleError = useCallback(
    (errorMsg: string) => {
      onError(errorMsg);
    },
    [onError]
  );

  const handleStart = useCallback(() => {
    onStart();
  }, [onStart]);

  const handleCancelled = useCallback(
    (searchId: string) => {
      if (searchId !== currentSearchId) return;
      onError('搜索已取消');
    },
    [currentSearchId, onError]
  );

  useSearchListeners({
    onProgress: handleProgress,
    onSummary: handleSummary,
    onComplete: handleComplete,
    onError: handleError,
    onStart: handleStart,
    onCancelled: handleCancelled,
  });
}
