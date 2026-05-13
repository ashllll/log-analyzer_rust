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
  const isCurrentSearch = useCallback(
    (searchId: string) => searchId === currentSearchId,
    [currentSearchId]
  );

  const handleProgress = useCallback(
    (searchId: string, count: number) => {
      if (!isCurrentSearch(searchId)) return;
      onProgress(count);
    },
    [isCurrentSearch, onProgress]
  );

  const handleSummary = useCallback(
    (searchId: string, summary: SearchResultSummary) => {
      if (!isCurrentSearch(searchId)) return;
      onSummary(summary);
    },
    [isCurrentSearch, onSummary]
  );

  const handleComplete = useCallback(
    (searchId: string, count: number) => {
      if (!isCurrentSearch(searchId)) return;
      onComplete(count);
      onRefetch().catch((error) => {
        logger.error('Refetch search page after completion failed:', error);
      });
      setTimeout(() => {
        onScrollToTop();
      }, 50);
    },
    [isCurrentSearch, onComplete, onRefetch, onScrollToTop]
  );

  const handleError = useCallback(
    (searchId: string, errorMsg: string) => {
      if (!isCurrentSearch(searchId)) return;
      onError(errorMsg);
    },
    [isCurrentSearch, onError]
  );

  const handleStart = useCallback((searchId: string) => {
    if (!isCurrentSearch(searchId)) return;
    onStart();
  }, [isCurrentSearch, onStart]);

  const handleCancelled = useCallback(
    (searchId: string) => {
      if (!isCurrentSearch(searchId)) return;
      onError('搜索已取消');
    },
    [isCurrentSearch, onError]
  );

  useSearchListeners({
    onProgress: handleProgress,
    onSummary: handleSummary,
    onComplete: handleComplete,
    onError: handleError,
    onStart: handleStart,
    onCancelled: handleCancelled,
    onTimeout: handleCancelled,
  });
}
