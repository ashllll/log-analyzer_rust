import { act, renderHook } from "@testing-library/react";
import type { FilterOptions } from "../../../../types/common";
import type { SearchQuery } from "../../../../types/search";
import type {
  SearchOrchestratorDeps,
  SearchOrchestratorStateCallbacks,
} from "../useSearchOrchestrator";
import { useSearchOrchestrator } from "../useSearchOrchestrator";

jest.mock("../../../../utils/logger", () => ({
  logger: {
    debug: jest.fn(),
    info: jest.fn(),
    warn: jest.fn(),
    error: jest.fn(),
  },
}));

const baseQuery: SearchQuery = {
  id: "q1",
  terms: [],
  globalOperator: "OR",
  metadata: { createdAt: 0, lastModified: 0, executionCount: 0 },
};

function makeDeps(
  overrides: Partial<SearchOrchestratorDeps> = {}
): SearchOrchestratorDeps {
  return {
    query: "error",
    workspaceId: "ws-1",
    enabledKeywordGroups: [],
    filterOptions: {} as FilterOptions,
    searchConfig: null,
    loadSearchConfig: jest.fn().mockResolvedValue(null),
    buildStructuredQuery: jest.fn().mockReturnValue(baseQuery),
    searchLogs: jest.fn().mockResolvedValue("search-1"),
    cancelSearch: jest.fn().mockResolvedValue(undefined),
    addToast: jest.fn(),
    t: (key: string) => key,
    ...overrides,
  };
}

function makeCallbacks(
  overrides: Partial<SearchOrchestratorStateCallbacks> = {}
): SearchOrchestratorStateCallbacks {
  return {
    setCurrentSearchId: jest.fn(),
    setLiveCount: jest.fn(),
    setSelectedId: jest.fn(),
    dispatchSearchExec: jest.fn(),
    setCurrentQuery: jest.fn(),
    scrollToTop: jest.fn(),
    ...overrides,
  };
}

describe("useSearchOrchestrator", () => {
  it("keeps execute/cancelCurrent identity stable across re-renders with new dep literals", () => {
    const { result, rerender } = renderHook(
      ({ deps, callbacks }) => useSearchOrchestrator(deps, callbacks),
      { initialProps: { deps: makeDeps(), callbacks: makeCallbacks() } }
    );

    const firstExecute = result.current.execute;
    const firstCancel = result.current.cancelCurrent;

    rerender({
      deps: makeDeps({ query: "timeout" }),
      callbacks: makeCallbacks(),
    });

    expect(result.current.execute).toBe(firstExecute);
    expect(result.current.cancelCurrent).toBe(firstCancel);
  });

  it("reads the latest dep values when execute is invoked", async () => {
    const searchLogs = jest.fn().mockResolvedValue("search-1");
    const { result, rerender } = renderHook(
      ({ deps, callbacks }) => useSearchOrchestrator(deps, callbacks),
      {
        initialProps: {
          deps: makeDeps({ searchLogs }),
          callbacks: makeCallbacks(),
        },
      }
    );

    rerender({
      deps: makeDeps({ query: "timeout", searchLogs }),
      callbacks: makeCallbacks(),
    });

    await act(async () => {
      await result.current.execute();
    });

    expect(searchLogs).toHaveBeenCalledTimes(1);
    expect(searchLogs).toHaveBeenCalledWith(
      expect.objectContaining({ query: "timeout" })
    );
  });

  it("still discards stale search ids via the request-sequence guard", async () => {
    let resolveFirst!: (id: string) => void;
    const firstSearch = new Promise<string>((resolve) => {
      resolveFirst = resolve;
    });
    const searchLogs = jest
      .fn()
      .mockImplementationOnce(() => firstSearch)
      .mockImplementationOnce(() => Promise.resolve("search-2"));

    const cancelSearch = jest.fn().mockResolvedValue(undefined);
    const setCurrentSearchId = jest.fn();

    const { result } = renderHook(() =>
      useSearchOrchestrator(
        makeDeps({ searchLogs, cancelSearch }),
        makeCallbacks({ setCurrentSearchId })
      )
    );

    let firstRun!: Promise<void>;
    act(() => {
      firstRun = result.current.execute();
    });

    await act(async () => {
      await result.current.execute();
    });

    await act(async () => {
      resolveFirst("search-1");
      await firstRun;
    });

    expect(cancelSearch).toHaveBeenCalledWith("search-1");
    // reset calls write "" before each attempt; the stale "search-1" must
    // never be applied, and the final applied id must be "search-2".
    expect(setCurrentSearchId).not.toHaveBeenCalledWith("search-1");
    expect(setCurrentSearchId).toHaveBeenLastCalledWith("search-2");
  });
});
