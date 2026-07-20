/**
 * 搜索过滤器组件
 * 包含 Level 过滤按钮组、Time Range 输入、File Pattern 输入
 */
import React, { memo } from "react";
import { Dispatch, SetStateAction } from "react";
import { RotateCcw } from "lucide-react";
import { Button, Input } from "../../../components/ui";
import { cn } from "../../../utils/classNames";
import type { FilterOptions } from "../../../types/common";

export interface SearchFiltersProps {
  filterOptions: FilterOptions;
  onFilterOptionsChange: Dispatch<SetStateAction<FilterOptions>>;
  onReset: () => void;
}

export const SearchFilters: React.FC<SearchFiltersProps> = memo(
  ({ filterOptions, onFilterOptionsChange, onReset }) => {
    const hasActiveFilters =
      filterOptions.levels.length > 0 ||
      filterOptions.timeRange.start ||
      filterOptions.timeRange.end ||
      filterOptions.filePattern;

    return (
      <div className="flex min-h-9 flex-wrap items-end gap-3">
        {/* 日志级别过滤 */}
        <div className="shrink-0">
          <label className="text-[10px] text-text-dim uppercase font-bold mb-1 block">
            Level
          </label>
          <div className="flex gap-1">
            {["ERROR", "WARN", "INFO", "DEBUG"].map((level) => (
              <button
                key={level}
                onClick={() => {
                  onFilterOptionsChange((prev) => ({
                    ...prev,
                    levels: prev.levels.includes(level)
                      ? prev.levels.filter((l) => l !== level)
                      : [...prev.levels, level],
                  }));
                }}
                className={cn(
                  "ui-pressable text-[10px] px-2 py-1 rounded-full border cursor-pointer font-medium",
                  filterOptions.levels.includes(level)
                    ? "bg-primary text-white border-primary shadow-sm"
                    : "bg-bg-card text-text-muted border-border-base hover:border-primary/50 hover:text-text-main"
                )}
                title={level}
                aria-pressed={filterOptions.levels.includes(level)}
              >
                {level}
              </button>
            ))}
          </div>
        </div>

        {/* 时间范围过滤 */}
        <div className="min-w-[min(100%,20rem)] flex-[1_1_20rem]">
          <span className="text-[10px] text-text-dim uppercase font-bold mb-1 block">
            Time Range
          </span>
          <div className="flex min-w-0 flex-col gap-1 sm:flex-row">
            <input
              type="datetime-local"
              value={filterOptions.timeRange.start || ""}
              onChange={(e) =>
                onFilterOptionsChange((prev) => ({
                  ...prev,
                  timeRange: {
                    ...prev.timeRange,
                    start: e.target.value || null,
                  },
                }))
              }
              className="h-7 min-w-0 flex-1 bg-bg-main border border-border-base rounded px-2 text-[11px] text-text-main focus:outline-none focus:border-primary/50"
              placeholder="Start"
              aria-label="Start time"
            />
            <span className="text-text-dim self-center">~</span>
            <input
              type="datetime-local"
              value={filterOptions.timeRange.end || ""}
              onChange={(e) =>
                onFilterOptionsChange((prev) => ({
                  ...prev,
                  timeRange: { ...prev.timeRange, end: e.target.value || null },
                }))
              }
              className="h-7 min-w-0 flex-1 bg-bg-main border border-border-base rounded px-2 text-[11px] text-text-main focus:outline-none focus:border-primary/50"
              placeholder="End"
              aria-label="End time"
            />
          </div>
        </div>

        {/* 文件来源过滤 */}
        <div className="min-w-[12rem] flex-[1_1_12rem]">
          <label className="text-[10px] text-text-dim uppercase font-bold mb-1 block">
            File Pattern
          </label>
          <Input
            value={filterOptions.filePattern}
            onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
              onFilterOptionsChange((prev) => ({
                ...prev,
                filePattern: e.target.value,
              }))
            }
            className="h-7 text-[11px]"
            placeholder="e.g. error.log"
            aria-label="File pattern"
          />
        </div>
        <Button
          variant="ghost"
          onClick={onReset}
          disabled={!hasActiveFilters}
          className="h-7 px-2 text-[11px]"
          icon={RotateCcw}
        >
          Reset
        </Button>
      </div>
    );
  }
);

SearchFilters.displayName = "SearchFilters";
