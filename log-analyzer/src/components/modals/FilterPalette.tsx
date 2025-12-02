import React from 'react';
import { Filter, CheckCircle2 } from 'lucide-react';
import { cn } from '../../utils/classNames';
import { COLOR_STYLES } from '../../constants/colors';
import type { FilterPaletteProps } from '../../types/ui';
import type { KeywordGroup, ColorKey } from '../../types/common';

/**
 * 过滤器面板组件
 * 显示关键词组并支持快速切换规则
 */
const FilterPalette: React.FC<FilterPaletteProps> = ({ 
  isOpen, 
  onClose, 
  groups, 
  currentQuery, 
  onToggleRule 
}) => {
  if (!isOpen) return null;

  const isPatternActive = (regex: string) => 
    currentQuery.split('|').map((t: string) => t.trim().toLowerCase()).includes(regex.toLowerCase());

  const colorOrder: ColorKey[] = ['red', 'orange', 'blue', 'purple', 'green'];

  return (
    <>
      {/* 背景遮罩 */}
      <div 
        className="fixed inset-0 z-[45] bg-transparent" 
        onClick={onClose}
      ></div>

      {/* 面板内容 */}
      <div className="absolute top-full right-0 mt-2 w-[600px] max-h-[60vh] overflow-y-auto bg-[#18181b] border border-border-base rounded-lg shadow-2xl z-50 p-4 grid gap-6 animate-in fade-in zoom-in-95 duration-100 origin-top-right ring-1 ring-white/10">
        {/* 标题 */}
        <div className="flex justify-between items-center pb-2 border-b border-white/10">
          <h3 className="text-sm font-bold text-text-main flex items-center gap-2">
            <Filter size={14} className="text-primary" />
            Filter Command Center
          </h3>
        </div>

        {/* 按颜色分组显示 */}
        {colorOrder.map(color => {
          const colorGroups = groups.filter((g: KeywordGroup) => g.color === color);
          if (colorGroups.length === 0) return null;

          return (
            <div key={color}>
              {/* 颜色分类标题 */}
              <div className={cn(
                "text-[10px] font-bold uppercase mb-2 flex items-center gap-2",
                COLOR_STYLES[color].text
              )}>
                <div className={cn("w-2 h-2 rounded-full", COLOR_STYLES[color].dot)}></div>
                {color} Priority Level
              </div>

              {/* 该颜色下的所有关键词组 */}
              <div className="grid grid-cols-2 gap-3">
                {colorGroups.map((group: KeywordGroup) => (
                  <div 
                    key={group.id} 
                    className="bg-bg-card/50 border border-white/5 rounded p-2"
                  >
                    {/* 组名 */}
                    <div className="text-xs font-semibold text-text-muted mb-2 px-1">
                      {group.name}
                    </div>

                    {/* 模式列表 */}
                    <div className="flex flex-wrap gap-2">
                      {group.patterns.map((p, idx) => {
                        const active = isPatternActive(p.regex);
                        return (
                          <button
                            key={idx}
                            onClick={() => onToggleRule(p.regex)}
                            className={cn(
                              "text-[11px] px-2 py-1 rounded border transition-all duration-150 flex items-center gap-1.5 cursor-pointer",
                              active 
                                ? COLOR_STYLES[color].activeBtn 
                                : `bg-bg-main text-text-dim border-border-base hover:bg-bg-hover ${COLOR_STYLES[color].hoverBorder}`
                            )}
                          >
                            {active && <CheckCircle2 size={10} />}
                            <span className="font-mono">{p.regex}</span>
                          </button>
                        );
                      })}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          );
        })}
      </div>
    </>
  );
};

export default FilterPalette;
