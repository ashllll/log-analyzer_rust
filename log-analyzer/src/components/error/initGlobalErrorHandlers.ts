/**
 * 全局错误处理器初始化
 *
 * 职责：
 * - 注册 unhandledrejection 和 error 事件处理器
 * - 错误日志持久化
 * - Toast 通知（生产环境）
 * - 错误去重（防止重复显示）
 */

import toast from "react-hot-toast";
import {
  addErrorLog,
  reportError,
  type ErrorLogEntry,
} from "./errorLogManager";

/**
 * 初始化全局错误处理器
 *
 * 注册以下错误处理器：
 * - unhandledrejection: Promise rejection
 * - error: 全局错误事件
 *
 * @returns 清理函数，用于移除事件监听器
 */
export function initGlobalErrorHandlers(): (() => void) | undefined {
  // 用于防止短时间内重复显示相同错误
  const recentErrors = new Map<string, number>();
  const ERROR_DEBOUNCE_MS = 5000; // 5秒内相同错误不重复显示
  const timeoutIds: ReturnType<typeof setTimeout>[] = [];

  // 获取错误签名（用于去重）
  const getErrorSignature = (error: unknown): string => {
    if (error instanceof Error) {
      return `${error.name}:${error.message}`;
    }
    return String(error);
  };

  // Promise rejection 处理
  const handleUnhandledRejection = (event: PromiseRejectionEvent) => {
    const reason = event.reason;

    const errorLog: ErrorLogEntry = {
      timestamp: new Date().toISOString(),
      message: reason instanceof Error ? reason.message : String(reason),
      stack: reason instanceof Error ? reason.stack : undefined,
      type: "unhandled_rejection",
    };

    addErrorLog(errorLog);
    event.preventDefault();

    // Toast 通知（生产环境）
    if (import.meta.env.PROD) {
      const signature = getErrorSignature(reason);
      const lastShown = recentErrors.get(signature);
      const now = Date.now();

      if (!lastShown || now - lastShown > ERROR_DEBOUNCE_MS) {
        recentErrors.set(signature, now);
        const timeoutId = setTimeout(
          () => recentErrors.delete(signature),
          ERROR_DEBOUNCE_MS
        );
        timeoutIds.push(timeoutId);

        toast.error("操作失败，请稍后重试", { duration: 4000 });
      }
    }

    reportError(reason, { source: "unhandled_rejection" });
  };

  // 全局错误处理
  const handleError = (event: ErrorEvent) => {
    const error = event.error;

    const errorLog: ErrorLogEntry = {
      timestamp: new Date().toISOString(),
      message: error instanceof Error ? error.message : String(error),
      stack: error instanceof Error ? error.stack : undefined,
      type: "global_error",
    };

    addErrorLog(errorLog);

    // Toast 通知（生产环境）
    if (import.meta.env.PROD) {
      const signature = getErrorSignature(error);
      const lastShown = recentErrors.get(signature);
      const now = Date.now();

      if (!lastShown || now - lastShown > ERROR_DEBOUNCE_MS) {
        recentErrors.set(signature, now);
        const timeoutId = setTimeout(
          () => recentErrors.delete(signature),
          ERROR_DEBOUNCE_MS
        );
        timeoutIds.push(timeoutId);

        toast.error("应用程序发生错误", { duration: 4000 });
      }
    }

    reportError(error, { source: "global_error" });
  };

  window.addEventListener("unhandledrejection", handleUnhandledRejection);
  window.addEventListener("error", handleError);

  // 清理函数
  return () => {
    window.removeEventListener("unhandledrejection", handleUnhandledRejection);
    window.removeEventListener("error", handleError);
    timeoutIds.forEach(clearTimeout);
    timeoutIds.length = 0;
    recentErrors.clear();
  };
}
