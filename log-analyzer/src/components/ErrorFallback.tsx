import React from 'react';
import { FallbackProps } from 'react-error-boundary';
import { Button } from './ui/Button';

// Tauri API types
declare global {
  interface Window {
    __TAURI__?: {
      invoke: (command: string, args?: any) => Promise<any>;
    };
  }
}

interface ErrorFallbackProps extends FallbackProps {
  title?: string;
  showDetails?: boolean;
}

/**
 * Error fallback component with user-friendly messages
 * Used by react-error-boundary to display errors gracefully
 */
export const ErrorFallback: React.FC<ErrorFallbackProps> = ({
  error,
  resetErrorBoundary,
  title = "Something went wrong",
  showDetails = false
}) => {
  const handleReportError = () => {
    // Report error to backend/Sentry
    if (window.__TAURI__) {
      // Report to backend via Tauri
      window.__TAURI__.invoke('report_frontend_error', {
        error: error.message,
        stack: error.stack,
        timestamp: new Date().toISOString(),
        userAgent: navigator.userAgent,
        url: window.location.href
      }).catch(console.error);
    }
    
    // Also log to console for development
    console.error('Error reported:', error);
  };

  const handleCopyError = async () => {
    const errorInfo = `
Error: ${error.message}
Stack: ${error.stack}
Timestamp: ${new Date().toISOString()}
URL: ${window.location.href}
User Agent: ${navigator.userAgent}
    `.trim();

    try {
      await navigator.clipboard.writeText(errorInfo);
      // Could show a toast here
    } catch (err) {
      console.error('Failed to copy error info:', err);
    }
  };

  return (
    <div className="min-h-[200px] flex items-center justify-center p-6">
      <div className="max-w-md w-full bg-white dark:bg-gray-800 rounded-lg shadow-lg p-6 text-center">
        {/* Error Icon */}
        <div className="mx-auto flex items-center justify-center h-12 w-12 rounded-full bg-red-100 dark:bg-red-900 mb-4">
          <svg
            className="h-6 w-6 text-red-600 dark:text-red-400"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
            aria-hidden="true"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.732 16.5c-.77.833.192 2.5 1.732 2.5z"
            />
          </svg>
        </div>

        {/* Error Title */}
        <h3 className="text-lg font-medium text-gray-900 dark:text-gray-100 mb-2">
          {title}
        </h3>

        {/* Error Message */}
        <p className="text-sm text-gray-500 dark:text-gray-400 mb-4">
          We apologize for the inconvenience. The application encountered an unexpected error.
        </p>

        {/* Error Details (if enabled) */}
        {showDetails && (
          <details className="mb-4 text-left">
            <summary className="cursor-pointer text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              Technical Details
            </summary>
            <div className="bg-gray-50 dark:bg-gray-700 rounded p-3 text-xs font-mono text-gray-600 dark:text-gray-300 overflow-auto max-h-32">
              <div className="mb-2">
                <strong>Error:</strong> {error.message}
              </div>
              {error.stack && (
                <div>
                  <strong>Stack:</strong>
                  <pre className="whitespace-pre-wrap mt-1">{error.stack}</pre>
                </div>
              )}
            </div>
          </details>
        )}

        {/* Action Buttons */}
        <div className="flex flex-col sm:flex-row gap-3 justify-center">
          <Button
            onClick={resetErrorBoundary}
            className="bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded-md text-sm font-medium"
          >
            Try Again
          </Button>
          
          <Button
            onClick={handleReportError}
            variant="outline"
            className="px-4 py-2 rounded-md text-sm font-medium"
          >
            Report Issue
          </Button>
          
          {showDetails && (
            <Button
              onClick={handleCopyError}
              variant="outline"
              className="px-4 py-2 rounded-md text-sm font-medium"
            >
              Copy Details
            </Button>
          )}
        </div>

        {/* Recovery Suggestions */}
        <div className="mt-4 text-xs text-gray-500 dark:text-gray-400">
          <p>You can try:</p>
          <ul className="list-disc list-inside mt-1 space-y-1">
            <li>Refreshing the page</li>
            <li>Clearing your browser cache</li>
            <li>Restarting the application</li>
          </ul>
        </div>
      </div>
    </div>
  );
};

/**
 * Minimal error fallback for critical errors
 */
export const MinimalErrorFallback: React.FC<FallbackProps> = ({
  error,
  resetErrorBoundary
}) => {
  return (
    <div className="min-h-screen flex items-center justify-center bg-gray-50 dark:bg-gray-900">
      <div className="text-center p-6">
        <h1 className="text-2xl font-bold text-gray-900 dark:text-gray-100 mb-4">
          Application Error
        </h1>
        <p className="text-gray-600 dark:text-gray-400 mb-4">
          {error.message}
        </p>
        <Button
          onClick={resetErrorBoundary}
          className="bg-blue-600 hover:bg-blue-700 text-white px-6 py-2 rounded-md"
        >
          Reload Application
        </Button>
      </div>
    </div>
  );
};

export default ErrorFallback;