import React, { useMemo, memo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { cn } from '../../utils/classNames';
import { COLOR_STYLES } from '../../constants/colors';
import type { HybridLogRendererProps } from '../../types/ui';
import type { ColorKey } from '../../types/common';

// 智能截断相关接口
interface KeywordPosition {
  start: number;
  end: number;
  keyword: string;
}

interface Snippet {
  start: number;
  end: number;
  text: string;
}

/**
 * 查找文本中所有关键词的位置
 */
const findKeywordPositions = (text: string, keywords: string[]): KeywordPosition[] => {
  const positions: KeywordPosition[] = [];
  const lowerText = text.toLowerCase();
  
  keywords.forEach(keyword => {
    const lowerKeyword = keyword.toLowerCase();
    let startIndex = 0;
    
    while (startIndex < text.length) {
      const index = lowerText.indexOf(lowerKeyword, startIndex);
      if (index === -1) break;
      
      positions.push({
        start: index,
        end: index + keyword.length,
        keyword
      });
      startIndex = index + 1;
    }
  });
  
  return positions.sort((a, b) => a.start - b.start);
};

/**
 * 提取关键词周围的文本片段
 */
const extractSnippets = (text: string, positions: KeywordPosition[], contextLength: number): Snippet[] => {
  if (positions.length === 0) return [];
  
  const snippets: Snippet[] = [];
  
  positions.forEach(pos => {
    const start = Math.max(0, pos.start - contextLength);
    const end = Math.min(text.length, pos.end + contextLength);
    snippets.push({ start, end, text: text.substring(start, end) });
  });
  
  return snippets;
};

/**
 * 合并重叠或相邻的片段
 */
const mergeOverlappingSnippets = (snippets: Snippet[]): Snippet[] => {
  if (snippets.length === 0) return [];
  
  const merged: Snippet[] = [];
  let current = snippets[0];
  
  for (let i = 1; i < snippets.length; i++) {
    const next = snippets[i];
    
    // 如果片段重叠或相邻（间隔小于10字符）
    if (next.start <= current.end + 10) {
      current = {
        start: current.start,
        end: Math.max(current.end, next.end),
        text: '' // 稍后重新提取
      };
    } else {
      merged.push(current);
      current = next;
    }
  }
  merged.push(current);
  
  return merged;
};

/**
 * 日志高亮渲染组件
 * 支持搜索关键词和关键词组的颜色高亮
 * 包含性能优化：智能截断、匹配数量限制、React.memo 防止不必要重渲染
 */
const HybridLogRendererInner: React.FC<HybridLogRendererProps> = ({ text, query, keywordGroups }) => {
  const { t } = useTranslation();
  const [isExpanded, setIsExpanded] = useState(false);
  const { patternMap, regexPattern } = useMemo(() => {
    const map = new Map();
    const patterns = new Set();

    // 处理关键词组中的模式
    keywordGroups
      .filter((g: any) => g.enabled)
      .forEach((group: any) => {
        group.patterns.forEach((p: any) => {
          if (p.regex?.trim()) {
            map.set(p.regex.toLowerCase(), {
              color: group.color,
              comment: p.comment
            });
            patterns.add(p.regex);
          }
        });
      });

    // 处理查询字符串中的关键词
    if (query) {
      query
        .split('|')
        .map((t: string) => t.trim())
        .filter((t: string) => t.length > 0)
        .forEach((term: string, index: number) => {
          if (!map.has(term.toLowerCase())) {
            map.set(term.toLowerCase(), {
              color: ['blue', 'purple', 'green', 'orange'][index % 4],
              comment: ""
            });
          }
          patterns.add(term);
        });
    }

    // 按长度排序，确保长模式优先匹配
    const sorted = Array.from(patterns).sort((a: any, b: any) => b.length - a.length);

    return {
      regexPattern: sorted.length > 0
        ? new RegExp(
            `(${sorted.map((p: any) => p.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')).join('|')})`,
            'gi'
          )
        : null,
      patternMap: map
    };
  }, [keywordGroups, query]);

  // 渲染带高亮的文本
  const renderHighlightedText = (textToRender: string, showEllipsis: { start?: boolean; end?: boolean } = {}) => {
    if (!regexPattern) return <span>{textToRender}</span>;

    const parts = textToRender.split(regexPattern);

    // 性能优化：匹配数量检查
    const matchCount = parts.filter(part => patternMap.has(part.toLowerCase())).length;
    if (matchCount > 20) {
      // 降级为纯文本+提示
      return (
        <span className="text-text-dim">
          {textToRender}{' '}
          <span className="text-[10px] bg-red-500/20 text-red-400 px-1 py-0.5 rounded border border-red-500/30 ml-1">
            [{matchCount} matches - rendering disabled for performance]
          </span>
        </span>
      );
    }

    return (
      <span>
        {showEllipsis.start && <span className="text-gray-400 dark:text-gray-600">...</span>}
        {parts.map((part: string, i: number) => {
          const info = patternMap.get(part.toLowerCase());
          if (info) {
            const style = COLOR_STYLES[info.color as ColorKey]?.highlight || COLOR_STYLES['blue'].highlight;
            return (
              <span key={i} className="inline-flex items-baseline mx-[1px]">
                <span className={cn("rounded-[2px] px-1 border font-bold break-all", style)}>
                  {part}
                </span>
                {info.comment && (
                  <span
                    className={cn(
                      "ml-1 px-1.5 rounded-[2px] text-[10px] font-normal border select-none whitespace-nowrap transform -translate-y-[1px]",
                      style.replace("bg-", "bg-opacity-10 bg-")
                    )}
                  >
                    {info.comment}
                  </span>
                )}
              </span>
            );
          }
          return <span key={i}>{part}</span>;
        })}
        {showEllipsis.end && <span className="text-gray-400 dark:text-gray-600">...</span>}
      </span>
    );
  };

  // 智能截断逻辑
  const TRUNCATE_THRESHOLD = 1000;
  const CONTEXT_LENGTH = 50;

  // 如果文本较短或已展开，直接显示完整文本
  if (text.length <= TRUNCATE_THRESHOLD || isExpanded) {
    return (
      <span>
        {renderHighlightedText(text)}
        {isExpanded && text.length > TRUNCATE_THRESHOLD && (
          <button
            onClick={() => setIsExpanded(false)}
            className="ml-2 text-xs text-blue-500 hover:text-blue-700 dark:text-blue-400 dark:hover:text-blue-300 underline cursor-pointer"
          >
            {t('search.collapse_text', 'Collapse')}
          </button>
        )}
      </span>
    );
  }

  // 文本过长，使用智能截断
  const keywords = Array.from(patternMap.keys()).map(k => 
    // 从map的key中找到原始关键词（保持大小写）
    query?.split('|').find(t => t.trim().toLowerCase() === k) || k
  );
  
  const positions = findKeywordPositions(text, keywords);
  
  if (positions.length === 0) {
    // 没有关键词，显示前1000字符
    return (
      <span>
        {renderHighlightedText(text.substring(0, TRUNCATE_THRESHOLD), { end: true })}
        <button
          onClick={() => setIsExpanded(true)}
          className="ml-2 text-xs text-blue-500 hover:text-blue-700 dark:text-blue-400 dark:hover:text-blue-300 underline cursor-pointer"
        >
          {t('search.expand_full_text', 'Expand Full Text')}
        </button>
      </span>
    );
  }

  // 提取关键词周围的片段
  const snippets = extractSnippets(text, positions, CONTEXT_LENGTH);
  const mergedSnippets = mergeOverlappingSnippets(snippets);

  return (
    <span>
      {mergedSnippets.map((snippet, index) => {
        const snippetText = text.substring(snippet.start, snippet.end);
        const showStartEllipsis = snippet.start > 0;
        const showEndEllipsis = snippet.end < text.length && index === mergedSnippets.length - 1;
        
        return (
          <span key={index}>
            {renderHighlightedText(snippetText, { 
              start: showStartEllipsis, 
              end: showEndEllipsis && index === mergedSnippets.length - 1
            })}
            {index < mergedSnippets.length - 1 && (
              <span className="text-gray-400 dark:text-gray-600"> ... </span>
            )}
          </span>
        );
      })}
      <button
        onClick={() => setIsExpanded(true)}
        className="ml-2 text-xs text-blue-500 hover:text-blue-700 dark:text-blue-400 dark:hover:text-blue-300 underline cursor-pointer"
      >
        {t('search.expand_full_text', 'Expand Full Text')}
      </button>
    </span>
  );
};

/**
 * 使用 React.memo 包装，避免不必要的重渲染
 * 自定义比较函数：仅当 text、query 或 keywordGroups 引用变化时才重新渲染
 */
const HybridLogRenderer = memo(HybridLogRendererInner, (prevProps, nextProps) => {
  // 返回 true 表示 props 相同，不需要重渲染
  return (
    prevProps.text === nextProps.text &&
    prevProps.query === nextProps.query &&
    prevProps.keywordGroups === nextProps.keywordGroups
  );
});

export default HybridLogRenderer;
