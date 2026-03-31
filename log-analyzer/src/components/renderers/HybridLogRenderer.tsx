import React, { useMemo, memo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { COLOR_STYLES } from '../../constants/colors';
import type { HybridLogRendererProps } from '../../types/ui';
import type { ColorKey, KeywordPattern } from '../../types/common';

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
    const map = new Map<string, { color: ColorKey | string; comment: string }>();
    const patterns = new Set<string>();

    // 处理关键词组中的模式
    keywordGroups
      .filter((g) => g.enabled)
      .forEach((group) => {
        group.patterns.forEach((p: KeywordPattern) => {
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
        .map((term: string) => term.trim())
        .filter((term: string) => term.length > 0)
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
    const sorted = Array.from(patterns).sort((a: string, b: string) => b.length - a.length);

    return {
      regexPattern: sorted.length > 0
        ? new RegExp(
            `(${sorted.map((p: string) => p.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')).join('|')})`,
            'gi'
          )
        : null,
      patternMap: map
    };
  }, [keywordGroups, query]);

  // 渲染带高亮的文本
  // 性能保护：每个关键词最多渲染 MAX_HIGHLIGHT_PER_KEYWORD 个高亮 span，
  // 超出后该关键词退化为纯文本，避免单行数千匹配导致 DOM 节点爆炸。
  // 不同关键词独立计数，不会相互影响。
  const MAX_HIGHLIGHT_PER_KEYWORD = 30;

  const renderHighlightedText = (textToRender: string, showEllipsis: { start?: boolean; end?: boolean } = {}) => {
    if (!regexPattern) return <span>{textToRender}</span>;

    const parts = textToRender.split(regexPattern);

    // 跟踪每个关键词已渲染的高亮次数
    const highlightCounts = new Map<string, number>();

    return (
      <span>
        {showEllipsis.start && <span className="text-gray-400 dark:text-gray-600">...</span>}
        {parts.map((part: string, i: number) => {
          const lowerPart = part.toLowerCase();
          const info = patternMap.get(lowerPart);
          if (info) {
            const currentCount = highlightCounts.get(lowerPart) ?? 0;
            if (currentCount >= MAX_HIGHLIGHT_PER_KEYWORD) {
              // 该关键词已达到渲染上限，退化为纯文本
              return <span key={i}>{part}</span>;
            }
            highlightCounts.set(lowerPart, currentCount + 1);
            const style = COLOR_STYLES[info.color as ColorKey]?.highlight || COLOR_STYLES['blue'].highlight;
            return (
              <span key={i} className="inline-block mx-[1px]">
                <span className={`rounded-[2px] px-1 border font-bold break-words ${style}`}>
                  {part}
                </span>
                {info.comment && currentCount === 0 && (
                  <span
                    className={`ml-1 px-1.5 rounded-[2px] text-[10px] font-normal border select-none whitespace-nowrap transform -translate-y-[1px] ${COLOR_STYLES[info.color as ColorKey]?.badge ?? COLOR_STYLES['blue'].badge}`}
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
            className="ml-2 text-xs text-primary hover:text-primary-hover underline cursor-pointer"
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
              <span className="text-text-dim"> ... </span>
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
