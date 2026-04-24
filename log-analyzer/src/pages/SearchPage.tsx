import React, { useState, useEffect, useRef, useCallback, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { save } from '@tauri-apps/plugin-dialog';
import { useVirtualizer } from '@tanstack/react-virtual';
import { Copy, Loader2, X, FolderOpen, Search, SearchX } from 'lucide-react';
import { Button, EmptyState } from '../components/ui';
import { HybridLogRenderer } from '../components/renderers';
import { LogRow } from './SearchPage/components/LogRow';
import { KeywordStatsPanel } from '../components/search/KeywordStatsPanel';
import { logger } from '../utils/logger';
import { SearchQueryBuilder } from '../services/SearchQueryBuilder';
import { looksLikeRegexPattern, splitQueryByPipe } from '../utils/searchPatterns';
import { SearchQuery, SearchResultSummary } from '../types/search';
import { saveQuery, loadQuery, clearQuery } from '../services/queryStorage';
import { api } from '../services/api';
import { getFullErrorMessage } from '../services/errors';
import { useInfiniteSearch } from '../hooks/useInfiniteSearch';
import { useSearchListeners } from '../hooks/useSearchListeners';
import { useWorkspaceStore } from '../stores/workspaceStore';
import { useWorkspaceSelection } from '../hooks/useWorkspaceSelection';
import { useKeywordStore } from '../stores/keywordStore';
import { useAppStore } from '../stores/appStore';
import { useToast } from '../hooks/useToast';
import { useConfig } from '../hooks/useConfig';
import type {
  FilterOptions,
} from '../types/common';

// 新组件导入
import { SearchControls } from './SearchPage/components/SearchControls';
import { SearchFilters } from './SearchPage/components/SearchFilters';
import { ActiveKeywords } from './SearchPage/components/ActiveKeywords';
import { useSearchState } from './SearchPage/hooks/useSearchState';
import {
  buildStructuredQueryForSearch,
  deriveActiveTerms,
  syncStructuredQueryWithSettings,
  shouldResetStructuredQuery,
} from './SearchPage/utils/queryState';

// ============================================================================
/**
 * 搜索页面组件
 * 核心功能:
 * 1. 日志搜索 - 支持关键词和正则表达式
 * 2. 高级过滤 - 时间范围、日志级别、文件模式
 * 3. 虚拟滚动 - 高性能渲染大量日志
 * 4. 结果导出 - 支持CSV和JSON格式
 * 5. 日志高亮 - 搜索关键词和关键词组颜色高亮
 */

const SearchPage: React.FC = () => {
  // 从 Store 直接读取状态，消除 props drilling
  const keywordGroups = useKeywordStore((state) => state.keywordGroups);
  const { activeWorkspace } = useWorkspaceSelection();
  const searchInputRef = useRef<HTMLInputElement>(null);
  const { t } = useTranslation();
  const { showToast: addToast } = useToast();
  const { searchConfig, loadSearchConfig } = useConfig();
  const workspaceLoading = useWorkspaceStore((state) => state.loading);
  const isInitialized = useAppStore((state) => state.isInitialized);
  // 缓存启用的关键词组，避免每次渲染都重新计算
  const enabledKeywordGroups = useMemo(() =>
    keywordGroups.filter(g => g.enabled),
    [keywordGroups]
  );
  const searchParsingOptions = useMemo(
    () => ({
      caseSensitive: searchConfig?.case_sensitive ?? false,
      regexEnabled: searchConfig?.regex_enabled ?? true,
    }),
    [searchConfig]
  );
  
  // 搜索状态
  const [query, setQuery] = useState("");
  // 虚拟列表总行数（由 search-progress/search-complete 事件驱动，磁盘直写架构）
  const [liveCount, setLiveCount] = useState(0);
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [isFilterPaletteOpen, setIsFilterPaletteOpen] = useState(false);
  // 搜索执行状态（isSearching / searchSummary / keywordStats）统一通过 useSearchState hook 管理
  const [searchExec, dispatchSearchExec] = useSearchState();
  const { isSearching, searchSummary, keywordStats } = searchExec;

  // 磁盘直写搜索分页：searchId 由 api.searchLogs() 返回，即可启用 InfiniteQuery
  const [currentSearchId, setCurrentSearchId] = useState<string>('');
  
  // 防抖搜索触发器
  const [searchTrigger, setSearchTrigger] = useState(0);

  // ========== 流式无限搜索 (磁盘直写 + 滑动窗口分页) ==========
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

  // 磁盘直写架构：已加载的条目（从磁盘分页读取），用于虚拟列表渲染
  const loadedEntries = useMemo(
    () => infiniteSearchData?.pages.flatMap(page => page.results) ?? [],
    [infiniteSearchData]
  );

  const totalResultCount = useMemo(() => {
    const pagedTotal = infiniteSearchData?.pages[0]?.totalCount;
    if (typeof pagedTotal === 'number' && pagedTotal >= 0) {
      return pagedTotal;
    }
    return liveCount;
  }, [infiniteSearchData, liveCount]);

  // 首页偏移量：maxPages 裁剪旧页面后，loadedEntries[0] 对应的虚拟索引
  const firstPageOffset = useMemo(
    () => (infiniteSearchData?.pageParams?.[0] as number) ?? 0,
    [infiniteSearchData]
  );

  // O(1) 日志查找 Map，替代 loadedEntries.find() 的 O(n) 线性查找
  const loadedEntriesMap = useMemo(
    () => new Map(loadedEntries.map(entry => [entry.id, entry])),
    [loadedEntries]
  );

  const currentSearchIdRef = useRef(currentSearchId);
  currentSearchIdRef.current = currentSearchId;

  const loadedEntriesLengthRef = useRef(loadedEntries.length);
  loadedEntriesLengthRef.current = loadedEntries.length;

  const refetchSearchPagesRef = useRef(refetchSearchPages);
  refetchSearchPagesRef.current = refetchSearchPages;

  const lastSearchPageRefetchTimeRef = useRef(0);

  // 处理无限搜索错误
  useEffect(() => {
    if (infiniteSearchError) {
      console.error('Infinite search error:', infiniteSearchError);
      addToast('error', `分页加载失败: ${infiniteSearchError.message}`);
    }
  }, [infiniteSearchError, addToast]);

  const lastFetchNextPageTimeRef = useRef(0);

  // 给每个关键词分配颜色 - 使用新的设计系统
  const keywordColors = useMemo(
    () => ['#3B82F6', '#8B5CF6', '#22C55E', '#F59E0B', '#EC4899', '#06B6D4'],
    []
  );
  
  // 结构化查询状态
  const [currentQuery, setCurrentQuery] = useState<SearchQuery | null>(null);
  
  // 高级过滤状态
  const [filterOptions, setFilterOptions] = useState<FilterOptions>({
    timeRange: { start: null, end: null },
    levels: [],
    filePattern: ""
  });
  
  const parentRef = useRef<HTMLDivElement>(null);

  // 使用 ref 存储虚拟滚动器实例，避免声明顺序问题
  const rowVirtualizerRef = useRef<ReturnType<typeof useVirtualizer<HTMLDivElement, Element>>>(null);
  // 清理未完成的滚动定时器，防止组件卸载后访问已释放的 ref
  const scrollTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    return () => {
      if (scrollTimeoutRef.current) {
        clearTimeout(scrollTimeoutRef.current);
        scrollTimeoutRef.current = null;
      }
    };
  }, []);

  // 使用 ref 存储分页状态的最新值，避免 scroll listener 频繁注销/重注册
  // 节流：避免 search-progress 事件风暴导致高频 setState
  const lastProgressTimeRef = useRef(0);

  // 监听搜索事件 — 通过 useSearchListeners hook 注册 Tauri 事件
  useSearchListeners({
    onProgress: useCallback((count: number) => {
      const now = Date.now();
      if (now - lastProgressTimeRef.current >= 200) {
        lastProgressTimeRef.current = now;
        setLiveCount(count);
      }
      if (
        count > 0 &&
        currentSearchIdRef.current &&
        loadedEntriesLengthRef.current < count &&
        now - lastSearchPageRefetchTimeRef.current >= 300
      ) {
        lastSearchPageRefetchTimeRef.current = now;
        refetchSearchPagesRef.current().catch((error) => {
          logger.error('Refetch search page after progress failed:', error);
        });
      }
      // 注意：被节流丢弃的中间值不影响最终结果，
      // 因为 onComplete 会设置精确的最终计数
    }, []),

    onSummary: useCallback((summary: SearchResultSummary) => {
      dispatchSearchExec({ type: 'SUMMARY', summary, keywordColors });
    }, [keywordColors, dispatchSearchExec]),

    onComplete: useCallback((count: number) => {
      dispatchSearchExec({ type: 'COMPLETE' });
      setLiveCount(count);
      if (currentSearchIdRef.current) {
        refetchSearchPagesRef.current().catch((error) => {
          logger.error('Refetch search page after completion failed:', error);
        });
      }
      if (scrollTimeoutRef.current) {
        clearTimeout(scrollTimeoutRef.current);
      }
      scrollTimeoutRef.current = setTimeout(() => {
        if (rowVirtualizerRef.current) {
          try { rowVirtualizerRef.current.scrollToIndex(0); } catch { /* silent */ }
        }
      }, 50);
      if (count > 0) {
        addToast('success', `找到 ${count.toLocaleString()} 条日志`);
      } else {
        addToast('info', t('search.no_results'));
      }
    }, [addToast, t, dispatchSearchExec]),

    onError: useCallback((errorMsg: string) => {
      dispatchSearchExec({ type: 'ERROR' });
      addToast('error', t('search.error', { message: errorMsg }));
    }, [addToast, t, dispatchSearchExec]),

    onCancelled: useCallback((searchId: string) => {
      if (searchId !== currentSearchIdRef.current) return;
      dispatchSearchExec({ type: 'ERROR' });
      addToast('info', t('search.cancelled', '搜索已取消'));
    }, [addToast, t, dispatchSearchExec]),

    onTimeout: useCallback(() => {
      dispatchSearchExec({ type: 'ERROR' });
    }, [dispatchSearchExec]),

    onStart: useCallback(() => {
      dispatchSearchExec({ type: 'START' });
      setLiveCount(0);
      setSelectedId(null);
      lastSearchPageRefetchTimeRef.current = 0;
      if (parentRef.current) parentRef.current.scrollTop = 0;
      if (rowVirtualizerRef.current) rowVirtualizerRef.current.scrollOffset = 0;
    }, [dispatchSearchExec]),
  });

  // 加载保存的查询
  useEffect(() => {
    const saved = loadQuery();
    if (saved) {
      setCurrentQuery(saved);
      const builder = SearchQueryBuilder.import(JSON.stringify(saved));
      if (builder) setQuery(builder.toQueryString());
    }
  }, []);

  // 自动保存查询变化
  useEffect(() => {
    if (currentQuery) {
      saveQuery(currentQuery);
    } else {
      clearQuery();
    }
  }, [currentQuery]);

  useEffect(() => {
    loadSearchConfig().catch((err) => {
      logger.warn('Failed to load search config for SearchPage, using defaults', err);
    });
  }, [loadSearchConfig]);

  // 监听查询变化，自动触发搜索（防抖500ms）
  useEffect(() => {
    if (!query.trim()) {
      setLiveCount(0);
      setCurrentSearchId('');
      return;
    }

    const timer = setTimeout(() => {
      setSearchTrigger(prev => prev + 1);
    }, 500);
    
    return () => clearTimeout(timer);
  }, [query]);

  // 搜索触发器变化时执行搜索
  // 通过 handleSearchRef 读取最新版本，避免旧闭包，同时不将 handleSearch 加入 deps 导致额外触发
  useEffect(() => {
    if (searchTrigger > 0 && activeWorkspace) {
      handleSearchRef.current();
    }
  }, [searchTrigger, activeWorkspace]);

  // 工作区切换时获取时间范围
  useEffect(() => {
    if (!activeWorkspace) {
      // 清空时间范围
      setFilterOptions(prev => ({
        ...prev,
        timeRange: { start: null, end: null }
      }));
      return;
    }

    // 获取工作区的时间范围
    let isMounted = true;

    const fetchTimeRange = async () => {
      try {
        const timeRange = await api.getWorkspaceTimeRange(activeWorkspace.id);
        if (!isMounted) return;
        if (timeRange.minTimestamp && timeRange.maxTimestamp) {
          // 将 ISO 8601 格式转换为 datetime-local 格式 (YYYY-MM-DDTHH:mm)
          const minDate = new Date(timeRange.minTimestamp);
          const maxDate = new Date(timeRange.maxTimestamp);

          const formatDateTimeLocal = (date: Date) => {
            const year = date.getFullYear();
            const month = String(date.getMonth() + 1).padStart(2, '0');
            const day = String(date.getDate()).padStart(2, '0');
            const hours = String(date.getHours()).padStart(2, '0');
            const minutes = String(date.getMinutes()).padStart(2, '0');
            return `${year}-${month}-${day}T${hours}:${minutes}`;
          };

          setFilterOptions(prev => ({
            ...prev,
            timeRange: {
              start: formatDateTimeLocal(minDate),
              end: formatDateTimeLocal(maxDate)
            }
          }));
        }
      } catch (error) {
        if (isMounted) {
          console.warn('Failed to fetch workspace time range:', error);
          // 失败时不清空已有的时间范围，保持用户手动设置
        }
      }
    };

    fetchTimeRange();
    return () => {
      isMounted = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [activeWorkspace?.id]);
  // 仅依赖 ID 而非对象引用，避免同一工作区的重复请求

  // 用 ref 存储最新的 handleSearch，避免 useEffect 中使用 eslint-disable
  const handleSearchRef = useRef<() => Promise<void>>(async () => {});

  /**
   * 执行搜索 - 使用 useCallback 确保稳定性
   */
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

    // 重置状态
    setLiveCount(0);
    setCurrentSearchId('');
    dispatchSearchExec({ type: 'START' });
    setSelectedId(null);

    // 重置滚动位置到顶部
    if (parentRef.current) {
      parentRef.current.scrollTop = 0;
    }
    if (rowVirtualizerRef.current && rowVirtualizerRef.current.scrollOffset !== 0) {
      rowVirtualizerRef.current.scrollOffset = 0;
    }

    try {
      const runtimeSearchConfig = searchConfig ?? await loadSearchConfig().catch((err) => {
        logger.warn('Failed to refresh search config before executing search, using defaults', err);
        return null;
      });
      const parsingOptions = {
        caseSensitive: runtimeSearchConfig?.case_sensitive ?? false,
        regexEnabled: runtimeSearchConfig?.regex_enabled ?? true,
      };
      const structuredQuery = buildStructuredQueryForSearch(
        trimmedQuery,
        currentQuery,
        enabledKeywordGroups,
        parsingOptions
      );

      // 构建过滤器对象
      const filters = {
        time_start: filterOptions.timeRange.start,
        time_end: filterOptions.timeRange.end,
        levels: filterOptions.levels,
        file_pattern: filterOptions.filePattern || null
      };

      // 后端返回 search_id，前端凭此 ID 从磁盘分页读取搜索结果
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
    currentQuery,
    searchConfig,
    loadSearchConfig,
    addToast,
    dispatchSearchExec,
    t,
  ]);

  // 同步 handleSearch 到 ref，供 useEffect 读取最新版本（避免旧闭包）
  useEffect(() => {
    handleSearchRef.current = handleSearch;
  }, [handleSearch]);

  /**
   * 重置过滤器
   */
  const handleResetFilters = () => {
    setFilterOptions({
      timeRange: { start: null, end: null },
      levels: [],
      filePattern: ""
    });
    addToast('info', '过滤器已重置');
  };

  /**
   * 从查询中删除单个关键词
   */
  const removeTermFromQuery = useCallback((termToRemove: string) => {
    if (currentQuery) {
      const builder = SearchQueryBuilder.import(JSON.stringify(currentQuery));
      if (builder) {
        const existing = builder.findTermByValue(termToRemove);
        if (existing) {
          builder.removeTerm(existing.id);
          const nextQuery = builder.getQuery();
          setCurrentQuery(nextQuery);
          setQuery(builder.toQueryString());
          return;
        }
      }
    }

    const terms = splitQueryByPipe(query);
    const newTerms = terms.filter(t => t.toLowerCase() !== termToRemove.toLowerCase());
    setQuery(newTerms.join('|'));
  }, [query, currentQuery]);

  /**
   * 切换规则在查询中的状态
   * | 仅作为分隔符，多个关键词用 OR 逻辑组合（匹配任意一个）
   */
  const toggleRuleInQuery = useCallback((ruleRegex: string) => {
    // 创建或更新查询构建器
    const builder = currentQuery
      ? (SearchQueryBuilder.import(JSON.stringify(currentQuery)) ?? SearchQueryBuilder.fromString(query, enabledKeywordGroups))
      : SearchQueryBuilder.fromString(query, enabledKeywordGroups);

    // 检查是否已存在
    const existing = builder.findTermByValue(ruleRegex);

    if (existing) {
      // 已存在：切换启用状态
      builder.toggleTerm(existing.id);
    } else {
      // 不存在：添加新项
      builder.addTerm(ruleRegex, {
        source: 'preset',
        isRegex: searchParsingOptions.regexEnabled && looksLikeRegexPattern(ruleRegex),
        operator: 'OR',
        caseSensitive: searchParsingOptions.caseSensitive,
      });
    }

    // 验证查询
    const validation = builder.validate();
    if (!validation.isValid) {
      const errors = validation.issues
        .filter(i => i.severity === 'error')
        .map(i => i.message)
        .join(', ');
      console.error('Query validation failed:', errors);
      addToast('error', `查询验证失败: ${errors}`);
      return;
    }

    // 更新状态
    const newQuery = syncStructuredQueryWithSettings(builder.getQuery(), searchParsingOptions);
    setCurrentQuery(newQuery);
    
    // 更新查询字符串（用于显示）
    const queryString = builder.toQueryString();
    setQuery(queryString);
  }, [query, enabledKeywordGroups, currentQuery, addToast, searchParsingOptions]);

  const activeTerms = useMemo(
    () => deriveActiveTerms(query, currentQuery),
    [currentQuery, query]
  );

  const handleQueryChange = useCallback((nextQuery: string) => {
    setQuery(nextQuery);

    if (shouldResetStructuredQuery(nextQuery, currentQuery)) {
      setCurrentQuery(null);
    }
  }, [currentQuery]);

  /**
   * 复制到剪贴板
   */
  const copyToClipboard = useCallback((text: string) => {
    navigator.clipboard.writeText(text).then(() => addToast('success', 'Copied'));
  }, [addToast]);
  
  /**
   * 尝试格式化JSON
   */
  const tryFormatJSON = (content: string) => { 
    try { 
      const start = content.indexOf('{'); 
      if (start === -1) return content; 
      const jsonPart = content.substring(start); 
      const obj = JSON.parse(jsonPart); 
      return JSON.stringify(obj, null, 2); 
    } catch { 
      return content; 
    } 
  };
  
  /**
   * 导出搜索结果
   */
  const handleExport = async (format: 'csv' | 'json') => {
    if (loadedEntries.length === 0) {
      addToast('error', '没有可导出的数据');
      return;
    }

    try {
      const defaultPath = `log-export-${Date.now()}.${format}`;
      const savePath = await save({
        defaultPath,
        filters: [{
          name: format.toUpperCase(),
          extensions: [format]
        }]
      });

      if (!savePath) {
        // 用户取消
        return;
      }

      logger.debug('Exporting to:', savePath);
      await api.exportResults({
        results: loadedEntries,
        format,
        savePath
      });

      addToast('success', `已导出 ${loadedEntries.length} 条日志到 ${format.toUpperCase()}`);
    } catch (e) {
      logger.error('Export error:', e);
      addToast('error', `导出失败: ${getFullErrorMessage(e)}`);
    }
  };
  
  /**
   * 虚拟滚动配置
   * 优化：固定 estimateSize 避免依赖 logs，添加边界条件处理
   * 将结果存储到 ref 中以便在其他 useEffect 中访问
   */
  const rowVirtualizer = useVirtualizer({
    count: totalResultCount,
    getScrollElement: () => parentRef.current,
    estimateSize: useCallback(() => 48, []), // 调整为 48px，更接近实际行高
    measureElement: useCallback((element: Element | null) => {
      return element?.getBoundingClientRect().height ?? 48;
    }, []),
    overscan: 20,
  });

  // 将虚拟滚动器存储到 ref 中（必须在 useEffect 中赋值，避免 render 阶段副作用）
  useEffect(() => {
    rowVirtualizerRef.current = rowVirtualizer;
  }, [rowVirtualizer]);

  // 范围加载：当可见行包含未加载的数据时，自动触发分页加载。
  // 解决用户滚动到中间区域时只看到骨架屏、永远无法加载的问题。
  const virtualItems = rowVirtualizer.getVirtualItems();
  const lastVisibleIndex = virtualItems.length > 0 ? virtualItems[virtualItems.length - 1].index : -1;
  const firstVisibleIndex = virtualItems.length > 0 ? virtualItems[0].index : -1;

  useEffect(() => {
    if (lastVisibleIndex < 0) return;

    const loadedEndIndex = firstPageOffset + loadedEntries.length;
    const now = Date.now();

    // 向前加载：最后一个可见行接近已加载数据的尾部边界
    if (lastVisibleIndex >= loadedEndIndex - 50 && hasNextPage && !isFetchingNextPage) {
      if (now - lastFetchNextPageTimeRef.current >= 500) {
        lastFetchNextPageTimeRef.current = now;
        fetchNextPage().catch(err => {
          logger.error('Range-based fetchNextPage failed:', err);
        });
      }
    }

    // 向后加载：第一个可见行接近已加载数据的头部边界
    if (firstVisibleIndex <= firstPageOffset + 50 && hasPreviousPage && !isFetchingPreviousPage) {
      if (now - lastFetchNextPageTimeRef.current >= 500) {
        lastFetchNextPageTimeRef.current = now;
        fetchPreviousPage().catch(err => {
          logger.error('Range-based fetchPreviousPage failed:', err);
        });
      }
    }
  }, [lastVisibleIndex, firstVisibleIndex, firstPageOffset, loadedEntries.length, hasNextPage, isFetchingNextPage, hasPreviousPage, isFetchingPreviousPage, fetchNextPage, fetchPreviousPage]);
  
  const activeLog = selectedId ? loadedEntriesMap.get(selectedId) : undefined;

  return (
    <div className="flex flex-col h-full relative">
      {/* 搜索控制区 */}
      <div className="p-4 border-b border-border-subtle bg-bg-sidebar space-y-3 shrink-0 relative z-40">
        <SearchControls
          query={query}
          onQueryChange={handleQueryChange}
          onSearch={handleSearch}
          onExport={handleExport}
          isFilterPaletteOpen={isFilterPaletteOpen}
          onFilterPaletteToggle={() => setIsFilterPaletteOpen(!isFilterPaletteOpen)}
          onFilterPaletteClose={() => setIsFilterPaletteOpen(false)}
          isSearching={isSearching}
          disabled={!activeWorkspace || !query.trim()}
          searchInputRef={searchInputRef}
          keywordGroups={keywordGroups}
          activeTerms={activeTerms}
          onToggleRule={toggleRuleInQuery}
        />

        <SearchFilters
          filterOptions={filterOptions}
          onFilterOptionsChange={setFilterOptions}
          onReset={handleResetFilters}
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
        {/* 日志列表 */}
        <div ref={parentRef} className="flex-1 overflow-auto bg-bg-main scrollbar-thin" style={{ willChange: 'transform' }}>
          {/* 表头 - 优化视觉层次 */}
          <div className="sticky top-0 z-10 grid grid-cols-[50px_160px_150px_1fr] px-3 py-2 bg-bg-elevated border-b border-border-base text-xs font-bold text-text-muted uppercase tracking-wider">
            <div>{t('search.table.level', '级别')}</div>
            <div>{t('search.table.time', '时间')}</div>
            <div>{t('search.table.file', '文件')}</div>
            <div>{t('search.table.content', '内容')}</div>
          </div>
          
          {/* 虚拟滚动列表 - 磁盘直写架构：count=liveCount，按需从磁盘加载可见页 */}
          <div style={{ height: `${rowVirtualizer.getTotalSize()}px`, width: '100%', position: 'relative' }}>
            {rowVirtualizer.getVirtualItems().map((virtualRow) => {
              const log = loadedEntries[virtualRow.index - firstPageOffset];
              if (!log) {
                // 未从磁盘加载的行显示骨架屏，搜索进行中时数据将按需加载
                return (
                  <div
                    key={virtualRow.key}
                    ref={rowVirtualizer.measureElement}
                    data-index={virtualRow.index}
                    style={{
                      transform: `translateY(${virtualRow.start}px)`,
                      minHeight: `${virtualRow.size}px`,
                    }}
                    className="absolute top-0 left-0 w-full grid grid-cols-[50px_160px_150px_1fr] px-3 py-1.5 border-b border-border-subtle"
                  >
                    <div className="h-4 bg-bg-elevated/60 rounded animate-pulse w-8" />
                    <div className="h-4 bg-bg-elevated/60 rounded animate-pulse w-32" />
                    <div className="h-4 bg-bg-elevated/60 rounded animate-pulse w-24" />
                    <div className="h-4 bg-bg-elevated/60 rounded animate-pulse w-3/4" />
                  </div>
                );
              }
              return (
                <LogRow
                  key={virtualRow.key}
                  log={log}
                  isActive={log.id === selectedId}
                  onClick={() => setSelectedId(log.id)}
                  query={query}
                  queryTerms={currentQuery?.terms ?? null}
                  keywordGroups={enabledKeywordGroups}
                  virtualIndex={virtualRow.index}
                  virtualStart={virtualRow.start}
                  virtualSize={virtualRow.size}
                  measureElement={rowVirtualizer.measureElement}
                />
              );
            })}
          </div>

          {/* 分页加载指示器 */}
          {(isFetchingNextPage || isFetchingPreviousPage) && (
            <div className="flex items-center justify-center py-4 bg-bg-sidebar/50 border-t border-border-base">
              <Loader2 className="animate-spin text-primary mr-2" size={16} />
              <span className="text-sm text-text-muted">
                {isFetchingNextPage
                  ? `正在加载更多结果... (${loadedEntries.length.toLocaleString()} 条已加载)`
                  : '正在加载历史结果...'}
              </span>
            </div>
          )}

          {/* 全部加载完成提示 */}
          {!!currentSearchId && !hasNextPage && !isFetchingNextPage && !hasPreviousPage && totalResultCount > 0 && (
            <div className="flex items-center justify-center py-3 text-text-muted text-xs">
              共 {totalResultCount.toLocaleString()} 条结果
            </div>
          )}

          {/* 空状态 - 根据不同场景显示不同提示 */}
          {liveCount === 0 && !isSearching && (
            <div className="flex items-center justify-center h-full min-h-[200px]">
              {(!isInitialized || workspaceLoading) ? (
                /* 场景1: 应用未初始化或工作区正在加载 */
                <div className="flex flex-col items-center gap-3 text-text-dim">
                  <Loader2 className="animate-spin text-primary" size={32} />
                  <p className="text-sm font-medium text-text-muted">
                    {t('search.empty_state.workspace_loading', '工作区加载中')}
                  </p>
                </div>
              ) : !activeWorkspace ? (
                /* 场景2: 没有工作区 */
                <EmptyState
                  icon={FolderOpen}
                  title={t('search.empty_state.no_workspace', '没有工作区')}
                  description={t('search.empty_state.no_workspace_hint', '请先创建或导入工作区以开始搜索日志')}
                />
              ) : !query.trim() ? (
                /* 场景3: 没有输入搜索关键词 */
                <EmptyState
                  icon={Search}
                  title={t('search.empty_state.no_query', '输入搜索关键词')}
                  description={t('search.empty_state.no_query_hint', '在上方输入框中输入关键词进行搜索')}
                />
              ) : (
                /* 场景4: 搜索已完成但没有结果 */
                <EmptyState
                  icon={SearchX}
                  title={t('search.empty_state.no_results', '没有搜索结果')}
                  description={t('search.empty_state.no_results_hint', '尝试调整搜索条件或关键词')}
                />
              )}
            </div>
          )}
        </div>
        
        {/* 日志详情面板 - 优化视觉层次 */}
        {activeLog && (
          <div className="w-[450px] bg-bg-sidebar border-l border-border-subtle flex flex-col shrink-0 shadow-elevated z-20 animate-slide-in">
            <div className="h-10 border-b border-border-subtle flex items-center justify-between px-4 bg-bg-elevated">
              <span className="text-xs font-bold text-text-muted uppercase tracking-wide">{t('search.inspector.title', '日志详情')}</span>
              <div className="flex gap-1">
                <Button
                  variant="ghost"
                  className="h-11 w-11 p-0"
                  onClick={() => copyToClipboard(activeLog.content)}
                  aria-label={t('search.inspector.copy', '复制内容')}
                >
                  <Copy size={14}/>
                </Button>
                <Button
                  variant="ghost"
                  className="h-11 w-11 p-0"
                  onClick={() => setSelectedId(null)}
                  aria-label={t('search.inspector.close', '关闭面板')}
                >
                  <X size={14}/>
                </Button>
              </div>
            </div>
            <div className="flex-1 overflow-auto p-4 font-mono text-xs">
              <div className="bg-bg-main p-3 rounded border border-border-base mb-4">
                <div className="text-text-dim text-xs uppercase mb-1">{t('search.inspector.message', '消息内容')}</div>
                <div className="text-text-main whitespace-pre-wrap break-all leading-relaxed">
                  <HybridLogRenderer 
                    text={tryFormatJSON(activeLog.content)} 
                    query={query} 
                    queryTerms={currentQuery?.terms ?? null}
                    keywordGroups={enabledKeywordGroups} 
                  />
                </div>
              </div>
              <div className="p-2 bg-bg-card border border-border-base rounded mb-2">
                <div className="text-xs text-text-dim uppercase">{t('search.inspector.file', '文件')}</div>
                <div className="break-all text-text-main">{activeLog?.real_path || t('search.inspector.not_available', '无')}</div>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

export default SearchPage;
