/**
 * useSearchListeners — 封装 Tauri 搜索事件监听
 *
 * 将 SearchPage 中的 `listen()` 调用提取到此 hook，
 * 消除组件内的事件配置样板代码，使 SearchPage 专注于渲染逻辑。
 *
 * 新架构（磁盘直写）：
 * - 不再监听 search-results（后端不发送原始数据，改为写磁盘）
 * - 改为监听 search-progress（count），前端据此调整虚拟列表大小
 *
 * 生命周期：组件挂载时注册监听器，卸载时自动解除。
 */

import { useEffect, useRef } from 'react';
import { logger } from '../utils/logger';
import { listen } from '@tauri-apps/api/event';
import type { SearchResultSummary } from '../types/search';
import {
  SearchCompleteEventSchema,
  SearchErrorEventSchema,
  SearchIdEventSchema,
  SearchProgressEventSchema,
  SearchSummaryEventSchema,
} from '../schemas/searchEventSchema';

export interface SearchListenerHandlers {
  /** 收到搜索进度更新（磁盘写入计数），前端据此调整虚拟列表大小 */
  onProgress?: (searchId: string, count: number) => void;
  /** 收到搜索汇总统计 */
  onSummary: (searchId: string, summary: SearchResultSummary) => void;
  /** 搜索完成，payload 为总结果数 */
  onComplete: (searchId: string, count: number) => void;
  /** 搜索出错，payload 为错误信息字符串 */
  onError: (searchId: string, errorMsg: string) => void;
  /** 新搜索开始（在后端开始发送结果之前触发） */
  onStart: (searchId: string) => void;
  /** 搜索取消，payload 为 searchId */
  onCancelled?: (searchId: string) => void;
  /** 搜索超时，payload 为 searchId */
  onTimeout?: (searchId: string) => void;
}

/**
 * @param handlers 各事件的处理函数对象
 * @param enabled 仅当为 true 时注册监听器（默认 true）
 */
export function useSearchListeners(
  handlers: SearchListenerHandlers,
  enabled = true,
): void {
  // handlers 对象每次渲染都可能是新引用，用 useRef 存储以避免 useEffect 重复执行。
  // 不使用 useMemo([handlers])：该写法每次 handlers 变化都重建对象，
  // 导致 useEffect 反复注销/注册所有 Tauri 监听器（内存泄漏+事件丢失风险）。
  const handlersRef = useRef(handlers);
  handlersRef.current = handlers;

  useEffect(() => {
    if (!enabled) return;

    const abortController = new AbortController();
    const unlisteners: Array<() => void> = [];

    const setup = async () => {
      try {
        // FIX(CR-08): 使用 Promise.allSettled 避免短路导致 unlisten 泄漏
        const results = await Promise.allSettled([
          listen<number>('search-progress', (e) => {
            const result = SearchProgressEventSchema.safeParse(e.payload);
            if (!result.success) return;
            handlersRef.current.onProgress?.(result.data.search_id, result.data.count);
          }),
          listen<SearchResultSummary>('search-summary', (e) => {
            const result = SearchSummaryEventSchema.safeParse(e.payload);
            if (!result.success) return;
            handlersRef.current.onSummary(result.data.search_id, result.data.summary);
          }),
          listen<number>('search-complete', (e) => {
            const result = SearchCompleteEventSchema.safeParse(e.payload);
            if (!result.success) return;
            handlersRef.current.onComplete(result.data.search_id, result.data.total_count);
          }),
          listen<string>('search-error', (e) => {
            const result = SearchErrorEventSchema.safeParse(e.payload);
            if (!result.success) return;
            handlersRef.current.onError(result.data.search_id, result.data.error);
          }),
          listen('search-start', (e) => {
            const result = SearchIdEventSchema.safeParse(e.payload);
            if (!result.success) return;
            handlersRef.current.onStart(result.data.search_id);
          }),
          listen('search-cancelled', (e) => {
            const result = SearchIdEventSchema.safeParse(e.payload);
            if (!result.success) return;
            handlersRef.current.onCancelled?.(result.data.search_id);
          }),
          listen('search-timeout', (e) => {
            const result = SearchIdEventSchema.safeParse(e.payload);
            if (!result.success) return;
            handlersRef.current.onTimeout?.(result.data.search_id);
          }),
        ]);

        const eventNames = [
          'search-progress',
          'search-summary',
          'search-complete',
          'search-error',
          'search-start',
          'search-cancelled',
          'search-timeout',
        ];

        const successfulUnlisteners: Array<() => void> = [];
        results.forEach((result, index) => {
          if (result.status === 'fulfilled') {
            successfulUnlisteners.push(result.value);
          } else if (!abortController.signal.aborted) {
            logger.error(`useSearchListeners: 注册 ${eventNames[index]} 监听器失败`, result.reason);
          }
        });

        if (abortController.signal.aborted) {
          successfulUnlisteners.forEach((u) => u());
          return;
        }

        unlisteners.push(...successfulUnlisteners);
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
  }, [enabled]); // 仅依赖 enabled；handlers 通过 handlersRef.current 访问最新值，不放入依赖数组
}
