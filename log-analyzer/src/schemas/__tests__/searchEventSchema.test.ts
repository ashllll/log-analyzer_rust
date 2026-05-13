import {
  SearchProgressEventSchema,
  SearchCompleteEventSchema,
  SearchErrorEventSchema,
  SearchIdEventSchema,
  SearchSummaryEventSchema,
} from '../searchEventSchema';

describe('SearchProgressEventSchema', () => {
  it('应正确解析有效进度事件', () => {
    const data = { search_id: 'test-123', count: 42 };
    const result = SearchProgressEventSchema.safeParse(data);
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.search_id).toBe('test-123');
      expect(result.data.count).toBe(42);
    }
  });

  it('应正确解析不包含可选字段的最小进度事件', () => {
    const data = { search_id: 'test-456', count: 0 };
    const result = SearchProgressEventSchema.safeParse(data);
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.search_id).toBe('test-456');
      expect(result.data.count).toBe(0);
    }
  });

  it('应在缺少 search_id 时拒绝解析', () => {
    const data = { count: 10 };
    const result = SearchProgressEventSchema.safeParse(data);
    expect(result.success).toBe(false);
  });

  it('应在 search_id 类型错误时拒绝解析', () => {
    const data = { search_id: 123, count: 10 };
    const result = SearchProgressEventSchema.safeParse(data);
    expect(result.success).toBe(false);
  });

  it('应在 count 为负数时仍接受（z.number 不限制范围）', () => {
    const data = { search_id: 'test', count: -1 };
    const result = SearchProgressEventSchema.safeParse(data);
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.count).toBe(-1);
    }
  });
});

describe('SearchSummaryEventSchema', () => {
  it('应正确解析带 search_id 的摘要事件', () => {
    const data = {
      search_id: 'summary-123',
      summary: {
        totalMatches: 10,
        keywordStats: [{ keyword: 'error', matchCount: 10, matchPercentage: 100 }],
        searchDurationMs: 25,
        truncated: false,
      },
    };
    const result = SearchSummaryEventSchema.safeParse(data);
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.search_id).toBe('summary-123');
      expect(result.data.summary.totalMatches).toBe(10);
    }
  });
});

describe('SearchIdEventSchema', () => {
  it('应正确解析仅包含 search_id 的终端事件', () => {
    const result = SearchIdEventSchema.safeParse({ search_id: 'search-1' });
    expect(result.success).toBe(true);
  });
});

describe('SearchCompleteEventSchema', () => {
  it('应正确解析有效的完成事件', () => {
    const data = { search_id: 'complete-123', total_count: 1000 };
    const result = SearchCompleteEventSchema.safeParse(data);
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.search_id).toBe('complete-123');
      expect(result.data.total_count).toBe(1000);
    }
  });

  it('应在 total_count 类型错误时拒绝解析', () => {
    const data = { search_id: 'complete-123', total_count: 'not-a-number' };
    const result = SearchCompleteEventSchema.safeParse(data);
    expect(result.success).toBe(false);
  });

  it('应在缺少 search_id 时拒绝解析', () => {
    const data = { total_count: 100 };
    const result = SearchCompleteEventSchema.safeParse(data);
    expect(result.success).toBe(false);
  });
});

describe('SearchErrorEventSchema', () => {
  it('应正确解析有效的错误事件', () => {
    const data = { search_id: 'error-123', error: '磁盘空间不足' };
    const result = SearchErrorEventSchema.safeParse(data);
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.search_id).toBe('error-123');
      expect(result.data.error).toBe('磁盘空间不足');
    }
  });

  it('应在缺少 error 字段时拒绝解析', () => {
    const data = { search_id: 'error-123' };
    const result = SearchErrorEventSchema.safeParse(data);
    expect(result.success).toBe(false);
  });

  it('应在缺少 search_id 时拒绝解析', () => {
    const data = { error: 'some error' };
    const result = SearchErrorEventSchema.safeParse(data);
    expect(result.success).toBe(false);
  });
});
