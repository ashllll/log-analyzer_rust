import React, { useState, useEffect, useRef, useCallback, useDeferredValue, memo, useMemo } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { save } from '@tauri-apps/plugin-dialog';
import { useVirtualizer } from '@tanstack/react-virtual';
import {
  Search,
  Download,
  Filter,
  X,
  ChevronDown,
  Hash,
  Copy,
  Loader2,
  RotateCcw
} from 'lucide-react';
import { Button, Input } from '../components/ui';
import { HybridLogRenderer } from '../components/renderers';
import { FilterPalette } from '../components/modals';
import { KeywordStatsPanel } from '../components/search/KeywordStatsPanel';
import { logger } from '../utils/logger';
import { cn } from '../utils/classNames';
import { SearchQueryBuilder } from '../services/SearchQueryBuilder';
import { SearchQuery, SearchResultSummary, KeywordStat } from '../types/search';
import { saveQuery, loadQuery } from '../services/queryStorage';
import { api } from '../services/api';
import { getFullErrorMessage } from '../services/errors';
import type {
  LogEntry,
  FilterOptions,
  Workspace,
  KeywordGroup,
  ToastType
} from '../types/common';

/**
 * 搜索页面组件
 * 核心功能:
 * 1. 日志搜索 - 支持关键词和正则表达式
 * 2. 高级过滤 - 时间范围、日志级别、文件模式
 * 3. 虚拟滚动 - 高性能渲染大量日志
 * 4. 结果导出 - 支持CSV和JSON格式
 * 5. 日志高亮 - 搜索关键词和关键词组颜色高亮
 */

interface SearchPageProps {
  keywordGroups: KeywordGroup[];
  addToast: (type: ToastType, message: string) => void;
  searchInputRef: React.RefObject<HTMLInputElement | null>;
  activeWorkspace: Workspace | null;
}

/**
 * 虚拟行组件 Props
 */
interface LogRowProps {
  log: LogEntry;
  isActive: boolean;
  onClick: () => void;
  query: string;
  keywordGroups: KeywordGroup[];
  virtualStart: number;
  virtualKey: React.Key;
  measureRef: (node: Element | null) => void;
}

/**
 * 虚拟行组件 - 使用 React.memo 优化
 * 只有当 log、isActive、query 或 keywordGroups 变化时才重新渲染
 */
const LogRow = memo<LogRowProps>(({ 
  log, 
  isActive, 
  onClick, 
  query, 
  keywordGroups, 
  virtualStart, 
  virtualKey,
  measureRef 
}) => {
  return (
    <div 
      key={virtualKey}
      data-index={virtualKey} 
      ref={measureRef} 
      onClick={onClick} 
      style={{ transform: `translateY(${virtualStart}px)` }} 
      className={cn(
        "absolute top-0 left-0 w-full grid grid-cols-[50px_160px_200px_1fr] px-3 py-0.5 border-b border-border-base/40 cursor-pointer text-[11px] font-mono hover:bg-bg-hover transition-colors items-start", 
        isActive && "bg-blue-500/10 border-l-2 border-l-primary"
      )}
    >
      <div className={cn(
        "font-bold", 
        log.level === 'ERROR' ? 'text-red-400' : 
        log.level === 'WARN' ? 'text-yellow-400' : 
        'text-blue-400'
      )}>
        {log.level.substring(0,1)}
      </div>
      <div className="text-text-muted whitespace-nowrap text-[10px]">
        {log.timestamp}
      </div>
      <div className="text-text-muted break-all pr-2 text-[10px] leading-tight">
        {log.file}:{log.line}
      </div>
      <div className="text-text-main whitespace-pre-wrap break-all leading-tight pr-2">
        <HybridLogRenderer 
          text={log.content} 
          query={query} 
          keywordGroups={keywordGroups} 
        />
      </div>
    </div>
  );
}, (prevProps, nextProps) => {
  // 返回 true 表示 props 相同，不需要重渲染
  return (
    prevProps.log === nextProps.log &&
    prevProps.isActive === nextProps.isActive &&
    prevProps.query === nextProps.query &&
    prevProps.keywordGroups === nextProps.keywordGroups &&
    prevProps.virtualStart === nextProps.virtualStart
  );
});

