/**
 * 虚拟滚动 Hook
 * 封装 @tanstack/react-virtual 配置，消除 SearchPage 中的 ref 滥用
 */
import { useRef, useCallback } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';

export interface UseVirtualScrollOptions {
  count: number;
  estimateSize?: number;
  overscan?: number;
}

export interface UseVirtualScrollReturn {
  parentRef: React.RefObject<HTMLDivElement | null>;
  virtualItems: ReturnType<ReturnType<typeof useVirtualizer>['getVirtualItems']>;
  totalSize: number;
  measureElement: (element: HTMLDivElement | null) => void;
  scrollToIndex: (index: number) => void;
  scrollToOffset: (offset: number) => void;
}

export function useVirtualScroll({
  count,
  estimateSize = 48,
  overscan = 20,
}: UseVirtualScrollOptions): UseVirtualScrollReturn {
  const parentRef = useRef<HTMLDivElement>(null);

  const virtualizer = useVirtualizer({
    count,
    getScrollElement: () => parentRef.current,
    estimateSize: useCallback(() => estimateSize, [estimateSize]),
    measureElement: useCallback((element: Element | null) => {
      return element?.getBoundingClientRect().height ?? estimateSize;
    }, [estimateSize]),
    overscan,
  });

  const virtualItems = virtualizer.getVirtualItems();
  const totalSize = virtualizer.getTotalSize();

  return {
    parentRef,
    virtualItems,
    totalSize,
    measureElement: virtualizer.measureElement,
    scrollToIndex: virtualizer.scrollToIndex,
    scrollToOffset: virtualizer.scrollToOffset,
  };
}
