import { act, renderHook } from "@testing-library/react";
import { SEARCH_TRIGGER_DEBOUNCE_MS, useSearchQuery } from "../useSearchQuery";

describe("useSearchQuery", () => {
  beforeEach(() => {
    jest.useFakeTimers();
    window.localStorage.clear();
  });

  afterEach(() => {
    jest.runOnlyPendingTimers();
    jest.useRealTimers();
  });

  it("triggers automatic search after the configured debounce", () => {
    const { result } = renderHook(() => useSearchQuery());

    act(() => {
      result.current.setQuery("error");
    });

    expect(result.current.searchTrigger).toBe(0);

    act(() => {
      jest.advanceTimersByTime(SEARCH_TRIGGER_DEBOUNCE_MS - 1);
    });
    expect(result.current.searchTrigger).toBe(0);

    act(() => {
      jest.advanceTimersByTime(1);
    });
    expect(result.current.searchTrigger).toBe(1);
  });
});
