import React, { useState } from 'react';
import { useTranslation } from 'react-i18next';
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
 * 关键词统计面板组件
 * 
 * 用于展示多关键词搜索的统计信息，包括：
 * - 每个关键词的匹配数量
 * - 匹配百分比
 * - 可视化进度条
 * - 总匹配数和搜索耗时
 * 
 * 注意：这是一个纯展示组件，不提供筛选功能
 */
export const KeywordStatsPanel: React.FC<KeywordStatsPanelProps> = ({
  keywords,
  totalMatches,
  searchDurationMs,
  // onClose 参数保留供未来使用，当前通过折叠功能代替
}) => {
  const { t } = useTranslation();
  const [isCollapsed, setIsCollapsed] = useState(false);

  const formatNumber = (num: number) => num.toLocaleString();

  return (
    <div className="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg shadow-sm mb-4">
      {/* 标题栏 */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-gray-200 dark:border-gray-700">
        <div className="flex items-center gap-2">
          <svg className="w-5 h-5 text-blue-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
          </svg>
          <h3 className="text-sm font-semibold text-gray-700 dark:text-gray-300">
            {t('search.statistics.title')}
          </h3>
        </div>
        <button
          onClick={() => setIsCollapsed(!isCollapsed)}
          className="text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200 transition-colors"
          aria-label={isCollapsed ? t('search.statistics.expand') : t('search.statistics.collapse')}
        >
          <svg className={`w-5 h-5 transition-transform ${isCollapsed ? 'rotate-180' : ''}`} fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
          </svg>
        </button>
      </div>

      {/* 内容区域 */}
      {!isCollapsed && (
        <div className="p-4">
          {/* 总览信息 */}
          <div className="mb-4 text-sm text-gray-600 dark:text-gray-400">
            Total: <span className="font-semibold text-gray-900 dark:text-gray-100">{formatNumber(totalMatches)}</span> {t('search.statistics.matches_count').replace(/\{\{count\}\}\s*/, '')} in <span className="font-semibold">{searchDurationMs}</span>ms
          </div>

          {/* 关键词统计列表 */}
          <div className="space-y-3">
            {keywords.map((stat, index) => (
              <div key={index} className="space-y-1">
                <div className="flex items-center justify-between text-sm">
                  <div className="flex items-center gap-2">
                    <span 
                      className="w-3 h-3 rounded-full" 
                      style={{ backgroundColor: stat.color }}
                    />
                    <span className="font-medium text-gray-700 dark:text-gray-300">
                      {stat.keyword}
                    </span>
                  </div>
                  <div className="text-gray-600 dark:text-gray-400">
                    <span className="font-semibold">{formatNumber(stat.matchCount)}</span>
                    <span className="ml-1">{t('search.statistics.matches_count').replace(/\{\{count\}\}\s*/, '')}</span>
                    <span className="ml-2 text-xs">({stat.matchPercentage.toFixed(1)}%)</span>
                  </div>
                </div>
                {/* 进度条 */}
                <div className="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-2 overflow-hidden">
                  <div 
                    className="h-full rounded-full transition-all duration-300 ease-out"
                    style={{ 
                      width: `${stat.matchPercentage}%`,
                      backgroundColor: stat.color,
                      opacity: 0.8
                    }}
                  />
                </div>
              </div>
            ))}
          </div>

          {/* 底部说明 */}
          <div className="mt-4 pt-4 border-t border-gray-200 dark:border-gray-700">
            <p className="text-xs text-gray-500 dark:text-gray-400 italic">
              {t('search.statistics.showing_all_results')}
            </p>
          </div>
        </div>
      )}
    </div>
  );
};
