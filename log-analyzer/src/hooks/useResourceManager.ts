import { useEffect, useRef, useCallback } from 'react';
import { logger } from '../utils/logger';

/**
 * Hook for managing resources with React's built-in cleanup patterns
 */
export const useResourceManager = () => {
  const timersRef = useRef<Set<NodeJS.Timeout>>(new Set());
  const intervalsRef = useRef<Set<NodeJS.Timeout>>(new Set());
  const subscriptionsRef = useRef<Set<() => void>>(new Set());
  const abortControllersRef = useRef<Set<AbortController>>(new Set());

  // Cleanup function that runs on unmount
  useEffect(() => {
    // Capture current ref values at effect execution time
    const timers = timersRef.current;
    const intervals = intervalsRef.current;
    const subscriptions = subscriptionsRef.current;
    const controllers = abortControllersRef.current;

    return () => {
      logger.debug('[RESOURCE_MANAGER] Cleaning up all resources');

      // Clear all timers
      timers.forEach(timer => {
        clearTimeout(timer);
      });
      timers.clear();

      // Clear all intervals
      intervals.forEach(interval => {
        clearInterval(interval);
      });
      intervals.clear();

      // Clean up all subscriptions
      subscriptions.forEach(cleanup => {
        try {
          cleanup();
        } catch (error) {
          logger.error('[RESOURCE_MANAGER] Error during subscription cleanup:', error);
        }
      });
      subscriptions.clear();

      // Abort all ongoing requests
      controllers.forEach(controller => {
        try {
          controller.abort();
        } catch (error) {
          logger.error('[RESOURCE_MANAGER] Error during request abort:', error);
        }
      });
      controllers.clear();
    };
  }, []);

  // Create a managed timer
  const createTimer = useCallback((callback: () => void, delay: number) => {
    const timer = setTimeout(() => {
      timersRef.current.delete(timer);
      callback();
    }, delay);
    
    timersRef.current.add(timer);
    return timer;
  }, []);

  // Create a managed interval
  const createInterval = useCallback((callback: () => void, delay: number) => {
    const interval = setInterval(callback, delay);
    intervalsRef.current.add(interval);
    return interval;
  }, []);

  // Create a managed subscription
  const createSubscription = useCallback((cleanup: () => void) => {
    subscriptionsRef.current.add(cleanup);
    return () => {
      subscriptionsRef.current.delete(cleanup);
      cleanup();
    };
  }, []);

  // Create a managed AbortController
  const createAbortController = useCallback(() => {
    const controller = new AbortController();
    abortControllersRef.current.add(controller);
    
    // Auto-cleanup when aborted
    controller.signal.addEventListener('abort', () => {
      abortControllersRef.current.delete(controller);
    });
    
    return controller;
  }, []);

  // Manual cleanup functions
  const clearTimer = useCallback((timer: NodeJS.Timeout) => {
    clearTimeout(timer);
    timersRef.current.delete(timer);
  }, []);

  const clearManagedInterval = useCallback((interval: NodeJS.Timeout) => {
    clearInterval(interval);
    intervalsRef.current.delete(interval);
  }, []);

  return {
    createTimer,
    createInterval,
    createSubscription,
    createAbortController,
    clearTimer,
    clearInterval: clearManagedInterval,
  };
};

/**
 * Hook for debounced operations using React patterns
 */
export const useDebounce = <T extends (...args: any[]) => any>(
  callback: T,
  delay: number
): T => {
  const { createTimer, clearTimer } = useResourceManager();
  const timerRef = useRef<NodeJS.Timeout | null>(null);
  const callbackRef = useRef(callback);

  // Update callback ref when callback changes
  useEffect(() => {
    callbackRef.current = callback;
  }, [callback]);

  const debouncedCallback = useCallback((...args: Parameters<T>) => {
    // Clear existing timer
    if (timerRef.current) {
      clearTimer(timerRef.current);
    }

    // Create new timer
    timerRef.current = createTimer(() => {
      callbackRef.current(...args);
      timerRef.current = null;
    }, delay);
  }, [delay, createTimer, clearTimer]) as T;

  return debouncedCallback;
};

/**
 * Hook for throttled operations using React patterns
 */
export const useThrottle = <T extends (...args: any[]) => any>(
  callback: T,
  delay: number
): T => {
  const lastCallRef = useRef<number>(0);
  const callbackRef = useRef(callback);

  // Update callback ref when callback changes
  useEffect(() => {
    callbackRef.current = callback;
  }, [callback]);

  const throttledCallback = useCallback((...args: Parameters<T>) => {
    const now = Date.now();
    if (now - lastCallRef.current >= delay) {
      lastCallRef.current = now;
      callbackRef.current(...args);
    }
  }, [delay]) as T;

  return throttledCallback;
};

/**
 * Hook for managing component lifecycle with proper cleanup
 */
export const useLifecycle = (
  onMount?: () => void | (() => void),
  onUnmount?: () => void
) => {
  const cleanupRef = useRef<(() => void) | null>(null);

  useEffect(() => {
    // Call onMount and store cleanup function if returned
    if (onMount) {
      const cleanup = onMount();
      if (typeof cleanup === 'function') {
        cleanupRef.current = cleanup;
      }
    }

    // Return cleanup function
    return () => {
      // Call stored cleanup function
      if (cleanupRef.current) {
        cleanupRef.current();
        cleanupRef.current = null;
      }
      
      // Call onUnmount
      if (onUnmount) {
        onUnmount();
      }
    };
  }, [onMount, onUnmount]);
};

/**
 * Hook for managing async operations with automatic cleanup
 */
export const useAsyncOperation = () => {
  const { createAbortController } = useResourceManager();
  const activeOperationsRef = useRef<Set<Promise<any>>>(new Set());

  const runAsyncOperation = useCallback(async <T>(
    operation: (signal: AbortSignal) => Promise<T>
  ): Promise<T | null> => {
    const controller = createAbortController();
    
    try {
      const promise = operation(controller.signal);
      activeOperationsRef.current.add(promise);
      
      const result = await promise;
      activeOperationsRef.current.delete(promise);
      
      return result;
    } catch (error) {
      activeOperationsRef.current.delete(operation as any);
      
      if (controller.signal.aborted) {
        logger.debug('[ASYNC_OPERATION] Operation was cancelled');
        return null;
      }
      
      throw error;
    }
  }, [createAbortController]);

  return { runAsyncOperation };
};