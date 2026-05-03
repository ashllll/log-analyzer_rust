/**
 * Integration tests for React resource management hooks
 * Tests proper cleanup and resource management using React patterns
 * Validates: Requirements 4.4, 4.5
 */

import { renderHook, act } from '@testing-library/react';
import { useResourceManager } from '../useResourceManager';

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

  describe('Integration with React lifecycle', () => {
    it('should properly cleanup all resources when component unmounts', () => {
      const timerCallback = jest.fn();
      const intervalCallback = jest.fn();
      const subscriptionCleanup = jest.fn();

      const { result, unmount } = renderHook(() => {
        const resourceManager = useResourceManager();
        return { resourceManager };
      });

      // Create various resources
      act(() => {
        result.current.resourceManager.createTimer(timerCallback, 1000);
        result.current.resourceManager.createInterval(intervalCallback, 200);
        result.current.resourceManager.createSubscription(subscriptionCleanup);
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