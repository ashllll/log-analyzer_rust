import { useCallback } from 'react';
import { useErrorManagement } from './useErrorManagement';

/**
 * 网络错误类型定义
 */
interface NetworkError {
  response?: {
    status: number;
  };
  request?: unknown;
  message?: string;
}

/**
 * 类型守卫：检查是否为网络错误
 */
// FIX(HI-18): 增加更严格的结构检查，避免任何带 request 属性的对象都被视为 NetworkError
const isNetworkError = (error: unknown): error is NetworkError => {
  if (typeof error !== 'object' || error === null) return false;
  const e = error as Record<string, unknown>;

  if ('response' in e) {
    const response = e.response;
    if (typeof response === 'object' && response !== null && 'status' in response) {
      return typeof (response as Record<string, unknown>).status === 'number';
    }
    return false;
  }

  if ('request' in e) {
    return e.request !== null && typeof e.request === 'object';
  }

  return false;
};

/**
 * React Query error handler hook
 * Provides consistent error handling for all queries and mutations
 */
export const useQueryErrorHandler = () => {
  const { handleNetworkError, reportError } = useErrorManagement();

  /**
   * Handle query errors with automatic retry logic
   */
  const handleQueryError = useCallback((error: unknown, context?: {
    queryKey?: string[];
    operation?: string;
    showToUser?: boolean;
  }) => {
    // 错误已传递给 handleNetworkError / reportError 处理，无需重复记录

    // Determine if this is a network error or other type
    if (isNetworkError(error)) {
      return handleNetworkError(error, { operation: context?.operation });
    }

    // Handle other types of errors
    const errorMessage = error instanceof Error ? error.message : String(error);
    return reportError(errorMessage, {
      component: 'ReactQuery',
      userAction: context?.operation || 'query_execution',
      severity: 'medium',
      showToUser: context?.showToUser !== false,
      recoverable: true
    });
  }, [handleNetworkError, reportError]);

  /**
   * Handle mutation errors with context about the operation
   */
  const handleMutationError = useCallback((error: unknown, context?: {
    operation?: string;
    variables?: unknown;
    showToUser?: boolean;
  }) => {
    // 错误已传递给 handleNetworkError / reportError 处理，无需重复记录

    if (isNetworkError(error)) {
      return handleNetworkError(error, { operation: context?.operation });
    }

    const errorMessage = error instanceof Error ? error.message : String(error);
    return reportError(errorMessage, {
      component: 'ReactQuery',
      userAction: context?.operation || 'mutation_execution',
      severity: 'high', // Mutations are usually more critical
      showToUser: context?.showToUser !== false,
      recoverable: true
    });
  }, [handleNetworkError, reportError]);

  /**
   * Create error handler for specific query
   */
  const createQueryErrorHandler = useCallback((queryKey: string[], operation?: string) => {
    return (error: unknown) => handleQueryError(error, { queryKey, operation });
  }, [handleQueryError]);

  /**
   * Create error handler for specific mutation
   */
  const createMutationErrorHandler = useCallback((operation: string) => {
    return (error: unknown, variables?: unknown) => handleMutationError(error, { operation, variables });
  }, [handleMutationError]);

  /**
   * Determine retry logic based on error type
   */
  const shouldRetry = useCallback((error: unknown, retryCount: number): boolean => {
    // Don't retry more than 3 times
    if (retryCount >= 3) {
      return false;
    }

    const networkError = isNetworkError(error) ? error : null;

    // Don't retry client errors (4xx)
    if (networkError?.response?.status && networkError.response.status >= 400 && networkError.response.status < 500) {
      return false;
    }

    // Retry server errors (5xx) and network errors
    if (networkError?.response?.status && networkError.response.status >= 500) {
      return true;
    }

    // Retry if no response (network error)
    if (!networkError?.response) {
      return true;
    }

    return false;
  }, []);

  /**
   * Calculate retry delay with exponential backoff
   */
  const getRetryDelay = useCallback((retryCount: number): number => {
    return Math.min(1000 * Math.pow(2, retryCount), 30000); // Max 30 seconds
  }, []);

  return {
    handleQueryError,
    handleMutationError,
    createQueryErrorHandler,
    createMutationErrorHandler,
    shouldRetry,
    getRetryDelay
  };
};

export default useQueryErrorHandler;