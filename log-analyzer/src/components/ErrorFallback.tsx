import React from 'react';
import { FallbackProps } from 'react-error-boundary';
import { Button } from './ui/Button';

/**
 * 错误回退组件 - 当组件树中发生错误时显示
 * 
 * 提供用户友好的错误信息和恢复选项
 */
export const ErrorFallback: React.FC<FallbackProps> = ({ error, resetErrorBoundary }) => {
  const [isReporting, setIsReporting] = React.useState(false);
  const [reported, setReported] = React.useState(false);

  // 报告错误到后端/Sentry
  const handleReportError = async () => {
    setIsReporting(true);
    try {
      // TODO: 集成 Sentry 或后端错误报告
      console.error('Error reported:', {
        message: error.message,
        stack: error.stack,
        timestamp: new Date().toISOString(),
      });
      
      // 模拟报告延迟
      await new Promise(resolve => setTimeout(resolve, 500));
      
      setReported(true);
    } catch (err) {
      console.error('Failed to report error:', err);
    } finally {
      setIsReporting(false);
    }
  };

  return (
    <div
      role="alert"
      className="flex flex-col items-center justify-center min-h-[400px] p-8 bg-gray-50 dark:bg-gray-900 rounded-lg"
    >
      <div className="max-w-md w-full space-y-6">
        {/* 错误图标 */}
        <div className="flex justify-center">
          <div className="w-16 h-16 bg-red-100 dark:bg-red-900/30 rounded-full flex items-center justify-center">
            <svg
              className="w-8 h-8 text-red-600 dark:text-red-400"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
              />
            </svg>
          </div>
        </div>

        {/* 错误标题 */}
        <div className="text-center">
          <h2 className="text-2xl font-bold text-gray-900 dark:text-gray-100 mb-2">
            出现了一些问题
          </h2>
          <p className="text-gray-600 dark:text-gray-400">
            应用程序遇到了意外错误。您可以尝试重新加载或报告此问题。
          </p>
        </div>

        {/* 错误详情（开发模式） */}
        {process.env.NODE_ENV === 'development' && (
          <details className="bg-white dark:bg-gray-800 rounded-lg p-4 border border-gray-200 dark:border-gray-700">
            <summary className="cursor-pointer text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              错误详情
            </summary>
            <div className="mt-2 space-y-2">
              <div>
                <p className="text-xs font-semibold text-gray-600 dark:text-gray-400">消息:</p>
                <p className="text-sm text-red-600 dark:text-red-400 font-mono">
                  {error.message}
                </p>
              </div>
              {error.stack && (
                <div>
                  <p className="text-xs font-semibold text-gray-600 dark:text-gray-400">堆栈跟踪:</p>
                  <pre className="text-xs text-gray-700 dark:text-gray-300 overflow-auto max-h-40 bg-gray-50 dark:bg-gray-900 p-2 rounded">
                    {error.stack}
                  </pre>
                </div>
              )}
            </div>
          </details>
        )}

        {/* 操作按钮 */}
        <div className="flex flex-col sm:flex-row gap-3">
          <Button
            onClick={resetErrorBoundary}
            className="flex-1 bg-blue-600 hover:bg-blue-700 text-white"
          >
            <svg
              className="w-4 h-4 mr-2"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
              />
            </svg>
            重试
          </Button>

          <Button
            onClick={handleReportError}
            disabled={isReporting || reported}
            className="flex-1 bg-gray-600 hover:bg-gray-700 text-white disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {isReporting ? (
              <>
                <svg
                  className="animate-spin w-4 h-4 mr-2"
                  fill="none"
                  viewBox="0 0 24 24"
                >
                  <circle
                    className="opacity-25"
                    cx="12"
                    cy="12"
                    r="10"
                    stroke="currentColor"
                    strokeWidth="4"
                  />
                  <path
                    className="opacity-75"
                    fill="currentColor"
                    d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                  />
                </svg>
                报告中...
              </>
            ) : reported ? (
              <>
                <svg
                  className="w-4 h-4 mr-2"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M5 13l4 4L19 7"
                  />
                </svg>
                已报告
              </>
            ) : (
              <>
                <svg
                  className="w-4 h-4 mr-2"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
                  />
                </svg>
                报告问题
              </>
            )}
          </Button>
        </div>

        {/* 帮助文本 */}
        <p className="text-xs text-center text-gray-500 dark:text-gray-500">
          如果问题持续存在，请联系技术支持
        </p>
      </div>
    </div>
  );
};

/**
 * 页面级错误回退组件 - 用于整个页面的错误边界
 */
export const PageErrorFallback: React.FC<FallbackProps> = ({ error, resetErrorBoundary }) => {
  return (
    <div className="min-h-screen flex items-center justify-center bg-gray-50 dark:bg-gray-900">
      <ErrorFallback error={error} resetErrorBoundary={resetErrorBoundary} />
    </div>
  );
};

/**
 * 小型错误回退组件 - 用于组件级别的错误边界
 */
export const CompactErrorFallback: React.FC<FallbackProps> = ({ error, resetErrorBoundary }) => {
  return (
    <div className="p-4 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg">
      <div className="flex items-start gap-3">
        <svg
          className="w-5 h-5 text-red-600 dark:text-red-400 flex-shrink-0 mt-0.5"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
          />
        </svg>
        <div className="flex-1 min-w-0">
          <h3 className="text-sm font-medium text-red-800 dark:text-red-300">
            加载失败
          </h3>
          <p className="text-sm text-red-700 dark:text-red-400 mt-1">
            {error.message}
          </p>
          <button
            onClick={resetErrorBoundary}
            className="mt-2 text-sm font-medium text-red-600 dark:text-red-400 hover:text-red-500 dark:hover:text-red-300"
          >
            重试
          </button>
        </div>
      </div>
    </div>
  );
};
