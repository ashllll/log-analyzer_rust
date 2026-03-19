/**
 * 搜索过滤器组件
 * 包含 Level 过滤按钮组、Time Range 输入、File Pattern 输入
 */
import React from 'react';
import { Dispatch, SetStateAction } from 'react';
import { RotateCcw } from 'lucide-react';
import { Button, Input } from '../../../components/ui';
import { cn } from '../../../utils/classNames';
import type { FilterOptions } from '../../../types/common';

export interface SearchFiltersProps {
  filterOptions: FilterOptions;
  onFilterOptionsChange: Dispatch<SetStateAction<FilterOptions>>;
  onReset: () => void;
}

export const SearchFilters: React.FC<SearchFiltersProps> = ({
  filterOptions,
  onFilterOptionsChange,
  onReset,
}) => {
  const hasActiveFilters =
    filterOptions.levels.length > 0 ||
    filterOptions.timeRange.start ||
    filterOptions.timeRange.end ||
    filterOptions.filePattern;

  return (
    <>
      {/* 高级过滤器 UI */}
      <div className="flex items-center gap-2 mb-2">
        <span className="text-[10px] font-bold text-text-dim uppercase">Advanced Filters</span>
        {hasActiveFilters && (
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
              onClick={onReset}
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
                  onFilterOptionsChange(prev => ({
                    ...prev,
                    levels: prev.levels.includes(level)
                      ? prev.levels.filter(l => l !== level)
                      : [...prev.levels, level]
                  }));
                }}
                className={cn(
                  "text-[10px] px-2 py-1 rounded border transition-all duration-200 cursor-pointer font-medium",
                  filterOptions.levels.includes(level)
                    ? "bg-primary text-white border-primary shadow-sm"
                    : "bg-bg-card text-text-muted border-border-base hover:border-primary/50 hover:text-text-main"
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
              onChange={(e) => onFilterOptionsChange(prev => ({
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
              onChange={(e) => onFilterOptionsChange(prev => ({
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
            onChange={(e: React.ChangeEvent<HTMLInputElement>) => onFilterOptionsChange(prev => ({ ...prev, filePattern: e.target.value }))}
            className="h-7 text-[11px]"
            placeholder="e.g. error.log"
          />
        </div>
      </div>
    </>
  );
};
