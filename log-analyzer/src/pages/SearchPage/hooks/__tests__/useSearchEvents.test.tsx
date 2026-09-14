import { renderHook, act } from "@testing-library/react";
import { useSearchListeners } from "../../../../hooks/useSearchListeners";
import { useSearchEvents } from "../useSearchEvents";
import type { SearchListenerHandlers } from "../../../../hooks/useSearchListeners";
import type { SearchResultSummary } from "../../../../types/search";

jest.mock("../../../../hooks/useSearchListeners", () => ({
  useSearchListeners: jest.fn(),
}));

jest.mock("../../../../utils/logger", () => ({
  logger: {
    debug: jest.fn(),
    info: jest.fn(),
    warn: jest.fn(),
    error: jest.fn(),
  },
}));

const mockUseSearchListeners = useSearchListeners as jest.MockedFunction<
  typeof useSearchListeners
>;

const summary: SearchResultSummary = {
  totalMatches: 1,
  keywordStats: [],
  searchDurationMs: 5,
  truncated: false,
};

function makeOptions(currentSearchId: string) {
  return {
    currentSearchId,
    onStart: jest.fn(),
    onProgress: jest.fn(),
    onSummary: jest.fn(),
    onComplete: jest.fn(),
    onError: jest.fn(),
    onCancelled: jest.fn(),
    onRefetch: jest.fn().mockResolvedValue(undefined),
    onScrollToTop: jest.fn(),
  };
}

function renderEvents(currentSearchId: string) {
  const options = makeOptions(currentSearchId);
  const utils = renderHook(
    ({ id }) => useSearchEvents({ ...options, currentSearchId: id }),
    { initialProps: { id: currentSearchId } }
  );
  const listeners = mockUseSearchListeners.mock.calls.at(-1)?.[0] as
    | SearchListenerHandlers
    | undefined;
  if (!listeners) throw new Error("useSearchListeners was not called");
  return { options, utils, listeners };
}

describe("useSearchEvents (P1 race fix)", () => {
  beforeEach(() => {
    jest.useFakeTimers();
    mockUseSearchListeners.mockClear();
  });

  afterEach(() => {
    jest.runOnlyPendingTimers();
    jest.useRealTimers();
  });

  it("buffers events arriving before the search id is committed, then replays them", () => {
    const { options, utils, listeners } = renderEvents("");

    act(() => {
      listeners.onStart?.("search-1");
      listeners.onProgress?.("search-1", 42);
      listeners.onComplete?.("search-1", 42);
    });

    // Nothing dispatched while the id is unknown.
    expect(options.onStart).not.toHaveBeenCalled();
    expect(options.onProgress).not.toHaveBeenCalled();
    expect(options.onComplete).not.toHaveBeenCalled();

    act(() => {
      utils.rerender({ id: "search-1" });
    });
    act(() => {
      jest.advanceTimersByTime(60);
    });

    expect(options.onStart).toHaveBeenCalledTimes(1);
    expect(options.onProgress).toHaveBeenCalledWith(42);
    expect(options.onComplete).toHaveBeenCalledWith(42);
  });

  it("drops buffered events from stale searches after a different id is committed", () => {
    const { options, utils, listeners } = renderEvents("");

    act(() => {
      listeners.onProgress?.("search-old", 10);
      listeners.onError?.("search-old", "boom");
    });

    act(() => {
      utils.rerender({ id: "search-new" });
    });

    expect(options.onProgress).not.toHaveBeenCalled();
    expect(options.onError).not.toHaveBeenCalled();
  });

  it("dispatches matching events immediately and drops non-matching ones", () => {
    const { options, listeners } = renderEvents("search-1");

    act(() => {
      listeners.onProgress?.("search-2", 99);
    });
    expect(options.onProgress).not.toHaveBeenCalled();

    act(() => {
      listeners.onSummary?.("search-1", summary);
      listeners.onTimeout?.("search-1");
    });
    expect(options.onSummary).toHaveBeenCalledWith(summary);
    // P3 修复：超时是正常终止，走 onCancelled 而非错误 toast。
    expect(options.onCancelled).toHaveBeenCalledTimes(1);
    expect(options.onError).not.toHaveBeenCalled();
  });

  it("routes a cancelled event to onCancelled, not to the error toast", () => {
    const { options, listeners } = renderEvents("search-1");

    act(() => {
      listeners.onCancelled?.("search-1");
    });
    expect(options.onCancelled).toHaveBeenCalledTimes(1);
    expect(options.onError).not.toHaveBeenCalled();
  });
});
