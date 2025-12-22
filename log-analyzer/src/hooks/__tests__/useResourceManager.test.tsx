/**
 * Integration tests for React resource management hooks
 * Tests proper cleanup and resource management using React patterns
 * Validates: Requirements 4.4, 4.5
 */

import { renderHook, act } from '@testing-library/react';
import { useResourceManager, useDebounce, useThrottle, useLifecycle } from '../useResourceManager';

// Mock logger
jest.mock('../../utils/logger', () => ({
  logger: {
    debug: jest.fn(),
    error: jest.fn(),
    info: jest.fn(),
    warn: jest.fn(),
  },
}));

describe('Resource Manager Integration Tests', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    jest.useFakeTimers();
  });

  afterEach(() => {
    jest.runOnlyPendingTimers();
    jest.useRealTimers();
  });

  describe('useResourceManager', () => {
    it('should create and manage timers with automatic cleanup', () => {
      const { result, unmount } = renderHook(() => useResourceManager());
      const callback = jest.fn();

      act(() => {
        result.current.createTimer(callback, 1000);
      });

      // Timer should not have fired yet
      expect(callback).not.toHaveBeenCalled();

      // Fast-forward time
      act(() => {
        jest.advanceTimersByTime(1000);
      });

      expect(callback).toHaveBeenCalledTimes(1);

      // Create another timer and unmount before it fires
      act(() => {
        result.current.createTimer(callback, 2000);
      });

      unmount();

      // Fast-forward time - callback should not fire because component unmounted
      act(() => {
        jest.advanceTimersByTime(2000);
      });

      expect(callback).toHaveBeenCalledTimes(1); // Still only called once
    });

    it('should create and manage intervals with automatic cleanup', () => {
      const { result, unmount } = renderHook(() => useResourceManager());
      const callback = jest.fn();

      act(() => {
        result.current.createInterval(callback, 500);
      });

      // Fast-forward time to trigger interval multiple times
      act(() => {
        jest.advanceTimersByTime(1500);
      });

      expect(callback).toHaveBeenCalledTimes(3);

      // Unmount and advance time - should not fire anymore
      unmount();

      act(() => {
        jest.advanceTimersByTime(1000);
      });

      expect(callback).toHaveBeenCalledTimes(3); // Still only 3 calls
    });

    it('should manage subscriptions with automatic cleanup', () => {
      const { result, unmount } = renderHook(() => useResourceManager());
      const cleanup = jest.fn();

      act(() => {
        result.current.createSubscription(cleanup);
      });

      // Cleanup should not have been called yet
      expect(cleanup).not.toHaveBeenCalled();

      // Unmount should trigger cleanup
      unmount();

      expect(cleanup).toHaveBeenCalledTimes(1);
    });

    it('should manage AbortControllers with automatic cleanup', () => {
      const { result, unmount } = renderHook(() => useResourceManager());
      let controller: AbortController;

      act(() => {
        controller = result.current.createAbortController();
      });

      expect(controller.signal.aborted).toBe(false);

      // Unmount should abort the controller
      unmount();

      expect(controller.signal.aborted).toBe(true);
    });

    it('should handle manual cleanup of resources', () => {
      const { result } = renderHook(() => useResourceManager());
      const callback = jest.fn();

      let timer: NodeJS.Timeout;
      let interval: NodeJS.Timeout;

      act(() => {
        timer = result.current.createTimer(callback, 1000);
        interval = result.current.createInterval(callback, 500);
      });

      // Manually clear timer
      act(() => {
        result.current.clearTimer(timer);
      });

      // Advance time - timer should not fire, but interval should
      act(() => {
        jest.advanceTimersByTime(1000);
      });

      expect(callback).toHaveBeenCalledTimes(2); // Only interval calls

      // Manually clear interval
      act(() => {
        result.current.clearInterval(interval);
      });

      // Advance time - nothing should fire
      act(() => {
        jest.advanceTimersByTime(1000);
      });

      expect(callback).toHaveBeenCalledTimes(2); // Still only 2 calls
    });
  });

  describe('useDebounce', () => {
    it('should debounce function calls correctly', () => {
      const callback = jest.fn();
      const { result } = renderHook(() => useDebounce(callback, 500));

      // Call multiple times rapidly
      act(() => {
        result.current('arg1');
        result.current('arg2');
        result.current('arg3');
      });

      // Should not have been called yet
      expect(callback).not.toHaveBeenCalled();

      // Fast-forward time
      act(() => {
        jest.advanceTimersByTime(500);
      });

      // Should only be called once with the last arguments
      expect(callback).toHaveBeenCalledTimes(1);
      expect(callback).toHaveBeenCalledWith('arg3');
    });

    it('should reset debounce timer on new calls', () => {
      const callback = jest.fn();
      const { result } = renderHook(() => useDebounce(callback, 500));

      act(() => {
        result.current('first');
      });

      // Advance time partially
      act(() => {
        jest.advanceTimersByTime(300);
      });

      // Call again - should reset timer
      act(() => {
        result.current('second');
      });

      // Advance remaining time from first call
      act(() => {
        jest.advanceTimersByTime(200);
      });

      // Should not have been called yet
      expect(callback).not.toHaveBeenCalled();

      // Advance full debounce time from second call
      act(() => {
        jest.advanceTimersByTime(300);
      });

      expect(callback).toHaveBeenCalledTimes(1);
      expect(callback).toHaveBeenCalledWith('second');
    });

    it('should cleanup debounce timer on unmount', () => {
      const callback = jest.fn();
      const { result, unmount } = renderHook(() => useDebounce(callback, 500));

      act(() => {
        result.current('test');
      });

      // Unmount before timer fires
      unmount();

      // Advance time
      act(() => {
        jest.advanceTimersByTime(500);
      });

      // Should not have been called
      expect(callback).not.toHaveBeenCalled();
    });
  });

  describe('useThrottle', () => {
    it('should throttle function calls correctly', () => {
      const callback = jest.fn();
      const { result } = renderHook(() => useThrottle(callback, 500));

      // First call should execute immediately
      act(() => {
        result.current('first');
      });

      expect(callback).toHaveBeenCalledTimes(1);
      expect(callback).toHaveBeenCalledWith('first');

      // Subsequent calls within throttle period should be ignored
      act(() => {
        result.current('second');
        result.current('third');
      });

      expect(callback).toHaveBeenCalledTimes(1);

      // After throttle period, next call should execute
      act(() => {
        jest.advanceTimersByTime(500);
        result.current('fourth');
      });

      expect(callback).toHaveBeenCalledTimes(2);
      expect(callback).toHaveBeenLastCalledWith('fourth');
    });
  });

  describe('useLifecycle', () => {
    it('should call onMount and onUnmount at appropriate times', () => {
      const onMount = jest.fn();
      const onUnmount = jest.fn();

      const { unmount } = renderHook(() => useLifecycle(onMount, onUnmount));

      expect(onMount).toHaveBeenCalledTimes(1);
      expect(onUnmount).not.toHaveBeenCalled();

      unmount();

      expect(onUnmount).toHaveBeenCalledTimes(1);
    });

    it('should call cleanup function returned from onMount', () => {
      const cleanup = jest.fn();
      const onMount = jest.fn(() => cleanup);
      const onUnmount = jest.fn();

      const { unmount } = renderHook(() => useLifecycle(onMount, onUnmount));

      expect(onMount).toHaveBeenCalledTimes(1);
      expect(cleanup).not.toHaveBeenCalled();
      expect(onUnmount).not.toHaveBeenCalled();

      unmount();

      expect(cleanup).toHaveBeenCalledTimes(1);
      expect(onUnmount).toHaveBeenCalledTimes(1);
    });

    it('should handle onMount without cleanup function', () => {
      const onMount = jest.fn(() => undefined);
      const onUnmount = jest.fn();

      const { unmount } = renderHook(() => useLifecycle(onMount, onUnmount));

      expect(onMount).toHaveBeenCalledTimes(1);

      // Should not throw when unmounting
      expect(() => unmount()).not.toThrow();
      expect(onUnmount).toHaveBeenCalledTimes(1);
    });
  });

  describe('Integration with React lifecycle', () => {
    it('should properly cleanup all resources when component unmounts', () => {
      const timerCallback = jest.fn();
      const intervalCallback = jest.fn();
      const subscriptionCleanup = jest.fn();

      const { result, unmount } = renderHook(() => {
        const resourceManager = useResourceManager();
        const debouncedFn = useDebounce(timerCallback, 300);
        
        return { resourceManager, debouncedFn };
      });

      // Create various resources
      act(() => {
        result.current.resourceManager.createTimer(timerCallback, 1000);
        result.current.resourceManager.createInterval(intervalCallback, 200);
        result.current.resourceManager.createSubscription(subscriptionCleanup);
        result.current.debouncedFn('test');
      });

      // Advance time partially
      act(() => {
        jest.advanceTimersByTime(250);
      });

      // Interval should have fired once
      expect(intervalCallback).toHaveBeenCalledTimes(1);
      expect(timerCallback).not.toHaveBeenCalled();
      expect(subscriptionCleanup).not.toHaveBeenCalled();

      // Unmount component
      unmount();

      // Advance time - nothing should fire after unmount
      act(() => {
        jest.advanceTimersByTime(2000);
      });

      // Only the interval call from before unmount should have happened
      expect(intervalCallback).toHaveBeenCalledTimes(1);
      expect(timerCallback).not.toHaveBeenCalled();
      expect(subscriptionCleanup).toHaveBeenCalledTimes(1);
    });
  });
});