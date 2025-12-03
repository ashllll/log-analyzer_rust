import React, { useMemo, memo } from 'react';
import { cn } from '../../utils/classNames';
import { COLOR_STYLES } from '../../constants/colors';
import type { HybridLogRendererProps } from '../../types/ui';
import type { ColorKey } from '../../types/common';

/**
 * 日志高亮渲染组件
 * 支持搜索关键词和关键词组的颜色高亮
 * 包含性能优化：超长文本截断、匹配数量限制、React.memo 防止不必要重渲染
 */
const HybridLogRendererInner: React.FC<HybridLogRendererProps> = ({ text, query, keywordGroups }) => {
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

  // 性能优化：文本长度检查
  if (text.length > 500) {
    // 超长文本截断显示
    const truncated = text.substring(0, 500) + '...';
    if (!regexPattern) return <span>{truncated}</span>;

    const parts = truncated.split(regexPattern);
    // 仅高亮可见部分
    return (
      <span>
        {parts.map((part: string, i: number) => {
          const info = patternMap.get(part.toLowerCase());
          if (info) {
            const style = COLOR_STYLES[info.color as ColorKey]?.highlight || COLOR_STYLES['blue'].highlight;
            return (
              <span key={i} className={cn("rounded-[2px] px-1 border font-bold break-all", style)}>
                {part}
              </span>
            );
          }
          return <span key={i}>{part}</span>;
        })}
        <span className="text-text-dim ml-1">[truncated]</span>
      </span>
    );
  }

  if (!regexPattern) return <span>{text}</span>;

  const parts = text.split(regexPattern);

  // 性能优化：匹配数量检查
  const matchCount = parts.filter(part => patternMap.has(part.toLowerCase())).length;
  if (matchCount > 20) {
    // 降级为纯文本+提示
    return (
      <span className="text-text-dim">
        {text}{' '}
        <span className="text-[10px] bg-red-500/20 text-red-400 px-1 py-0.5 rounded border border-red-500/30 ml-1">
          [{matchCount} matches - rendering disabled for performance]
        </span>
      </span>
    );
  }

  return (
    <span>
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
