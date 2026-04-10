import type { KeywordGroup } from '../../../types/common';
import type { SearchQuery } from '../../../types/search';
import { SearchQueryBuilder } from '../../../services/SearchQueryBuilder';

export const buildStructuredQueryForSearch = (
  rawQuery: string,
  currentQuery: SearchQuery | null,
  keywordGroups: KeywordGroup[]
): SearchQuery => {
  const baseQuery = currentQuery ?? SearchQueryBuilder.fromString(rawQuery, keywordGroups).getQuery();

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
