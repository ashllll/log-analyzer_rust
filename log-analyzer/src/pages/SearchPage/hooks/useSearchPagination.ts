/**
 * 搜索分页加载 Hook
 * 封装范围检测和自动分页加载逻辑，消除 SearchPage 中的 ref 滥用
 */
import { useEffect, useRef } from 'react';
import { logger } from '../../../utils/logger';

export interface UseSearchPaginationOptions {
  firstVisibleIndex: number;
  lastVisibleIndex: number;
  firstPageOffset: number;
  loadedCount: number;
  hasNextPage: boolean;
  hasPreviousPage: boolean;
  isFetchingNextPage: boolean;
  isFetchingPreviousPage: boolean;
  fetchNextPage: () => Promise<unknown>;
  fetchPreviousPage: () => Promise<unknown>;
  threshold?: number;
  debounceMs?: number;
}

export function useSearchPagination({
  firstVisibleIndex,
  lastVisibleIndex,
  firstPageOffset,
  loadedCount,
  hasNextPage,
  hasPreviousPage,
  isFetchingNextPage,
  isFetchingPreviousPage,
  fetchNextPage,
  fetchPreviousPage,
  threshold = 50,
  debounceMs = 500,
}: UseSearchPaginationOptions): void {
  const lastFetchTimeRef = useRef(0);
  const fetchNextPageRef = useRef(fetchNextPage);
  const fetchPreviousPageRef = useRef(fetchPreviousPage);

  fetchNextPageRef.current = fetchNextPage;
  fetchPreviousPageRef.current = fetchPreviousPage;

  useEffect(() => {
    if (lastVisibleIndex < 0) return;

    const loadedEndIndex = firstPageOffset + loadedCount;
    const now = Date.now();

    if (now - lastFetchTimeRef.current < debounceMs) return;

    // 向前加载
    if (lastVisibleIndex >= loadedEndIndex - threshold && hasNextPage && !isFetchingNextPage) {
      lastFetchTimeRef.current = now;
      fetchNextPageRef.current().catch((err: unknown) => {
        logger.error('Range-based fetchNextPage failed:', err);
      });
      return;
    }

    // 向后加载
    if (firstVisibleIndex <= firstPageOffset + threshold && hasPreviousPage && !isFetchingPreviousPage) {
      lastFetchTimeRef.current = now;
      fetchPreviousPageRef.current().catch((err: unknown) => {
        logger.error('Range-based fetchPreviousPage failed:', err);
      });
    }
  }, [
    lastVisibleIndex,
    firstVisibleIndex,
    firstPageOffset,
    loadedCount,
    hasNextPage,
    isFetchingNextPage,
    hasPreviousPage,
    isFetchingPreviousPage,
    threshold,
    debounceMs,
  ]);
}
