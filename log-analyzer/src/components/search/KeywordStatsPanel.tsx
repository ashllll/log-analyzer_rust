import React, { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { BarChart3, ChevronDown, X } from 'lucide-react';
import { KeywordStat } from '../../types/search';

/**
 * 关键词统计面板组件属性
 */
interface KeywordStatsPanelProps {
  /** 关键词统计数组 */
  keywords: KeywordStat[];
  
  /** 总匹配数 */
  totalMatches: number;
  
  /** 搜索耗时（毫秒） */
  searchDurationMs: number;
  
  /** 关闭回调（可选） */
  onClose?: () => void;
}

/**
 * 关键词统计面板组件 - 优化版
 * 
 * 特性：
 * - 紧凑的设计，不占用过多空间
 * - 可折叠/展开
 * - 可关闭
 * - 流畅的动画效果
 */
export const KeywordStatsPanel: React.FC<KeywordStatsPanelProps> = ({
  keywords,
  totalMatches,
  searchDurationMs,
  onClose,
}) => {
  const { t } = useTranslation();
  const [isCollapsed, setIsCollapsed] = useState(false);

  const formatNumber = (num: number) => num.toLocaleString();

  return (
    <div className="bg-bg-card/50 border border-border-base rounded-lg shadow-sm backdrop-blur-sm animate-in slide-in-from-top duration-200">
      {/* 紧凑的标题栏 */}
      <div className="flex items-center justify-between px-3 py-2 border-b border-border-base/50">
        <div className="flex items-center gap-2">
          <BarChart3 className="text-primary" size={14} />
          <span className="text-xs font-semibold text-text-main">
            {t('search.statistics.title')}
          </span>
          <span className="text-[10px] text-text-dim">
            {formatNumber(totalMatches)} 条匹配 · {searchDurationMs}ms
          </span>
        </div>
        <div className="flex items-center gap-1">
          <button
            onClick={() => setIsCollapsed(!isCollapsed)}
            className="p-1 text-text-dim hover:text-text-main transition-colors rounded hover:bg-bg-hover"
            aria-label={isCollapsed ? '展开' : '折叠'}
          >
            <ChevronDown 
              size={14} 
              className={`transition-transform duration-200 ${isCollapsed ? 'rotate-180' : ''}`} 
            />
          </button>
          {onClose && (
            <button
              onClick={onClose}
              className="p-1 text-text-dim hover:text-red-400 transition-colors rounded hover:bg-bg-hover"
              aria-label="关闭"
            >
              <X size={14} />
            </button>
          )}
        </div>
      </div>

      {/* 内容区域 - 可折叠 */}
      {!isCollapsed && (
        <div className="p-3 space-y-2">
          {keywords.map((stat, index) => (
            <div key={index} className="space-y-1">
              <div className="flex items-center justify-between text-xs">
                <div className="flex items-center gap-1.5">
                  <span 
                    className="w-2 h-2 rounded-full flex-shrink-0" 
                    style={{ backgroundColor: stat.color }}
                  />
                  <span className="font-medium text-text-main font-mono">
                    {stat.keyword}
                  </span>
                </div>
                <div className="text-text-muted flex items-center gap-1">
                  <span className="font-semibold text-text-main">{formatNumber(stat.matchCount)}</span>
                  <span className="text-[10px]">({stat.matchPercentage.toFixed(1)}%)</span>
                </div>
              </div>
              {/* 紧凑的进度条 */}
              <div className="w-full bg-bg-main rounded-full h-1 overflow-hidden">
                <div 
                  className="h-full rounded-full transition-all duration-300 ease-out"
                  style={{ 
                    width: `${stat.matchPercentage}%`,
                    backgroundColor: stat.color,
                  }}
                />
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};
