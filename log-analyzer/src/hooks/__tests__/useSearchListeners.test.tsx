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

    listeners.get('search-progress')?.({ payload: { search_id: 'search-1', count: 12 } });
    listeners.get('search-summary')?.({
      payload: {
        search_id: 'search-1',
        summary: {
          totalMatches: 12,
          keywordStats: [{ keyword: 'error', matchCount: 12, matchPercentage: 100 }],
          searchDurationMs: 25,
          truncated: false,
        },
      },
    });
    listeners.get('search-complete')?.({ payload: { search_id: 'search-1', total_count: 12 } });
    listeners.get('search-error')?.({ payload: { search_id: 'search-1', error: 'boom' } });
    listeners.get('search-start')?.({ payload: { search_id: 'search-1' } });
    listeners.get('search-cancelled')?.({ payload: { search_id: 'search-1' } });
    listeners.get('search-timeout')?.({ payload: { search_id: 'search-2' } });

    expect(onProgress).toHaveBeenCalledWith('search-1', 12);
    expect(onSummary).toHaveBeenCalledWith(
      'search-1',
      expect.objectContaining({ totalMatches: 12 }),
    );
    expect(onComplete).toHaveBeenCalledWith('search-1', 12);
    expect(onError).toHaveBeenCalledWith('search-1', 'boom');
    expect(onStart).toHaveBeenCalledWith('search-1');
    expect(onCancelled).toHaveBeenCalledWith('search-1');
    expect(onTimeout).toHaveBeenCalledWith('search-2');
  });
});
