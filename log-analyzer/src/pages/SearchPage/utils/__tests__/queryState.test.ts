import type { KeywordGroup } from '../../../../types/common';
import type { SearchQuery } from '../../../../types/search';
import {
  buildStructuredQueryForSearch,
  deriveActiveTerms,
  shouldResetStructuredQuery,
} from '../queryState';

describe('queryState helpers', () => {
  const keywordGroups: KeywordGroup[] = [
    {
      id: 'preset-group',
      name: 'Presets',
      color: 'red',
      patterns: [{ regex: 'error', comment: 'error preset' }],
      enabled: true,
    },
  ];

  const baseQuery: SearchQuery = {
    id: 'query_1',
    globalOperator: 'OR',
    metadata: {
      createdAt: 1,
      lastModified: 1,
      executionCount: 3,
    },
    terms: [
      {
        id: 'term_1',
        value: 'error|warning',
        operator: 'OR',
        source: 'preset',
        presetGroupId: 'preset-group',
        isRegex: true,
        priority: 1,
        enabled: true,
        caseSensitive: false,
      },
    ],
  };

  it('increments execution count for an existing structured query', () => {
    const next = buildStructuredQueryForSearch('error|warning', baseQuery, keywordGroups);

    expect(next.metadata.executionCount).toBe(4);
    expect(next.terms[0].value).toBe('error|warning');
    expect(next).not.toBe(baseQuery);
  });

  it('builds and persists structured queries for plain input searches', () => {
    const next = buildStructuredQueryForSearch('error', null, keywordGroups);

    expect(next.terms).toHaveLength(1);
    expect(next.terms[0].value).toBe('error');
    expect(next.terms[0].source).toBe('preset');
    expect(next.metadata.executionCount).toBe(1);
  });

  it('derives active terms from structured query without splitting regex alternation', () => {
    expect(deriveActiveTerms('ignored', baseQuery)).toEqual(['error|warning']);
  });

  it('falls back to parsing plain query text when structured query is absent', () => {
    expect(deriveActiveTerms('error|warning', null)).toEqual(['error', 'warning']);
  });

  it('keeps structured query when the visible query still matches it', () => {
    expect(shouldResetStructuredQuery('error|warning', baseQuery)).toBe(false);
  });

  it('resets structured query when the user edits the visible query', () => {
    expect(shouldResetStructuredQuery('error|warning|timeout', baseQuery)).toBe(true);
  });
});
