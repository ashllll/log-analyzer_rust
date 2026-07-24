/**
 * ErrorFallbacks — 错误回退 UI 组件
 *
 * 职责：
 * - CompactErrorFallback: 嵌套错误边界的紧凑回退
 * - PageErrorFallback: 页面级错误的完整回退
 */

import { AlertTriangle, X } from "lucide-react";
import {
  createApiError,
  ErrorCode,
  isRetryableError,
} from "../../services/errors";
import { ErrorCard } from "./ErrorCard";

// ============================================================================
// CompactErrorFallback
// ============================================================================

interface CompactErrorFallbackProps {
  error: unknown;
  resetErrorBoundary: () => void;
}

export function CompactErrorFallback({
  error,
  resetErrorBoundary,
}: CompactErrorFallbackProps) {
  // 安全处理未知错误类型
  const errorObj = error instanceof Error ? error : new Error(String(error));
  const apiError = createApiError("component", errorObj);

  return (
    <div className="flex items-center justify-center min-h-screen bg-bg-main p-6">
      <div className="w-full max-w-md bg-bg-card border border-status-error/50 rounded-lg p-6">
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-2">
            <AlertTriangle className="w-6 h-6 text-status-error" />
            <h2 className="text-lg font-bold text-text-main">出错了</h2>
          </div>
          <button
            onClick={resetErrorBoundary}
            className="text-text-muted hover:text-text-main transition-colors"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        <p className="text-text-main mb-4">{apiError.getUserMessage()}</p>

        {apiError.help && (
          <p className="text-sm text-text-dim mb-4">提示：{apiError.help}</p>
        )}

        <div className="flex gap-2">
          {isRetryableError(apiError) && (
            <button
              onClick={resetErrorBoundary}
              className="px-3 py-1.5 bg-primary text-white rounded hover:bg-primary/90 transition-colors"
            >
              重试
            </button>
          )}
          <button
            onClick={() => window.location.reload()}
            className="px-3 py-1.5 bg-bg-base border border-border-base text-text-main rounded hover:bg-bg-sidebar transition-colors"
          >
            刷新页面
          </button>
        </div>
      </div>
    </div>
  );
}

// ============================================================================
// PageErrorFallback
// ============================================================================

/**
 * 页面级错误回退组件（用于 react-error-boundary）
 *
 * 用于整个页面的错误边界，提供完整的错误信息显示和恢复选项
 */
export function PageErrorFallback({
  error,
  resetErrorBoundary,
  onGoHome,
}: {
  error: unknown;
  resetErrorBoundary: () => void;
  onGoHome?: () => void;
}) {
  // 安全处理未知错误类型
  const errorObj = error instanceof Error ? error : new Error(String(error));
  const apiError = createApiError("page", errorObj);

  return (
    <div className="flex items-center justify-center min-h-screen bg-bg-main p-6">
      <ErrorCard
        title={errorObj.name || "页面错误"}
        message={apiError.getUserMessage()}
        details={apiError.details as string}
        stack={errorObj.stack}
        code={apiError.code as ErrorCode}
        canRetry={isRetryableError(apiError)}
        onRetry={resetErrorBoundary}
        onGoHome={
          onGoHome ??
          (() => {
            resetErrorBoundary();
            window.location.hash = "/workspaces";
          })
        }
      />
    </div>
  );
}
