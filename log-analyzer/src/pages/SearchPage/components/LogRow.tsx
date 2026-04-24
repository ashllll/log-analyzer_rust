import { memo } from 'react';
import { cn } from '../../../utils/classNames';
import { HybridLogRenderer } from '../../../components/renderers';
import type { LogEntry, KeywordGroup } from '../../../types/common';
import type { SearchTerm } from '../../../types/search';

/**
 * 虚拟行组件 Props
 */
interface LogRowProps {
  log: LogEntry;
  isActive: boolean;
  onClick: () => void;
  query: string;
  queryTerms: SearchTerm[] | null;
  keywordGroups: KeywordGroup[];
  virtualIndex: number;
  virtualStart: number;
  virtualSize: number;
  measureElement: (element: HTMLDivElement | null) => void;
}

/**
 * 虚拟行组件 - 使用 React.memo 优化
 * 只有当 log、isActive、query 或 keywordGroups 变化时才重新渲染
 */
export const LogRow = memo<LogRowProps>(({
  log,
  isActive,
  onClick,
  query,
  queryTerms,
  keywordGroups,
  virtualIndex,
  virtualStart,
  virtualSize,
  measureElement,
}) => {
  // 防御性 fallback：后端数据可能缺失字段
  const level = log.level ?? 'INFO';
  const timestamp = log.timestamp ?? '';
  const file = log.file ?? '';
  const line = log.line ?? 0;
  const content = log.content ?? '';

  return (
    <div
      ref={measureElement}
      data-index={virtualIndex}
      onClick={onClick}
      style={{
        transform: `translateY(${virtualStart}px)`,
        minHeight: `${virtualSize}px`,
      }}
      className={cn(
        "absolute top-0 left-0 w-full grid grid-cols-[50px_160px_150px_1fr] px-3 py-1.5 border-b border-border-subtle cursor-pointer text-xs font-mono hover:bg-bg-hover/50 transition-colors duration-150 items-start",
        isActive && "bg-primary/10 border-l-2 border-l-primary"
      )}
    >
      <div className="flex items-center">
        <span className={cn(
          "inline-block text-xs font-bold px-1.5 py-0.5 rounded leading-none",
          level === 'ERROR' ? 'bg-log-error/20 text-log-error' :
          level === 'WARN'  ? 'bg-log-warn/20 text-log-warn' :
          level === 'INFO'  ? 'bg-log-info/20 text-log-info' :
          'bg-log-debug/20 text-log-debug'
        )}>
          {typeof level === 'string' ? level.substring(0, 1) : '?'}
        </span>
      </div>
      <div className="text-text-muted whitespace-nowrap text-xs">
        {timestamp}
      </div>
      <div
        className="text-text-muted truncate pr-2 text-xs leading-tight"
        title={`${file}:${line}`}
      >
        {(file.split('/').pop() ?? file).split('\\').pop() ?? file}:{line}
      </div>
      <div className="text-text-main whitespace-pre-wrap break-words leading-tight pr-2">
        <HybridLogRenderer
          text={content}
          query={query}
          queryTerms={queryTerms}
          keywordGroups={keywordGroups}
          matchDetails={log.match_details}
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
    prevProps.queryTerms === nextProps.queryTerms &&
    prevProps.keywordGroups === nextProps.keywordGroups &&
    prevProps.virtualIndex === nextProps.virtualIndex &&
    prevProps.virtualStart === nextProps.virtualStart &&
    prevProps.virtualSize === nextProps.virtualSize
  );
});

LogRow.displayName = 'LogRow';
