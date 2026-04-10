/**
 * 激活关键词展示组件
 * 展示当前搜索查询中的关键词，支持单个删除
 */
import React from 'react';
import { Hash, X } from 'lucide-react';

export interface ActiveKeywordsProps {
  activeTerms: string[];
  onRemoveTerm: (term: string) => void;
}

export const ActiveKeywords: React.FC<ActiveKeywordsProps> = ({
  activeTerms,
  onRemoveTerm,
}) => {
  return (
    <div className="flex items-center gap-2 overflow-x-auto pb-1 scrollbar-none h-6 min-h-[24px]">
      <span className="text-[10px] font-bold text-text-dim uppercase">Active:</span>
      {activeTerms.length > 0 ? activeTerms.map((term: string) => {
        const trimmedTerm = term.trim();
        if (!trimmedTerm) return null;
        return (
          <span
            key={trimmedTerm}
            className="flex items-center text-[10px] bg-bg-card border border-border-base px-1.5 py-0.5 rounded text-text-main whitespace-nowrap group gap-1"
          >
            <Hash size={8} className="mr-0.5 opacity-50"/>
            {trimmedTerm}
            <button
              onClick={() => onRemoveTerm(trimmedTerm)}
              className="opacity-0 group-hover:opacity-100 hover:text-red-400 transition-all ml-0.5"
              title="删除关键词"
              aria-label={`删除关键词 ${trimmedTerm}`}
            >
              <X size={10} />
            </button>
          </span>
        );
      }) : <span className="text-[10px] text-text-dim italic">None</span>}
    </div>
  );
};
