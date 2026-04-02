import { renderHook, waitFor } from '@testing-library/react';
import { useSearchListeners } from '../useSearchListeners';

const listeners = new Map<string, (event: { payload: unknown }) => void>();
const unlisten = jest.fn();

jest.mock('@tauri-apps/api/event', () => ({
  listen: jest.fn(async (eventName: string, handler: (event: { payload: unknown }) => void) => {
    listeners.set(eventName, handler);
    return unlisten;
  }),
}));

describe('useSearchListeners', () => {
  beforeEach(() => {
    listeners.clear();
    unlisten.mockClear();
  });

  it('registers terminal search communication events including cancel and timeout', async () => {
    const onProgress = jest.fn();
    const onSummary = jest.fn();
    const onComplete = jest.fn();
    const onError = jest.fn();
    const onStart = jest.fn();
    const onCancelled = jest.fn();
    const onTimeout = jest.fn();

    renderHook(() =>
      useSearchListeners({
        onProgress,
        onSummary,
        onComplete,
        onError,
        onStart,
        onCancelled,
        onTimeout,
      }),
    );

    await waitFor(() => {
      expect(listeners.has('search-progress')).toBe(true);
      expect(listeners.has('search-summary')).toBe(true);
      expect(listeners.has('search-complete')).toBe(true);
      expect(listeners.has('search-error')).toBe(true);
      expect(listeners.has('search-start')).toBe(true);
      expect(listeners.has('search-cancelled')).toBe(true);
      expect(listeners.has('search-timeout')).toBe(true);
    });

    listeners.get('search-progress')?.({ payload: 12 });
    listeners.get('search-summary')?.({
      payload: {
        totalMatches: 12,
        keywordStats: [{ keyword: 'error', matchCount: 12, matchPercentage: 100 }],
        searchDurationMs: 25,
        truncated: false,
      },
    });
    listeners.get('search-complete')?.({ payload: 12 });
    listeners.get('search-error')?.({ payload: 'boom' });
    listeners.get('search-start')?.({ payload: null });
    listeners.get('search-cancelled')?.({ payload: 'search-1' });
    listeners.get('search-timeout')?.({ payload: 'search-2' });

    expect(onProgress).toHaveBeenCalledWith(12);
    expect(onSummary).toHaveBeenCalledWith(
      expect.objectContaining({ totalMatches: 12 }),
    );
    expect(onComplete).toHaveBeenCalledWith(12);
    expect(onError).toHaveBeenCalledWith('boom');
    expect(onStart).toHaveBeenCalled();
    expect(onCancelled).toHaveBeenCalledWith('search-1');
    expect(onTimeout).toHaveBeenCalledWith('search-2');
  });
});
