import { memo } from "react";
import { cn } from "../../../utils/classNames";
import { HybridLogRenderer } from "../../../components/renderers";
import type { LogEntry, KeywordGroup } from "../../../types/common";
import type { SearchTerm } from "../../../types/search";

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
export const LogRow = memo<LogRowProps>(
  ({
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
    const level = log.level ?? "INFO";
    const timestamp = log.timestamp ?? "";
    const file = log.file ?? "";
    const line = log.line ?? 0;
    const content = log.content ?? "";

    const handleKeyDown = (e: React.KeyboardEvent<HTMLDivElement>) => {
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        onClick();
      }
    };

    return (
      <div
        ref={measureElement}
        data-index={virtualIndex}
        data-log-id={log.id}
        role="button"
        tabIndex={0}
        aria-pressed={isActive}
        onClick={onClick}
        onKeyDown={handleKeyDown}
        style={{
          transform: `translateY(${virtualStart}px)`,
          minHeight: `${virtualSize}px`,
        }}
        className={cn(
          "absolute left-0 top-0 grid w-full grid-cols-[70px_160px_170px_1fr] items-start border-b border-border-subtle px-4 py-2 font-mono text-xs cursor-pointer transition-colors duration-150 hover:bg-bg-hover/60",
          isActive &&
            "bg-primary/10 shadow-[inset_3px_0_0_rgb(var(--color-accent))]"
        )}
      >
        <div className="flex items-center">
          <span
            className={cn(
              "inline-block rounded-full px-2 py-0.5 text-[10px] font-semibold leading-none",
              level === "ERROR"
                ? "bg-log-error/20 text-log-error"
                : level === "WARN"
                  ? "bg-log-warn/20 text-log-warn"
                  : level === "INFO"
                    ? "bg-log-info/20 text-log-info"
                    : "bg-log-debug/20 text-log-debug"
            )}
          >
            {typeof level === "string" ? level : "?"}
          </span>
        </div>
        <div className="text-text-muted whitespace-nowrap text-xs">
          {timestamp}
        </div>
        <div
          className="text-text-muted truncate pr-2 text-xs leading-tight"
          title={`${file}:${line}`}
        >
          {(file.split("/").pop() ?? file).split("\\").pop() ?? file}:{line}
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
  },
  (prevProps, nextProps) => {
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
  }
);

LogRow.displayName = "LogRow";
