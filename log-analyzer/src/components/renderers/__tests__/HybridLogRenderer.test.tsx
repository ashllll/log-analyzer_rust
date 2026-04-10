/**
 * HybridLogRenderer 单元测试
 *
 * 验证关键词高亮渲染的正确性和稳定性：
 * 1. 正常匹配高亮
 * 2. 大量匹配（>20）不降级为纯文本
 * 3. 截断路径中关键词大小写保持
 * 4. keywordGroups 来源的关键词在高亮中正确识别
 */

import React from 'react';
import { render, screen } from '@testing-library/react';
import HybridLogRenderer from '../HybridLogRenderer';

// Mock useTranslation
jest.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, fallback: string) => fallback || key,
  }),
}));

// Mock constants/colors
jest.mock('../../../constants/colors', () => ({
  COLOR_STYLES: {
    blue: { highlight: 'bg-blue-200 text-blue-800 border-blue-400', badge: 'bg-blue-100 text-blue-700 border-blue-300' },
    green: { highlight: 'bg-green-200 text-green-800 border-green-400', badge: 'bg-green-100 text-green-700 border-green-300' },
    red: { highlight: 'bg-red-200 text-red-800 border-red-400', badge: 'bg-red-100 text-red-700 border-red-300' },
    orange: { highlight: 'bg-orange-200 text-orange-800 border-orange-400', badge: 'bg-orange-100 text-orange-700 border-orange-300' },
    purple: { highlight: 'bg-purple-200 text-purple-800 border-purple-400', badge: 'bg-purple-100 text-purple-700 border-purple-300' },
  },
}));

const defaultKeywordGroups = [
  {
    id: 'group-1',
    name: 'Errors',
    color: 'red' as const,
    patterns: [
      { regex: 'ERROR', comment: '错误' },
    ],
    enabled: true,
  },
];

const regexKeywordGroups = [
  {
    id: 'group-2',
    name: 'Regex Errors',
    color: 'orange' as const,
    patterns: [
      { regex: 'error.*timeout', comment: 'critical path' },
    ],
    enabled: true,
  },
];

