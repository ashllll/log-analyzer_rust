import { useCallback, useState } from 'react';
import { useToastManager } from './useToastManager';

// Tauri API types
declare global {
  interface Window {
    __TAURI__?: {
      invoke: (command: string, args?: any) => Promise<any>;
    };
  }
}

export interface ErrorInfo {
  id: string;
  message: string;
  type: 'validation' | 'network' | 'system' | 'user';
  severity: 'low' | 'medium' | 'high' | 'critical';
  timestamp: Date;
  context?: Record<string, any>;
  stack?: string;
  recoverable: boolean;
}

export interface ValidationError {
  field: string;
  message: string;
  code?: string;
}

export interface FormErrors {
  [field: string]: string | string[];
}

/**
 * Comprehensive error management hook
 * Provides centralized error handling, reporting, and user feedback
 */
export const useErrorManagement = () => {
  const [errors, setErrors] = useState<ErrorInfo[]>([]);
  const [formErrors, setFormErrors] = useState<FormErrors>({});
  const { showError, showInfo } = useToastManager();

  /**
   * Report an error to the backend and optionally to the user
   */
  const reportError = useCallback(async (
    error: Error | string,
    context?: {
      component?: string;
      userAction?: string;
      severity?: ErrorInfo['severity'];
      showToUser?: boolean;
      recoverable?: boolean;
    }
  ) => {
    const errorMessage = typeof error === 'string' ? error : error.message;
    const errorStack = typeof error === 'string' ? undefined : error.stack;
    
    const errorInfo: ErrorInfo = {
      id: `error_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
      message: errorMessage,
      type: 'system',
      severity: context?.severity || 'medium',
      timestamp: new Date(),
      context: context ? { ...context } : undefined,
      stack: errorStack,
      recoverable: context?.recoverable ?? true
    };

    // Add to local error list
    setErrors(prev => [...prev.slice(-9), errorInfo]); // Keep last 10 errors

    // Report to backend
    try {
      if (window.__TAURI__) {
        await window.__TAURI__.invoke('report_frontend_error', {
          error: errorMessage,
          stack: errorStack,
          timestamp: errorInfo.timestamp.toISOString(),
          userAgent: navigator.userAgent,
          url: window.location.href,
          component: context?.component,
          user_action: context?.userAction
        });
      }
    } catch (reportingError) {
      console.error('Failed to report error to backend:', reportingError);
    }

    // Show to user if requested
    if (context?.showToUser !== false) {
      const duration = errorInfo.severity === 'critical' ? 0 : 5000; // Critical errors don't auto-dismiss
      
      if (errorInfo.severity === 'critical' || errorInfo.severity === 'high') {
        showError(errorMessage, duration);
      } else {
        showInfo(errorMessage, duration);
      }
    }

    return errorInfo;
  }, [showError, showInfo]);

  /**
   * Handle validation errors for forms
   */
  const setValidationErrors = useCallback((errors: ValidationError[] | FormErrors) => {
    if (Array.isArray(errors)) {
      // Convert ValidationError[] to FormErrors
      const formErrorsObj: FormErrors = {};
      errors.forEach(error => {
        if (formErrorsObj[error.field]) {
          // If field already has errors, make it an array
          if (Array.isArray(formErrorsObj[error.field])) {
            (formErrorsObj[error.field] as string[]).push(error.message);
          } else {
            formErrorsObj[error.field] = [formErrorsObj[error.field] as string, error.message];
          }
        } else {
          formErrorsObj[error.field] = error.message;
        }
      });
      setFormErrors(formErrorsObj);
    } else {
      setFormErrors(errors);
    }
  }, []);

  /**
   * Clear validation errors for specific fields or all fields
   */
  const clearValidationErrors = useCallback((fields?: string[]) => {
    if (fields) {
      setFormErrors(prev => {
        const newErrors = { ...prev };
        fields.forEach(field => {
          delete newErrors[field];
        });
        return newErrors;
      });
    } else {
      setFormErrors({});
    }
  }, []);

  /**
   * Get validation error for a specific field
   */
  const getFieldError = useCallback((field: string): string | string[] | undefined => {
    return formErrors[field];
  }, [formErrors]);

  /**
   * Check if a field has validation errors
   */
  const hasFieldError = useCallback((field: string): boolean => {
    return !!formErrors[field];
  }, [formErrors]);

  /**
   * Handle network errors (for react-query integration)
   */
  const handleNetworkError = useCallback((error: any, context?: { operation?: string }) => {
    let errorMessage = 'Network error occurred';
    let severity: ErrorInfo['severity'] = 'medium';

    if (error?.response?.status) {
      switch (error.response.status) {
        case 400:
          errorMessage = 'Invalid request';
          severity = 'low';
          break;
        case 401:
          errorMessage = 'Authentication required';
          severity = 'high';
          break;
        case 403:
          errorMessage = 'Access denied';
          severity = 'high';
          break;
        case 404:
          errorMessage = 'Resource not found';
          severity = 'low';
          break;
        case 500:
          errorMessage = 'Server error';
          severity = 'high';
          break;
        default:
          errorMessage = `Request failed (${error.response.status})`;
      }
    } else if (error?.message) {
      errorMessage = error.message;
    }

    return reportError(errorMessage, {
      component: 'NetworkLayer',
      userAction: context?.operation || 'network_request',
      severity,
      showToUser: true,
      recoverable: true
    });
  }, [reportError]);

  /**
   * Clear all errors
   */
  const clearErrors = useCallback(() => {
    setErrors([]);
    setFormErrors({});
  }, []);

  /**
   * Get error statistics
   */
  const getErrorStats = useCallback(() => {
    const now = Date.now();
    const last24h = errors.filter(e => now - e.timestamp.getTime() < 24 * 60 * 60 * 1000);
    const critical = errors.filter(e => e.severity === 'critical');
    
    return {
      total: errors.length,
      last24h: last24h.length,
      critical: critical.length,
      hasFormErrors: Object.keys(formErrors).length > 0
    };
  }, [errors, formErrors]);

  return {
    // Error reporting
    reportError,
    handleNetworkError,
    
    // Form validation
    setValidationErrors,
    clearValidationErrors,
    getFieldError,
    hasFieldError,
    formErrors,
    
    // Error state
    errors,
    clearErrors,
    getErrorStats,
    
    // Utilities
    isFormValid: Object.keys(formErrors).length === 0
  };
};

export default useErrorManagement;