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
