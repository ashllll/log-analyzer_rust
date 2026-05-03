import { saveQuery, loadQuery, clearQuery } from '../queryStorage';
import type { SearchQuery } from '../../types/search';

const STORAGE_KEY = 'log_analyzer_current_query';

describe('queryStorage', () => {
  let storage: Record<string, string> = {};

  beforeEach(() => {
    storage = {};
    Object.defineProperty(window, 'localStorage', {
      value: {
        setItem: jest.fn((key: string, value: string) => {
          storage[key] = value;
        }),
        getItem: jest.fn((key: string) => storage[key] || null),
        removeItem: jest.fn((key: string) => {
          delete storage[key];
        }),
      },
      writable: true,
    });
    jest.spyOn(console, 'error').mockImplementation(() => {});
  });

  afterEach(() => {
    jest.restoreAllMocks();
  });

  describe('saveQuery', () => {
    it('应将查询对象序列化后写入 localStorage', () => {
      const query: SearchQuery = {
        id: 'q-1',
        terms: [],
        globalOperator: 'AND',
        metadata: { createdAt: 0, lastModified: 0, executionCount: 0 },
      };
      saveQuery(query);
      expect(window.localStorage.setItem).toHaveBeenCalledWith(
        STORAGE_KEY,
        JSON.stringify(query)
      );
    });

    it('应在 localStorage 写入异常时静默失败并调用 console.error', () => {
      (window.localStorage.setItem as jest.Mock).mockImplementation(() => {
        throw new Error('Quota exceeded');
      });
      const query: SearchQuery = {
        id: 'q-2',
        terms: [],
        globalOperator: 'OR',
        metadata: { createdAt: 0, lastModified: 0, executionCount: 0 },
      };
      expect(() => saveQuery(query)).not.toThrow();
      expect(console.error).toHaveBeenCalledWith(
        'Failed to save query:',
        expect.any(Error)
      );
    });
  });

  describe('loadQuery', () => {
    it('应从 localStorage 反序列化并返回查询对象', () => {
      const query: SearchQuery = {
        id: 'q-3',
        terms: [
          {
            id: 't-1',
            value: 'error',
            operator: 'AND',
            source: 'user',
            isRegex: false,
            priority: 1,
            enabled: true,
            caseSensitive: false,
          },
        ],
        globalOperator: 'AND',
        metadata: { createdAt: 0, lastModified: 0, executionCount: 0 },
      };
      storage[STORAGE_KEY] = JSON.stringify(query);
      const result = loadQuery();
      expect(result).toEqual(query);
    });

    it('应在无存储数据时返回 null', () => {
      const result = loadQuery();
      expect(result).toBeNull();
    });

    it('应在存储数据为非法 JSON 时返回 null 且不抛异常', () => {
      storage[STORAGE_KEY] = '{ invalid json';
      const result = loadQuery();
      expect(result).toBeNull();
      expect(console.error).toHaveBeenCalledWith(
        'Failed to load query:',
        expect.any(SyntaxError)
      );
    });

    it('应在 getItem 抛出异常时返回 null', () => {
      (window.localStorage.getItem as jest.Mock).mockImplementation(() => {
        throw new Error('Storage disabled');
      });
      const result = loadQuery();
      expect(result).toBeNull();
    });
  });

  describe('clearQuery', () => {
    it('应从 localStorage 移除指定 key', () => {
      storage[STORAGE_KEY] = '{}';
      clearQuery();
      expect(window.localStorage.removeItem).toHaveBeenCalledWith(STORAGE_KEY);
      expect(storage[STORAGE_KEY]).toBeUndefined();
    });
  });
});
