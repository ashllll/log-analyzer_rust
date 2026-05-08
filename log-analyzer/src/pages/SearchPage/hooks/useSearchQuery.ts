/**
 * 搜索查询管理 Hook
 * 封装查询字符串、结构化查询、自动保存/加载、防抖触发逻辑
 */
import { useState, useEffect, useCallback } from 'react';
import { SearchQueryBuilder } from '../../../services/SearchQueryBuilder';
import { saveQuery, loadQuery, clearQuery } from '../../../services/queryStorage';
import { splitQueryByPipe } from '../../../utils/searchPatterns';
import type { SearchQuery } from '../../../types/search';
import type { KeywordGroup } from '../../../types/common';

export interface SearchParsingOptions {
  caseSensitive: boolean;
  regexEnabled: boolean;
}

export interface UseSearchQueryReturn {
  query: string;
  currentQuery: SearchQuery | null;
  activeTerms: string[];
  searchTrigger: number;
  setQuery: (q: string) => void;
  setCurrentQuery: (q: SearchQuery | null) => void;
  buildStructuredQuery: (
    rawQuery: string,
    keywordGroups: KeywordGroup[],
    options: SearchParsingOptions
  ) => SearchQuery;
  removeTermFromQuery: (termToRemove: string) => void;
  toggleRuleInQuery: (
    ruleRegex: string,
    keywordGroups: KeywordGroup[],
    options: SearchParsingOptions,
    onError: (msg: string) => void
  ) => void;
}

function shouldResetStructuredQuery(nextQuery: string, currentQuery: SearchQuery | null): boolean {
  if (!currentQuery) return false;
  const currentDisplayQuery = currentQuery.terms
    .filter((t) => t.enabled)
    .map((t) => t.value)
    .join('|');
  return nextQuery !== currentDisplayQuery;
}

function syncStructuredQueryWithSettings(
  query: SearchQuery,
  options: SearchParsingOptions
): SearchQuery {
  return {
    ...query,
    terms: query.terms.map((term) => ({
      ...term,
      caseSensitive: options.caseSensitive,
      isRegex: options.regexEnabled && (term.isRegex || /[.*+?^${}()|[\]\\]/.test(term.value)),
    })),
  };
}

export function useSearchQuery(): UseSearchQueryReturn {
  const [query, setQueryState] = useState('');
  const [currentQuery, setCurrentQueryState] = useState<SearchQuery | null>(null);
  const [searchTrigger, setSearchTrigger] = useState(0);

  // 加载保存的查询
  useEffect(() => {
    const saved = loadQuery();
    if (saved) {
      setCurrentQueryState(saved);
      const builder = SearchQueryBuilder.import(JSON.stringify(saved));
      if (builder) setQueryState(builder.toQueryString());
    }
  }, []);

  // 自动保存查询变化
  useEffect(() => {
    if (currentQuery) {
      saveQuery(currentQuery);
    } else {
      clearQuery();
    }
  }, [currentQuery]);

  // 防抖搜索触发
  useEffect(() => {
    if (!query.trim()) return;
    const timer = setTimeout(() => {
      setSearchTrigger((prev) => prev + 1);
    }, 500);
    return () => clearTimeout(timer);
  }, [query]);

  const setQuery = useCallback((nextQuery: string) => {
    setQueryState(nextQuery);
    if (shouldResetStructuredQuery(nextQuery, currentQuery)) {
      setCurrentQueryState(null);
    }
  }, [currentQuery]);

  const activeTerms = currentQuery
    ? currentQuery.terms.filter((t) => t.enabled).map((t) => t.value)
    : splitQueryByPipe(query);

  const buildStructuredQuery = useCallback(
    (rawQuery: string, keywordGroups: KeywordGroup[], options: SearchParsingOptions): SearchQuery => {
      const baseQuery = currentQuery
        ? syncStructuredQueryWithSettings(currentQuery, options)
        : SearchQueryBuilder.fromString(rawQuery, keywordGroups, options).getQuery();

      return {
        ...baseQuery,
        metadata: {
          ...baseQuery.metadata,
          executionCount: baseQuery.metadata.executionCount + 1,
          lastModified: Date.now(),
        },
      };
    },
    [currentQuery]
  );

  const removeTermFromQuery = useCallback((termToRemove: string) => {
    if (currentQuery) {
      const builder = SearchQueryBuilder.import(JSON.stringify(currentQuery));
      if (builder) {
        const existing = builder.findTermByValue(termToRemove);
        if (existing) {
          builder.removeTerm(existing.id);
          setCurrentQueryState(builder.getQuery());
          setQueryState(builder.toQueryString());
          return;
        }
      }
    }
    const terms = splitQueryByPipe(query);
    const newTerms = terms.filter((t) => t.toLowerCase() !== termToRemove.toLowerCase());
    setQueryState(newTerms.join('|'));
  }, [query, currentQuery]);

  const toggleRuleInQuery = useCallback(
    (
      ruleRegex: string,
      keywordGroups: KeywordGroup[],
      options: SearchParsingOptions,
      onError: (msg: string) => void
    ) => {
      const builder = currentQuery
        ? (SearchQueryBuilder.import(JSON.stringify(currentQuery)) ??
           SearchQueryBuilder.fromString(query, keywordGroups, options))
        : SearchQueryBuilder.fromString(query, keywordGroups, options);

      const existing = builder.findTermByValue(ruleRegex);
      if (existing) {
        builder.toggleTerm(existing.id);
      } else {
        builder.addTerm(ruleRegex, {
          source: 'preset',
          isRegex: options.regexEnabled && /[.*+?^${}()|[\]\\]/.test(ruleRegex),
          operator: 'OR',
          caseSensitive: options.caseSensitive,
        });
      }

      const validation = builder.validate();
      if (!validation.isValid) {
        const errors = validation.issues
          .filter((i) => i.severity === 'error')
          .map((i) => i.message)
          .join(', ');
        onError(`查询验证失败: ${errors}`);
        return;
      }

      const newQuery = syncStructuredQueryWithSettings(builder.getQuery(), options);
      setCurrentQueryState(newQuery);
      setQueryState(builder.toQueryString());
    },
    [query, currentQuery]
  );

  return {
    query,
    currentQuery,
    activeTerms,
    searchTrigger,
    setQuery,
    setCurrentQuery: setCurrentQueryState,
    buildStructuredQuery,
    removeTermFromQuery,
    toggleRuleInQuery,
  };
}