const SearchPage: React.FC<SearchPageProps> = ({ 
  keywordGroups, 
  addToast, 
  searchInputRef, 
  activeWorkspace 
}) => {
  // 缓存启用的关键词组，避免每次渲染都重新计算
  const enabledKeywordGroups = useMemo(() => 
    keywordGroups.filter(g => g.enabled),
    [keywordGroups]
  );
  
  // 搜索状态
  const [query, setQuery] = useState("");
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const deferredLogs = useDeferredValue(logs); // 使用延迟值优化渲染
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [isFilterPaletteOpen, setIsFilterPaletteOpen] = useState(false);
  const [isSearching, setIsSearching] = useState(false);
  
  // 防抖搜索触发器
  const [searchTrigger, setSearchTrigger] = useState(0);

  // 刷新状态管理
  const [isRefreshing, setIsRefreshing] = useState(false);
  const isRefreshingRef = useRef(false);
  const lastRefreshTimeRef = useRef(0);
  const lastScrollTopRef = useRef(0);
  const refreshLogsRef = useRef<(() => void) | null>(null);
  const isNearBottomRef = useRef<((scrollTop: number, clientHeight: number, scrollHeight: number) => boolean) | null>(null);
  const REFRESH_THRESHOLD = 50;
  const REFRESH_DEBOUNCE_MS = 1000;

  // 搜索统计状态
  const [searchSummary, setSearchSummary] = useState<SearchResultSummary | null>(null);
  const [keywordStats, setKeywordStats] = useState<KeywordStat[]>([]);
  
  // 给每个关键词分配颜色
  const keywordColors = useMemo(
    () => ['#3b82f6', '#8b5cf6', '#10b981', '#f59e0b', '#ec4899', '#06b6d4'],
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

  // ResizeObserver优化：监听容器尺寸变化，即时更新虚拟滚动
  useEffect(() => {
    if (!parentRef.current) return;
    
    const resizeObserver = new ResizeObserver(() => {
      // 当容器尺寸变化时，虚拟滚动会自动重新计算
      // 这里无需额外操作，useVirtualizer会自动响应
    });
    
    resizeObserver.observe(parentRef.current);
    
    return () => {
      resizeObserver.disconnect();
    };
  }, []);

  // 滚动事件监听（用于底部刷新）
  useEffect(() => {
    const element = parentRef.current;
    if (!element) return;

    const handleScrollEvent = (event: Event) => {
      const target = event.target as HTMLDivElement;
      const { scrollTop, clientHeight, scrollHeight } = target;
      
      lastScrollTopRef.current = scrollTop;
      
      if (isRefreshingRef.current) return;
      
      if (isNearBottomRef.current && !isNearBottomRef.current(scrollTop, clientHeight, scrollHeight)) return;
      
      const now = Date.now();
      if (now - lastRefreshTimeRef.current < REFRESH_DEBOUNCE_MS) return;
      
      lastRefreshTimeRef.current = now;
      isRefreshingRef.current = true;
      setIsRefreshing(true);
      
      if (refreshLogsRef.current) {
        refreshLogsRef.current();
      }
    };

    element.addEventListener('scroll', handleScrollEvent, { passive: true });

    return () => {
      element.removeEventListener('scroll', handleScrollEvent);
    };
  }, []);

  // 监听搜索事件
  useEffect(() => {
    const abortController = new AbortController();
    const unlisteners: Array<() => void> = [];

    const setupListeners = async () => {
      try {
        const [
          resultsUnlisten,
          summaryUnlisten,
          completeUnlisten,
          errorUnlisten,
          startUnlisten
        ] = await Promise.all([
          listen<LogEntry[]>('search-results', (e) => {
            if (!e.payload || !Array.isArray(e.payload)) {
              console.warn('Invalid search results payload:', e.payload);
              return;
            }
            setLogs(prev => [...prev, ...e.payload]);
          }),
          listen<SearchResultSummary>('search-summary', (e) => {
            const summary = e.payload;
            if (!summary) {
              console.warn('Invalid search summary payload:', e.payload);
              return;
            }
            setSearchSummary(summary);

            const stats: KeywordStat[] = summary.keywordStats.map((stat, index) => ({
              ...stat,
              color: keywordColors[index % keywordColors.length]
            }));
            setKeywordStats(stats);
          }),
          listen('search-complete', (e) => {
            setIsSearching(false);
            const count = typeof e.payload === 'number' ? e.payload : 0;

            if (query.trim() && activeWorkspace) {
              // TODO: 添加到 API 层
              // await api.addSearchHistory(...) when implemented
              invoke('add_search_history', {
                query: query.trim(),
                workspaceId: activeWorkspace.id,
                resultCount: count,
              }).catch(err => {
                logger.error('Failed to save search history:', getFullErrorMessage(err));
              });
            }

            // 滚动到顶部显示最新结果
            setTimeout(() => {
              if (deferredLogs.length > 0 && rowVirtualizerRef.current) {
                try {
                  rowVirtualizerRef.current.scrollToIndex(0);
                } catch {
                  // 静默处理滚动错误
                }
              }
            }, 50);

            if (count > 0) {
              addToast('success', `找到 ${count.toLocaleString()} 条日志`);
            } else {
              addToast('info', '未找到匹配的日志');
            }
          }),
          listen('search-error', (e) => {
            setIsSearching(false);
            const errorMsg = String(e.payload);
            addToast('error', `搜索失败: ${errorMsg}`);
          }),
          listen('search-start', () => {
            // 清空并重置滚动
            setLogs([]);
            setSearchSummary(null);
            setKeywordStats([]);
            if (parentRef.current) {
              parentRef.current.scrollTop = 0;
            }
            if (rowVirtualizerRef.current) {
              rowVirtualizerRef.current.scrollOffset = 0;
            }
          })
        ]);

        if (abortController.signal.aborted) {
          [resultsUnlisten, summaryUnlisten, completeUnlisten, errorUnlisten, startUnlisten].forEach(unlisten => unlisten());
          return;
        }

        unlisteners.push(...[resultsUnlisten, summaryUnlisten, completeUnlisten, errorUnlisten, startUnlisten]);
      } catch (error) {
        if (!abortController.signal.aborted) {
          console.error('Failed to setup event listeners:', error);
        }
      }
    };

    setupListeners();

    return () => {
      abortController.abort();
      unlisteners.forEach(unlisten => {
        try {
          unlisten();
        } catch (error) {
          console.debug('Failed to unlisten:', error);
        }
      });
    };
  }, [addToast, keywordColors, rowVirtualizerRef, deferredLogs.length, parentRef, query, activeWorkspace]);

  // 加载保存的查询
  useEffect(() => {
    const saved = loadQuery();
    if (saved) {
      setCurrentQuery(saved);
      const builder = SearchQueryBuilder.import(JSON.stringify(saved));
      setQuery(builder.toQueryString());
    }
  }, []);

  // 自动保存查询变化
  useEffect(() => {
    if (currentQuery) {
      saveQuery(currentQuery);
    }
  }, [currentQuery]);

  // 监听查询变化，自动触发搜索（防抖500ms）
  useEffect(() => {
    if (!query.trim()) {
      setLogs([]);
      return;
    }
    
    const timer = setTimeout(() => {
      setSearchTrigger(prev => prev + 1);
    }, 500);
    
    return () => clearTimeout(timer);
  }, [query]);

  // 搜索触发器变化时执行搜索
  useEffect(() => {
    if (searchTrigger > 0 && activeWorkspace) {
      handleSearch();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [searchTrigger, activeWorkspace]);

  /**
   * 检查是否接近滚动底部
   * @param scrollTop - 当前滚动位置
   * @param clientHeight - 视口高度
   * @param scrollHeight - 内容总高度
   * @returns 是否接近底部
   */
  const isNearBottom = useCallback((
    scrollTop: number,
    clientHeight: number,
    scrollHeight: number
  ): boolean => {
    return scrollHeight - scrollTop - clientHeight <= REFRESH_THRESHOLD;
  }, []);

  // 存储 isNearBottom 到 ref 供滚动事件使用
  useEffect(() => {
    isNearBottomRef.current = isNearBottom;
  }, [isNearBottom]);

  /**
   * 刷新日志数据
   * 追加获取新数据，不替换现有结果
   */
  const refreshLogs = useCallback(async () => {
    if (!activeWorkspace) {
      isRefreshingRef.current = false;
      setIsRefreshing(false);
      return;
    }

    const trimmedQuery = query.trim();
    if (!trimmedQuery) {
      isRefreshingRef.current = false;
      setIsRefreshing(false);
      return;
    }

    const currentCount = logs.length;
    const refreshLimit = 100;

    try {
      const filters = {
        time_start: filterOptions.timeRange.start,
        time_end: filterOptions.timeRange.end,
        levels: filterOptions.levels,
        file_pattern: filterOptions.filePattern || null
      };

      const result = await invoke<{results: Array<{id: {to_string: () => string}, timestamp: string, level: string, message: string, source_file: string, line_number: number}>, total_count: number}>("search_logs", {
        query: trimmedQuery,
        searchPath: activeWorkspace.path,
        filters,
        offset: currentCount,
        limit: refreshLimit,
      });

      if (result.results && result.results.length > 0) {
        const newLogs: LogEntry[] = result.results.map((r, i) => ({
          id: currentCount + i + 1,
          timestamp: r.timestamp,
          level: r.level,
          content: r.message,
          file: r.source_file,
          line: r.line_number,
          real_path: r.source_file,
          tags: [],
          match_details: null,
          matched_keywords: undefined,
        }));

        setLogs(prev => [...prev, ...newLogs]);
      }
    } catch (err) {
      console.error('Refresh failed:', err);
      addToast('error', `刷新失败: ${err}`);
    } finally {
      isRefreshingRef.current = false;
      setIsRefreshing(false);
    }
  }, [query, activeWorkspace, filterOptions, logs.length, addToast]);

  // 存储 refreshLogs 到 ref 供滚动事件使用
  useEffect(() => {
    refreshLogsRef.current = refreshLogs;
  }, [refreshLogs]);

  /**
   * 执行搜索 - 使用 useCallback 确保稳定性
   */
  const handleSearch = useCallback(async () => {
    if (!activeWorkspace) {
      addToast('error', 'Select a workspace first.');
      return;
    }

    const trimmedQuery = query.trim();
    if (!trimmedQuery) {
      setLogs([]);
      return;
    }

    // 重置状态
    setLogs([]);
    setSearchSummary(null);
    setKeywordStats([]);
    setIsSearching(true);
    setSelectedId(null);

    // 重置滚动位置到顶部
    if (parentRef.current) {
      parentRef.current.scrollTop = 0;
    }
    if (rowVirtualizerRef.current && rowVirtualizerRef.current.scrollOffset !== 0) {
      rowVirtualizerRef.current.scrollOffset = 0;
    }

    try {
      // 构建过滤器对象
      const filters = {
        time_start: filterOptions.timeRange.start,
        time_end: filterOptions.timeRange.end,
        levels: filterOptions.levels,
        file_pattern: filterOptions.filePattern || null
      };

      await api.searchLogs({
        query: trimmedQuery,
        workspaceId: activeWorkspace.id,
        filters,
      });

      // 如果使用了结构化查询，更新执行次数
      if (currentQuery) {
        currentQuery.metadata.executionCount += 1;
        setCurrentQuery({...currentQuery});
      }
    } catch (err) {
      logger.error('Search failed:', err);
      setIsSearching(false);
      addToast('error', `搜索失败: ${getFullErrorMessage(err)}`);
    }
  }, [query, activeWorkspace, filterOptions, currentQuery, addToast, rowVirtualizerRef, parentRef]);
  
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
    const terms = query.split('|').map(t => t.trim()).filter(t => t.length > 0);
    const newTerms = terms.filter(t => t.toLowerCase() !== termToRemove.toLowerCase());
    setQuery(newTerms.join('|'));
    
    // 同时更新结构化查询
    if (currentQuery) {
      const builder = SearchQueryBuilder.import(JSON.stringify(currentQuery));
      const existing = builder.findTermByValue(termToRemove);
      if (existing) {
        builder.removeTerm(existing.id);
        setCurrentQuery(builder.getQuery());
      }
    }
  }, [query, currentQuery]);

  /**
   * 切换规则在查询中的状态
   * | 仅作为分隔符，多个关键词用 OR 逻辑组合（匹配任意一个）
   */
  const toggleRuleInQuery = useCallback((ruleRegex: string) => {
    // 创建或更新查询构建器
    const builder = currentQuery 
      ? SearchQueryBuilder.import(JSON.stringify(currentQuery))
      : SearchQueryBuilder.fromString(query, keywordGroups);

    // 检查是否已存在
    const existing = builder.findTermByValue(ruleRegex);

    if (existing) {
      // 已存在：切换启用状态
      builder.toggleTerm(existing.id);
    } else {
      // 不存在：添加新项
      builder.addTerm(ruleRegex, {
        source: 'preset',
        isRegex: true,
        operator: 'AND'
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
    const newQuery = builder.getQuery();
    setCurrentQuery(newQuery);
    
    // 更新查询字符串（用于显示）
    const queryString = builder.toQueryString();
    setQuery(queryString);
  }, [query, keywordGroups, currentQuery, addToast]);

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
    if (logs.length === 0) {
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
        results: logs,
        format,
        savePath
      });

      addToast('success', `已导出 ${logs.length} 条日志到 ${format.toUpperCase()}`);
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
    count: deferredLogs.length, 
    getScrollElement: () => {
      if (!parentRef.current) return null;
      return parentRef.current;
    }, 
    estimateSize: useCallback(() => 32, []),
    overscan: 15,
    measureElement: (element) => {
      if (!element) return 32;
      try {
        const rect = element.getBoundingClientRect();
        return rect.height > 0 ? rect.height : 32;
      } catch {
        return 32;
      }
    },
  });
  
  // 将虚拟滚动器存储到 ref 中
  rowVirtualizerRef.current = rowVirtualizer;
  
  const activeLog = deferredLogs.find(l => l.id === selectedId);

  return (
    <div className="flex flex-col h-full relative">
      {/* 搜索控制区 */}
      <div className="p-4 border-b border-border-base bg-bg-sidebar space-y-3 shrink-0 relative z-40">
        {/* 搜索输入和操作按钮 */}
        <div className="flex gap-2">
          <div className="relative flex-1">
            <Search className="absolute left-3 top-2.5 text-text-dim" size={16} />
            <Input
              ref={searchInputRef}
              value={query}
              onChange={(e: React.ChangeEvent<HTMLInputElement>) => {
                // 规范化输入：移除 | 前后的空格
                const normalized = e.target.value.replace(/\s*\|\s*/g, '|');
                setQuery(normalized);
              }}
              className="pl-9 pr-10 font-mono bg-bg-main"
              placeholder="Search keywords separated by | ..."
              onKeyDown={(e: React.KeyboardEvent<HTMLInputElement>) => e.key === 'Enter' && handleSearch()}
            />
          </div>

          <div className="relative">
            <Button 
              variant={isFilterPaletteOpen ? "active" : "secondary"} 
              icon={Filter} 
              onClick={() => setIsFilterPaletteOpen(!isFilterPaletteOpen)} 
              className="w-[140px] justify-between"
            >
              Filters 
              <ChevronDown 
                size={14} 
                className={cn(
                  "transition-transform", 
                  isFilterPaletteOpen ? "rotate-180" : ""
                )}
              />
            </Button>
            <FilterPalette 
              isOpen={isFilterPaletteOpen} 
              onClose={() => setIsFilterPaletteOpen(false)} 
              groups={keywordGroups} 
              currentQuery={query} 
              onToggleRule={toggleRuleInQuery} 
            />
          </div>
          <Button 
            icon={Download} 
            onClick={() => handleExport('csv')} 
            disabled={logs.length === 0} 
            variant="secondary" 
            title="Export to CSV"
          >
            CSV
          </Button>
          <Button 
            icon={Download} 
            onClick={() => handleExport('json')} 
            disabled={logs.length === 0} 
            variant="secondary" 
            title="Export to JSON"
          >
            JSON
          </Button>
          <Button 
            icon={isSearching ? Loader2 : Search} 
            onClick={handleSearch} 
            disabled={isSearching} 
            className={isSearching ? "animate-pulse" : ""}
          >
            {isSearching ? '...' : 'Search'}
          </Button>
        </div>
        
        {/* 高级过滤器 UI */}
        <div className="flex items-center gap-2 mb-2">
          <span className="text-[10px] font-bold text-text-dim uppercase">Advanced Filters</span>
          {(filterOptions.levels.length > 0 || filterOptions.timeRange.start || filterOptions.timeRange.end || filterOptions.filePattern) && (
            <>
              <span className="text-[10px] bg-primary/20 text-primary px-1.5 py-0.5 rounded border border-primary/30">
                {[
                  filterOptions.levels.length > 0 && `${filterOptions.levels.length} levels`,
                  filterOptions.timeRange.start && 'time range',
                  filterOptions.filePattern && 'file pattern'
                ].filter(Boolean).join(', ')}
              </span>
              <Button 
                variant="ghost" 
                onClick={handleResetFilters} 
                className="h-5 text-[10px] px-2" 
                icon={RotateCcw}
              >
                Reset
              </Button>
            </>
          )}
        </div>
        
        <div className="grid grid-cols-4 gap-2">
          {/* 日志级别过滤 */}
          <div className="col-span-1">
            <label className="text-[10px] text-text-dim uppercase font-bold mb-1 block">Level</label>
            <div className="flex gap-1">
              {['ERROR', 'WARN', 'INFO', 'DEBUG'].map((level) => (
                <button
                  key={level}
                  onClick={() => {
                    setFilterOptions(prev => ({
                      ...prev,
                      levels: prev.levels.includes(level) 
                        ? prev.levels.filter(l => l !== level)
                        : [...prev.levels, level]
                    }));
                  }}
                  className={cn(
                    "text-[10px] px-2 py-1 rounded border transition-all",
                    filterOptions.levels.includes(level)
                      ? "bg-primary text-white border-primary"
                      : "bg-bg-main text-text-dim border-border-base hover:border-primary/50"
                  )}
                  title={level}
                >
                  {level.substring(0, 1)}
                </button>
              ))}
            </div>
          </div>
          
          {/* 时间范围过滤 */}
          <div className="col-span-2">
            <label className="text-[10px] text-text-dim uppercase font-bold mb-1 block">Time Range</label>
            <div className="flex gap-1">
              <input
                type="datetime-local"
                value={filterOptions.timeRange.start || ""}
                onChange={(e) => setFilterOptions(prev => ({
                  ...prev,
                  timeRange: { ...prev.timeRange, start: e.target.value || null }
                }))}
                className="h-7 text-[11px] flex-1 bg-bg-main border border-border-base rounded px-2 text-text-main focus:outline-none focus:border-primary/50"
                placeholder="Start"
              />
              <span className="text-text-dim self-center">~</span>
              <input
                type="datetime-local"
                value={filterOptions.timeRange.end || ""}
                onChange={(e) => setFilterOptions(prev => ({
                  ...prev,
                  timeRange: { ...prev.timeRange, end: e.target.value || null }
                }))}
                className="h-7 text-[11px] flex-1 bg-bg-main border border-border-base rounded px-2 text-text-main focus:outline-none focus:border-primary/50"
                placeholder="End"
              />
            </div>
          </div>
          
          {/* 文件来源过滤 */}
          <div className="col-span-1">
            <label className="text-[10px] text-text-dim uppercase font-bold mb-1 block">File Pattern</label>
            <Input
              value={filterOptions.filePattern}
              onChange={(e: React.ChangeEvent<HTMLInputElement>) => setFilterOptions(prev => ({ ...prev, filePattern: e.target.value }))}
              className="h-7 text-[11px]"
              placeholder="e.g. error.log"
            />
          </div>
        </div>
        
        {/* 当前激活的搜索关键词 */}
        <div className="flex items-center gap-2 overflow-x-auto pb-1 scrollbar-none h-6 min-h-[24px]">
          <span className="text-[10px] font-bold text-text-dim uppercase">Active:</span>
          {query ? query.split('|').map((term:string, i:number) => {
            const trimmedTerm = term.trim();
            return (
              <span 
                key={i} 
                className="flex items-center text-[10px] bg-bg-card border border-border-base px-1.5 py-0.5 rounded text-text-main whitespace-nowrap group gap-1"
              >
                <Hash size={8} className="mr-0.5 opacity-50"/> 
                {trimmedTerm}
                <button 
                  onClick={() => removeTermFromQuery(trimmedTerm)} 
                  className="opacity-0 group-hover:opacity-100 hover:text-red-400 transition-all ml-0.5"
                  title="Remove keyword"
                >
                  <X size={10} />
                </button>
              </span>
            );
          }) : <span className="text-[10px] text-text-dim italic">None</span>}
        </div>
        
        {/* 关键词统计面板 */}
        {searchSummary && keywordStats.length > 0 && (
          <KeywordStatsPanel
            keywords={keywordStats}
            totalMatches={searchSummary.totalMatches}
            searchDurationMs={searchSummary.searchDurationMs}
            onClose={() => setSearchSummary(null)}
          />
        )}
      </div>
      
      {/* 结果展示区 */}
      <div className="flex-1 flex overflow-hidden">
        {/* 日志列表 */}
        <div ref={parentRef} className="flex-1 overflow-auto bg-bg-main scrollbar-thin">
          {/* 表头 */}
          <div className="sticky top-0 z-10 grid grid-cols-[50px_160px_200px_1fr] px-3 py-1.5 bg-bg-main border-b border-border-base text-[10px] font-bold text-text-dim uppercase tracking-wider">
            <div>Lvl</div> 
            <div>Time</div> 
            <div>File</div> 
            <div>Content</div>
          </div>
          
          {/* 虚拟滚动列表 - 使用 LogRow 组件优化渲染 */}
          <div style={{ height: `${rowVirtualizer.getTotalSize()}px`, width: '100%', position: 'relative' }}>
            {rowVirtualizer.getVirtualItems().map((virtualRow) => {
              const log = deferredLogs[virtualRow.index];
              if (!log) return null; // 防止延迟渲染时索引越界
              return (
                <LogRow
                  key={virtualRow.key}
                  log={log}
                  isActive={log.id === selectedId}
                  onClick={() => setSelectedId(log.id)}
                  query={query}
                  keywordGroups={enabledKeywordGroups}
                  virtualStart={virtualRow.start}
                  virtualKey={virtualRow.key}
                  measureRef={rowVirtualizer.measureElement}
                />
              );
            })}
          </div>
          
          {/* 加载指示器 - 底部刷新时显示 */}
          {isRefreshing && (
            <div className="flex items-center justify-center py-4">
              <Loader2 className="animate-spin text-primary" size={20} />
              <span className="ml-2 text-sm text-text-muted">加载更多...</span>
            </div>
          )}
          
          {/* 空状态 */}
          {logs.length === 0 && !isSearching && (
            <div className="flex items-center justify-center h-full text-text-dim">
              No logs found. Select workspace & search.
            </div>
          )}
        </div>
        
        {/* 日志详情面板 */}
        {activeLog && (
          <div className="w-[450px] bg-bg-sidebar border-l border-border-base flex flex-col shrink-0 shadow-xl z-20 animate-in slide-in-from-right duration-200">
            <div className="h-10 border-b border-border-base flex items-center justify-between px-4 bg-bg-card/50">
              <span className="text-xs font-semibold text-text-muted uppercase">Log Inspector</span>
              <div className="flex gap-1">
                <Button 
                  variant="ghost" 
                  className="h-6 w-6 p-0" 
                  onClick={() => copyToClipboard(activeLog.content)}
                >
                  <Copy size={14}/>
                </Button>
                <Button 
                  variant="ghost" 
                  className="h-6 w-6 p-0" 
                  onClick={() => setSelectedId(null)}
                >
                  <X size={14}/>
                </Button>
              </div>
            </div>
            <div className="flex-1 overflow-auto p-4 font-mono text-xs">
              <div className="bg-bg-main p-3 rounded border border-border-base mb-4">
                <div className="text-text-dim text-[10px] uppercase mb-1">Message Body</div>
                <div className="text-text-main whitespace-pre-wrap break-all leading-relaxed">
                  <HybridLogRenderer 
                    text={tryFormatJSON(activeLog.content)} 
                    query={query} 
                    keywordGroups={enabledKeywordGroups} 
                  />
                </div>
              </div>
              <div className="p-2 bg-bg-card border border-border-base rounded mb-2">
                <div className="text-[10px] text-text-dim uppercase">File</div>
                <div className="break-all text-text-main">{activeLog?.real_path || 'N/A'}</div>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

export default SearchPage;
