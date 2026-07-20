/**
 * 搜索控制组件
 * 包含搜索输入框、过滤器按钮、导出按钮、搜索按钮
 */
import React, { memo, useRef } from "react";
import { Search, Download, Filter, ChevronDown, Loader2 } from "lucide-react";
import { Button, Input } from "../../../components/ui";
import { FilterPalette } from "../../../components/modals";
import { cn } from "../../../utils/classNames";
import { useTranslation } from "react-i18next";
import type { KeywordGroup } from "../../../types/common";

export interface SearchControlsProps {
  query: string;
  onQueryChange: (q: string) => void;
  onSearch: () => void;
  onExport: (format: "csv" | "json") => void;
  isFilterPaletteOpen: boolean;
  onFilterPaletteToggle: () => void;
  onFilterPaletteClose: () => void;
  isSearching: boolean;
  disabled: boolean;
  keywordGroups: KeywordGroup[];
  activeTerms: string[];
  onToggleRule: (regex: string) => void;
}

export const SearchControls: React.FC<SearchControlsProps> = memo(
  ({
    query,
    onQueryChange,
    onSearch,
    onExport,
    isFilterPaletteOpen,
    onFilterPaletteToggle,
    onFilterPaletteClose,
    isSearching,
    disabled,
    keywordGroups,
    activeTerms,
    onToggleRule,
  }) => {
    const { t } = useTranslation();
    const keywordGroupsTriggerRef = useRef<HTMLButtonElement>(null);

    return (
      <div className="flex items-center gap-2">
        <div className="relative flex-1">
          <label htmlFor="search-input" className="sr-only">
            搜索关键词
          </label>
          <Search
            className="absolute left-3 top-1/2 -translate-y-1/2 text-text-dim"
            size={16}
            aria-hidden="true"
          />
          <Input
            id="search-input"
            value={query}
            onChange={(e: React.ChangeEvent<HTMLInputElement>) => {
              // 规范化输入：移除 | 前后的空格
              const normalized = e.target.value.replace(/\s*\|\s*/g, "|");
              onQueryChange(normalized);
            }}
            className="h-9 bg-bg-main pl-10 pr-10 font-mono"
            placeholder="输入关键词，用 | 分隔..."
            onKeyDown={(e: React.KeyboardEvent<HTMLInputElement>) =>
              e.key === "Enter" && onSearch()
            }
          />
        </div>

        <div className="relative">
          <Button
            ref={keywordGroupsTriggerRef}
            variant={isFilterPaletteOpen ? "active" : "secondary"}
            icon={Filter}
            onClick={onFilterPaletteToggle}
            className="w-[140px] justify-between"
            aria-label="Open keyword groups"
            aria-expanded={isFilterPaletteOpen}
            aria-controls="filter-palette"
          >
            Keyword Groups
            <ChevronDown
              size={14}
              className={cn(
                "transition-transform duration-200",
                isFilterPaletteOpen ? "rotate-180" : ""
              )}
            />
          </Button>
          <FilterPalette
            id="filter-palette"
            isOpen={isFilterPaletteOpen}
            onClose={onFilterPaletteClose}
            groups={keywordGroups}
            activeTerms={activeTerms}
            onToggleRule={onToggleRule}
            triggerRef={keywordGroupsTriggerRef}
          />
        </div>
        <label className="relative">
          <span className="sr-only">Export format</span>
          <Download
            className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-text-muted"
            size={15}
          />
          <select
            className="h-9 rounded-[10px] border border-border-base bg-bg-card pl-9 pr-8 text-sm text-text-main hover:bg-bg-hover"
            defaultValue=""
            disabled={disabled}
            onChange={(event) => {
              if (event.target.value)
                onExport(event.target.value as "csv" | "json");
              event.target.value = "";
            }}
          >
            <option value="" disabled>
              Export
            </option>
            <option value="csv">CSV</option>
            <option value="json">JSON</option>
          </select>
        </label>
        <Button
          icon={isSearching ? undefined : Search}
          onClick={onSearch}
          disabled={isSearching || disabled}
          aria-label={disabled ? t("search.no_workspace_selected") : undefined}
        >
          {isSearching && (
            <Loader2 className="animate-spin" size={16} aria-hidden="true" />
          )}
          {isSearching
            ? t("search.searching", "搜索中")
            : t("search.action", "搜索")}
        </Button>
      </div>
    );
  }
);

SearchControls.displayName = "SearchControls";
