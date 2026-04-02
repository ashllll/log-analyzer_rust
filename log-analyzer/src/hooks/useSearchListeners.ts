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

export interface SearchListenerHandlers {
  /** 收到搜索进度更新（磁盘写入计数），前端据此调整虚拟列表大小 */
  onProgress?: (count: number) => void;
  /** 收到搜索汇总统计 */
  onSummary: (summary: SearchResultSummary) => void;
  /** 搜索完成，payload 为总结果数 */
  onComplete: (count: number) => void;
  /** 搜索出错，payload 为错误信息字符串 */
  onError: (errorMsg: string) => void;
  /** 新搜索开始（在后端开始发送结果之前触发） */
  onStart: () => void;
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
        const [
          progressUnlisten,
          summaryUnlisten,
          completeUnlisten,
          errorUnlisten,
          startUnlisten,
          cancelledUnlisten,
          timeoutUnlisten,
        ] = await Promise.all([
          listen<number>('search-progress', (e) => {
            const count = typeof e.payload === 'number' ? e.payload : 0;
            handlersRef.current.onProgress?.(count);
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
          listen<string>('search-cancelled', (e) => {
            handlersRef.current.onCancelled?.(String(e.payload));
          }),
          listen<string>('search-timeout', (e) => {
            handlersRef.current.onTimeout?.(String(e.payload));
          }),
        ]);

        if (abortController.signal.aborted) {
          [
            progressUnlisten,
            summaryUnlisten,
            completeUnlisten,
            errorUnlisten,
            startUnlisten,
            cancelledUnlisten,
            timeoutUnlisten,
          ].forEach(
            (u) => u(),
          );
          return;
        }

        unlisteners.push(
          progressUnlisten,
          summaryUnlisten,
          completeUnlisten,
          errorUnlisten,
          startUnlisten,
          cancelledUnlisten,
          timeoutUnlisten,
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
  }, [enabled]); // 仅依赖 enabled；handlers 通过 handlersRef.current 访问最新值，不放入依赖数组
}
