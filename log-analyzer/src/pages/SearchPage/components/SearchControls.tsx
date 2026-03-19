/**
 * 搜索控制组件
 * 包含搜索输入框、过滤器按钮、导出按钮、搜索按钮
 */
import React from 'react';
import { Search, Download, Filter, ChevronDown, Loader2 } from 'lucide-react';
import { Button, Input } from '../../../components/ui';
import { FilterPalette } from '../../../components/modals';
import { cn } from '../../../utils/classNames';
import { useTranslation } from 'react-i18next';
import type { KeywordGroup } from '../../../types/common';

export interface SearchControlsProps {
  query: string;
  onQueryChange: (q: string) => void;
  onSearch: () => void;
  onExport: (format: 'csv' | 'json') => void;
  isFilterPaletteOpen: boolean;
  onFilterPaletteToggle: () => void;
  onFilterPaletteClose: () => void;
  isSearching: boolean;
  disabled: boolean;
  searchInputRef: React.RefObject<HTMLInputElement | null>;
  keywordGroups: KeywordGroup[];
  currentQuery: string;
  onToggleRule: (regex: string) => void;
}

export const SearchControls: React.FC<SearchControlsProps> = ({
  query,
  onQueryChange,
  onSearch,
  onExport,
  isFilterPaletteOpen,
  onFilterPaletteToggle,
  onFilterPaletteClose,
  isSearching,
  disabled,
  searchInputRef,
  keywordGroups,
  currentQuery,
  onToggleRule,
}) => {
  const { t } = useTranslation();

  return (
    <div className="flex gap-2">
      <div className="relative flex-1">
        <Search className="absolute left-3 top-2.5 text-text-dim" size={16} />
        <Input
          ref={searchInputRef}
          value={query}
          onChange={(e: React.ChangeEvent<HTMLInputElement>) => {
            // 规范化输入：移除 | 前后的空格
            const normalized = e.target.value.replace(/\s*\|\s*/g, '|');
            onQueryChange(normalized);
          }}
          className="pl-9 pr-10 font-mono bg-bg-main"
          placeholder="Search keywords separated by | ..."
          onKeyDown={(e: React.KeyboardEvent<HTMLInputElement>) => e.key === 'Enter' && onSearch()}
        />
      </div>

      <div className="relative">
        <Button
          variant={isFilterPaletteOpen ? "active" : "secondary"}
          icon={Filter}
          onClick={onFilterPaletteToggle}
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
          onClose={onFilterPaletteClose}
          groups={keywordGroups}
          currentQuery={currentQuery}
          onToggleRule={onToggleRule}
        />
      </div>
      <Button
        icon={Download}
        onClick={() => onExport('csv')}
        disabled={disabled}
        variant="secondary"
        title="Export to CSV"
      >
        CSV
      </Button>
      <Button
        icon={Download}
        onClick={() => onExport('json')}
        disabled={disabled}
        variant="secondary"
        title="Export to JSON"
      >
        JSON
      </Button>
      <Button
        icon={isSearching ? Loader2 : Search}
        onClick={onSearch}
        disabled={isSearching || disabled}
        className={isSearching ? "animate-pulse" : ""}
        title={disabled ? t('search.no_workspace_selected') : undefined}
      >
        {isSearching ? '...' : 'Search'}
      </Button>
    </div>
  );
};
