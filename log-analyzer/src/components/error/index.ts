/**
 * error 模块 — 统一导出
 *
 * 拆分自原 ErrorBoundary.tsx（717 行），现分为：
 * - errorLogManager.ts: 错误日志存储和报告器配置
 * - ErrorCard.tsx: 错误信息展示卡片
 * - ErrorFallbacks.tsx: 错误回退 UI 组件
 * - ErrorBoundaries.tsx: React 错误边界组件
 * - initGlobalErrorHandlers.ts: 全局错误处理器初始化
 */

// 错误日志管理
export {
  configureErrorReporter,
  getErrorLogs,
  clearErrorLogs,
  exportErrorLogs,
  type ErrorReporter,
  type ErrorLogEntry,
  type ErrorReportContext,
} from "./errorLogManager";

// 错误卡片
export { ErrorCard, type ErrorInfoProps } from "./ErrorCard";

// 错误回退组件
export { CompactErrorFallback, PageErrorFallback } from "./ErrorFallbacks";

// 错误边界组件
export { AppErrorBoundary, PageErrorBoundary } from "./ErrorBoundaries";

// 全局错误处理器
export { initGlobalErrorHandlers } from "./initGlobalErrorHandlers";
