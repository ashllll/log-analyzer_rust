/**
 * 全局错误处理组件（向后兼容层）
 *
 * 本文件为原 ErrorBoundary.tsx 的向后兼容入口。
 * 实现已拆分至 error/ 子模块：
 * - error/errorLogManager.ts: 错误日志存储和报告器配置
 * - error/ErrorCard.tsx: 错误信息展示卡片
 * - error/ErrorFallbacks.tsx: 错误回退 UI 组件
 * - error/ErrorBoundaries.tsx: React 错误边界组件
 * - error/initGlobalErrorHandlers.ts: 全局错误处理器初始化
 *
 * 新代码请直接从 error/ 子模块导入。
 */

// 向后兼容：从 error/ 模块重新导出所有公开 API
export {
  configureErrorReporter,
  getErrorLogs,
  clearErrorLogs,
  exportErrorLogs,
  ErrorCard,
  CompactErrorFallback,
  PageErrorFallback,
  AppErrorBoundary,
  PageErrorBoundary,
  initGlobalErrorHandlers,
  type ErrorReporter,
  type ErrorLogEntry,
  type ErrorReportContext,
  type ErrorInfoProps,
} from "./error";
