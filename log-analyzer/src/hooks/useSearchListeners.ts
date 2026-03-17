/**
 * useSearchListeners — 封装 Tauri 搜索事件监听
 *
 * 将 SearchPage 中 5 个 `listen()` 调用提取到此 hook，
 * 消除组件内的事件配置样板代码，使 SearchPage 专注于渲染逻辑。
 *
 * 生命周期：组件挂载时注册监听器，卸载时自动解除。
 */

import { useEffect } from 'react';
import { logger } from '../utils/logger';
import { listen } from '@tauri-apps/api/event';
import type { LogEntry } from '../types/common';
import type { SearchResultSummary } from '../types/search';

export interface SearchListenerHandlers {
  /** 收到一批流式搜索结果 */
  onResults: (results: LogEntry[]) => void;
  /** 收到搜索汇总统计 */
  onSummary: (summary: SearchResultSummary) => void;
  /** 搜索完成，payload 为总结果数 */
  onComplete: (count: number) => void;
  /** 搜索出错，payload 为错误信息字符串 */
  onError: (errorMsg: string) => void;
  /** 新搜索开始（在后端开始发送结果之前触发） */
  onStart: () => void;
}

/**
 * @param enabled 仅当为 true 时注册监听器（默认 true）
 * @param handlers 各事件的处理函数对象
 */
export function useSearchListeners(
  handlers: SearchListenerHandlers,
  enabled = true,
): void {
  // handlers 对象每次渲染都可能是新引用，用 ref 存储以避免 useEffect 重复执行
  const handlersRef = { current: handlers };
  handlersRef.current = handlers;

  useEffect(() => {
    if (!enabled) return;

    const abortController = new AbortController();
    const unlisteners: Array<() => void> = [];

    const setup = async () => {
      try {
        const [
          resultsUnlisten,
          summaryUnlisten,
          completeUnlisten,
          errorUnlisten,
          startUnlisten,
        ] = await Promise.all([
          listen<LogEntry[]>('search-results', (e) => {
            if (!e.payload || !Array.isArray(e.payload)) return;
            handlersRef.current.onResults(e.payload);
          }),
          listen<SearchResultSummary>('search-summary', (e) => {
            if (!e.payload) return;
            handlersRef.current.onSummary(e.payload);
          }),
          listen<number>('search-complete', (e) => {
            const count = typeof e.payload === 'number' ? e.payload : 0;
            handlersRef.current.onComplete(count);
          }),
          listen<string>('search-error', (e) => {
            handlersRef.current.onError(String(e.payload));
          }),
          listen('search-start', () => {
            handlersRef.current.onStart();
          }),
        ]);

        if (abortController.signal.aborted) {
          [resultsUnlisten, summaryUnlisten, completeUnlisten, errorUnlisten, startUnlisten].forEach(
            (u) => u(),
          );
          return;
        }

        unlisteners.push(
          resultsUnlisten,
          summaryUnlisten,
          completeUnlisten,
          errorUnlisten,
          startUnlisten,
        );
      } catch (err) {
        if (!abortController.signal.aborted) {
          logger.error('useSearchListeners: 注册监听器失败', err);
        }
      }
    };

    setup();

    return () => {
      abortController.abort();
      unlisteners.forEach((unlisten) => {
        try {
          unlisten();
        } catch {
          // 静默处理
        }
      });
    };
  }, [enabled]); // handlers 通过 ref 传递，不加入依赖以避免重复注册
}
