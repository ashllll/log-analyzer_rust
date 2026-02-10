/**
 * 全局错误处理组件
 *
 * 提供统一的错误捕获和显示机制
 * 包括：
 * - React Error Boundary
 * - 全局未捕获错误处理
 * - Promise rejection 处理
 * - Tauri 命令错误处理
 * - 错误日志持久化
 * - Toast 通知
 */

import React, { Component, ErrorInfo } from 'react';
import { AlertTriangle, X, RefreshCw, Home, Bug } from 'lucide-react';
import { createApiError, ErrorCode, isRetryableError } from '../services/errors';

// ============================================================================
// 类型定义
// ============================================================================

interface ErrorInfoProps {
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
// 错误日志存储
// ============================================================================

/**
 * 错误日志条目
 */
interface ErrorLogEntry {
  timestamp: string;
  message: string;
  stack?: string;
  componentStack?: string;
  type: 'error' | 'unhandled_rejection' | 'global_error';
}

/**
 * 错误日志存储配置
 */
const ERROR_LOG_CONFIG = {
  /** 存储键名 */
  STORAGE_KEY: 'app_error_logs',
  /** 最大保存条目数 */
  MAX_ENTRIES: 100,
  /** 日志保留天数 */
  RETENTION_DAYS: 7,
};

/**
 * 获取所有错误日志
 */
function getErrorLogs(): ErrorLogEntry[] {
  try {
    const stored = localStorage.getItem(ERROR_LOG_CONFIG.STORAGE_KEY);
    if (stored) {
      const logs = JSON.parse(stored) as ErrorLogEntry[];
      // 过滤过期日志
      const cutoffDate = new Date();
      cutoffDate.setDate(cutoffDate.getDate() - ERROR_LOG_CONFIG.RETENTION_DAYS);
      return logs.filter(log => new Date(log.timestamp) > cutoffDate);
    }
  } catch (e) {
    console.warn('[ErrorBoundary] Failed to read error logs:', e);
  }
  return [];
}

/**
 * 添加错误日志
 */
function addErrorLog(entry: ErrorLogEntry): void {
  try {
    const logs = getErrorLogs();
    logs.unshift(entry); // 添加到开头
    // 限制条目数量
    const trimmedLogs = logs.slice(0, ERROR_LOG_CONFIG.MAX_ENTRIES);
    localStorage.setItem(ERROR_LOG_CONFIG.STORAGE_KEY, JSON.stringify(trimmedLogs));
  } catch (e) {
    console.warn('[ErrorBoundary] Failed to write error log:', e);
  }
}

/**
 * 清除所有错误日志
 */
export function clearErrorLogs(): void {
  try {
    localStorage.removeItem(ERROR_LOG_CONFIG.STORAGE_KEY);
  } catch (e) {
    console.warn('[ErrorBoundary] Failed to clear error logs:', e);
  }
}

/**
 * 导出错误日志（用于报告问题）
 */
export function exportErrorLogs(): string {
  const logs = getErrorLogs();
  return JSON.stringify(logs, null, 2);
}

// ============================================================================
// 错误显示组件
// ============================================================================

/**
 * 错误信息卡片
 */
function ErrorCard({ title, message, details, stack, code, canRetry, onRetry, onGoHome, onReport }: ErrorInfoProps) {
  return (
    <div className="w-full max-w-2xl mx-auto p-6">
      <div className="bg-red-500/10 border border-red-500/50 rounded-lg p-6">
        {/* 标题 */}
        <div className="flex items-center gap-3 mb-4">
          <AlertTriangle className="w-8 h-8 text-red-500 flex-shrink-0" />
          <h2 className="text-xl font-bold text-red-500">{title}</h2>
        </div>

        {/* 错误消息 */}
        <p className="text-text-main mb-4">{message}</p>

        {/* 错误代码 */}
        {code && (
          <div className="mb-4">
            <span className="text-xs text-text-dim uppercase font-bold">错误代码：</span>
            <span className="ml-2 px-2 py-1 bg-red-500/20 text-red-400 rounded text-sm font-mono">
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
              className="px-4 py-2 bg-blue-500/20 text-blue-400 border border-blue-500/50 rounded-lg hover:bg-blue-500/30 transition-colors flex items-center gap-2"
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

// ============================================================================
// 错误边界组件
// ============================================================================

/**
 * 应用级错误边界
 *
 * 捕获组件树中的所有错误，防止整个应用崩溃
 */
interface AppErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
  errorInfo: ErrorInfo | null;
}

interface AppErrorBoundaryProps {
  children: React.ReactNode;
}

export class AppErrorBoundary extends Component<
  AppErrorBoundaryProps,
  AppErrorBoundaryState
> {
  constructor(props: AppErrorBoundaryProps) {
    super(props);
    this.state = {
      hasError: false,
      error: null,
      errorInfo: null
    };
  }

  static getDerivedStateFromError(error: Error): AppErrorBoundaryState {
    return {
      hasError: true,
      error,
      errorInfo: null
    };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error('ErrorBoundary caught an error:', error);
    console.error('ErrorInfo:', errorInfo);

    // 记录错误到错误追踪服务（如果可用）
    this.logErrorToService(error, errorInfo);
  }

  /**
   * 记录错误到错误追踪服务
   *
   * 实现功能：
   * - 错误日志持久化到 localStorage
   * - 控制台输出（开发环境）
   * - 可选：集成远程错误追踪服务（如 Sentry）
   */
  logErrorToService(error: Error, errorInfo: ErrorInfo) {
    // 创建错误日志条目
    const errorLog: ErrorLogEntry = {
      timestamp: new Date().toISOString(),
      message: error.message,
      stack: error.stack || undefined,
      componentStack: errorInfo.componentStack || undefined,
      type: 'error',
    };

    // 持久化到 localStorage
    addErrorLog(errorLog);

    // 控制台输出
    if (process.env.NODE_ENV === 'development') {
      console.error('[ErrorBoundary]', JSON.stringify(errorLog, null, 2));
    }

    // TODO: 集成远程错误追踪服务（如 Sentry）
    // 生产环境可以考虑：
    // if (process.env.NODE_ENV === 'production') {
    //   Sentry.captureException(error, { extra: { errorInfo } });
    // }
  }

  handleReset = () => {
    this.setState({
      hasError: false,
      error: null,
      errorInfo: null
    });
  };

  handleRetry = () => {
    this.handleReset();
    window.location.reload();
  };

  render() {
    if (this.state.hasError) {
      const { error } = this.state;

      // 尝试将错误转换为 ApiError 以获取更多信息
      let apiError = createApiError('component', error);
      let displayError: ErrorInfoProps;

      if (error instanceof Error) {
        displayError = {
          title: '应用程序错误',
          message: apiError.getUserMessage(),
          details: apiError.details as string,
          stack: error.stack,
          code: apiError.code as ErrorCode,
          canRetry: isRetryableError(apiError),
          onRetry: this.handleRetry,
          onGoHome: () => {
            this.handleReset();
            // 导航到首页
            window.location.hash = '/workspaces';
          }
        };
      } else {
        displayError = {
          title: '未知错误',
          message: '发生了未知错误',
          details: String(error),
          canRetry: false,
          onRetry: this.handleRetry,
          onGoHome: () => {
            this.handleReset();
            window.location.hash = '/workspaces';
          }
        };
      }

      return (
        <div className="flex items-center justify-center min-h-screen bg-bg-main p-6">
          <ErrorCard {...displayError} />
        </div>
      );
    }

    return this.props.children;
  }
}

/**
 * 紧凑错误边界回退组件
 *
 * 用于嵌套的错误边界，显示更紧凑的错误界面
 */
interface CompactErrorFallbackProps {
  error: unknown;
  resetErrorBoundary: () => void;
}

export function CompactErrorFallback({ error, resetErrorBoundary }: CompactErrorFallbackProps) {
  // 安全处理未知错误类型
  const errorObj = error instanceof Error ? error : new Error(String(error));
  const apiError = createApiError('component', errorObj);

  return (
    <div className="flex items-center justify-center min-h-screen bg-bg-main p-6">
      <div className="w-full max-w-md bg-bg-card border border-red-500/50 rounded-lg p-6">
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-2">
            <AlertTriangle className="w-6 h-6 text-red-500" />
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

/**
 * 页面级错误边界
 *
 * 用于单个页面的错误捕获
 */
interface PageErrorBoundaryProps {
  children: React.ReactNode;
  fallback?: React.ComponentType<{ error: Error; resetErrorBoundary: () => void }>;
  onError?: (error: Error, errorInfo: ErrorInfo) => void;
}

interface PageErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
}

export class PageErrorBoundary extends Component<
  PageErrorBoundaryProps,
  PageErrorBoundaryState
> {
  constructor(props: PageErrorBoundaryProps) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): PageErrorBoundaryState {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    this.props.onError?.(error, errorInfo);
  }

  handleReset = () => {
    this.setState({ hasError: false, error: null });
  };

  render() {
    if (this.state.hasError) {
      if (this.props.fallback) {
        return (
          <this.props.fallback
            error={this.state.error!}
            resetErrorBoundary={this.handleReset}
          />
        );
      }
      return <CompactErrorFallback error={this.state.error!} resetErrorBoundary={this.handleReset} />;
    }

    return this.props.children;
  }
}

/**
 * 页面级错误回退组件（用于 react-error-boundary）
 *
 * 用于整个页面的错误边界，提供完整的错误信息显示和恢复选项
 */
export function PageErrorFallback({ error, resetErrorBoundary }: { error: unknown; resetErrorBoundary: () => void }) {
  // 安全处理未知错误类型
  const errorObj = error instanceof Error ? error : new Error(String(error));
  const apiError = createApiError('page', errorObj);

  return (
    <div className="flex items-center justify-center min-h-screen bg-bg-main p-6">
      <ErrorCard
        title={errorObj.name || '页面错误'}
        message={apiError.getUserMessage()}
        details={apiError.details as string}
        stack={errorObj.stack}
        code={apiError.code as ErrorCode}
        canRetry={isRetryableError(apiError)}
        onRetry={resetErrorBoundary}
        onGoHome={() => {
          resetErrorBoundary();
          window.location.hash = '/workspaces';
        }}
      />
    </div>
  );
}

// ============================================================================
// 全局错误处理器初始化
// ============================================================================

/**
 * 初始化全局错误处理器
 *
 * 注册以下错误处理器：
 * - unhandledrejection: Promise rejection
 * - error: 全局错误事件
 *
 * 功能：
 * - 错误日志持久化到 localStorage
 * - Toast 通知用户（仅生产环境）
 * - 控制台输出（开发环境）
 * - 防止重复错误泛滥
 */
export function initGlobalErrorHandlers(): (() => void) | undefined {
  // 用于防止短时间内重复显示相同错误
  const recentErrors = new Map<string, number>();
  const ERROR_DEBOUNCE_MS = 5000; // 5秒内相同错误不重复显示

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

    // 创建日志条目
    const errorLog: ErrorLogEntry = {
      timestamp: new Date().toISOString(),
      message: reason instanceof Error ? reason.message : String(reason),
      stack: reason instanceof Error ? reason.stack : undefined,
      type: 'unhandled_rejection',
    };

    // 持久化日志
    addErrorLog(errorLog);

    // 控制台输出
    console.error('[GlobalError] Unhandled Promise rejection:', reason);

    // 阻止默认的控制台错误
    event.preventDefault();

    // Toast 通知（生产环境）
    if (process.env.NODE_ENV === 'production') {
      const signature = getErrorSignature(reason);
      const lastShown = recentErrors.get(signature);
      const now = Date.now();

      if (!lastShown || now - lastShown > ERROR_DEBOUNCE_MS) {
        recentErrors.set(signature, now);
        // 延迟清理
        setTimeout(() => recentErrors.delete(signature), ERROR_DEBOUNCE_MS);

        // 使用 react-hot-toast 显示错误
        try {
          const toast = require('react-hot-toast');
          toast.error('操作失败，请稍后重试', { duration: 4000 });
        } catch (e) {
          console.warn('[GlobalError] Failed to show toast:', e);
        }
      }
    }

    // TODO: 集成远程错误追踪服务（如 Sentry）
    // if (process.env.NODE_ENV === 'production') {
    //   Sentry.captureException(reason);
    // }
  };

  // 全局错误处理
  const handleError = (event: ErrorEvent) => {
    const error = event.error;

    // 创建日志条目
    const errorLog: ErrorLogEntry = {
      timestamp: new Date().toISOString(),
      message: error instanceof Error ? error.message : String(error),
      stack: error instanceof Error ? error.stack : undefined,
      type: 'global_error',
    };

    // 持久化日志
    addErrorLog(errorLog);

    // 控制台输出
    console.error('[GlobalError] Global error:', error);

    // Toast 通知（生产环境）
    if (process.env.NODE_ENV === 'production') {
      const signature = getErrorSignature(error);
      const lastShown = recentErrors.get(signature);
      const now = Date.now();

      if (!lastShown || now - lastShown > ERROR_DEBOUNCE_MS) {
        recentErrors.set(signature, now);
        setTimeout(() => recentErrors.delete(signature), ERROR_DEBOUNCE_MS);

        try {
          const toast = require('react-hot-toast');
          toast.error('应用程序发生错误', { duration: 4000 });
        } catch (e) {
          console.warn('[GlobalError] Failed to show toast:', e);
        }
      }
    }

    // TODO: 集成远程错误追踪服务（如 Sentry）
    // if (process.env.NODE_ENV === 'production') {
    //   Sentry.captureException(error);
    // }
  };

  window.addEventListener('unhandledrejection', handleUnhandledRejection);
  window.addEventListener('error', handleError);

  console.log('[GlobalError] Global error handlers initialized');

  // 清理函数
  return () => {
    window.removeEventListener('unhandledrejection', handleUnhandledRejection);
    window.removeEventListener('error', handleError);
  };
}

// ============================================================================
// 导出
// ============================================================================

export type { ErrorInfoProps };

/**
 * 获取所有错误日志（用于错误报告功能）
 */
export { getErrorLogs };
