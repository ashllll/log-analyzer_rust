/**
 * 搜索事件处理 Hook
 * 封装 useSearchListeners 的回调配置，消除 SearchPage 中的 ref 滥用
 *
 * P1 修复：后端 search_logs 响应与 search-* 事件走不同 IPC 通道，无顺序保证。
 * 快速搜索时事件可能在 currentSearchId 提交到 state 之前到达并被 isCurrentSearch
 * 全部丢弃（isSearching 永久卡死）。现在改为缓冲 + 重放：
 * - searchId 与当前 id 匹配 → 立即分发
 * - 当前 id 为空（搜索已启动、id 未提交）→ 进入缓冲区
 * - 当前 id 非空且不匹配 → 旧搜索的过期事件，丢弃
 * currentSearchId 一旦提交，重放缓冲区中匹配该 id 的事件。
 */
import { useCallback, useEffect, useRef } from "react";
import { useSearchListeners } from "../../../hooks/useSearchListeners";
import type { SearchResultSummary } from "../../../types/search";
import { logger } from "../../../utils/logger";

export interface UseSearchEventsOptions {
  currentSearchId: string;
  onStart: () => void;
  onProgress: (count: number) => void;
  onSummary: (summary: SearchResultSummary) => void;
  onComplete: (count: number) => void;
  onError: (errorMsg: string) => void;
  /** 搜索被取消或超时（正常终止，不是错误——不得弹错误 toast） */
  onCancelled: () => void;
  onRefetch: () => Promise<unknown>;
  onScrollToTop: () => void;
}

type BufferedSearchEvent =
  | { kind: "start"; searchId: string }
  | { kind: "progress"; searchId: string; count: number }
  | { kind: "summary"; searchId: string; summary: SearchResultSummary }
  | { kind: "complete"; searchId: string; count: number }
  | { kind: "error"; searchId: string; errorMsg: string }
  | { kind: "cancelled"; searchId: string }
  | { kind: "timeout"; searchId: string };

type SearchEventWithoutId =
  | { kind: "start" }
  | { kind: "progress"; count: number }
  | { kind: "summary"; summary: SearchResultSummary }
  | { kind: "complete"; count: number }
  | { kind: "error"; errorMsg: string }
  | { kind: "cancelled" }
  | { kind: "timeout" };

const MAX_BUFFERED_EVENTS = 200;

export function useSearchEvents({
  currentSearchId,
  onStart,
  onProgress,
  onSummary,
  onComplete,
  onError,
  onCancelled,
  onRefetch,
  onScrollToTop,
}: UseSearchEventsOptions): void {
  const isCurrentSearch = useCallback(
    (searchId: string) => searchId === currentSearchId,
    [currentSearchId]
  );

  const bufferedEventsRef = useRef<BufferedSearchEvent[]>([]);

  // 将缓冲事件分发到具体回调
  const dispatchBuffered = useCallback(
    (event: BufferedSearchEvent) => {
      switch (event.kind) {
        case "start":
          onStart();
          break;
        case "progress":
          onProgress(event.count);
          break;
        case "summary":
          onSummary(event.summary);
          break;
        case "complete":
          onComplete(event.count);
          onRefetch().catch((error) => {
            logger.error("Refetch search page after completion failed:", error);
          });
          setTimeout(() => {
            onScrollToTop();
          }, 50);
          break;
        case "error":
          onError(event.errorMsg);
          break;
        case "cancelled":
        case "timeout":
          // P3 修复：取消/超时是正常终止，不得弹错误 toast；
          // 仅复位搜索状态。
          onCancelled();
          break;
      }
    },
    [
      onStart,
      onProgress,
      onSummary,
      onComplete,
      onError,
      onCancelled,
      onRefetch,
      onScrollToTop,
    ]
  );

  // 路由：当前 → 立即分发；pending（空 id）→ 缓冲；过期 → 丢弃
  const routeEvent = useCallback(
    (searchId: string, event: SearchEventWithoutId) => {
      if (isCurrentSearch(searchId)) {
        dispatchBuffered({ ...event, searchId } as BufferedSearchEvent);
        return;
      }
      if (currentSearchId === "") {
        const buffer = bufferedEventsRef.current;
        if (buffer.length < MAX_BUFFERED_EVENTS) {
          buffer.push({ ...event, searchId } as BufferedSearchEvent);
        }
      }
      // 其余情况：旧搜索的过期事件，丢弃
    },
    [isCurrentSearch, currentSearchId, dispatchBuffered]
  );

  // currentSearchId 提交后，重放缓冲区中匹配的事件
  useEffect(() => {
    if (!currentSearchId) return;
    const buffer = bufferedEventsRef.current;
    bufferedEventsRef.current = [];
    for (const event of buffer) {
      if (event.searchId === currentSearchId) {
        dispatchBuffered(event);
      }
    }
  }, [currentSearchId, dispatchBuffered]);

  const handleProgress = useCallback(
    (searchId: string, count: number) => {
      routeEvent(searchId, { kind: "progress", count });
    },
    [routeEvent]
  );

  const handleSummary = useCallback(
    (searchId: string, summary: SearchResultSummary) => {
      routeEvent(searchId, { kind: "summary", summary });
    },
    [routeEvent]
  );

  const handleComplete = useCallback(
    (searchId: string, count: number) => {
      routeEvent(searchId, { kind: "complete", count });
    },
    [routeEvent]
  );

  const handleError = useCallback(
    (searchId: string, errorMsg: string) => {
      routeEvent(searchId, { kind: "error", errorMsg });
    },
    [routeEvent]
  );

  const handleStart = useCallback(
    (searchId: string) => {
      routeEvent(searchId, { kind: "start" });
    },
    [routeEvent]
  );

  const handleCancelled = useCallback(
    (searchId: string) => {
      routeEvent(searchId, { kind: "cancelled" });
    },
    [routeEvent]
  );

  const handleTimeout = useCallback(
    (searchId: string) => {
      routeEvent(searchId, { kind: "timeout" });
    },
    [routeEvent]
  );

  useSearchListeners({
    onProgress: handleProgress,
    onSummary: handleSummary,
    onComplete: handleComplete,
    onError: handleError,
    onStart: handleStart,
    onCancelled: handleCancelled,
    onTimeout: handleTimeout,
  });
}
