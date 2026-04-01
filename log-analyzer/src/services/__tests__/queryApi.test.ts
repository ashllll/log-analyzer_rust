/**
 * queryApi 单元测试
 * 测试结构化查询 API 的执行和验证功能
 */

import { invoke } from '@tauri-apps/api/core';
import { api, queryApi } from '../api';

jest.mock('@tauri-apps/api/core', () => ({
  invoke: jest.fn(),
}));

// Mock logger
jest.mock('../../utils/logger', () => ({
  logger: {
    debug: jest.fn(),
    error: jest.fn(),
    info: jest.fn(),
    warn: jest.fn(),
  },
}));

const mockInvoke = invoke as jest.MockedFunction<typeof invoke>;

describe('queryApi', () => {
  const query = {
    id: 'test-query',
    terms: [],
    globalOperator: 'AND' as const,
    metadata: { createdAt: Date.now() },
  };

  beforeEach(() => {
    jest.clearAllMocks();
  });

  describe('executeStructuredQuery', () => {
    it('should return empty array when logs is empty', async () => {
      const result = await api.executeStructuredQuery(query, []);

      expect(result).toEqual([]);
      expect(mockInvoke).not.toHaveBeenCalled();
    });

    it('should call invoke with correct parameters', async () => {
      mockInvoke.mockResolvedValue(['line1', 'line2']);

      const logs = ['log1', 'log2'];
      const result = await api.executeStructuredQuery(query, logs);

      expect(mockInvoke).toHaveBeenCalledWith('execute_structured_query', {
        query,
        logs,
      });
      expect(result).toEqual(['line1', 'line2']);
    });

    it('should return empty array when result is not an array', async () => {
      mockInvoke.mockResolvedValue('not an array');

      const result = await api.executeStructuredQuery(query, ['log1']);

      expect(result).toEqual([]);
    });

    it('should throw error when invoke fails', async () => {
      mockInvoke.mockRejectedValue(new Error('invoke failed'));

      await expect(api.executeStructuredQuery(query, ['log1'])).rejects.toThrow(
        '查询执行失败: invoke failed'
      );
    });
  });

  describe('validateQuery', () => {
    it('should return false when query is null', async () => {
      const result = await api.validateQuery(null as never);

      expect(result).toBe(false);
      expect(mockInvoke).not.toHaveBeenCalled();
    });

    it('should return false when query is undefined', async () => {
      const result = await api.validateQuery(undefined as never);

      expect(result).toBe(false);
      expect(mockInvoke).not.toHaveBeenCalled();
    });

    it('should return false when query is not an object', async () => {
      const result = await api.validateQuery('not an object' as never);

      expect(result).toBe(false);
      expect(mockInvoke).not.toHaveBeenCalled();
    });

    it('should return true when invoke returns true', async () => {
      mockInvoke.mockResolvedValue(true);

      const result = await api.validateQuery(query);

      expect(result).toBe(true);
    });

    it('should return false when invoke returns false', async () => {
      mockInvoke.mockResolvedValue(false);

      const result = await api.validateQuery(query);

      expect(result).toBe(false);
    });

    it('should return false when invoke throws error', async () => {
      mockInvoke.mockRejectedValue(new Error('network error'));

      const result = await api.validateQuery(query);

      expect(result).toBe(false);
    });

    it('should call invoke with correct parameters', async () => {
      mockInvoke.mockResolvedValue(true);

      await api.validateQuery(query);

      expect(mockInvoke).toHaveBeenCalledWith('validate_query', { query });
    });
  });

  describe('queryApi object', () => {
    it('should expose execute and validate methods', async () => {
      mockInvoke.mockResolvedValueOnce(['line1']).mockResolvedValueOnce(true);

      await expect(queryApi.execute(query, ['log1'])).resolves.toEqual(['line1']);
      await expect(queryApi.validate(query)).resolves.toBe(true);
    });
  });
});
