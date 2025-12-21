import { useCallback } from 'react';
import { useErrorManagement } from './useErrorManagement';

/**
 * React Query error handler hook
 * Provides consistent error handling for all queries and mutations
 */
export const useQueryErrorHandler = () => {
  const { handleNetworkError, reportError } = useErrorManagement();

  /**
   * Handle query errors with automatic retry logic
   */
  const handleQueryError = useCallback((error: any, context?: { 
    queryKey?: string[];
    operation?: string;
    showToUser?: boolean;
  }) => {
    // Log the error for debugging
    console.error('Query error:', error, context);

    // Determine if this is a network error or other type
    if (error?.response || error?.request) {
      return handleNetworkError(error, { operation: context?.operation });
    }

    // Handle other types of errors
    return reportError(error, {
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
  const handleMutationError = useCallback((error: any, context?: {
    operation?: string;
    variables?: any;
    showToUser?: boolean;
  }) => {
    console.error('Mutation error:', error, context);

    if (error?.response || error?.request) {
      return handleNetworkError(error, { operation: context?.operation });
    }

    return reportError(error, {
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
    return (error: any) => handleQueryError(error, { queryKey, operation });
  }, [handleQueryError]);

  /**
   * Create error handler for specific mutation
   */
  const createMutationErrorHandler = useCallback((operation: string) => {
    return (error: any, variables?: any) => handleMutationError(error, { operation, variables });
  }, [handleMutationError]);

  /**
   * Determine retry logic based on error type
   */
  const shouldRetry = useCallback((error: any, retryCount: number): boolean => {
    // Don't retry more than 3 times
    if (retryCount >= 3) {
      return false;
    }

    // Don't retry client errors (4xx)
    if (error?.response?.status >= 400 && error?.response?.status < 500) {
      return false;
    }

    // Retry server errors (5xx) and network errors
    if (error?.response?.status >= 500 || !error?.response) {
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