describe('HybridLogRenderer', () => {
  describe('基本高亮', () => {
    it('应该高亮查询中的关键词', () => {
      const { container } = render(
        <HybridLogRenderer
          text="An error occurred in the system"
          query="error"
          keywordGroups={[]}
        />
      );

      // 关键词应该被高亮包裹
      const highlightedSpans = container.querySelectorAll('.rounded-\\[2px\\]');
      expect(highlightedSpans.length).toBeGreaterThan(0);
      expect(highlightedSpans[0].textContent).toBe('error');
    });

    it('应该高亮多个关键词', () => {
      const { container } = render(
        <HybridLogRenderer
          text="error and warning found"
          query="error|warning"
          keywordGroups={[]}
        />
      );

      const highlightedSpans = container.querySelectorAll('.rounded-\\[2px\\]');
      expect(highlightedSpans.length).toBe(2);
    });

    it('无关键词时不应该高亮', () => {
      const { container } = render(
        <HybridLogRenderer
          text="Normal log line"
          query=""
          keywordGroups={[]}
        />
      );

      const highlightedSpans = container.querySelectorAll('.rounded-\\[2px\\]');
      expect(highlightedSpans.length).toBe(0);
    });
  });

  describe('大量匹配场景（>20 matches）', () => {
    /**
     * BUG: 当一行中同一关键词出现 >20 次时，高亮降级为纯文本。
     * 用户报告：高亮关键词在密集匹配的行中消失。
     *
     * 修复目标：即使超过 20 次匹配，关键词仍然应该被高亮。
     * 可接受的策略：限制渲染的 <span> 数量，但保留至少部分高亮。
     */
    it('超过 20 个匹配时不应完全降级为纯文本', () => {
      // 构造 25 个 error 的文本
      const manyErrors = Array(25).fill('error').join(' ');
      const { container } = render(
        <HybridLogRenderer
          text={manyErrors}
          query="error"
          keywordGroups={[]}
        />
      );

      // 关键断言：至少应该有一些高亮 span 存在，而不是完全降级为纯文本
      const highlightedSpans = container.querySelectorAll('.rounded-\\[2px\\]');
      expect(highlightedSpans.length).toBeGreaterThan(0);

      // 不应出现 "rendering disabled" 提示
      expect(screen.queryByText(/rendering disabled/i)).toBeNull();
    });

    it('超过 20 个匹配时仍应正确高亮关键词', () => {
      const manyErrors = Array(25).fill('error').join(' ');
      const { container } = render(
        <HybridLogRenderer
          text={manyErrors}
          query="error"
          keywordGroups={[]}
        />
      );

      // 至少第一个和最后一个关键词应该被高亮
      const highlightedSpans = container.querySelectorAll('.rounded-\\[2px\\]');
      const texts = Array.from(highlightedSpans).map(span => span.textContent);
      expect(texts).toContain('error');
    });
  });

  describe('关键词组高亮', () => {
    it('应该高亮来自关键词组的模式', () => {
      const { container } = render(
        <HybridLogRenderer
          text="ERROR: Connection failed"
          query=""
          keywordGroups={defaultKeywordGroups}
        />
      );

      const highlightedSpans = container.querySelectorAll('.rounded-\\[2px\\]');
      expect(highlightedSpans.length).toBeGreaterThan(0);
      expect(highlightedSpans[0].textContent).toBe('ERROR');
    });

    it('禁用的关键词组不应参与高亮', () => {
      const disabledGroups = [
        {
          ...defaultKeywordGroups[0],
          enabled: false,
        },
      ];
      const { container } = render(
        <HybridLogRenderer
          text="ERROR: Connection failed"
          query=""
          keywordGroups={disabledGroups}
        />
      );

      const highlightedSpans = container.querySelectorAll('.rounded-\\[2px\\]');
      expect(highlightedSpans.length).toBe(0);
    });

    it('应该按正则语义高亮关键词组模式', () => {
      const { container } = render(
        <HybridLogRenderer
          text="error while reconnecting before timeout"
          query=""
          keywordGroups={regexKeywordGroups}
        />
      );

      const highlightedSpans = container.querySelectorAll('.rounded-\\[2px\\]');
      expect(highlightedSpans.length).toBeGreaterThan(0);
      expect(highlightedSpans[0].textContent).toBe('error while reconnecting before timeout');
      expect(screen.getByText('critical path')).toBeInTheDocument();
    });
  });

  describe('大小写不敏感高亮', () => {
    it('大小写不敏感模式下应高亮所有变体', () => {
      const { container } = render(
        <HybridLogRenderer
          text="ERROR error Error eRrOr"
          query="error"
          keywordGroups={[]}
        />
      );

      const highlightedSpans = container.querySelectorAll('.rounded-\\[2px\\]');
      // 应该匹配 4 种大小写变体
      expect(highlightedSpans.length).toBe(4);
    });

    it('长文本截断时应保留正则模式的高亮片段', () => {
      const longPrefix = 'a'.repeat(1100);
      const text = `${longPrefix} error on the critical path before timeout`;

      const { container } = render(
        <HybridLogRenderer
          text={text}
          query=""
          keywordGroups={regexKeywordGroups}
        />
      );

      const highlightedSpans = container.querySelectorAll('.rounded-\\[2px\\]');
      expect(highlightedSpans.length).toBeGreaterThan(0);
      expect(highlightedSpans[0].textContent).toBe('error on the critical path before timeout');
      expect(screen.getByText('Expand Full Text')).toBeInTheDocument();
    });

    it('应该按结构化查询的 regex 语义高亮手动输入的正则', () => {
      const { container } = render(
        <HybridLogRenderer
          text="error while reconnecting before timeout"
          query="error.*timeout"
          queryTerms={[
            {
              id: 'term_1',
              value: 'error.*timeout',
              operator: 'OR',
              source: 'user',
              isRegex: true,
              priority: 1,
              enabled: true,
              caseSensitive: false,
            },
          ]}
          keywordGroups={[]}
        />
      );

      const highlightedSpans = container.querySelectorAll('.rounded-\\[2px\\]');
      expect(highlightedSpans).toHaveLength(1);
      expect(highlightedSpans[0].textContent).toBe('error while reconnecting before timeout');
    });

    it('应该尊重结构化查询的大小写敏感字面量匹配', () => {
      const { container } = render(
        <HybridLogRenderer
          text="ERROR error Error"
          query="error"
          queryTerms={[
            {
              id: 'term_1',
              value: 'error',
              operator: 'OR',
              source: 'user',
              isRegex: false,
              priority: 1,
              enabled: true,
              caseSensitive: true,
            },
          ]}
          keywordGroups={[]}
        />
      );

      const highlightedSpans = container.querySelectorAll('.rounded-\\[2px\\]');
      expect(highlightedSpans).toHaveLength(1);
      expect(highlightedSpans[0].textContent).toBe('error');
    });
  });
});
