import React, { useState, useEffect, useRef, useCallback } from 'react';
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
import { logger } from '../utils/logger';
import { cn } from '../utils/classNames';
import { SearchQueryBuilder } from '../services/SearchQueryBuilder';
import { SearchQuery } from '../types/search';
import { saveQuery, loadQuery } from '../services/queryStorage';
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

const SearchPage: React.FC<SearchPageProps> = ({ 
  keywordGroups, 
  addToast, 
  searchInputRef, 
  activeWorkspace 
}) => {
  // 搜索状态
  const [query, setQuery] = useState("");
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [isFilterPaletteOpen, setIsFilterPaletteOpen] = useState(false);
  const [isSearching, setIsSearching] = useState(false);
  
  // 结构化查询状态
  const [currentQuery, setCurrentQuery] = useState<SearchQuery | null>(null);
  
  // 高级过滤状态
  const [filterOptions, setFilterOptions] = useState<FilterOptions>({
    timeRange: { start: null, end: null },
    levels: [],
    filePattern: ""
  });
  
  const parentRef = useRef<HTMLDivElement>(null);

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

  // 监听搜索事件
  useEffect(() => {
    const unlistenResults = listen<LogEntry[]>('search-results', (e) => setLogs(prev => [...prev, ...e.payload]));
    const unlistenComplete = listen('search-complete', (e) => { setIsSearching(false); addToast('success', `Found ${e.payload} logs.`); });
    const unlistenError = listen('search-error', (e) => { setIsSearching(false); addToast('error', `${e.payload}`); });
    return () => { unlistenResults.then(f => f()); unlistenComplete.then(f => f()); unlistenError.then(f => f()); }
  }, [addToast]);

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

  /**
   * 执行搜索
   */
  const handleSearch = async () => {
    if (!activeWorkspace) return addToast('error', 'Select a workspace first.');
    setLogs([]); 
    setIsSearching(true);
    
    try { 
      // 构建过滤器对象
      const filters = {
        time_start: filterOptions.timeRange.start,
        time_end: filterOptions.timeRange.end,
        levels: filterOptions.levels,
        file_pattern: filterOptions.filePattern || null
      };
      
      await invoke("search_logs", { 
        query, 
        searchPath: activeWorkspace.path,
        filters: filters
      }); 
      
      // 如果使用了结构化查询，更新执行次数
      if (currentQuery) {
        currentQuery.metadata.executionCount += 1;
        setCurrentQuery({...currentQuery});
      }
    } catch(_e) { 
      setIsSearching(false); 
    }
  };
  
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
  const copyToClipboard = (text: string) => { 
    navigator.clipboard.writeText(text).then(() => addToast('success', 'Copied')); 
  };
  
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
    } catch (_e) { 
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
      await invoke('export_results', {
        results: logs,
        format,
        savePath
      });

      addToast('success', `已导出 ${logs.length} 条日志到 ${format.toUpperCase()}`);
    } catch (e) {
      logger.error('Export error:', e);
      addToast('error', `导出失败: ${e}`);
    }
  };
  
  /**
   * 虚拟滚动配置
   * 优化：动态高度估算 - 提高显示密度，添加ResizeObserver支持
   */
  const rowVirtualizer = useVirtualizer({ 
    count: logs.length, 
    getScrollElement: () => parentRef.current, 
    estimateSize: useCallback((index: number) => {
      const log = logs[index];
      if (!log) return 28;  // 减小最小高度从 46px 到 28px
      // 根据内容长度估算高度，使用更紧凑的行高
      const lines = Math.ceil(log.content.length / 140);
      return Math.max(28, Math.min(lines * 16, 120));  // 最小 28px，最大 120px，行高从 22px 减到 16px
    }, [logs]),
    overscan: 25,  // 增加 overscan 以保证流畅滚动
    measureElement: (element) => element?.getBoundingClientRect().height || 28,
    // 启用动态测量以响应尺寸变化
    enabled: true
  });
  
  const activeLog = logs.find(l => l.id === selectedId);

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
              onChange={(e: any) => {
                // 规范化输入：移除 | 前后的空格
                const normalized = e.target.value.replace(/\s*\|\s*/g, '|');
                setQuery(normalized);
              }} 
              className="pl-9 font-mono bg-bg-main" 
              placeholder="Search keywords separated by | ..." 
              onKeyDown={(e:any) => e.key === 'Enter' && handleSearch()} 
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
              onChange={(e: any) => setFilterOptions(prev => ({ ...prev, filePattern: e.target.value }))}
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
          
          {/* 虚拟滚动列表 */}
          <div style={{ height: `${rowVirtualizer.getTotalSize()}px`, width: '100%', position: 'relative' }}>
            {rowVirtualizer.getVirtualItems().map((virtualRow) => {
              const log = logs[virtualRow.index];
              const isActive = log.id === selectedId;
              return (
                <div 
                  key={virtualRow.key} 
                  data-index={virtualRow.index} 
                  ref={rowVirtualizer.measureElement} 
                  onClick={() => setSelectedId(log.id)} 
                  style={{ transform: `translateY(${virtualRow.start}px)` }} 
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
            })}
          </div>
          
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
                    keywordGroups={keywordGroups} 
                  />
                </div>
              </div>
              <div className="p-2 bg-bg-card border border-border-base rounded mb-2">
                <div className="text-[10px] text-text-dim uppercase">File</div>
                <div className="break-all text-text-main">{activeLog.real_path}</div>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

export default SearchPage;
