import React from 'react';
import { ErrorBoundary } from 'react-error-boundary';
import { ErrorFallback, MinimalErrorFallback } from './ErrorFallback';

// Tauri API types
declare global {
  interface Window {
    __TAURI__?: {
      invoke: (command: string, args?: any) => Promise<any>;
    };
  }
}

interface ErrorBoundaryWrapperProps {
  children: React.ReactNode;
  fallback?: React.ComponentType<any>;
  onError?: (error: Error, errorInfo: React.ErrorInfo) => void;
  resetKeys?: Array<string | number | boolean | null | undefined>;
  level?: 'page' | 'component' | 'critical';
}

/**
 * Wrapper component for react-error-boundary with different error handling levels
 */
export const ErrorBoundaryWrapper: React.FC<ErrorBoundaryWrapperProps> = ({
  children,
  fallback,
  onError,
  resetKeys,
  level = 'component'
}) => {
  const handleError = (error: Error, errorInfo: React.ErrorInfo) => {
    // Log error to console
    console.error('Error caught by boundary:', error, errorInfo);

    // Report to backend if available
    if (window.__TAURI__) {
      window.__TAURI__.invoke('report_frontend_error', {
        error: error.message,
        stack: error.stack,
        timestamp: new Date().toISOString(),
        userAgent: navigator.userAgent,
        url: window.location.href,
        component: errorInfo.componentStack?.split('\n')[1]?.trim(),
        user_action: 'component_error'
      }).catch(console.error);
    }

    // Call custom error handler if provided
    if (onError) {
      onError(error, errorInfo);
    }
  };

  const getFallbackComponent = () => {
    if (fallback) {
      return fallback;
    }

    switch (level) {
      case 'critical':
        return MinimalErrorFallback;
      case 'page':
        return (props: any) => <ErrorFallback {...props} showDetails={true} />;
      case 'component':
      default:
        return ErrorFallback;
    }
  };

  return (
    <ErrorBoundary
      FallbackComponent={getFallbackComponent()}
      onError={handleError}
      resetKeys={resetKeys}
    >
      {children}
    </ErrorBoundary>
  );
};

/**
 * Page-level error boundary for wrapping entire pages
 */
export const PageErrorBoundary: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  return (
    <ErrorBoundaryWrapper level="page">
      {children}
    </ErrorBoundaryWrapper>
  );
};

/**
 * Component-level error boundary for wrapping individual components
 */
export const ComponentErrorBoundary: React.FC<{ 
  children: React.ReactNode;
  componentName?: string;
}> = ({ children, componentName }) => {
  const handleError = (error: Error, errorInfo: React.ErrorInfo) => {
    console.error(`Error in ${componentName || 'component'}:`, error, errorInfo);
  };

  return (
    <ErrorBoundaryWrapper 
      level="component" 
      onError={handleError}
    >
      {children}
    </ErrorBoundaryWrapper>
  );
};

/**
 * Critical error boundary for the entire application
 */
export const CriticalErrorBoundary: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  return (
    <ErrorBoundaryWrapper 
      level="critical"
    >
      {children}
    </ErrorBoundaryWrapper>
  );
};

export default ErrorBoundaryWrapper;