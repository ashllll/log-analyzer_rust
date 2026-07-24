/**
 * ErrorBoundaries — React 错误边界组件
 *
 * 职责：
 * - AppErrorBoundary: 应用级错误边界，捕获整个组件树的错误
 * - PageErrorBoundary: 页面级错误边界，捕获单个页面的错误
 */

import React, { Component, ErrorInfo } from "react";
import {
  createApiError,
  ErrorCode,
  isRetryableError,
} from "../../services/errors";
import { ErrorCard, type ErrorInfoProps } from "./ErrorCard";
import { CompactErrorFallback } from "./ErrorFallbacks";
import {
  addErrorLog,
  reportError,
  type ErrorLogEntry,
} from "./errorLogManager";

// ============================================================================
// AppErrorBoundary — 应用级错误边界
// ============================================================================

interface AppErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
  errorInfo: ErrorInfo | null;
}

interface AppErrorBoundaryProps {
  children: React.ReactNode;
  onNavigateHome?: () => void;
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
      errorInfo: null,
    };
  }

  static getDerivedStateFromError(error: Error): AppErrorBoundaryState {
    return {
      hasError: true,
      error,
      errorInfo: null,
    };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    this.logErrorToService(error, errorInfo);
  }

  /**
   * 记录错误到错误追踪服务
   */
  private logErrorToService(error: Error, errorInfo: ErrorInfo) {
    const errorLog: ErrorLogEntry = {
      timestamp: new Date().toISOString(),
      message: error.message,
      stack: error.stack || undefined,
      componentStack: errorInfo.componentStack || undefined,
      type: "error",
    };

    addErrorLog(errorLog);

    if (import.meta.env.DEV) {
      import("../../utils/logger")
        .then(({ logger }) => {
          logger.error("[ErrorBoundary]", JSON.stringify(errorLog, null, 2));
        })
        .catch(() => {
          /* logger 加载失败时静默 */
        });
    }

    reportError(error, {
      source: "error",
      componentStack: errorInfo.componentStack || undefined,
    });
  }

  private handleReset = () => {
    this.setState({
      hasError: false,
      error: null,
      errorInfo: null,
    });
  };

  private handleRetry = () => {
    this.handleReset();
    window.location.reload();
  };

  render() {
    if (this.state.hasError) {
      const { error } = this.state;

      let apiError = createApiError("component", error);
      let displayError: ErrorInfoProps;

      if (error instanceof Error) {
        displayError = {
          title: "应用程序错误",
          message: apiError.getUserMessage(),
          details: apiError.details as string,
          stack: error.stack,
          code: apiError.code as ErrorCode,
          canRetry: isRetryableError(apiError),
          onRetry: this.handleRetry,
          onGoHome: () => {
            this.handleReset();
            if (this.props.onNavigateHome) {
              this.props.onNavigateHome();
            } else {
              window.location.hash = "/workspaces";
            }
          },
        };
      } else {
        displayError = {
          title: "未知错误",
          message: "发生了未知错误",
          details: String(error),
          canRetry: false,
          onRetry: this.handleRetry,
          onGoHome: () => {
            this.handleReset();
            if (this.props.onNavigateHome) {
              this.props.onNavigateHome();
            } else {
              window.location.hash = "/workspaces";
            }
          },
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

// ============================================================================
// PageErrorBoundary — 页面级错误边界
// ============================================================================

interface PageErrorBoundaryProps {
  children: React.ReactNode;
  fallback?: React.ComponentType<{
    error: Error;
    resetErrorBoundary: () => void;
  }>;
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

  private handleReset = () => {
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
      return (
        <CompactErrorFallback
          error={this.state.error!}
          resetErrorBoundary={this.handleReset}
        />
      );
    }

    return this.props.children;
  }
}
