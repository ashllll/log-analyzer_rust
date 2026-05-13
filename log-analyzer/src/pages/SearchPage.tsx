import React, { useState, useEffect, useCallback, useMemo, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { save } from '@tauri-apps/plugin-dialog';
import { useWorkspaceStore } from '../stores/workspaceStore';
import { useWorkspaceSelection } from '../hooks/useWorkspaceSelection';
import { useKeywordStore } from '../stores/keywordStore';
import { useAppStore } from '../stores/appStore';
import { useShallow } from 'zustand/shallow';
import { useToast } from '../hooks/useToast';
import { useConfig } from '../hooks/useConfig';
import { useInfiniteSearch } from '../hooks/useInfiniteSearch';
import { api } from '../services/api';
import { getFullErrorMessage } from '../services/errors';
import { logger } from '../utils/logger';
import { KeywordStatsPanel } from '../components/search/KeywordStatsPanel';
import type { FilterOptions } from '../types/common';

import {
  SearchControls,
  SearchFilters,
  ActiveKeywords,
  SearchResults,
  LogDetailPanel,
} from './SearchPage/components';
import {
  useSearchState,
  useVirtualScroll,
  useSearchPagination,
  useSearchEvents,
  useSearchQuery,
  useWorkspaceTimeRange,
} from './SearchPage/hooks';

const SearchPage: React.FC = () => {
  const { t } = useTranslation();
  const { showToast: addToast } = useToast();
  const { searchConfig, loadSearchConfig } = useConfig();
  const { activeWorkspace } = useWorkspaceSelection();
  const keywordGroups = useKeywordStore(useShallow((state) => state.keywordGroups));
  const workspaceLoading = useWorkspaceStore((state) => state.loading);
  const isInitialized = useAppStore((state) => state.isInitialized);

  const enabledKeywordGroups = useMemo(
    () => keywordGroups.filter((g) => g.enabled),
    [keywordGroups]
  );

  const searchParsingOptions = useMemo(
    () => ({
      caseSensitive: searchConfig?.case_sensitive ?? false,
      regexEnabled: searchConfig?.regex_enabled ?? true,
    }),
    [searchConfig]
  );

  // 搜索执行状态
  const [searchExec, dispatchSearchExec] = useSearchState();
  const { isSearching, searchSummary, keywordStats } = searchExec;

  // 搜索查询管理
  const {
    query,
    currentQuery,
    activeTerms,
    searchTrigger,
    setQuery,
    setCurrentQuery,
    buildStructuredQuery,
    removeTermFromQuery,
    toggleRuleInQuery,
  } = useSearchQuery();

  // 工作区时间范围
  const { filterOptions, setFilterOptions, resetFilters } = useWorkspaceTimeRange({
    activeWorkspaceId: activeWorkspace?.id,
  });

  // 当前搜索 ID
  const [currentSearchId, setCurrentSearchId] = useState('');
  const [liveCount, setLiveCount] = useState(0);
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [isFilterPaletteOpen, setIsFilterPaletteOpen] = useState(false);
  const lastProgressRefetchAt = useRef(0);

  // 无限搜索
  const {
    data: infiniteSearchData,
    refetch: refetchSearchPages,
    fetchNextPage,
    hasNextPage,
    isFetchingNextPage,
    fetchPreviousPage,
    hasPreviousPage,
    isFetchingPreviousPage,
    error: infiniteSearchError,
  } = useInfiniteSearch({
    searchId: currentSearchId,
    query,
    workspaceId: activeWorkspace?.id ?? null,
    enabled: !!currentSearchId,
    pageSize: 1000,
    maxPages: 10,
  });

  const loadedEntries = useMemo(
    () => infiniteSearchData?.pages.flatMap((page) => page.results) ?? [],
    [infiniteSearchData]
  );

  const totalResultCount = useMemo(() => {
    const pagedTotal = infiniteSearchData?.pages[0]?.totalCount;
    if (typeof pagedTotal === 'number' && pagedTotal >= 0) {
      return pagedTotal;
    }
    return liveCount;
  }, [infiniteSearchData, liveCount]);

  const firstPageOffset = useMemo(
    () => (infiniteSearchData?.pageParams?.[0] as number) ?? 0,
    [infiniteSearchData]
  );

  const loadedEntriesMap = useMemo(
    () => new Map(loadedEntries.map((entry) => [entry.id, entry])),
    [loadedEntries]
  );

  // 虚拟滚动
  const {
    parentRef,
    virtualItems,
    totalSize,
    measureElement,
    scrollToIndex,
  } = useVirtualScroll({
    count: totalResultCount,
    estimateSize: 48,
    overscan: 20,
  });

  const lastVisibleIndex = virtualItems.length > 0 ? virtualItems[virtualItems.length - 1].index : -1;
  const firstVisibleIndex = virtualItems.length > 0 ? virtualItems[0].index : -1;

  // 分页加载
  useSearchPagination({
    firstVisibleIndex,
    lastVisibleIndex,
    firstPageOffset,
    loadedCount: loadedEntries.length,
    hasNextPage,
    hasPreviousPage,
    isFetchingNextPage,
    isFetchingPreviousPage,
    fetchNextPage,
    fetchPreviousPage,
  });

  // 搜索事件监听
  useSearchEvents({
    currentSearchId,
    onStart: useCallback(() => {
      dispatchSearchExec({ type: 'START' });
      setLiveCount(0);
      setSelectedId(null);
      if (parentRef.current) parentRef.current.scrollTop = 0;
    }, [dispatchSearchExec, parentRef]),
    onProgress: useCallback((count: number) => {
      setLiveCount(count);
      const now = Date.now();
      if (now - lastProgressRefetchAt.current < 300) return;
      lastProgressRefetchAt.current = now;
      refetchSearchPages().catch((error) => {
        logger.error('Refetch search page after progress failed:', error);
      });
    }, [refetchSearchPages]),
    onSummary: useCallback((summary) => {
      dispatchSearchExec({ type: 'SUMMARY', summary, keywordColors: ['#3B82F6', '#8B5CF6', '#22C55E', '#F59E0B', '#EC4899', '#06B6D4'] });
    }, [dispatchSearchExec]),
    onComplete: useCallback((count: number) => {
      dispatchSearchExec({ type: 'COMPLETE' });
      setLiveCount(count);
    }, [dispatchSearchExec]),
    onError: useCallback((errorMsg: string) => {
      dispatchSearchExec({ type: 'ERROR' });
      addToast('error', t('search.error', { message: errorMsg }));
    }, [addToast, t, dispatchSearchExec]),
    onRefetch: refetchSearchPages,
    onScrollToTop: useCallback(() => {
      scrollToIndex(0);
    }, [scrollToIndex]),
  });

  // 无限搜索错误处理
  useEffect(() => {
    if (infiniteSearchError) {
      console.error('Infinite search error:', infiniteSearchError);
      addToast('error', `分页加载失败: ${infiniteSearchError.message}`);
    }
  }, [infiniteSearchError, addToast]);

  // 加载搜索配置
  useEffect(() => {
    loadSearchConfig().catch((err) => {
      logger.warn('Failed to load search config for SearchPage, using defaults', err);
    });
  }, [loadSearchConfig]);

  // 搜索触发器变化时执行搜索
  useEffect(() => {
    if (searchTrigger > 0 && activeWorkspace) {
      handleSearch();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [searchTrigger, activeWorkspace]);

  // 执行搜索
  const handleSearch = useCallback(async () => {
    if (!activeWorkspace) {
      addToast('error', t('search.no_workspace_selected'));
      return;
    }

    const trimmedQuery = query.trim();
    if (!trimmedQuery) {
      setLiveCount(0);
      setCurrentSearchId('');
      return;
    }

    dispatchSearchExec({ type: 'START' });
    setLiveCount(0);
    setCurrentSearchId('');
    setSelectedId(null);

    if (parentRef.current) {
      parentRef.current.scrollTop = 0;
    }

    try {
      const runtimeSearchConfig = searchConfig ?? (await loadSearchConfig().catch(() => null));
      const parsingOptions = {
        caseSensitive: runtimeSearchConfig?.case_sensitive ?? false,
        regexEnabled: runtimeSearchConfig?.regex_enabled ?? true,
      };
      const structuredQuery = buildStructuredQuery(trimmedQuery, enabledKeywordGroups, parsingOptions);

      const filters: FilterOptions = {
        timeRange: filterOptions.timeRange,
        levels: filterOptions.levels,
        filePattern: filterOptions.filePattern,
      };

      const searchId = await api.searchLogs({
        query: trimmedQuery,
        structuredQuery,
        workspaceId: activeWorkspace.id,
        filters,
      });

      setCurrentSearchId(searchId);
      setCurrentQuery(structuredQuery);
    } catch (err) {
      logger.error('Search failed:', err);
      dispatchSearchExec({ type: 'ERROR' });
      addToast('error', `搜索失败: ${getFullErrorMessage(err)}`);
    }
  }, [
    query,
    activeWorkspace,
    enabledKeywordGroups,
    filterOptions,
    searchConfig,
    loadSearchConfig,
    addToast,
    dispatchSearchExec,
    t,
    buildStructuredQuery,
    setCurrentQuery,
    parentRef,
  ]);

  // 导出搜索结果
  const handleExport = useCallback(async (format: 'csv' | 'json') => {
    if (loadedEntries.length === 0) {
      addToast('error', '没有可导出的数据');
      return;
    }

    try {
      const defaultPath = `log-export-${Date.now()}.${format}`;
      const savePath = await save({
        defaultPath,
        filters: [{ name: format.toUpperCase(), extensions: [format] }],
      });

      if (!savePath) return;

      await api.exportResults({ results: loadedEntries, format, savePath });
      addToast('success', `已导出 ${loadedEntries.length} 条日志到 ${format.toUpperCase()}`);
    } catch (e) {
      logger.error('Export error:', e);
      addToast('error', `导出失败: ${getFullErrorMessage(e)}`);
    }
  }, [loadedEntries, addToast]);

  // 复制到剪贴板
  const copyToClipboard = useCallback((text: string) => {
    if (navigator.clipboard) {
      navigator.clipboard.writeText(text).then(() => addToast('success', 'Copied'));
    } else {
      addToast('error', 'Clipboard not available');
    }
  }, [addToast]);

  // 切换规则
  const handleToggleRule = useCallback((ruleRegex: string) => {
    toggleRuleInQuery(ruleRegex, enabledKeywordGroups, searchParsingOptions, (msg) => {
      addToast('error', msg);
    });
  }, [toggleRuleInQuery, enabledKeywordGroups, searchParsingOptions, addToast]);

  // 稳定化 queryTerms
  const queryTerms = useMemo(
    () => currentQuery?.terms ?? null,
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [currentQuery?.id, currentQuery?.terms?.length]
  );

  const activeLog = selectedId ? loadedEntriesMap.get(selectedId) : undefined;

  return (
    <div className="flex flex-col h-full relative">
      {/* 搜索控制区 */}
      <div className="p-4 border-b border-border-subtle bg-bg-sidebar space-y-3 shrink-0 relative z-40">
        <SearchControls
          query={query}
          onQueryChange={setQuery}
          onSearch={handleSearch}
          onExport={handleExport}
          isFilterPaletteOpen={isFilterPaletteOpen}
          onFilterPaletteToggle={() => setIsFilterPaletteOpen(!isFilterPaletteOpen)}
          onFilterPaletteClose={() => setIsFilterPaletteOpen(false)}
          isSearching={isSearching}
          disabled={!activeWorkspace || !query.trim()}
          keywordGroups={keywordGroups}
          activeTerms={activeTerms}
          onToggleRule={handleToggleRule}
        />

        <SearchFilters
          filterOptions={filterOptions}
          onFilterOptionsChange={setFilterOptions}
          onReset={resetFilters}
        />

        <ActiveKeywords
          activeTerms={activeTerms}
          onRemoveTerm={removeTermFromQuery}
        />

        {/* 关键词统计面板 */}
        {searchSummary && keywordStats.length > 0 && (
          <KeywordStatsPanel
            keywords={keywordStats}
            totalMatches={searchSummary.totalMatches}
            searchDurationMs={searchSummary.searchDurationMs}
            onClose={() => dispatchSearchExec({ type: 'RESET' })}
          />
        )}
      </div>

      {/* 结果展示区 */}
      <div className="flex-1 flex overflow-hidden">
        <SearchResults
          virtualItems={virtualItems}
          totalSize={totalSize}
          measureElement={measureElement}
          loadedEntries={loadedEntries}
          firstPageOffset={firstPageOffset}
          selectedId={selectedId}
          onSelectId={setSelectedId}
          query={query}
          queryTerms={queryTerms}
          keywordGroups={enabledKeywordGroups}
          isFetchingNextPage={isFetchingNextPage}
          isFetchingPreviousPage={isFetchingPreviousPage}
          hasNextPage={hasNextPage}
          hasPreviousPage={hasPreviousPage}
          totalResultCount={totalResultCount}
          liveCount={liveCount}
          isSearching={isSearching}
          isInitialized={isInitialized}
          workspaceLoading={workspaceLoading}
          activeWorkspace={activeWorkspace}
          parentRef={parentRef}
        />

        {/* 日志详情面板 */}
        {activeLog && (
          <LogDetailPanel
            log={activeLog}
            query={query}
            queryTerms={queryTerms}
            keywordGroups={enabledKeywordGroups}
            onClose={() => setSelectedId(null)}
            onCopy={copyToClipboard}
          />
        )}
      </div>
    </div>
  );
};

export default SearchPage;
