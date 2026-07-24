/**
 * ErrorCard — 错误信息展示卡片
 *
 * 纯展示组件，接收错误信息并渲染。
 */

import { AlertTriangle, RefreshCw, Home, Bug } from "lucide-react";
import type { ErrorCode } from "../../services/errors";

// ============================================================================
// 类型定义
// ============================================================================

export interface ErrorInfoProps {
  title: string;
  message: string;
  details?: string;
  stack?: string;
  code?: ErrorCode;
  canRetry?: boolean;
  onRetry?: () => void;
  onGoHome?: () => void;
  onReport?: () => void;
}

// ============================================================================
// 组件
// ============================================================================

export function ErrorCard({
  title,
  message,
  details,
  stack,
  code,
  canRetry,
  onRetry,
  onGoHome,
  onReport,
}: ErrorInfoProps) {
  return (
    <div className="w-full max-w-2xl mx-auto p-6">
      <div className="bg-status-error/10 border border-status-error/50 rounded-lg p-6">
        {/* 标题 */}
        <div className="flex items-center gap-3 mb-4">
          <AlertTriangle className="w-8 h-8 text-status-error flex-shrink-0" />
          <h2 className="text-xl font-bold text-status-error">{title}</h2>
        </div>

        {/* 错误消息 */}
        <p className="text-text-main mb-4">{message}</p>

        {/* 错误代码 */}
        {code && (
          <div className="mb-4">
            <span className="text-xs text-text-dim uppercase font-bold">
              错误代码：
            </span>
            <span className="ml-2 px-2 py-1 bg-log-error/20 text-log-error rounded text-sm font-mono">
              {code}
            </span>
          </div>
        )}

        {/* 详细信息 */}
        {details && (
          <div className="mb-4">
            <details className="group">
              <summary className="text-sm text-text-dim cursor-pointer hover:text-text-main mb-2">
                查看详细信息
              </summary>
              <div className="mt-2 p-3 bg-bg-base border border-border-base rounded text-sm font-mono text-text-dim">
                {details}
              </div>
            </details>
          </div>
        )}

        {/* 堆栈跟踪 */}
        {stack && (
          <div className="mb-4">
            <details className="group">
              <summary className="text-sm text-text-dim cursor-pointer hover:text-text-main mb-2">
                查看堆栈跟踪
              </summary>
              <pre className="mt-2 p-3 bg-bg-base border border-border-base rounded text-xs overflow-auto max-h-40 text-text-dim">
                {stack}
              </pre>
            </details>
          </div>
        )}

        {/* 操作按钮 */}
        <div className="flex flex-wrap gap-3">
          {canRetry && onRetry && (
            <button
              onClick={onRetry}
              className="px-4 py-2 bg-primary text-white rounded-lg hover:bg-primary/90 transition-colors flex items-center gap-2"
            >
              <RefreshCw className="w-4 h-4" />
              重试
            </button>
          )}
          {onGoHome && (
            <button
              onClick={onGoHome}
              className="px-4 py-2 bg-bg-card border border-border-base text-text-main rounded-lg hover:bg-bg-base transition-colors flex items-center gap-2"
            >
              <Home className="w-4 h-4" />
              返回首页
            </button>
          )}
          {onReport && (
            <button
              onClick={onReport}
              className="px-4 py-2 bg-log-info/20 text-log-info border border-log-info/50 rounded-lg hover:bg-log-info/30 transition-colors flex items-center gap-2"
            >
              <Bug className="w-4 h-4" />
              报告问题
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
