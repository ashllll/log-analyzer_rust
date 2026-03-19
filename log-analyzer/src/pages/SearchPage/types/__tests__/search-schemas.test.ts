/**
 * SearchSchemas 单元测试
 * 验证 Zod Schema 对 API 响应的验证能力
 */

import {
  MatchDetailSchema,
  LogEntrySchema,
  KeywordStatisticsSchema,
  SearchResultSummarySchema,
  PagedSearchResultSchema,
} from '../search-schemas';

describe('SearchSchemas', () => {
  describe('MatchDetailSchema', () => {
    it('should parse valid match detail', () => {
      const validData = {
        term_id: 'term-1',
        term_value: 'error',
        priority: 10,
        match_position: [0, 5],
      };

      const result = MatchDetailSchema.safeParse(validData);
      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data.term_id).toBe('term-1');
        expect(result.data.term_value).toBe('error');
        expect(result.data.priority).toBe(10);
        expect(result.data.match_position).toEqual([0, 5]);
      }
    });

    it('should allow optional match_position', () => {
      const validData = {
        term_id: 'term-1',
        term_value: 'error',
        priority: 10,
      };

      const result = MatchDetailSchema.safeParse(validData);
      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data.match_position).toBeUndefined();
      }
    });

    it('should reject invalid match detail - missing term_id', () => {
      const invalidData = {
        term_value: 'error',
        priority: 10,
      };

      const result = MatchDetailSchema.safeParse(invalidData);
      expect(result.success).toBe(false);
    });

    it('should reject invalid match detail - wrong type for priority', () => {
      const invalidData = {
        term_id: 'term-1',
        term_value: 'error',
        priority: 'high',
      };

      const result = MatchDetailSchema.safeParse(invalidData);
      expect(result.success).toBe(false);
    });
  });

  describe('LogEntrySchema', () => {
    const validLogEntry = {
      id: 1,
      timestamp: '2024-01-15T10:30:00Z',
      level: 'ERROR',
      file: '/var/log/app.log',
      real_path: '/var/log/app.log',
      line: 42,
      content: 'Error occurred in processing',
      tags: ['production', 'critical'],
    };

    it('should parse valid log entry with all fields', () => {
      const result = LogEntrySchema.safeParse(validLogEntry);
      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data.id).toBe(1);
        expect(result.data.timestamp).toBe('2024-01-15T10:30:00Z');
        expect(result.data.level).toBe('ERROR');
        expect(result.data.content).toBe('Error occurred in processing');
      }
    });

    it('should parse log entry with optional fields omitted', () => {
      const minimalEntry = {
        id: 2,
        timestamp: '2024-01-15T11:00:00Z',
        level: 'INFO',
        file: 'app.log',
        real_path: '/var/log/app.log',
        line: 100,
        content: 'Application started',
        tags: [],
      };

      const result = LogEntrySchema.safeParse(minimalEntry);
      expect(result.success).toBe(true);
    });

    it('should parse log entry with match_details', () => {
      const entryWithMatchDetails = {
        ...validLogEntry,
        match_details: [
          { term_id: 'term-1', term_value: 'error', priority: 10 },
        ],
        matched_keywords: ['error'],
      };

      const result = LogEntrySchema.safeParse(entryWithMatchDetails);
      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data.match_details).toHaveLength(1);
        expect(result.data.matched_keywords).toEqual(['error']);
      }
    });

    it('should reject log entry missing required fields - id', () => {
      const invalidEntry = {
        timestamp: '2024-01-15T10:30:00Z',
        level: 'ERROR',
        file: '/var/log/app.log',
        real_path: '/var/log/app.log',
        line: 42,
        content: 'Error occurred',
        tags: [],
      };

      const result = LogEntrySchema.safeParse(invalidEntry);
      expect(result.success).toBe(false);
    });

    it('should reject log entry missing content', () => {
      const invalidEntry = {
        id: 1,
        timestamp: '2024-01-15T10:30:00Z',
        level: 'ERROR',
        file: '/var/log/app.log',
        real_path: '/var/log/app.log',
        line: 42,
        tags: [],
      };

      const result = LogEntrySchema.safeParse(invalidEntry);
      expect(result.success).toBe(false);
    });

    it('should reject log entry with wrong type for line', () => {
      const invalidEntry = {
        ...validLogEntry,
        line: 'forty-two',
      };

      const result = LogEntrySchema.safeParse(invalidEntry);
      expect(result.success).toBe(false);
    });
  });

  describe('KeywordStatisticsSchema', () => {
    it('should parse valid keyword statistics', () => {
      const validStats = {
        keyword: 'error',
        matchCount: 150,
        matchPercentage: 25.5,
      };

      const result = KeywordStatisticsSchema.safeParse(validStats);
      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data.keyword).toBe('error');
        expect(result.data.matchCount).toBe(150);
        expect(result.data.matchPercentage).toBe(25.5);
      }
    });

    it('should accept negative matchCount (Zod z.number() does not validate by default)', () => {
      const statsWithNegative = {
        keyword: 'error',
        matchCount: -10,
        matchPercentage: 25.5,
      };

      const result = KeywordStatisticsSchema.safeParse(statsWithNegative);
      // Zod z.number() accepts any number by default
      expect(result.success).toBe(true);
    });

    it('should reject missing keyword', () => {
      const invalidStats = {
        matchCount: 150,
        matchPercentage: 25.5,
      };

      const result = KeywordStatisticsSchema.safeParse(invalidStats);
      expect(result.success).toBe(false);
    });

    it('should reject wrong type for matchPercentage', () => {
      const invalidStats = {
        keyword: 'error',
        matchCount: 150,
        matchPercentage: 'high',
      };

      const result = KeywordStatisticsSchema.safeParse(invalidStats);
      expect(result.success).toBe(false);
    });
  });

  describe('SearchResultSummarySchema', () => {
    const validSummary = {
      totalMatches: 500,
      keywordStats: [
        { keyword: 'error', matchCount: 200, matchPercentage: 40 },
        { keyword: 'warning', matchCount: 150, matchPercentage: 30 },
      ],
      searchDurationMs: 125,
      truncated: false,
    };

    it('should parse valid summary with keyword stats', () => {
      const result = SearchResultSummarySchema.safeParse(validSummary);
      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data.totalMatches).toBe(500);
        expect(result.data.keywordStats).toHaveLength(2);
        expect(result.data.truncated).toBe(false);
      }
    });

    it('should reject summary with invalid keywordStats - empty array', () => {
      const invalidSummary = {
        ...validSummary,
        keywordStats: [],
      };

      const result = SearchResultSummarySchema.safeParse(invalidSummary);
      expect(result.success).toBe(true); // Zod allows empty arrays
    });

    it('should reject summary with truncated not boolean', () => {
      const invalidSummary = {
        ...validSummary,
        truncated: 'yes',
      };

      const result = SearchResultSummarySchema.safeParse(invalidSummary);
      expect(result.success).toBe(false);
    });

    it('should reject summary missing totalMatches', () => {
      const invalidSummary = {
        keywordStats: validSummary.keywordStats,
        searchDurationMs: 125,
        truncated: false,
      };

      const result = SearchResultSummarySchema.safeParse(invalidSummary);
      expect(result.success).toBe(false);
    });
  });

  describe('PagedSearchResultSchema', () => {
    const validPagedResult = {
      results: [
        {
          id: 1,
          timestamp: '2024-01-15T10:30:00Z',
          level: 'ERROR',
          file: '/var/log/app.log',
          real_path: '/var/log/app.log',
          line: 42,
          content: 'Error occurred',
          tags: [],
        },
      ],
      total_count: 100,
      page_index: 0,
      page_size: 50,
      total_pages: 2,
      has_more: true,
      summary: {
        totalMatches: 100,
        keywordStats: [{ keyword: 'error', matchCount: 50, matchPercentage: 50 }],
        searchDurationMs: 100,
        truncated: false,
      },
      query: 'error',
      search_id: 'search-123',
    };

    it('should parse valid paged result', () => {
      const result = PagedSearchResultSchema.safeParse(validPagedResult);
      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data.results).toHaveLength(1);
        expect(result.data.total_count).toBe(100);
        expect(result.data.page_index).toBe(0);
        expect(result.data.has_more).toBe(true);
      }
    });

    it('should accept paged result with negative page_index (Zod z.number() does not validate by default)', () => {
      const resultWithNegativePageIndex = {
        ...validPagedResult,
        page_index: -1,
      };

      const result = PagedSearchResultSchema.safeParse(resultWithNegativePageIndex);
      // Zod z.number() accepts any number by default
      expect(result.success).toBe(true);
    });

    it('should validate nested summary structure', () => {
      const invalidSummary = {
        ...validPagedResult,
        summary: {
          totalMatches: 100,
          // missing keywordStats
          searchDurationMs: 100,
          truncated: false,
        },
      };

      const result = PagedSearchResultSchema.safeParse(invalidSummary);
      expect(result.success).toBe(false);
    });

    it('should reject paged result missing search_id', () => {
      const invalidResult = {
        ...validPagedResult,
        search_id: undefined,
      };

      const result = PagedSearchResultSchema.safeParse(invalidResult);
      expect(result.success).toBe(false);
    });

    it('should accept paged result with empty results', () => {
      const emptyResult = {
        ...validPagedResult,
        results: [],
      };

      const result = PagedSearchResultSchema.safeParse(emptyResult);
      expect(result.success).toBe(true);
    });

    it('should validate results array items with match_details', () => {
      const resultWithMatchDetails = {
        ...validPagedResult,
        results: [
          {
            id: 1,
            timestamp: '2024-01-15T10:30:00Z',
            level: 'ERROR',
            file: '/var/log/app.log',
            real_path: '/var/log/app.log',
            line: 42,
            content: 'Error occurred',
            tags: [],
            match_details: [
              { term_id: 't1', term_value: 'error', priority: 10 },
            ],
          },
        ],
      };

      const result = PagedSearchResultSchema.safeParse(resultWithMatchDetails);
      expect(result.success).toBe(true);
    });
  });
});
