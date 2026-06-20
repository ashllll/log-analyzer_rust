/**
 * useSearchOrchestrator — search request lifecycle with race-condition guard.
 *
 * Extracted from SearchPage.handleSearch (P10 architecture review #4).
 * Owns the request-sequence state machine and the active search ID ref,
 * making the race-condition logic independently testable.
 *
 * Interface: { execute, cancelCurrent }
 *  - execute(): start a new search; cancels any in-flight request.
 *  - cancelCurrent(): cancel the active search.
 */

import { useRef, useCallback } from "react";
import { logger } from "../../../utils/logger";
import { getFullErrorMessage } from "../../../services/errors";
import type { SearchConfig } from "../../../services/api";
import type { FilterOptions } from "../../../types/common";
import type { SearchQuery } from "../../../types/search";

// ── Types ──

export interface SearchOrchestratorDeps {
  /** Raw search query string (trimmed) */
  query: string;
  /** Currently selected workspace */
  workspaceId: string | null;
  /** Enabled keyword groups for structured query building */
  enabledKeywordGroups: ReadonlyArray<{ enabled: boolean }>;
  /** Current time/level/file filters */
  filterOptions: FilterOptions;
  /** Preloaded search config (may be null) */
  searchConfig: SearchConfig | null;
  /** Async loader for search config (fallback) */
  loadSearchConfig: () => Promise<SearchConfig | null | undefined>;
  /** Build a structured query from raw query + keyword groups */
  buildStructuredQuery: (
    raw: string,
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    groups: any[],
    parsingOptions: { caseSensitive: boolean; regexEnabled: boolean }
  ) => SearchQuery;
  /** Call API to start a search; returns searchId */
  searchLogs: (params: {
    query: string;
    structuredQuery: SearchQuery;
    workspaceId: string;
    filters: FilterOptions;
  }) => Promise<string>;
  /** Call API to cancel an active search */
  cancelSearch: (searchId: string) => Promise<void>;
  /** Show a toast notification */
  addToast: (type: "error" | "info", message: string) => void;
  /** i18n translate function */
  t: (key: string) => string;
}

export interface SearchOrchestratorStateCallbacks {
  /** Set the current search ID (empty string = no active search) */
  setCurrentSearchId: (id: string) => void;
  /** Set the live result count */
  setLiveCount: (count: number) => void;
  /** Set the selected detail entry */
  setSelectedId: (id: number | null) => void;
  /** Dispatch a search-execution state action */
  dispatchSearchExec: (action: { type: "START" | "ERROR" }) => void;
  /** Set the structured query that was last executed */
  setCurrentQuery: (query: SearchQuery) => void;
  /** Reset scroll position (ref to scroll container) */
  scrollToTop: () => void;
}

export interface SearchOrchestrator {
  /** Execute a search. Idempotent: cancels any previous in-flight request. */
  execute: () => Promise<void>;
  /** Cancel the current active search. No-op if none active. */
  cancelCurrent: () => Promise<void>;
  /** Whether a search is currently in flight (ref-based, no re-render). */
  isActive: () => boolean;
}

// ── Hook ──

export function useSearchOrchestrator(
  deps: SearchOrchestratorDeps,
  callbacks: SearchOrchestratorStateCallbacks
): SearchOrchestrator {
  const searchRequestSeqRef = useRef(0);
  const activeSearchIdRef = useRef("");

  // ── cancel current ──

  const { cancelSearch } = deps;
  const cancelCurrent = useCallback(async () => {
    const id = activeSearchIdRef.current;
    if (id) {
      activeSearchIdRef.current = "";
      cancelSearch(id).catch((error) => {
        logger.warn("Cancel search failed:", error);
      });
    }
  }, [cancelSearch]);

  // ── execute ──

  const execute = useCallback(async () => {
    const {
      query,
      workspaceId,
      filterOptions,
      enabledKeywordGroups,
      searchConfig,
      loadSearchConfig,
      buildStructuredQuery,
      searchLogs,
      cancelSearch,
      addToast,
      t,
    } = deps;
    const {
      setCurrentSearchId,
      setLiveCount,
      setSelectedId,
      dispatchSearchExec,
      setCurrentQuery,
      scrollToTop,
    } = callbacks;

    // Guard: no workspace
    if (!workspaceId) {
      addToast("error", t("search.no_workspace_selected"));
      return;
    }

    // Guard: empty query → cancel + reset
    const trimmed = query.trim();
    if (!trimmed) {
      searchRequestSeqRef.current += 1;
      setLiveCount(0);
      await cancelCurrent();
      setCurrentSearchId("");
      return;
    }

    // Claim a new sequence number
    const requestSeq = searchRequestSeqRef.current + 1;
    searchRequestSeqRef.current = requestSeq;

    // Cancel any previous in-flight search
    await cancelCurrent();

    // Reset UI state
    dispatchSearchExec({ type: "START" });
    setLiveCount(0);
    setCurrentSearchId("");
    setSelectedId(null);
    scrollToTop();

    try {
      const runtimeConfig =
        searchConfig ?? (await loadSearchConfig().catch(() => null));
      const parsingOptions = {
        caseSensitive: runtimeConfig?.case_sensitive ?? false,
        regexEnabled: runtimeConfig?.regex_enabled ?? true,
      };
      const structuredQuery = buildStructuredQuery(
        trimmed,
        enabledKeywordGroups as any[], // eslint-disable-line @typescript-eslint/no-explicit-any,
        parsingOptions
      );

      const searchId = await searchLogs({
        query: trimmed,
        structuredQuery,
        workspaceId,
        filters: {
          timeRange: filterOptions.timeRange,
          levels: filterOptions.levels,
          filePattern: filterOptions.filePattern,
        },
      });

      // ── Race-condition guard ──
      // If another execute() call incremented the sequence number while
      // we were awaiting the API response, discard this stale result.
      if (requestSeq !== searchRequestSeqRef.current) {
        cancelSearch(searchId).catch((error) => {
          logger.warn("Cancel stale search failed:", error);
        });
        return;
      }

      activeSearchIdRef.current = searchId;
      setCurrentSearchId(searchId);
      setCurrentQuery(structuredQuery);
    } catch (err) {
      // Stale request → discard
      if (requestSeq !== searchRequestSeqRef.current) {
        return;
      }
      logger.error("Search failed:", err);
      dispatchSearchExec({ type: "ERROR" });
      addToast("error", `Search failed: ${getFullErrorMessage(err)}`);
    }
  }, [deps, callbacks, cancelCurrent]);

  const isActive = useCallback(() => activeSearchIdRef.current !== "", []);

  return { execute, cancelCurrent, isActive };
}
