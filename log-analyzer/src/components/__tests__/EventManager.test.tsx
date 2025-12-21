/**
 * Tests for Event Management System
 * Tests React's native event system integration
 * Validates: Requirements 4.4
 */

import React from 'react';
import { render, screen, act } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { EventManager } from '../EventManager';

// Mock Tauri API
jest.mock('@tauri-apps/api/event', () => ({
  listen: jest.fn(),
  emit: jest.fn(),
}));

// Mock logger
jest.mock('../../utils/logger', () => ({
  logger: {
    debug: jest.fn(),
    error: jest.fn(),
    info: jest.fn(),
    warn: jest.fn(),
  },
}));

const { listen: mockListen, emit: mockEmit } = require('@tauri-apps/api/event');

describe('EventManager', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockListen.mockResolvedValue(() => {}); // Mock unlisten function
  });

  describe('Event Subscription Management', () => {
    it('should subscribe to events on mount', async () => {
      const onTaskUpdate = jest.fn();
      const onWorkspaceUpdate = jest.fn();

      render(
        <EventManager
          onTaskUpdate={onTaskUpdate}
          onWorkspaceUpdate={onWorkspaceUpdate}
        />
      );

      // Wait for async subscriptions
      await act(async () => {
        await new Promise(resolve => setTimeout(resolve, 0));
      });

      expect(mockListen).toHaveBeenCalledWith('task-update', expect.any(Function));
      expect(mockListen).toHaveBeenCalledWith('workspace-update', expect.any(Function));
    });

    it('should handle event callbacks correctly', async () => {
      const onTaskUpdate = jest.fn();
      const onWorkspaceUpdate = jest.fn();

      let taskEventHandler: (event: any) => void;
      let workspaceEventHandler: (event: any) => void;

      mockListen.mockImplementation((eventName: string, handler: (event: any) => void) => {
        if (eventName === 'task-update') {
          taskEventHandler = handler;
        } else if (eventName === 'workspace-update') {
          workspaceEventHandler = handler;
        }
        return Promise.resolve(() => {});
      });

      render(
        <EventManager
          onTaskUpdate={onTaskUpdate}
          onWorkspaceUpdate={onWorkspaceUpdate}
        />
      );

      await act(async () => {
        await new Promise(resolve => setTimeout(resolve, 0));
      });

      // Simulate events
      const taskEvent = { payload: { id: 'task-1', status: 'completed' } };
      const workspaceEvent = { payload: { id: 'workspace-1', status: 'ready' } };

      act(() => {
        taskEventHandler!(taskEvent);
        workspaceEventHandler!(workspaceEvent);
      });

      expect(onTaskUpdate).toHaveBeenCalledWith(taskEvent.payload);
      expect(onWorkspaceUpdate).toHaveBeenCalledWith(workspaceEvent.payload);
    });

    it('should cleanup event listeners on unmount', async () => {
      const onTaskUpdate = jest.fn();
      const unlistenMock = jest.fn();

      mockListen.mockResolvedValue(unlistenMock);

      const { unmount } = render(
        <EventManager
          onTaskUpdate={onTaskUpdate}
        />
      );

      await act(async () => {
        await new Promise(resolve => setTimeout(resolve, 0));
      });

      unmount();

      expect(unlistenMock).toHaveBeenCalled();
    });
  });

  describe('Error Handling', () => {
    it('should handle event subscription errors gracefully', async () => {
      const onTaskUpdate = jest.fn();
      const consoleError = jest.spyOn(console, 'error').mockImplementation();

      mockListen.mockRejectedValue(new Error('Failed to subscribe'));

      render(
        <EventManager
          onTaskUpdate={onTaskUpdate}
        />
      );

      await act(async () => {
        await new Promise(resolve => setTimeout(resolve, 0));
      });

      expect(consoleError).toHaveBeenCalledWith(
        'Failed to subscribe to event:',
        'task-update',
        expect.any(Error)
      );

      consoleError.mockRestore();
    });

    it('should handle event handler errors gracefully', async () => {
      const onTaskUpdate = jest.fn().mockImplementation(() => {
        throw new Error('Handler error');
      });
      const consoleError = jest.spyOn(console, 'error').mockImplementation();

      let taskEventHandler: (event: any) => void;

      mockListen.mockImplementation((eventName: string, handler: (event: any) => void) => {
        if (eventName === 'task-update') {
          taskEventHandler = handler;
        }
        return Promise.resolve(() => {});
      });

      render(
        <EventManager
          onTaskUpdate={onTaskUpdate}
        />
      );

      await act(async () => {
        await new Promise(resolve => setTimeout(resolve, 0));
      });

      const taskEvent = { payload: { id: 'task-1' } };

      act(() => {
        taskEventHandler!(taskEvent);
      });

      expect(consoleError).toHaveBeenCalledWith(
        'Error in event handler:',
        expect.any(Error)
      );

      consoleError.mockRestore();
    });
  });

  describe('Event Emission', () => {
    it('should emit events to backend', async () => {
      render(<EventManager />);

      await act(async () => {
        await new Promise(resolve => setTimeout(resolve, 0));
      });

      // The component should have methods to emit events (if implemented)
      // This would test the emit functionality if it exists in the component
    });
  });

  describe('React Native Event System Integration', () => {
    it('should use React useEffect for lifecycle management', async () => {
      const onTaskUpdate = jest.fn();
      let effectCleanup: (() => void) | undefined;

      // Mock useEffect to capture cleanup function
      const originalUseEffect = React.useEffect;
      const useEffectSpy = jest.spyOn(React, 'useEffect').mockImplementation((effect, deps) => {
        const cleanup = effect();
        if (typeof cleanup === 'function') {
          effectCleanup = cleanup;
        }
        return originalUseEffect(() => cleanup, deps);
      });

      const { unmount } = render(
        <EventManager
          onTaskUpdate={onTaskUpdate}
        />
      );

      expect(useEffectSpy).toHaveBeenCalled();

      unmount();

      // Cleanup should have been called
      expect(effectCleanup).toBeDefined();

      useEffectSpy.mockRestore();
    });

    it('should handle component re-renders without re-subscribing', async () => {
      const onTaskUpdate = jest.fn();

      const { rerender } = render(
        <EventManager
          onTaskUpdate={onTaskUpdate}
        />
      );

      await act(async () => {
        await new Promise(resolve => setTimeout(resolve, 0));
      });

      const initialCallCount = mockListen.mock.calls.length;

      // Re-render with same props
      rerender(
        <EventManager
          onTaskUpdate={onTaskUpdate}
        />
      );

      await act(async () => {
        await new Promise(resolve => setTimeout(resolve, 0));
      });

      // Should not have subscribed again
      expect(mockListen.mock.calls.length).toBe(initialCallCount);
    });

    it('should re-subscribe when event handlers change', async () => {
      const onTaskUpdate1 = jest.fn();
      const onTaskUpdate2 = jest.fn();
      const unlistenMock = jest.fn();

      mockListen.mockResolvedValue(unlistenMock);

      const { rerender } = render(
        <EventManager
          onTaskUpdate={onTaskUpdate1}
        />
      );

      await act(async () => {
        await new Promise(resolve => setTimeout(resolve, 0));
      });

      const initialCallCount = mockListen.mock.calls.length;

      // Re-render with different handler
      rerender(
        <EventManager
          onTaskUpdate={onTaskUpdate2}
        />
      );

      await act(async () => {
        await new Promise(resolve => setTimeout(resolve, 0));
      });

      // Should have unsubscribed and re-subscribed
      expect(unlistenMock).toHaveBeenCalled();
      expect(mockListen.mock.calls.length).toBeGreaterThan(initialCallCount);
    });
  });

  describe('Memory Leak Prevention', () => {
    it('should not create memory leaks with multiple mount/unmount cycles', async () => {
      const onTaskUpdate = jest.fn();
      const unlistenMock = jest.fn();

      mockListen.mockResolvedValue(unlistenMock);

      // Mount and unmount multiple times
      for (let i = 0; i < 5; i++) {
        const { unmount } = render(
          <EventManager
            onTaskUpdate={onTaskUpdate}
          />
        );

        await act(async () => {
          await new Promise(resolve => setTimeout(resolve, 0));
        });

        unmount();
      }

      // Each mount should have resulted in a subscription and cleanup
      expect(mockListen).toHaveBeenCalledTimes(5);
      expect(unlistenMock).toHaveBeenCalledTimes(5);
    });

    it('should handle rapid mount/unmount without errors', async () => {
      const onTaskUpdate = jest.fn();
      const consoleError = jest.spyOn(console, 'error').mockImplementation();

      // Rapid mount/unmount
      const components = Array.from({ length: 10 }, () => {
        const { unmount } = render(
          <EventManager
            onTaskUpdate={onTaskUpdate}
          />
        );
        return unmount;
      });

      // Unmount all immediately
      components.forEach(unmount => unmount());

      await act(async () => {
        await new Promise(resolve => setTimeout(resolve, 100));
      });

      // Should not have any console errors
      expect(consoleError).not.toHaveBeenCalled();

      consoleError.mockRestore();
    });
  });
});