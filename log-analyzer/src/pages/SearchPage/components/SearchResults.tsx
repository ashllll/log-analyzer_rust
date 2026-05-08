/**
 * 搜索结果列表组件
 * 封装虚拟滚动列表、骨架屏、加载指示器、空状态
 */
import React, { memo } from 'react';
import { Loader2, FolderOpen, Search, SearchX } from 'lucide-react';
import { EmptyState } from '../../../components/ui';
import { LogRow } from './LogRow';
import { useTranslation } from 'react-i18next';
import type { LogEntry, KeywordGroup } from '../../../types/common';
import type { SearchTerm } from '../../../types/search';

export interface SearchResultsProps {
  virtualItems: ReturnType<ReturnType<typeof import('@tanstack/react-virtual').useVirtualizer>['getVirtualItems']>;
  totalSize: number;
  measureElement: (element: HTMLDivElement | null) => void;
  loadedEntries: LogEntry[];
  firstPageOffset: number;
  selectedId: number | null;
  onSelectId: (id: number | null) => void;
  query: string;
  queryTerms: SearchTerm[] | null;
  keywordGroups: KeywordGroup[];
  isFetchingNextPage: boolean;
  isFetchingPreviousPage: boolean;
  hasNextPage: boolean;
  hasPreviousPage: boolean;
  totalResultCount: number;
  liveCount: number;
  isSearching: boolean;
  isInitialized: boolean;
  workspaceLoading: boolean;
  activeWorkspace: unknown;
  parentRef: React.RefObject<HTMLDivElement | null>;
}

export const SearchResults = memo<SearchResultsProps>(({
  virtualItems,
  totalSize,
  measureElement,
  loadedEntries,
  firstPageOffset,
  selectedId,
  onSelectId,
  query,
  queryTerms,
  keywordGroups,
  isFetchingNextPage,
  isFetchingPreviousPage,
  hasNextPage,
  hasPreviousPage,
  totalResultCount,
  liveCount,
  isSearching,
  isInitialized,
  workspaceLoading,
  activeWorkspace,
  parentRef,
}) => {
  const { t } = useTranslation();

  return (
    <div ref={parentRef} className="flex-1 overflow-auto bg-bg-main scrollbar-thin" style={{ willChange: 'transform' }}>
      {/* 表头 */}
      <div className="sticky top-0 z-10 grid grid-cols-[50px_160px_150px_1fr] px-3 py-2 bg-bg-elevated border-b border-border-base text-xs font-bold text-text-muted uppercase tracking-wider">
        <div>{t('search.table.level', '级别')}</div>
        <div>{t('search.table.time', '时间')}</div>
        <div>{t('search.table.file', '文件')}</div>
        <div>{t('search.table.content', '内容')}</div>
      </div>

      {/* 虚拟滚动列表 */}
      <div style={{ height: `${totalSize}px`, width: '100%', position: 'relative' }}>
        {virtualItems.map((virtualRow) => {
          const log = loadedEntries[virtualRow.index - firstPageOffset];
          if (!log) {
            return (
              <div
                key={virtualRow.key}
                ref={measureElement}
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
              onClick={() => onSelectId(log.id)}
              query={query}
              queryTerms={queryTerms}
              keywordGroups={keywordGroups}
              virtualIndex={virtualRow.index}
              virtualStart={virtualRow.start}
              virtualSize={virtualRow.size}
              measureElement={measureElement}
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
      {liveCount > 0 && !hasNextPage && !isFetchingNextPage && !hasPreviousPage && totalResultCount > 0 && (
        <div className="flex items-center justify-center py-3 text-text-muted text-xs">
          共 {totalResultCount.toLocaleString()} 条结果
        </div>
      )}

      {/* 空状态 */}
      {liveCount === 0 && !isSearching && (
        <div className="flex items-center justify-center h-full min-h-[200px]">
          {(!isInitialized || workspaceLoading) ? (
            <div className="flex flex-col items-center gap-3 text-text-dim">
              <Loader2 className="animate-spin text-primary" size={32} />
              <p className="text-sm font-medium text-text-muted">
                {t('search.empty_state.workspace_loading', '工作区加载中')}
              </p>
            </div>
          ) : !activeWorkspace ? (
            <EmptyState
              icon={FolderOpen}
              title={t('search.empty_state.no_workspace', '没有工作区')}
              description={t('search.empty_state.no_workspace_hint', '请先创建或导入工作区以开始搜索日志')}
            />
          ) : !query.trim() ? (
            <EmptyState
              icon={Search}
              title={t('search.empty_state.no_query', '输入搜索关键词')}
              description={t('search.empty_state.no_query_hint', '在上方输入框中输入关键词进行搜索')}
            />
          ) : (
            <EmptyState
              icon={SearchX}
              title={t('search.empty_state.no_results', '没有搜索结果')}
              description={t('search.empty_state.no_results_hint', '尝试调整搜索条件或关键词')}
            />
          )}
        </div>
      )}
    </div>
  );
});

SearchResults.displayName = 'SearchResults';
