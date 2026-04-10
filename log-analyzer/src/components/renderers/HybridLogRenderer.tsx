import React, { useMemo, memo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { COLOR_STYLES } from '../../constants/colors';
import type { HybridLogRendererProps } from '../../types/ui';
import type { ColorKey, KeywordPattern } from '../../types/common';
import { escapeRegexLiteral, looksLikeRegexPattern } from '../../utils/searchPatterns';

// 智能截断相关接口
interface KeywordPosition {
  start: number;
  end: number;
}

interface Snippet {
  start: number;
  end: number;
  text: string;
}

interface HighlightPattern {
  id: string;
  raw: string;
  color: ColorKey | string;
  comment: string;
  mode: 'literal' | 'regex';
  caseSensitive: boolean;
  matcher?: RegExp;
}

interface HighlightMatch {
  start: number;
  end: number;
  text: string;
  pattern: HighlightPattern;
}

const createRegexMatcher = (pattern: string, caseSensitive: boolean): RegExp => {
  try {
    return new RegExp(pattern, caseSensitive ? 'g' : 'gi');
  } catch {
    return new RegExp(escapeRegexLiteral(pattern), caseSensitive ? 'g' : 'gi');
  }
};

/**
 * 查找文本中所有高亮模式的位置
 */
const collectMatches = (text: string, patterns: HighlightPattern[]): HighlightMatch[] => {
  const matches: HighlightMatch[] = [];
  const lowerText = text.toLowerCase();

  patterns.forEach((pattern) => {
    if (pattern.mode === 'literal') {
      const literalNeedle = pattern.caseSensitive ? pattern.raw : pattern.raw.toLowerCase();
      if (!literalNeedle) {
        return;
      }

      let startIndex = 0;
      while (startIndex < text.length) {
        const haystack = pattern.caseSensitive ? text : lowerText;
        const index = haystack.indexOf(literalNeedle, startIndex);
        if (index === -1) {
          break;
        }

        matches.push({
          start: index,
          end: index + pattern.raw.length,
          text: text.slice(index, index + pattern.raw.length),
          pattern,
        });
        startIndex = index + 1;
      }

      return;
    }

    if (!pattern.matcher) {
      return;
    }

    pattern.matcher.lastIndex = 0;
    let match: RegExpExecArray | null;
    while ((match = pattern.matcher.exec(text)) !== null) {
      if (match[0].length === 0) {
        pattern.matcher.lastIndex += 1;
        continue;
      }

      matches.push({
        start: match.index,
        end: match.index + match[0].length,
        text: match[0],
        pattern,
      });
    }
  });

  matches.sort((left, right) => {
    if (left.start !== right.start) {
      return left.start - right.start;
    }

    const leftLength = left.end - left.start;
    const rightLength = right.end - right.start;
    if (leftLength !== rightLength) {
      return rightLength - leftLength;
    }

    return right.pattern.raw.length - left.pattern.raw.length;
  });

  const nonOverlapping: HighlightMatch[] = [];
  let currentEnd = -1;
  for (const match of matches) {
    if (match.start < currentEnd) {
      continue;
    }

    nonOverlapping.push(match);
    currentEnd = match.end;
  }

  return nonOverlapping;
};

const findKeywordPositions = (text: string, patterns: HighlightPattern[]): KeywordPosition[] => {
  return collectMatches(text, patterns).map((match) => ({
    start: match.start,
    end: match.end,
  }));
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
const HybridLogRendererInner: React.FC<HybridLogRendererProps> = ({
  text,
  query,
  queryTerms,
  keywordGroups,
}) => {
  const { t } = useTranslation();
  const [isExpanded, setIsExpanded] = useState(false);
  const highlightPatterns = useMemo(() => {
    const patterns = new Map<string, HighlightPattern>();

    keywordGroups
      .filter((g) => g.enabled)
      .forEach((group) => {
        group.patterns.forEach((p: KeywordPattern) => {
          if (p.regex?.trim()) {
            const mode = looksLikeRegexPattern(p.regex) ? 'regex' : 'literal';
            const key = `${mode}:${p.regex.toLowerCase()}`;

            if (!patterns.has(key)) {
              patterns.set(key, {
                id: key,
                raw: p.regex,
                color: group.color,
                comment: p.comment,
                mode,
                caseSensitive: false,
                matcher: mode === 'regex' ? createRegexMatcher(p.regex, false) : undefined,
              });
            }
          }
        });
      });

    const resolvedQueryTerms = queryTerms?.filter((term) => term.enabled) ?? [];
    if (resolvedQueryTerms.length > 0) {
      resolvedQueryTerms.forEach((term, index) => {
        const mode = term.isRegex ? 'regex' : 'literal';
        const key = `${mode}:${term.caseSensitive ? 'cs' : 'ci'}:${term.value.toLowerCase()}`;

        if (!patterns.has(key)) {
          patterns.set(key, {
            id: key,
            raw: term.value,
            color: ['blue', 'purple', 'green', 'orange'][index % 4],
            comment: '',
            mode,
            caseSensitive: term.caseSensitive,
            matcher: mode === 'regex'
              ? createRegexMatcher(term.value, term.caseSensitive)
              : undefined,
          });
        }
      });
    } else if (query) {
      query
        .split('|')
        .map((term: string) => term.trim())
        .filter((term: string) => term.length > 0)
        .forEach((term: string, index: number) => {
          const key = `literal:ci:${term.toLowerCase()}`;
          if (!patterns.has(key)) {
            patterns.set(key, {
              id: key,
              raw: term,
              color: ['blue', 'purple', 'green', 'orange'][index % 4],
              comment: '',
              mode: 'literal',
              caseSensitive: false,
            });
          }
        });
    }

    return Array.from(patterns.values()).sort((a, b) => b.raw.length - a.raw.length);
  }, [keywordGroups, query, queryTerms]);

  // 渲染带高亮的文本
  // 性能保护：每个关键词最多渲染 MAX_HIGHLIGHT_PER_KEYWORD 个高亮 span，
  // 超出后该关键词退化为纯文本，避免单行数千匹配导致 DOM 节点爆炸。
  // 不同关键词独立计数，不会相互影响。
  const MAX_HIGHLIGHT_PER_KEYWORD = 30;

  const renderHighlightedText = (textToRender: string, showEllipsis: { start?: boolean; end?: boolean } = {}) => {
    if (highlightPatterns.length === 0) {
      return <span>{textToRender}</span>;
    }

    const matches = collectMatches(textToRender, highlightPatterns);
    if (matches.length === 0) {
      return (
        <span>
          {showEllipsis.start && <span className="text-gray-400 dark:text-gray-600">...</span>}
          <span>{textToRender}</span>
          {showEllipsis.end && <span className="text-gray-400 dark:text-gray-600">...</span>}
        </span>
      );
    }

    // 跟踪每个关键词已渲染的高亮次数
    const highlightCounts = new Map<string, number>();
    const segments: React.ReactNode[] = [];
    let cursor = 0;

    matches.forEach((match, index) => {
      if (cursor < match.start) {
        segments.push(<span key={`text-${index}-${cursor}`}>{textToRender.slice(cursor, match.start)}</span>);
      }

      const currentCount = highlightCounts.get(match.pattern.id) ?? 0;
      if (currentCount >= MAX_HIGHLIGHT_PER_KEYWORD) {
        segments.push(<span key={`plain-${index}-${match.start}`}>{match.text}</span>);
      } else {
        highlightCounts.set(match.pattern.id, currentCount + 1);
        const style =
          COLOR_STYLES[match.pattern.color as ColorKey]?.highlight || COLOR_STYLES.blue.highlight;

        segments.push(
          <span key={`highlight-${index}-${match.start}`} className="inline-block mx-[1px]">
            <span className={`rounded-[2px] px-1 border font-bold break-words ${style}`}>
              {match.text}
            </span>
            {match.pattern.comment && currentCount === 0 && (
              <span
                className={`ml-1 px-1.5 rounded-[2px] text-[10px] font-normal border select-none whitespace-nowrap transform -translate-y-[1px] ${
                  COLOR_STYLES[match.pattern.color as ColorKey]?.badge ?? COLOR_STYLES.blue.badge
                }`}
              >
                {match.pattern.comment}
              </span>
            )}
          </span>
        );
      }

      cursor = match.end;
    });

    if (cursor < textToRender.length) {
      segments.push(<span key={`tail-${cursor}`}>{textToRender.slice(cursor)}</span>);
    }

    return (
      <span>
        {showEllipsis.start && <span className="text-gray-400 dark:text-gray-600">...</span>}
        {segments}
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
  const positions = findKeywordPositions(text, highlightPatterns);
  
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
              end: showEndEllipsis
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
    prevProps.queryTerms === nextProps.queryTerms &&
    prevProps.keywordGroups === nextProps.keywordGroups
  );
});

export default HybridLogRenderer;
