/**
 * queryApi 单元测试
 * 测试结构化查询 API 的执行和验证功能
 */

import { executeStructuredQuery, validateQuery, queryApi } from '../queryApi';
import * as nullSafeApi from '../nullSafeApi';

// Mock logger
jest.mock('../../utils/logger', () => ({
  logger: {
    debug: jest.fn(),
    error: jest.fn(),
    info: jest.fn(),
    warn: jest.fn(),
  },
}));

// Mock safeInvoke
jest.mock('../nullSafeApi', () => ({
  safeInvoke: jest.fn(),
  isEmptyArray: jest.fn(),
}));

describe('queryApi', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  describe('executeStructuredQuery', () => {
    it('should return empty array when logs is empty', async () => {
      (nullSafeApi.isEmptyArray as jest.Mock).mockReturnValue(true);

      const result = await executeStructuredQuery(
        { terms: [], globalOperator: 'AND', id: 'test', metadata: { createdAt: Date.now() } },
        []
      );

      expect(result).toEqual([]);
    });

    it('should call safeInvoke with correct parameters', async () => {
      (nullSafeApi.isEmptyArray as jest.Mock).mockReturnValue(false);
      (nullSafeApi.safeInvoke as jest.Mock).mockResolvedValue(['line1', 'line2']);

      const query = {
        id: 'test-query',
        terms: [],
        globalOperator: 'AND',
        metadata: { createdAt: Date.now() }
      };
      const logs = ['log1', 'log2'];

      const result = await executeStructuredQuery(query, logs);

      expect(nullSafeApi.safeInvoke).toHaveBeenCalledWith(
        'execute_structured_query',
        { query, logs },
        { timeoutMs: 30000 }
      );
      expect(result).toEqual(['line1', 'line2']);
    });

    it('should return empty array when result is not an array', async () => {
      (nullSafeApi.isEmptyArray as jest.Mock).mockReturnValue(false);
      (nullSafeApi.safeInvoke as jest.Mock).mockResolvedValue('not an array');

      const result = await executeStructuredQuery(
        { terms: [], globalOperator: 'AND', id: 'test', metadata: { createdAt: Date.now() } },
        ['log1']
      );

      expect(result).toEqual([]);
    });

    it('should throw error when safeInvoke fails', async () => {
      (nullSafeApi.isEmptyArray as jest.Mock).mockReturnValue(false);
      (nullSafeApi.safeInvoke as jest.Mock).mockRejectedValue(new Error('invoke failed'));

      await expect(
        executeStructuredQuery(
          { terms: [], globalOperator: 'AND', id: 'test', metadata: { createdAt: Date.now() } },
          ['log1']
        )
      ).rejects.toThrow('查询执行失败');
    });
  });

  describe('validateQuery', () => {
    it('should return false when query is null', async () => {
      const result = await validateQuery(null as any);
      expect(result).toBe(false);
    });

    it('should return false when query is undefined', async () => {
      const result = await validateQuery(undefined as any);
      expect(result).toBe(false);
    });

    it('should return false when query is not an object', async () => {
      const result = await validateQuery('not an object' as any);
      expect(result).toBe(false);
    });

    it('should return true when safeInvoke returns true', async () => {
      (nullSafeApi.safeInvoke as jest.Mock).mockResolvedValue(true);

      const result = await validateQuery({
        id: 'test',
        terms: [],
        globalOperator: 'AND',
        metadata: { createdAt: Date.now() }
      });

      expect(result).toBe(true);
    });

    it('should return false when safeInvoke returns false', async () => {
      (nullSafeApi.safeInvoke as jest.Mock).mockResolvedValue(false);

      const result = await validateQuery({
        id: 'test',
        terms: [],
        globalOperator: 'AND',
        metadata: { createdAt: Date.now() }
      });

      expect(result).toBe(false);
    });

    it('should return false when safeInvoke throws error', async () => {
      (nullSafeApi.safeInvoke as jest.Mock).mockRejectedValue(new Error('network error'));

      const result = await validateQuery({
        id: 'test',
        terms: [],
        globalOperator: 'AND',
        metadata: { createdAt: Date.now() }
      });

      expect(result).toBe(false);
    });

    it('should call safeInvoke with correct parameters', async () => {
      (nullSafeApi.safeInvoke as jest.Mock).mockResolvedValue(true);

      const query = {
        id: 'test-query',
        terms: [],
        globalOperator: 'AND',
        metadata: { createdAt: Date.now() }
      };

      await validateQuery(query);

      expect(nullSafeApi.safeInvoke).toHaveBeenCalledWith(
        'validate_query',
        { query },
        { timeoutMs: 5000, fallback: false }
      );
    });
  });

  describe('queryApi object', () => {
    it('should have execute and validate methods', () => {
      expect(queryApi.execute).toBe(executeStructuredQuery);
      expect(queryApi.validate).toBe(validateQuery);
    });
  });
});
