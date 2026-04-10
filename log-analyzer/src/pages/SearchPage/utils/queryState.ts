import type { KeywordGroup } from '../../../types/common';
import type { SearchQuery, SearchTerm } from '../../../types/search';
import { SearchQueryBuilder } from '../../../services/SearchQueryBuilder';
import { looksLikeRegexPattern } from '../../../utils/searchPatterns';

export interface SearchParsingOptions {
  caseSensitive: boolean;
  regexEnabled: boolean;
}

export const syncStructuredQueryWithSettings = (
  query: SearchQuery,
  options: SearchParsingOptions
): SearchQuery => ({
  ...query,
  terms: query.terms.map((term: SearchTerm) => ({
    ...term,
    caseSensitive: options.caseSensitive,
    isRegex: options.regexEnabled && (term.isRegex || looksLikeRegexPattern(term.value)),
  })),
});

export const buildStructuredQueryForSearch = (
  rawQuery: string,
  currentQuery: SearchQuery | null,
  keywordGroups: KeywordGroup[],
  options: SearchParsingOptions
): SearchQuery => {
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
};

export const deriveActiveTerms = (query: string, currentQuery: SearchQuery | null): string[] => {
  if (currentQuery) {
    return currentQuery.terms
      .filter((term) => term.enabled)
      .map((term) => term.value);
  }

  return query
    .split('|')
    .map((term) => term.trim())
    .filter((term) => term.length > 0);
};

export const shouldResetStructuredQuery = (
  nextQuery: string,
  currentQuery: SearchQuery | null
): boolean => {
  if (!currentQuery) {
    return false;
  }

  const currentDisplayQuery = currentQuery.terms
    .filter((term) => term.enabled)
    .map((term) => term.value)
    .join('|');

  return nextQuery !== currentDisplayQuery;
};
